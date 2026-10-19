use clap::Parser;
use datadog_cli::cli::{Cli, Command, LogsAction};
use datadog_cli::config::{Config, Credentials, Defaults, Network};
use datadog_cli::datadog::DatadogClient;
use datadog_cli::handlers::{
    downtimes::DowntimesHandler, llm_obs::LlmObsHandler, logs::LogsHandler,
    metrics::MetricsHandler, monitors::MonitorsHandler, services::ServicesHandler, slo::SloHandler,
    spans::SpansHandler, teams::TeamsHandler,
};
use serde_json::{Value, json};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> DatadogClient {
    DatadogClient::with_base_url(
        &Config {
            credentials: Credentials::Token("fixture-token".into()),
            site: "datadoghq.com".into(),
            defaults: Defaults::default(),
            network: Network {
                max_retries: 0,
                ..Network::default()
            },
        },
        server.uri(),
    )
    .unwrap()
}

#[tokio::test]
async fn llm_query_and_structured_filters_are_distinct_request_modes() {
    for (argv, expected_query, expected_app) in [
        (vec!["datadog-cli", "llm-obs"], None, None),
        (
            vec![
                "datadog-cli",
                "llm-obs",
                "@ml_app:chatbot AND @meta.span.kind:llm",
            ],
            Some("@ml_app:chatbot AND @meta.span.kind:llm"),
            None,
        ),
        (
            vec![
                "datadog-cli",
                "llm-obs",
                "--ml-app",
                "chatbot",
                "--span-kind",
                "llm",
            ],
            None,
            Some("chatbot"),
        ),
    ] {
        let server = MockServer::start().await;
        respond(
            &server,
            "/api/v2/llm-obs/v1/spans/events/search",
            json!({
                "data":[], "meta":{"elapsed":1,"request_id":"fixture","status":"done","page":{}}
            }),
        )
        .await;
        let Command::LlmObs(args) = Cli::parse_from(argv).command else {
            panic!()
        };
        LlmObsHandler::search(&client(&server), &args)
            .await
            .unwrap();
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].headers.get("content-type").unwrap(),
            "application/json"
        );
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        let filter = &body["data"]["attributes"]["filter"];
        assert_eq!(filter.get("query").and_then(Value::as_str), expected_query);
        assert_eq!(filter.get("ml_app").and_then(Value::as_str), expected_app);
        if expected_app.is_some() {
            assert_eq!(filter["span_kind"], "llm");
        }
    }
    for args in [
        vec!["datadog-cli", "llm-obs", "*", "--ml-app", "chatbot"],
        vec![
            "datadog-cli",
            "llm-obs",
            "@ml_app:chatbot",
            "--span-kind",
            "llm",
        ],
        vec!["datadog-cli", "events", "--limit", "1001"],
        vec!["datadog-cli", "monitors", "list", "--page-size", "1001"],
        vec!["datadog-cli", "events", "--sort", "banana"],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
    assert!(Cli::try_parse_from(["datadog-cli", "events", "--limit", "1000"]).is_ok());
    assert!(
        Cli::try_parse_from(["datadog-cli", "monitors", "list", "--page-size", "1000"]).is_ok()
    );
}

#[tokio::test]
async fn malformed_required_responses_fail_instead_of_becoming_empty_results() {
    use datadog_cli::datadog::{LlmObsFilter, LlmObsSearchParams};
    for body in [
        json!({}),
        json!({"data":null}),
        json!({"data":{}}),
        json!({"data":[42]}),
    ] {
        let server = MockServer::start().await;
        respond(&server, "/api/v2/incidents", body).await;
        assert_eq!(
            client(&server)
                .list_incidents(1, 0, None)
                .await
                .unwrap_err()
                .exit_code(),
            7
        );
    }
    for body in [json!({}), json!({"data":null}), json!({"data":[]})] {
        let server = MockServer::start().await;
        respond(&server, "/api/v2/incidents/incident-1", body).await;
        assert_eq!(
            client(&server)
                .get_incident("incident-1", None)
                .await
                .unwrap_err()
                .exit_code(),
            7
        );
    }
    for body in [
        json!({"meta":{"elapsed":1,"request_id":"fixture","status":"done","page":{}}}),
        json!({"data":[]}),
        json!({"data":[],"meta":{"elapsed":"invalid","request_id":"fixture","status":"done","page":{}}}),
        json!({"data":[{"id":"span-1","type":"span"}],"meta":{"elapsed":1,"request_id":"fixture","status":"done","page":{}}}),
    ] {
        let server = MockServer::start().await;
        respond(&server, "/api/v2/llm-obs/v1/spans/events/search", body).await;
        let params = LlmObsSearchParams {
            filter: LlmObsFilter::Structured {
                ml_app: None,
                span_kind: None,
            },
            from: "2026-10-19T00:00:00Z",
            to: "2026-10-19T00:05:00Z",
            limit: 1,
            cursor: None,
            sort: None,
        };
        assert_eq!(
            client(&server)
                .search_llm_obs_spans(&params)
                .await
                .unwrap_err()
                .exit_code(),
            7
        );
    }
}

