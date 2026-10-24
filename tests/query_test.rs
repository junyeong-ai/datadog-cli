use std::time::Duration;

use datadog_cli::config::{Config, Credentials, Defaults, Network};
use datadog_cli::datadog::DatadogClient;
use datadog_cli::error::DatadogError;
use datadog_cli::query::PreparedQuery;
use serde_json::{Value, json};
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> DatadogClient {
    DatadogClient::with_base_url(
        &Config {
            credentials: Credentials::Token("test-token".into()),
            site: "datadoghq.com".into(),
            defaults: Defaults::default(),
            network: Network {
                timeout_secs: 2,
                max_retries: 3,
                ..Network::default()
            },
        },
        server.uri(),
    )
    .unwrap()
}

#[tokio::test]
async fn optional_metric_metadata_accepts_null_without_accepting_invalid_types() {
    let server = MockServer::start().await;
    let input = json!({"path":{"metric_name":"system.cpu.user"}});
    Mock::given(method("GET"))
        .and(path("/api/v1/metrics/system.cpu.user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"type":"gauge","unit":"percent","per_unit":null,"statsd_interval":null}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let result = execute(&server, "metrics", "v1.GetMetricMetadata", input.clone())
        .await
        .unwrap();
    assert!(result["data"]["per_unit"].is_null());
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/metrics/system.cpu.user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"per_unit":42})))
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        execute(&server, "metrics", "v1.GetMetricMetadata", input)
            .await
            .unwrap_err(),
        DatadogError::DecodeError(_)
    ));
}

#[tokio::test]
async fn monitor_notify_by_accepts_null_without_accepting_other_types() {
    let server = MockServer::start().await;
    let monitor = |notify_by: Value| json!([{"type":"metric alert","query":"avg(last_5m):avg:system.cpu.user{*} > 1","options":{"notify_by":notify_by}}]);
    Mock::given(method("GET"))
        .and(path("/api/v1/monitor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(monitor(Value::Null)))
        .expect(1)
        .mount(&server)
        .await;
    let result = execute(&server, "monitors", "v1.ListMonitors", json!({}))
        .await
        .unwrap();
    assert!(result["data"][0]["options"]["notify_by"].is_null());
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/monitor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(monitor(json!("cluster"))))
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        execute(&server, "monitors", "v1.ListMonitors", json!({}))
            .await
            .unwrap_err(),
        DatadogError::DecodeError(_)
    ));
}

async fn execute(
    server: &MockServer,
    domain: &str,
    id: &str,
    input: Value,
) -> datadog_cli::Result<Value> {
    PreparedQuery::prepare(domain, id, &input, None)?
        .execute(
            &client(server),
            "datadoghq.com",
            Duration::from_secs(5),
            1024 * 1024,
            4,
        )
        .await
}

#[tokio::test]
async fn ci_search_preserves_attributes_and_api_metadata() {
    let server = MockServer::start().await;
    let body = json!({"filter":{"query":"@ci.pipeline.name:build","from":"2026-10-01T00:00:00Z","to":"2026-10-02T00:00:00Z"},"page":{"limit":1}});
    let response = json!({"data":[{"id":"pipeline-1","type":"cipipeline","attributes":{"attributes":{"pipeline":{"id":"run-1"},"future":{"value":42}},"ci_level":"pipeline","tags":["env:ci"]}}],"meta":{"page":{"after":"next-page"},"status":"done"}});
    Mock::given(method("POST"))
        .and(path("/api/v2/ci/pipelines/events/search"))
        .and(header("authorization", "Bearer test-token"))
        .and(body_json(&body))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&response)
                .insert_header("x-ratelimit-remaining", "299"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let result = execute(
        &server,
        "ci-visibility-pipelines",
        "v2.SearchCIAppPipelineEvents",
        json!({"body":body}),
    )
    .await
    .unwrap();
    assert_eq!(result["data"], response);
    assert_eq!(result["meta"]["api"]["page"]["after"], "next-page");
    assert_eq!(
        result["meta"]["http"]["headers"]["x-ratelimit-remaining"],
        "299"
    );
    assert_eq!(result["meta"]["execution"]["calls"], 1);
    assert_eq!(result["meta"]["result"]["processing"], "complete");
}

