use std::time::{Duration, Instant};

use datadog_cli::config::{Config, Credentials, Defaults, Network};
use datadog_cli::datadog::{DatadogClient, SearchParams};
use datadog_cli::error::DatadogError;
use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client_for(server: &MockServer, max_retries: u32) -> DatadogClient {
    let config = Config {
        credentials: Credentials::Keys {
            api_key: "test-api-key".to_string(),
            app_key: "test-app-key".to_string(),
        },
        site: "datadoghq.com".to_string(),
        defaults: Defaults::default(),
        network: Network {
            timeout_secs: 5,
            max_retries,
            ..Network::default()
        },
    };

    DatadogClient::with_base_url(&config, server.uri()).unwrap()
}

#[tokio::test]
async fn search_logs_sends_auth_headers_and_body() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/logs/events/search"))
        .and(header("DD-API-KEY", "test-api-key"))
        .and(header("DD-APPLICATION-KEY", "test-app-key"))
        .and(body_partial_json(json!({
            "filter": {
                "query": "service:web",
                "from": "2026-07-10T00:00:00Z",
                "to": "2026-07-10T01:00:00Z"
            },
            "page": { "limit": 25 }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{
                "id": "log-1",
                "type": "log",
                "attributes": {
                    "timestamp": "2026-07-10T00:00:00Z",
                    "message": "hello",
                    "service": "web",
                    "status": "info",
                    "tags": ["env:prod"]
                }
            }],
            "meta": { "page": { "after": "cursor-abc" } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let response = client
        .search_logs(
            &SearchParams {
                query: "service:web",
                from: "2026-07-10T00:00:00Z",
                to: "2026-07-10T01:00:00Z",
                limit: 25,
                cursor: None,
                sort: None,
            },
            None,
        )
        .await
        .unwrap();

    let data = response.data.unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0].id, "log-1");
    assert_eq!(
        response.meta.unwrap().page.unwrap().after.as_deref(),
        Some("cursor-abc")
    );
}

#[tokio::test]
async fn query_metrics_sends_query_params() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/query"))
        .and(query_param("query", "avg:system.cpu.user{*}"))
        .and(query_param("from", "1000"))
        .and(query_param("to", "2000"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "ok",
            "series": [],
            "query": "avg:system.cpu.user{*}"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let response = client
        .query_metrics("avg:system.cpu.user{*}", 1000, 2000)
        .await
        .unwrap();

    assert_eq!(response.status, "ok");
    assert!(response.series.is_empty());
}

#[tokio::test]
async fn unauthorized_fails_immediately_without_retry() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/monitor/42"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({"errors": ["Unauthorized"]})))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let started = Instant::now();
    let error = client.get_monitor(42).await.unwrap_err();

    assert!(matches!(error, DatadogError::AuthError(_)));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[tokio::test]
async fn bad_request_fails_immediately_without_retry() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/query"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(json!({"errors": ["Invalid query"]})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let error = client.query_metrics("bad{query", 0, 1).await.unwrap_err();

    match error {
        DatadogError::ApiError { status, message } => {
            assert_eq!(status, 400);
            assert!(message.contains("Invalid query"));
        }
        other => panic!("expected ApiError, got {other:?}"),
    }
}

#[tokio::test]
async fn decode_error_fails_immediately_without_retry() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"series": "not-a-list"})))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let error = client.query_metrics("q", 0, 1).await.unwrap_err();

    assert!(matches!(error, DatadogError::DecodeError(_)));
}

#[tokio::test]
async fn server_error_is_retried_until_recovery() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/monitor/7"))
        .respond_with(ResponseTemplate::new(503).set_body_string("try later"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v1/monitor/7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 7,
            "name": "cpu high",
            "type": "metric alert",
            "query": "avg:cpu > 90",
            "tags": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 1);
    let monitor = client.get_monitor(7).await.unwrap();

    assert_eq!(monitor.id, 7);
    assert_eq!(monitor.name, "cpu high");
}