#[tokio::test]
async fn authoritative_totals_end_full_final_pages_without_guessing() {
    for (argv, endpoint, metadata_key, metadata) in [
        (
            vec!["datadog-cli", "teams", "--page-size", "1"],
            "/api/v2/team",
            "meta",
            json!({"pagination":{"type":"number_size","total":1,"next_number":null}}),
        ),
        (
            vec!["datadog-cli", "services", "--count", "1"],
            "/api/v2/catalog/entity",
            "meta",
            json!({"count":1,"includeCount":0}),
        ),
        (
            vec!["datadog-cli", "downtimes", "--count", "1"],
            "/api/v2/downtime",
            "meta",
            json!({"page":{"total_filtered_count":1}}),
        ),
        (
            vec!["datadog-cli", "slo", "list", "--limit", "1"],
            "/api/v1/slo",
            "metadata",
            json!({"page":{"total_filtered_count":1}}),
        ),
    ] {
        let server = MockServer::start().await;
        let mut body = json!({"data":[{"id":"record-1"}]});
        body[metadata_key] = metadata.clone();
        respond(&server, endpoint, body).await;
        let client = client(&server);
        let output = match Cli::parse_from(argv).command {
            Command::Teams(args) => TeamsHandler::list(&client, &args).await,
            Command::Services(args) => ServicesHandler::list(&client, &args).await,
            Command::Downtimes(args) => DowntimesHandler::list(&client, &args).await,
            Command::Slo {
                action: datadog_cli::cli::SloAction::List(args),
            } => SloHandler::list(&client, &args).await,
            _ => panic!(),
        }
        .unwrap();
        assert_eq!(output["pagination"]["returned"], 1, "{endpoint}");
        assert_eq!(output["pagination"]["total_count"], 1, "{endpoint}");
        assert_eq!(output["pagination"]["has_next"], false, "{endpoint}");
        assert_eq!(
            output["pagination"]["has_next_is_estimate"], false,
            "{endpoint}"
        );
        assert_eq!(output["meta"]["api"], metadata, "{endpoint}");
    }
}

#[tokio::test]
async fn unknown_and_invalid_totals_are_distinct() {
    for total in [
        None,
        Some(json!(null)),
        Some(json!(42)),
        Some(json!("invalid")),
        Some(json!(-1)),
    ] {
        let server = MockServer::start().await;
        let mut body = json!({"data":[{"id":"team-1"}]});
        if let Some(total) = &total {
            body["meta"] = json!({"pagination":{"total":total}});
        }
        respond(&server, "/api/v2/team", body).await;
        let Command::Teams(args) =
            Cli::parse_from(["datadog-cli", "teams", "--page-size", "1"]).command
        else {
            panic!()
        };
        let result = TeamsHandler::list(&client(&server), &args).await;
        if total
            .as_ref()
            .is_some_and(|v| v.is_string() || v.as_i64() == Some(-1))
        {
            assert_eq!(result.unwrap_err().exit_code(), 7);
        } else {
            let output = result.unwrap();
            assert_eq!(output["pagination"]["returned"], 1);
            assert_eq!(
                output["pagination"]["total_count"],
                total.clone().unwrap_or(Value::Null)
            );
            assert_eq!(
                output["pagination"]["has_next_is_estimate"],
                total.as_ref().is_none_or(Value::is_null)
            );
        }
    }
}

#[tokio::test]
async fn explicit_team_continuation_is_authoritative_without_a_total() {
    for next in [json!(null), json!(1)] {
        let server = MockServer::start().await;
        respond(
            &server,
            "/api/v2/team",
            json!({"data":[{"id":"team-1"}],"meta":{"pagination":{"next_number":next}}}),
        )
        .await;
        let Command::Teams(args) =
            Cli::parse_from(["datadog-cli", "teams", "--page-size", "1"]).command
        else {
            panic!()
        };
        let result = TeamsHandler::list(&client(&server), &args).await.unwrap();
        assert_eq!(result["pagination"]["has_next"], !next.is_null());
        assert_eq!(result["pagination"]["has_next_is_estimate"], false);
        assert!(result["pagination"]["total_count"].is_null());
    }
}