#[tokio::test]
async fn declared_array_serialization_and_encoded_paths_reach_only_the_selected_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/team"))
        .and(query_param("fields[team]", "name,handle"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[]})))
        .expect(1)
        .mount(&server)
        .await;
    execute(
        &server,
        "teams",
        "v2.ListTeams",
        json!({"query":{"fields[team]":["name","handle"]}}),
    )
    .await
    .unwrap();
    Mock::given(method("GET"))
        .and(path("/api/v1/metrics/a%2Fb%3Ftoken%3Dx"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"type":"gauge"})))
        .expect(1)
        .mount(&server)
        .await;
    execute(
        &server,
        "metrics",
        "v1.GetMetricMetadata",
        json!({"path":{"metric_name":"a/b?token=x"}}),
    )
    .await
    .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert!(requests[1].url.query().is_none());
    assert!(
        PreparedQuery::prepare(
            "metrics",
            "v1.GetMetricMetadata",
            &json!({"path":{"metric_name":".."}}),
            None
        )
        .is_err()
    );
}

#[tokio::test]
async fn response_limits_and_request_limits_are_observable_errors() {
    let server = MockServer::start().await;
    assert!(
        PreparedQuery::prepare(
            "containers",
            "v2.ListContainers",
            &json!({"query":{"unknown":"secret-value"}}),
            None
        )
        .is_err()
    );
    Mock::given(method("GET"))
        .and(path("/api/v2/containers"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"data":[],"extra":"x".repeat(100)})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let prepared =
        PreparedQuery::prepare("containers", "v2.ListContainers", &json!({}), None).unwrap();
    let error = prepared
        .execute(
            &client(&server),
            "datadoghq.com",
            Duration::from_secs(2),
            32,
            4,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, DatadogError::QueryError(_)));
    assert!(error.to_string().contains("byte execution budget"));
}

#[tokio::test]
async fn query_network_timeout_respects_execution_deadline_without_overflow() {
    for (total_timeout_secs, response_delay, succeeds) in [
        (u64::MAX, Duration::ZERO, true),
        (1, Duration::from_millis(1500), false),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/containers"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data":[]}))
                    .set_delay(response_delay),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = DatadogClient::with_base_url(
            &Config {
                credentials: Credentials::Token("test-token".into()),
                site: "datadoghq.com".into(),
                defaults: Defaults::default(),
                network: Network {
                    timeout_secs: 2,
                    total_timeout_secs,
                    max_retries: 0,
                },
            },
            server.uri(),
        )
        .unwrap();
        let result = PreparedQuery::prepare("containers", "v2.ListContainers", &json!({}), None)
            .unwrap()
            .execute(&client, "datadoghq.com", Duration::from_secs(3), 4096, 1)
            .await;
        if succeeds {
            assert_eq!(result.unwrap()["data"]["data"], json!([]));
        } else {
            assert!(matches!(result, Err(DatadogError::TimeoutError)));
        }
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn query_deadlines_include_retry_wait_and_denials_never_retry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/containers"))
        .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "5"))
        .expect(1)
        .mount(&server)
        .await;
    let prepared =
        PreparedQuery::prepare("containers", "v2.ListContainers", &json!({}), None).unwrap();
    assert!(matches!(
        prepared
            .execute(
                &client(&server),
                "datadoghq.com",
                Duration::from_millis(200),
                4096,
                4
            )
            .await
            .unwrap_err(),
        DatadogError::TimeoutError
    ));
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/containers"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(json!({"errors":["Permission denied"]})),
        )
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        execute(&server, "containers", "v2.ListContainers", json!({}))
            .await
            .unwrap_err(),
        DatadogError::PermissionDenied(_)
    ));
}

#[tokio::test]
async fn query_submissions_are_not_retried_and_api_call_budgets_bound_search_retries() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/ddsql/query/tabular"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;
    let input = json!({"body":{"data":{"type":"ddsql_query_request","attributes":{"query":"SELECT 1","time":{"from_timestamp":1,"to_timestamp":2}}}}});
    assert!(matches!(
        execute(&server, "ddsql", "v2.ExecuteDdsqlTabularQuery", input)
            .await
            .unwrap_err(),
        DatadogError::ApiError { status: 503, .. }
    ));
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/containers"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;
    let prepared =
        PreparedQuery::prepare("containers", "v2.ListContainers", &json!({}), None).unwrap();
    assert!(matches!(
        prepared
            .execute(
                &client(&server),
                "datadoghq.com",
                Duration::from_secs(2),
                4096,
                1
            )
            .await
            .unwrap_err(),
        DatadogError::ApiError { status: 503, .. }
    ));
}