#[tokio::test]
async fn server_error_surfaces_after_retry_budget() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/monitor/7"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .expect(2)
        .mount(&server)
        .await;

    let client = client_for(&server, 1);
    let error = client.get_monitor(7).await.unwrap_err();

    match error {
        DatadogError::ApiError { status, .. } => assert_eq!(status, 500),
        other => panic!("expected ApiError, got {other:?}"),
    }
}

#[tokio::test]
async fn rate_limit_waits_for_server_reset_then_recovers() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/monitor/9"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-ratelimit-reset", "1")
                .set_body_string("slow down"),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v1/monitor/9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 9,
            "name": "ok",
            "type": "metric alert",
            "query": "q",
            "tags": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 1);
    let started = Instant::now();
    let monitor = client.get_monitor(9).await.unwrap();

    assert_eq!(monitor.id, 9);
    assert!(started.elapsed() >= Duration::from_millis(900));
}

#[tokio::test]
async fn rate_limit_with_distant_reset_fails_fast() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/monitor/9"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-ratelimit-reset", "3600")
                .set_body_string("slow down"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let started = Instant::now();
    let error = client.get_monitor(9).await.unwrap_err();

    match error {
        DatadogError::RateLimitError { reset_secs } => assert_eq!(reset_secs, Some(3600)),
        other => panic!("expected RateLimitError, got {other:?}"),
    }
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[tokio::test]
async fn connection_error_is_retried() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let uri = format!("http://{}", listener.local_addr().unwrap());
    let connections = tokio::spawn(async move {
        for _ in 0..2 {
            let (stream, _) = listener.accept().await.unwrap();
            drop(stream);
        }
    });

    let config = Config {
        credentials: Credentials::Keys {
            api_key: "k".to_string(),
            app_key: "a".to_string(),
        },
        site: "datadoghq.com".to_string(),
        defaults: Defaults::default(),
        network: Network {
            timeout_secs: 5,
            max_retries: 1,
            ..Network::default()
        },
    };
    let client = DatadogClient::with_base_url(&config, uri).unwrap();

    let error = client.get_monitor(1).await.unwrap_err();

    assert!(
        matches!(
            error,
            DatadogError::NetworkError(_) | DatadogError::TimeoutError
        ),
        "unexpected error: {error:?}"
    );
    tokio::time::timeout(Duration::from_secs(5), connections)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn search_events_uses_flat_body() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/events/search"))
        .and(body_partial_json(json!({
            "filter": { "query": "source:alert", "from": "2026-07-10T00:00:00+00:00" },
            "page": { "limit": 5 }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{
                "id": "evt-1",
                "type": "event",
                "attributes": { "timestamp": 1783641600000i64, "message": "deploy", "tags": ["env:prod"] }
            }],
            "meta": { "page": { "after": "evt-cursor" } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let response = client
        .search_events(&SearchParams {
            query: "source:alert",
            from: "2026-07-10T00:00:00+00:00",
            to: "2026-07-10T01:00:00+00:00",
            limit: 5,
            cursor: None,
            sort: None,
        })
        .await
        .unwrap();

    let data = response.data.unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0].id, "evt-1");
    assert_eq!(
        response.meta.unwrap().page.unwrap().after.as_deref(),
        Some("evt-cursor")
    );
}

#[tokio::test]
async fn search_spans_uses_json_api_envelope() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/spans/events/search"))
        .and(body_partial_json(json!({
            "data": {
                "type": "search_request",
                "attributes": {
                    "filter": {
                        "query": "service:api",
                        "from": "2026-07-10T00:00:00+00:00",
                        "to": "2026-07-10T01:00:00+00:00"
                    },
                    "page": { "limit": 10 }
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{ "id": "span-1", "type": "spans", "attributes": { "service": "api" } }],
            "meta": { "page": { "after": "span-cursor" } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let response = client
        .search_spans(&SearchParams {
            query: "service:api",
            from: "2026-07-10T00:00:00+00:00",
            to: "2026-07-10T01:00:00+00:00",
            limit: 10,
            cursor: None,
            sort: None,
        })
        .await
        .unwrap();

    assert_eq!(response["data"][0]["id"], "span-1");
    assert_eq!(response["meta"]["page"]["after"], "span-cursor");
}

#[tokio::test]
async fn search_logs_sends_storage_tier() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/logs/events/search"))
        .and(body_partial_json(json!({
            "filter": { "query": "*", "storage_tier": "flex" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": [] })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let response = client
        .search_logs(
            &SearchParams {
                query: "*",
                from: "2026-07-10T00:00:00+00:00",
                to: "2026-07-10T01:00:00+00:00",
                limit: 10,
                cursor: None,
                sort: None,
            },
            Some("flex"),
        )
        .await
        .unwrap();

    assert_eq!(response.data.map(|d| d.len()), Some(0));
}

#[tokio::test]
async fn query_timeseries_sends_envelope_with_millis_and_formulas() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/query/timeseries"))
        .and(body_partial_json(json!({
            "data": {
                "type": "timeseries_request",
                "attributes": {
                    "from": 1000000i64,
                    "to": 2000000i64,
                    "queries": [
                        { "data_source": "metrics", "query": "sum:errors{*}", "name": "a" },
                        { "data_source": "metrics", "query": "sum:hits{*}", "name": "b" }
                    ],
                    "formulas": [ { "formula": "a / b * 100" } ]
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "type": "timeseries_response",
                "attributes": {
                    "series": [{ "group_tags": [], "query_index": 0 }],
                    "times": [1000000i64],
                    "values": [[42.0]]
                }
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let response = client
        .query_timeseries(
            &["sum:errors{*}".to_string(), "sum:hits{*}".to_string()],
            &["a / b * 100".to_string()],
            1_000_000,
            2_000_000,
            None,
        )
        .await
        .unwrap();

    assert_eq!(response["data"]["attributes"]["values"][0][0], 42.0);
}

#[tokio::test]
async fn list_catalog_entities_sends_offset_pagination() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v2/catalog/entity"))
        .and(query_param("filter[kind]", "service"))
        .and(query_param("page[offset]", "20"))
        .and(query_param("page[limit]", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{ "id": "entity-1", "type": "entity" }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let response = client
        .list_catalog_entities("service", None, None, None, 20, 10)
        .await
        .unwrap();

    assert_eq!(response["data"][0]["id"], "entity-1");
}

#[tokio::test]
async fn search_llm_obs_spans_uses_spans_envelope_and_vnd_content_type() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/llm-obs/v1/spans/events/search"))
        .and(header("content-type", "application/vnd.api+json"))
        .and(body_partial_json(json!({
            "data": {
                "type": "spans",
                "attributes": {
                    "filter": { "query": "@ml_app:chatbot", "ml_app": "chatbot" },
                    "page": { "limit": 50 }
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{ "id": "llm-span-1", "type": "spans" }],
            "meta": { "page": { "after": "llm-cursor" } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let response = client
        .search_llm_obs_spans(
            &SearchParams {
                query: "@ml_app:chatbot",
                from: "2026-07-10T00:00:00+00:00",
                to: "2026-07-10T01:00:00+00:00",
                limit: 50,
                cursor: None,
                sort: None,
            },
            Some("chatbot"),
            None,
        )
        .await
        .unwrap();

    assert_eq!(response["data"][0]["id"], "llm-span-1");
    assert_eq!(response["meta"]["page"]["after"], "llm-cursor");
}

#[tokio::test]
async fn search_error_issues_sends_track_and_epoch_millis() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v2/error-tracking/issues/search"))
        .and(body_partial_json(json!({
            "data": {
                "type": "search_request",
                "attributes": {
                    "query": "service:api",
                    "track": "trace",
                    "from": 1000000i64,
                    "to": 2000000i64
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{ "id": "issue-1", "type": "error_tracking_issue" }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let response = client
        .search_error_issues("service:api", "trace", 1_000_000, 2_000_000, None)
        .await
        .unwrap();

    assert_eq!(response["data"][0]["id"], "issue-1");
}

#[tokio::test]
async fn list_teams_sends_page_number_params() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v2/team"))
        .and(query_param("page[number]", "2"))
        .and(query_param("page[size]", "50"))
        .and(query_param("filter[keyword]", "platform"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{ "id": "team-1", "type": "team" }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 3);
    let response = client
        .list_teams(Some("platform"), false, 2, 50)
        .await
        .unwrap();

    assert_eq!(response["data"][0]["id"], "team-1");
}

#[tokio::test]
async fn token_credentials_send_bearer_header() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/monitor/1"))
        .and(header("authorization", "Bearer ddpat_test_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 1,
            "name": "m",
            "type": "metric alert",
            "query": "q",
            "tags": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = Config {
        credentials: Credentials::Token("ddpat_test_token".to_string()),
        site: "datadoghq.com".to_string(),
        defaults: Defaults::default(),
        network: Network::default(),
    };
    let client = DatadogClient::with_base_url(&config, server.uri()).unwrap();

    let monitor = client.get_monitor(1).await.unwrap();
    assert_eq!(monitor.id, 1);
}

#[tokio::test]
async fn rate_limit_without_reset_header_uses_backoff() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/monitor/11"))
        .respond_with(ResponseTemplate::new(429).set_body_string("slow down"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v1/monitor/11"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 11,
            "name": "ok",
            "type": "metric alert",
            "query": "q",
            "tags": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, 1);
    let started = Instant::now();
    let monitor = client.get_monitor(11).await.unwrap();

    assert_eq!(monitor.id, 11);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn redirects_do_not_forward_credentials() {
    let origin = MockServer::start().await;
    let destination = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&destination)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", destination.uri()))
        .expect(1)
        .mount(&origin)
        .await;
    let error = client_for(&origin, 3).get_monitor(1).await.unwrap_err();
    assert!(matches!(error, DatadogError::ApiError { status: 302, .. }));
}

#[tokio::test]
async fn total_timeout_includes_rate_limit_wait() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "30"))
        .expect(1)
        .mount(&server)
        .await;
    let config = Config {
        credentials: Credentials::Token("token".into()),
        site: "datadoghq.com".into(),
        defaults: Defaults::default(),
        network: Network {
            total_timeout_secs: 1,
            ..Network::default()
        },
    };
    let client = DatadogClient::with_base_url(&config, server.uri()).unwrap();
    let started = Instant::now();
    assert!(matches!(
        client.get_monitor(1).await.unwrap_err(),
        DatadogError::TimeoutError
    ));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn retry_after_seconds_are_preserved() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "60"))
        .expect(1)
        .mount(&server)
        .await;
    let error = client_for(&server, 3).get_monitor(1).await.unwrap_err();
    assert!(matches!(
        error,
        DatadogError::RateLimitError {
            reset_secs: Some(60)
        }
    ));
}

#[tokio::test]
async fn search_handlers_preserve_documented_record_and_cursor_shapes() {
    use clap::Parser;
    use datadog_cli::cli::{Cli, Command, LogsAction};
    use datadog_cli::handlers::{logs::LogsHandler, spans::SpansHandler};

    let server = MockServer::start().await;
    Mock::given(path("/api/v2/logs/events/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{ "id": "log-1", "attributes": {"service": "api", "message": "failed"} }],
            "meta": { "page": { "after": "logs-next" } }
        })))
        .mount(&server)
        .await;
    Mock::given(path("/api/v2/spans/events/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{ "id": "span-1", "type": "spans", "attributes": {"service": "api", "custom": {"team": "platform"}} }],
            "meta": { "page": { "after": "spans-next" } }
        })))
        .mount(&server).await;
    let client = client_for(&server, 0);
    let cli = Cli::try_parse_from([
        "datadog-cli",
        "logs",
        "search",
        "service:api",
        "--format",
        "json",
    ])
    .unwrap();
    let Command::Logs {
        action: LogsAction::Search(args),
    } = cli.command
    else {
        panic!("expected logs search")
    };
    let logs = LogsHandler::search(&client, &args).await.unwrap();
    assert_eq!(logs["data"][0]["service"], "api");
    assert_eq!(logs["data"][0]["message"], "failed");
    assert_eq!(logs["pagination"]["next_cursor"], "logs-next");

    let cli =
        Cli::try_parse_from(["datadog-cli", "--format", "jsonl", "spans", "service:api"]).unwrap();
    let Command::Spans(args) = cli.command else {
        panic!("expected spans")
    };
    let spans = SpansHandler::list(&client, &args).await.unwrap();
    assert_eq!(spans["data"][0]["attributes"]["service"], "api");
    assert_eq!(spans["data"][0]["attributes"]["custom"]["team"], "platform");
    assert_eq!(spans["pagination"]["next_cursor"], "spans-next");
}

#[tokio::test]
async fn timeseries_interval_rejects_overflow_before_http() {
    use clap::Parser;
    use datadog_cli::cli::{Cli, Command};
    use datadog_cli::handlers::metrics::MetricsHandler;

    assert!(
        Cli::try_parse_from([
            "datadog-cli",
            "timeseries",
            "q",
            "--interval",
            "9223372036854775807"
        ])
        .is_err()
    );
    let cli = Cli::try_parse_from(["datadog-cli", "timeseries", "q", "--interval", "1"]).unwrap();
    let Command::Timeseries(mut args) = cli.command else {
        panic!("expected timeseries")
    };
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let client = client_for(&server, 0);
    for interval in [0, -1, i64::MAX] {
        args.interval = Some(interval);
        assert!(matches!(
            MetricsHandler::timeseries(&client, &args)
                .await
                .unwrap_err(),
            DatadogError::InvalidInput(_)
        ));
    }
}

#[tokio::test]
async fn response_body_timeouts_are_retried_and_remain_timeout_errors() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let uri = format!("http://{}", listener.local_addr().unwrap());
    let requests = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\n\r\n{\"data\":").await.unwrap();
            let mut buffer = [0; 1024];
            while matches!(stream.read(&mut buffer).await, Ok(size) if size > 0) {}
        }
    });
    let config = Config {
        credentials: Credentials::Token("fixture-token".into()),
        site: "datadoghq.com".into(),
        defaults: Defaults::default(),
        network: Network {
            timeout_secs: 1,
            max_retries: 1,
            total_timeout_secs: 10,
        },
    };
    let client = DatadogClient::with_base_url(&config, uri).unwrap();
    assert!(matches!(
        client.get_monitor(1).await.unwrap_err(),
        DatadogError::TimeoutError
    ));
    tokio::time::timeout(Duration::from_secs(5), requests)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn metrics_preserves_complete_query_expressions() {
    use clap::Parser;
    use datadog_cli::cli::{Cli, Command};
    use datadog_cli::handlers::metrics::MetricsHandler;

    for query in [
        "avg:system.cpu.user{env:prod,service:api},avg:system.cpu.system{*}.rollup(avg, 3600)",
        "(max:system.cpu.user{*}.rollup(max, 60) + min:system.cpu.system{*}) * 2",
    ] {
        let server = MockServer::start().await;
        Mock::given(path("/api/v1/query"))
            .and(query_param("query", query))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "ok", "series": [], "query": query
            })))
            .expect(1)
            .mount(&server)
            .await;
        let cli = Cli::try_parse_from(["datadog-cli", "metrics", query]).unwrap();
        let Command::Metrics(args) = cli.command else {
            panic!("expected metrics")
        };
        let result = MetricsHandler::query(&client_for(&server, 0), &args)
            .await
            .unwrap();
        assert_eq!(result["meta"]["query"], query);
        assert!(result["meta"].get("rollup_applied").is_none());
    }
    let error = Cli::try_parse_from(["datadog-cli", "metrics", "q", "--max-points", "2"])
        .err()
        .expect("removed option must fail");
    assert_eq!(error.exit_code(), 2);
}

#[tokio::test]
async fn service_unavailable_respects_retry_after_without_changing_error_class() {
    for value in [
        "60".to_string(),
        httpdate::fmt_http_date(std::time::SystemTime::now() + Duration::from_secs(120)),
    ] {
        let server = MockServer::start().await;
        Mock::given(path("/api/v1/monitor/7"))
            .respond_with(
                ResponseTemplate::new(503)
                    .insert_header("Retry-After", value.as_str())
                    .set_body_string("maintenance"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = client_for(&server, 3).get_monitor(7).await.unwrap_err();
        assert_eq!(error.exit_code(), 4);
        assert!(matches!(error, DatadogError::ApiError { status: 503, .. }));
    }
}

#[tokio::test]
async fn service_unavailable_with_zero_or_invalid_retry_after_can_recover() {
    for value in ["0", "Sun, 06 Nov 1994 08:49:37 GMT", "invalid"] {
        let server = MockServer::start().await;
        Mock::given(path("/api/v1/monitor/7"))
            .respond_with(ResponseTemplate::new(503).insert_header("Retry-After", value))
            .up_to_n_times(1)
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/api/v1/monitor/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 7, "name": "ok", "type": "metric alert", "query": "q", "tags": []
            })))
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(client_for(&server, 1).get_monitor(7).await.unwrap().id, 7);
    }
}

#[tokio::test]
async fn conflicting_rate_limit_headers_report_the_effective_wait() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v1/monitor/7"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-ratelimit-reset", "0")
                .insert_header("Retry-After", "60"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let error = client_for(&server, 3).get_monitor(7).await.unwrap_err();
    assert_eq!(error.exit_code(), 5);
    assert!(matches!(
        error,
        DatadogError::RateLimitError {
            reset_secs: Some(60)
        }
    ));
}

#[tokio::test]
async fn metric_query_methods_validate_counts_before_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let client = client_for(&server, 0);
    for count in [0, 27, 160, 256] {
        let queries = vec!["avg:system.cpu.user{*}".to_string(); count];
        let results = [
            client.query_timeseries(&queries, &[], 0, 1, None).await,
            client.query_scalar(&queries, &[], "avg", 0, 1).await,
        ];
        for result in results {
            assert!(matches!(result, Err(DatadogError::InvalidInput(_))));
        }
    }
}

#[tokio::test]
async fn metric_query_methods_accept_both_count_boundaries_with_unique_names() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {}})))
        .expect(4)
        .mount(&server)
        .await;
    let client = client_for(&server, 0);
    for count in [1, 26] {
        let queries = vec!["avg:system.cpu.user{*}".to_string(); count];
        client
            .query_timeseries(&queries, &[], 0, 1, None)
            .await
            .unwrap();
        client
            .query_scalar(&queries, &[], "avg", 0, 1)
            .await
            .unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    for (request, count) in requests.iter().zip([1, 1, 26, 26]) {
        let body: serde_json::Value = request.body_json().unwrap();
        let queries = body["data"]["attributes"]["queries"].as_array().unwrap();
        assert_eq!(queries.len(), count);
        let names: Vec<&str> = queries
            .iter()
            .map(|query| query["name"].as_str().unwrap())
            .collect();
        assert_eq!(names[0], "a");
        assert_eq!(names.last(), Some(&if count == 1 { "a" } else { "z" }));
        assert_eq!(
            names.iter().collect::<std::collections::HashSet<_>>().len(),
            count
        );
    }
}