#[tokio::test]
async fn host_metadata_and_identity_survive_without_bypassing_tag_filters() {
    for filter in ["env:", ""] {
        let server = MockServer::start().await;
        respond(
            &server,
            "/api/v1/hosts",
            json!({
                "total_matching":1,"total_returned":1,"host_list":[{
                    "id":42,"name":"host-1","host_name":"host-1","up":true,"is_muted":false,
                    "aliases":["alias-1"],"meta":{"agent_version":"fixture","platform":"linux"},
                    "metrics":{"cpu":12.5},"tags_by_source":{"agent":["env:prod","team:api"]}
                }]
            }),
        )
        .await;
        let Command::Hosts(args) =
            Cli::parse_from(["datadog-cli", "hosts", "--tag-filter", filter]).command
        else {
            panic!()
        };
        let result = datadog_cli::handlers::hosts::HostsHandler::list(&client(&server), &args)
            .await
            .unwrap();
        assert_eq!(result["data"][0]["id"], 42);
        assert_eq!(result["data"][0]["aliases"], json!(["alias-1"]));
        assert_eq!(result["data"][0]["meta"]["agent_version"], "fixture");
        assert_eq!(result["data"][0]["metrics"]["cpu"], 12.5);
        assert!(result["data"][0].get("tags_by_source").is_none());
        if filter.is_empty() {
            assert!(result["data"][0].get("tags").is_none());
        } else {
            assert_eq!(result["data"][0]["tags"], json!({"agent":["env:prod"]}));
        }
    }
}