#[tokio::test]
async fn successful_http_does_not_hide_processing_or_schema_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/query/scalar"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"errors":"Invalid metric query"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let input = json!({"body":{"data":{"type":"scalar_request","attributes":{"from":1,"to":2,"queries":[{"data_source":"metrics","query":"bad","name":"a","aggregator":"avg"}]}}}});
    assert!(matches!(
        execute(&server, "metrics", "v2.QueryScalarData", input)
            .await
            .unwrap_err(),
        DatadogError::QueryError(_)
    ));
    Mock::given(method("GET"))
        .and(path("/api/v2/containers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":"invalid"})))
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        execute(&server, "containers", "v2.ListContainers", json!({}))
            .await
            .unwrap_err(),
        DatadogError::DecodeError(_)
    ));
}

#[tokio::test]
async fn conditional_reads_are_not_empty_resources_and_downloads_preserve_bytes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/csm/ownership/resource/team"))
        .and(header("if-none-match", "\"version-1\""))
        .respond_with(ResponseTemplate::new(304).insert_header("etag", "\"version-1\""))
        .expect(1)
        .mount(&server)
        .await;
    let result = execute(&server,"csm-ownership","v2.GetOwnershipInference",json!({"path":{"resource_id":"resource","owner_type":"team"},"headers":{"If-None-Match":"\"version-1\""}})).await.unwrap();
    assert!(result["data"].is_null());
    assert_eq!(result["meta"]["result"]["not_modified"], true);
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("records.csv");
    let input = json!({"path":{"project_id":"project","dataset_id":"dataset"}});
    assert!(
        PreparedQuery::prepare(
            "agent-observability",
            "v2.ExportLLMObsDataset",
            &input,
            None
        )
        .is_err()
    );
    Mock::given(method("GET"))
        .and(path("/api/v2/llm-obs/v1/project/datasets/dataset/export"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"name,value\nitem,1\n".to_vec())
                .insert_header("content-type", "text/csv"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let prepared = PreparedQuery::prepare(
        "agent-observability",
        "v2.ExportLLMObsDataset",
        &input,
        Some(output.clone()),
    )
    .unwrap();
    prepared
        .execute(
            &client(&server),
            "datadoghq.com",
            Duration::from_secs(2),
            4096,
            1,
        )
        .await
        .unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), b"name,value\nitem,1\n");
    assert!(
        PreparedQuery::prepare(
            "agent-observability",
            "v2.ExportLLMObsDataset",
            &input,
            Some(output)
        )
        .is_err()
    );
}

#[tokio::test]
async fn explicit_projection_preserves_partial_query_metadata_and_rejects_missing_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/api/v2/logs/events/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[],"meta":{"status":"timeout","warnings":[{"code":"partial","detail":"Query timed out"}]}})))
        .expect(2).mount(&server).await;
    let input = json!({"body":{"filter":{"query":"service:api"},"page":{"limit":1}}});
    let prepared = PreparedQuery::prepare("logs", "v2.ListLogs", &input, None)
        .unwrap()
        .with_selection(&["/data".into()])
        .unwrap();
    let result = prepared
        .execute(
            &client(&server),
            "datadoghq.com",
            Duration::from_secs(2),
            4096,
            1,
        )
        .await
        .unwrap();
    assert_eq!(result["data"]["/data"], json!([]));
    assert_eq!(result["meta"]["result"]["processing"], "partial");
    assert_eq!(result["meta"]["api"]["warnings"][0]["code"], "partial");
    let prepared = PreparedQuery::prepare("logs", "v2.ListLogs", &input, None)
        .unwrap()
        .with_selection(&["/missing".into()])
        .unwrap();
    assert!(matches!(
        prepared
            .execute(
                &client(&server),
                "datadoghq.com",
                Duration::from_secs(2),
                4096,
                1
            )
            .await
            .unwrap_err(),
        DatadogError::QueryError(_)
    ));
    assert!(
        PreparedQuery::prepare("logs", "v2.ListLogs", &input, None)
            .unwrap()
            .with_selection(&["/bad~2escape".into()])
            .is_err()
    );
}