async fn respond(server: &MockServer, endpoint: &str, body: Value) {
    Mock::given(path(endpoint))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn log_custom_attributes_preserve_correlation_and_business_context() {
    let server = MockServer::start().await;
    let attributes = json!({
        "trace_id":"trace-1", "span_id":"span-1", "customer_id":"customer-1",
        "error":{"stack":"frame", "custom":[false,null,12]}, "service":"custom-service"
    });
    respond(
        &server,
        "/api/v2/logs/events/search",
        json!({
            "data":[{"id":"log-1","attributes":{
                "service":"api", "tags":["env:prod","team:api"], "attributes":attributes
            }}]
        }),
    )
    .await;
    let Command::Logs {
        action: LogsAction::Search(args),
    } = Cli::parse_from(["datadog-cli", "logs", "search", "--tag-filter", "env:"]).command
    else {
        panic!()
    };
    let output = LogsHandler::search(&client(&server), &args).await.unwrap();
    assert_eq!(output["data"][0]["attributes"], attributes);
    assert_eq!(output["data"][0]["service"], "api");
    assert_eq!(output["data"][0]["tags"], json!(["env:prod"]));
}

#[tokio::test]
async fn log_groups_are_ranked_on_the_server_and_keep_query_diagnostics() {
    for mode in ["aggregate", "timeseries"] {
        let server = MockServer::start().await;
        let data = json!({"buckets":[{"by":{"service":"api"},"computes":{"c0":42}}]});
        let metadata =
            json!({"status":"timeout","warnings":[{"code":"partial","detail":"fixture"}]});
        respond(
            &server,
            "/api/v2/logs/analytics/aggregate",
            json!({"data":data,"meta":metadata}),
        )
        .await;
        let Command::Logs { action } = Cli::parse_from([
            "datadog-cli",
            "logs",
            mode,
            "status:error",
            "--group-by",
            "service",
            "--group-limit",
            "3",
        ])
        .command
        else {
            panic!()
        };
        let client = client(&server);
        let result = match action {
            LogsAction::Aggregate(args) => LogsHandler::aggregate(&client, &args).await,
            LogsAction::Timeseries(args) => LogsHandler::timeseries(&client, &args).await,
            _ => panic!(),
        }
        .unwrap();
        assert_eq!(result["data"], data);
        assert_eq!(result["meta"]["api"], metadata);
        assert_eq!(result["meta"]["group_by"], "service");
        assert_eq!(result["meta"]["group_limit"], 3);
        let requests = server.received_requests().await.unwrap();
        let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["filter"]["query"], "status:error");
        assert_eq!(
            body["group_by"],
            json!([{"facet":"service","limit":3,"sort":{"type":"measure","aggregation":"count","order":"desc"}}])
        );
        assert_eq!(
            body["compute"][0]["type"],
            if mode == "aggregate" {
                "total"
            } else {
                "timeseries"
            }
        );
    }
    for argv in [
        vec!["datadog-cli", "logs", "aggregate", "--group-limit", "3"],
        vec![
            "datadog-cli",
            "logs",
            "aggregate",
            "--group-by",
            "service",
            "--group-limit",
            "10001",
        ],
        vec!["datadog-cli", "logs", "aggregate", "--group-by", ""],
    ] {
        assert!(Cli::try_parse_from(argv).is_err());
    }
    let server = MockServer::start().await;
    let error = client(&server)
        .aggregate_logs_with_group_by("*", "now-5m", "now", vec![], Some(" "), 10)
        .await
        .unwrap_err();
    assert_eq!(error.exit_code(), 1);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn monitor_details_preserve_evaluation_settings_and_extensible_fields() {
    let server = MockServer::start().await;
    let original = json!({
        "id":1, "name":"monitor", "type":"query alert", "query":"formula(\"query1\") > 90",
        "tags":[], "draft_status":"published", "assets":[{"name":"runbook"}],
        "creator":{"name":"owner", "extension":true},
        "options":{
            "evaluation_delay":300, "require_full_window":false,
            "on_missing_data":"show_no_data", "new_group_delay":60,
            "variables":[{"name":"query1","data_source":"metrics","query":"avg:system.cpu.user{*}"}],
            "thresholds":{"critical":90,"critical_recovery":80,"warning_recovery":70,"extension":null}
        }
    });
    respond(&server, "/api/v1/monitor/1", original.clone()).await;
    let output = MonitorsHandler::get(&client(&server), 1).await.unwrap();
    for key in ["draft_status", "assets"] {
        assert_eq!(output["data"][key], original[key]);
    }
    for (key, value) in original["options"].as_object().unwrap() {
        if key != "thresholds" {
            assert_eq!(&output["data"]["options"][key], value);
        }
    }
    assert_eq!(
        output["data"]["options"]["thresholds"]["critical_recovery"],
        80
    );
    assert_eq!(
        output["data"]["options"]["thresholds"]["warning_recovery"],
        70
    );
    assert!(
        output["data"]["options"]["thresholds"]
            .get("extension")
            .is_some()
    );
    assert_eq!(output["data"]["creator"]["extension"], true);
}

#[tokio::test]
async fn permission_denials_have_a_distinct_machine_readable_kind() {
    for (status, kind) in [(401, "authentication"), (403, "permission_denied")] {
        let server = MockServer::start().await;
        Mock::given(path("/api/v1/monitor/1"))
            .respond_with(ResponseTemplate::new(status).set_body_json(json!({"errors":["denied"]})))
            .expect(1)
            .mount(&server)
            .await;
        let error = client(&server).get_monitor(1).await.unwrap_err();
        assert_eq!(error.exit_code(), 3);
        assert_eq!(error.diagnostic()["error"]["kind"], kind);
        if status == 403 {
            assert_eq!(error.diagnostic()["error"]["http_status"], 403);
        }
    }
}

#[tokio::test]
async fn metric_units_preserve_denominators_scaling_and_null_positions() {
    for units in [
        json!([
            {"name":"byte","family":"bytes","plural":"bytes","scale_factor":1024.0,"short_name":"B","extension":true},
            {"name":"second","family":"time","plural":"seconds","scale_factor":1.0,"short_name":"s"}
        ]),
        json!([null,{"name":"second","family":"time","plural":"seconds","scale_factor":1.0,"short_name":"s"}]),
        json!([null, null]),
    ] {
        let server = MockServer::start().await;
        respond(&server, "/api/v1/query", json!({
            "status":"ok", "query":"avg:system.net.bytes_rcvd{*}",
            "series":[{"metric":"system.net.bytes_rcvd","scope":"*","expression":"query", "unit":units}]
        })).await;
        let Command::Metrics(args) =
            Cli::parse_from(["datadog-cli", "metrics", "avg:system.net.bytes_rcvd{*}"]).command
        else {
            panic!()
        };
        let output = MetricsHandler::query(&client(&server), &args)
            .await
            .unwrap();
        for (index, original) in units.as_array().unwrap().iter().enumerate() {
            if let Some(fields) = original.as_object() {
                for (key, value) in fields {
                    assert_eq!(&output["data"][0]["units"][index][key], value);
                }
            } else {
                assert!(output["data"][0]["units"][index].is_null());
            }
        }
    }
}

#[tokio::test]
async fn spans_preserve_http_and_messaging_details_with_both_stack_modes() {
    let stack = (1..=12)
        .map(|n| format!("frame {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    let custom = json!({
        "http":{"useragent_details":{"browser":"fixture"}},
        "messaging":{"kafka":{"bootstrap":{"servers":"server:9092,".repeat(30)}}},
        "error":{"stack":stack}
    });
    for full in [false, true] {
        let server = MockServer::start().await;
        respond(
            &server,
            "/api/v2/spans/events/search",
            json!({
                "data":[{"id":"span-1","attributes":{"custom":custom}}]
            }),
        )
        .await;
        let mut argv = vec!["datadog-cli", "spans"];
        if full {
            argv.push("--full-stack-trace");
        }
        let Command::Spans(args) = Cli::parse_from(argv).command else {
            panic!()
        };
        let output = SpansHandler::list(&client(&server), &args).await.unwrap();
        let mut expected = custom.clone();
        if !full {
            expected["error"]["stack"] =
                json!(datadog_cli::utils::truncate_stack_trace(&stack, 10));
        }
        assert_eq!(output["data"][0]["attributes"]["custom"], expected);
    }
}