#[tokio::test]
async fn cursor_collection_preserves_pages_fixed_scope_and_partial_status() {
    let server = MockServer::start().await;
    let input = json!({"body":{"filter":{"from":"2026-10-01T00:00:00Z","to":"2026-10-02T00:00:00Z"},"page":{"limit":1}}});
    Mock::given(method("POST")).and(path("/api/v2/logs/events/search"))
        .and(body_json(&input["body"]))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":"first"}],"meta":{"page":{"after":"opaque"},"status":"timeout","warnings":[{"code":"partial","detail":"limited"}]}})))
        .expect(1).mount(&server).await;
    let mut next = input.clone();
    next["body"]["page"]["cursor"] = json!("opaque");
    Mock::given(method("POST"))
        .and(body_json(&next["body"]))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"data":[{"id":"second"}],"meta":{"status":"done"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let result = PreparedQuery::prepare("logs", "v2.ListLogs", &input, None)
        .unwrap()
        .with_collection(2, false)
        .unwrap()
        .execute(
            &client(&server),
            "datadoghq.com",
            Duration::from_secs(5),
            1024 * 1024,
            2,
        )
        .await
        .unwrap();
    assert_eq!(result["data"]["pages"][1]["data"][0]["id"], "second");
    assert_eq!(result["meta"]["result"]["processing"], "partial");
    assert_eq!(result["meta"]["result"]["collection"], "exhausted");
    assert_eq!(
        result["meta"]["api"]["pages"][0]["warnings"][0]["code"],
        "partial"
    );
    assert_eq!(result["meta"]["execution"]["calls"], 2);
    assert!(
        PreparedQuery::prepare("logs", "v2.ListLogs", &json!({}), None)
            .unwrap()
            .with_collection(2, false)
            .is_err()
    );
    assert!(
        PreparedQuery::prepare(
            "logs",
            "v2.ListLogs",
            &json!({"body":{"filter":{"from":"now-1h","to":"now"}}}),
            None
        )
        .unwrap()
        .with_collection(2, false)
        .is_err()
    );
}

#[tokio::test]
async fn cursor_page_limit_and_repeated_cursor_do_not_claim_exhaustion() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/containers"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"data":[],"meta":{"pagination":{"next_cursor":"same"}}})),
        )
        .mount(&server)
        .await;
    let prepared = PreparedQuery::prepare("containers", "v2.ListContainers", &json!({}), None)
        .unwrap()
        .with_collection(2, false)
        .unwrap();
    let error = prepared
        .execute(
            &client(&server),
            "datadoghq.com",
            Duration::from_secs(5),
            1024 * 1024,
            4,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, DatadogError::DecodeError(_)));
    server.reset().await;
    Mock::given(method("GET"))
        .and(query_param("page[cursor]", "two"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"data":[],"meta":{"pagination":{"next_cursor":"three"}}})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::query_param_is_missing("page[cursor]"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"data":[],"meta":{"pagination":{"next_cursor":"two"}}})),
        )
        .mount(&server)
        .await;
    let result = prepared
        .execute(
            &client(&server),
            "datadoghq.com",
            Duration::from_secs(5),
            1024 * 1024,
            4,
        )
        .await
        .unwrap();
    assert_eq!(result["meta"]["result"]["collection"], "page_limit");
    assert_eq!(result["meta"]["pagination"]["has_next"], true);
    assert_eq!(result["meta"]["pagination"]["next_cursor"], "three");
}

#[tokio::test]
async fn ddsql_wait_polls_without_resubmitting_and_validates_column_alignment() {
    let server = MockServer::start().await;
    let response = |attributes| json!({"data":{"id":"q","type":"ddsql_query_response","attributes":attributes},"meta":{"elapsed":1,"request_id":"r"}});
    Mock::given(method("POST"))
        .and(path("/api/v2/ddsql/query/tabular"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(response(json!({"state":"running","query_id":"opaque-job"}))),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST")).and(path("/api/v2/ddsql/query/tabular/fetch"))
        .and(body_json(json!({"data":{"type":"ddsql_query_fetch_request","attributes":{"query_id":"opaque-job"}}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(response(json!({"state":"completed","columns":[{"name":"one","type":"BIGINT","values":[1]}]}))))
        .expect(1).mount(&server).await;
    let prepared=PreparedQuery::prepare("ddsql","v2.ExecuteDdsqlTabularQuery",&json!({"body":{"data":{"type":"ddsql_query_request","attributes":{"query":"SELECT 1","time":{"from_timestamp":0,"to_timestamp":1},"row_limit":1}}}}),None).unwrap().with_collection(1,true).unwrap();
    let result = prepared
        .execute(
            &client(&server),
            "datadoghq.com",
            Duration::from_secs(5),
            1024 * 1024,
            2,
        )
        .await
        .unwrap();
    assert_eq!(result["meta"]["job"]["polls"], 1);
    assert_eq!(result["meta"]["result"]["processing"], "complete");
    assert_eq!(
        result["data"]["data"]["attributes"]["columns"][0]["values"][0],
        1
    );
    server.reset().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(response(json!({"state":"completed","columns":[{"name":"a","type":"BIGINT","values":[1]},{"name":"a","type":"BIGINT","values":[]}]})))).mount(&server).await;
    assert!(matches!(
        prepared
            .execute(
                &client(&server),
                "datadoghq.com",
                Duration::from_secs(5),
                1024 * 1024,
                2
            )
            .await
            .unwrap_err(),
        DatadogError::DecodeError(_)
    ));
}

#[tokio::test]
async fn current_user_organization_null_fields_preserve_relationships_and_reject_wrong_types() {
    let server = MockServer::start().await;
    let response = json!({"data":{"type":"users","id":"user-1"},"included":[{"type":"orgs","id":"org-1","attributes":{"name":"Example","description":null,"url":null}}]});
    Mock::given(method("GET"))
        .and(path("/api/v2/current_user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response.clone()))
        .mount(&server)
        .await;
    let result = execute(&server, "users", "v2.GetCurrentUser", json!({}))
        .await
        .unwrap();
    assert_eq!(
        result["data"]["included"][0]["attributes"]["name"],
        "Example"
    );
    assert!(result["data"]["included"][0]["attributes"]["url"].is_null());
    server.reset().await;
    let mut invalid = response;
    invalid["included"][0]["attributes"]["url"] = json!(42);
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(invalid))
        .mount(&server)
        .await;
    assert!(matches!(
        execute(&server, "users", "v2.GetCurrentUser", json!({}))
            .await
            .unwrap_err(),
        DatadogError::DecodeError(_)
    ));
}

#[tokio::test]
async fn sentinel_response_omits_write_only_fields_without_relaxing_required_response_fields() {
    let server = MockServer::start().await;
    let destination = json!({"type":"custom_destination","id":"destination-id","attributes":{"forwarder_destination":{"type":"microsoft_sentinel","tenant_id":"tenant","client_id":"client","data_collection_endpoint":"https://example.com","data_collection_rule_id":"rule"}}});
    for (id, endpoint, input, response) in [
        (
            "v2.GetLogsCustomDestination",
            "/api/v2/logs/config/custom-destinations/destination-id",
            json!({"path":{"custom_destination_id":"destination-id"}}),
            json!({"data":destination.clone()}),
        ),
        (
            "v2.ListLogsCustomDestinations",
            "/api/v2/logs/config/custom-destinations",
            json!({}),
            json!({"data":[destination.clone()]}),
        ),
    ] {
        server.reset().await;
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(response.clone()))
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(
            execute(&server, "logs-custom-destinations", id, input.clone())
                .await
                .unwrap()["data"],
            response
        );
        server.reset().await;
        let mut invalid = response;
        let item = if invalid["data"].is_array() {
            &mut invalid["data"][0]
        } else {
            &mut invalid["data"]
        };
        item["attributes"]["forwarder_destination"]
            .as_object_mut()
            .unwrap()
            .remove("tenant_id");
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(invalid))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            execute(&server, "logs-custom-destinations", id, input)
                .await
                .unwrap_err(),
            DatadogError::DecodeError(_)
        ));
    }
}

#[tokio::test]
async fn nullable_references_and_compositions_preserve_null_and_reject_invalid_values() {
    for (domain, operation, endpoint, input, response, pointer) in [
        (
            "dashboards",
            "v1.GetDashboard",
            "/api/v1/dashboard/dashboard-1",
            json!({"path":{"dashboard_id":"dashboard-1"}}),
            json!({"title":"Overview","layout_type":"ordered","widgets":[],"default_timeframe":null}),
            "/default_timeframe",
        ),
        (
            "static-analysis",
            "v2.GetCustomRule",
            "/api/v2/static-analysis/custom/rulesets/ruleset-1/rules/rule-1",
            json!({"path":{"ruleset_name":"ruleset-1","rule_name":"rule-1"}}),
            json!({"data":{"id":"rule-1","type":"custom_rule","attributes":{"name":"rule-1","created_at":"2026-10-19T00:00:00Z","created_by":"user-1","last_revision":null,"revisions":[]}}}),
            "/data/attributes/last_revision",
        ),
        (
            "logs-archives",
            "v2.GetLogsArchive",
            "/api/v2/logs/config/archives/archive-1",
            json!({"path":{"archive_id":"archive-1"}}),
            json!({"data":{"type":"archives","id":"archive-1","attributes":{"name":"Archive","query":"*","destination":null}}}),
            "/data/attributes/destination",
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(response.clone()))
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(
            execute(&server, domain, operation, input.clone())
                .await
                .unwrap()["data"],
            response
        );
        for value in [json!(42), json!({})] {
            server.reset().await;
            let mut invalid = response.clone();
            *invalid.pointer_mut(pointer).unwrap() = value;
            Mock::given(method("GET"))
                .and(path(endpoint))
                .respond_with(ResponseTemplate::new(200).set_body_json(invalid))
                .expect(1)
                .mount(&server)
                .await;
            assert!(matches!(
                execute(&server, domain, operation, input.clone())
                    .await
                    .unwrap_err(),
                DatadogError::DecodeError(_)
            ));
        }
    }
}

#[tokio::test]
async fn nullable_enums_preserve_null_and_reject_unknown_values() {
    let server = MockServer::start().await;
    let response = json!({"data": [{"id":"schedule-1","type":"schedule","attributes": {
        "delivery_format":null,"description":"Weekly report","next_recurrence":null,
        "recipients":[],"resource_id":"dashboard-1","resource_type":"dashboard",
        "rrule":"FREQ=WEEKLY","status":"inactive","tab_id":null,
        "template_variables":[],"timeframe":null,"timezone":"UTC","title":"Weekly report"
    },"relationships":{"author":{"data":{"id":"user-1","type":"users"}}}}]});
    for value in [
        Value::Null,
        json!("pdf"),
        json!("png"),
        json!("pdf_and_png"),
        json!("unknown"),
        json!(42),
    ] {
        server.reset().await;
        let mut body = response.clone();
        body["data"][0]["attributes"]["delivery_format"] = value.clone();
        Mock::given(method("GET"))
            .and(path("/api/v2/reporting/schedule/list"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let result = execute(
            &server,
            "report-schedules",
            "v2.ListReportSchedules",
            json!({}),
        )
        .await;
        if value.is_null() || ["pdf", "png", "pdf_and_png"].contains(&value.as_str().unwrap_or(""))
        {
            assert_eq!(result.unwrap()["data"], body);
        } else {
            assert!(matches!(result.unwrap_err(), DatadogError::DecodeError(_)));
        }
    }
}

#[tokio::test]
async fn dataset_records_preserve_arbitrary_json_without_relaxing_record_fields() {
    let server = MockServer::start().await;
    for value in [
        json!("question"),
        Value::Null,
        json!(42),
        json!(true),
        json!({"nested":[null,[]]}),
        json!(["answer", 42, true]),
        json!([[null]]),
    ] {
        server.reset().await;
        let response = json!({"data":[{"id":"record-1","dataset_id":"dataset-1",
            "input":value,"expected_output":value,"metadata":null,
            "created_at":"2026-10-19T00:00:00Z","updated_at":"2026-10-19T00:00:00Z"}]});
        Mock::given(method("GET"))
            .and(path(
                "/api/v2/llm-obs/v1/project-1/datasets/dataset-1/records",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(response.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let input = json!({"path":{"project_id":"project-1","dataset_id":"dataset-1"}});
        assert_eq!(
            execute(
                &server,
                "agent-observability",
                "v2.ListLLMObsDatasetRecords",
                input.clone()
            )
            .await
            .unwrap()["data"],
            response
        );
        server.reset().await;
        let mut invalid = response;
        invalid["data"][0]["metadata"] = json!(42);
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(invalid))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            execute(
                &server,
                "agent-observability",
                "v2.ListLLMObsDatasetRecords",
                input
            )
            .await
            .unwrap_err(),
            DatadogError::DecodeError(_)
        ));
    }
}
