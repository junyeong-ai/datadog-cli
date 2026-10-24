use std::time::Duration;

use datadog_cli::{
    Config,
    config::{Credentials, Defaults, Network},
    error::DatadogError,
    mcp::PreparedMcp,
};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, Request, Respond, ResponseTemplate,
    matchers::{method, path},
};

const TOOL: &str = "find_datadog_database_instances";
fn config() -> Config {
    Config {
        credentials: Credentials::Token("test-token".into()),
        site: "datadoghq.com".into(),
        defaults: Defaults::default(),
        network: Network {
            timeout_secs: 3,
            ..Network::default()
        },
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    Json,
    Sse,
    Mutating,
    ToolError,
    RepeatedCursor,
    OversizedSse,
    Expired,
    BadOutput,
    ProtocolError,
    ResumableResponse,
    ResumableStateless,
    ResumableBackground,
}
struct Server(Scenario);
impl Respond for Server {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        let result = match body["method"].as_str().unwrap() {
            "initialize" => {
                assert_eq!(body["params"]["protocolVersion"], "2025-11-25");
                assert_eq!(body["params"]["capabilities"], json!({}));
                json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"test-datadog","version":"1"}})
            }
            "notifications/initialized" => return ResponseTemplate::new(202),
            "tools/list" => {
                let mut result = json!({"tools":[{"name":TOOL,"description":"Find database instances","inputSchema":{"type":"object","properties":{"limit":{"type":"integer","minimum":1}},"required":["limit"],"additionalProperties":false},"annotations":{"readOnlyHint":!matches!(self.0,Scenario::Mutating)}}]});
                if matches!(self.0, Scenario::BadOutput) {
                    result["tools"][0]["outputSchema"] = json!({"type":"object","properties":{"instances":{"type":"integer"}},"required":["instances"]});
                }
                if matches!(self.0, Scenario::RepeatedCursor) {
                    result["nextCursor"] = json!("repeat");
                }
                result
            }
            "tools/call" => {
                if matches!(
                    self.0,
                    Scenario::ResumableResponse | Scenario::ResumableStateless
                ) {
                    return ResponseTemplate::new(200)
                        .set_body_raw("id: resume-me\nretry: 0\n\n", "text/event-stream");
                }
                if matches!(self.0, Scenario::Expired) {
                    return ResponseTemplate::new(404);
                }
                if matches!(self.0, Scenario::ProtocolError) {
                    return ResponseTemplate::new(200).set_body_json(json!({"jsonrpc":"2.0","id":body["id"],"error":{"code":-32602,"message":"Invalid database filter"}}));
                }
                json!({"content":[{"type":"text","text":"database evidence"}],"structuredContent":{"instances":[]},"isError":matches!(self.0,Scenario::ToolError)})
            }
            other => panic!("Unexpected MCP method {other}"),
        };
        let message = json!({"jsonrpc":"2.0","id":body["id"],"result":result});
        let response = ResponseTemplate::new(200);
        let response = if matches!(self.0, Scenario::ResumableStateless) {
            response
        } else {
            response.insert_header("mcp-session-id", "test-session")
        };
        if matches!(self.0, Scenario::Sse | Scenario::OversizedSse) {
            let content = if matches!(self.0, Scenario::OversizedSse) {
                format!(
                    ":{}\n\nevent: message\ndata: {message}\n\n",
                    "x".repeat(4096)
                )
            } else {
                format!("event: message\ndata: {message}\n\n")
            };
            response.set_body_raw(content, "text/event-stream")
        } else {
            response.set_body_json(message)
        }
    }
}
async fn server(scenario: Scenario) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(Server(scenario))
        .mount(&server)
        .await;
    let background = if matches!(scenario, Scenario::ResumableBackground) {
        ResponseTemplate::new(200).set_body_raw("retry: 0\n\n", "text/event-stream")
    } else {
        ResponseTemplate::new(405)
    };
    Mock::given(method("GET"))
        .respond_with(background)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    server
}
async fn execute(
    server: &MockServer,
    input: Value,
    max_calls: u32,
    max_bytes: usize,
) -> datadog_cli::Result<Value> {
    PreparedMcp::prepare(TOOL, Some(input))?
        .execute_at(
            &config(),
            format!("{}/mcp", server.uri()),
            Duration::from_secs(5),
            max_calls,
            max_bytes,
        )
        .await
}

#[tokio::test]
async fn json_and_sse_sessions_use_reviewed_tools_current_schemas_and_one_identity() {
    for scenario in [Scenario::Json, Scenario::Sse] {
        let server = server(scenario).await;
        let result = execute(&server, json!({"limit":1}), 8, 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(result["data"]["structuredContent"]["instances"], json!([]));
        assert_eq!(result["meta"]["source"]["toolset"], "dbm");
        assert_eq!(result["meta"]["source"]["backend"], "mcp");
        assert_eq!(result["meta"]["result"]["processing"], "unknown");
        let requests = server.received_requests().await.unwrap();
        assert_eq!(result["meta"]["execution"]["calls"], requests.len());
        assert!(
            requests
                .iter()
                .all(|r| r.headers["authorization"] == "Bearer test-token")
        );
        let calls = requests
            .iter()
            .filter_map(|r| serde_json::from_slice::<Value>(&r.body).ok())
            .filter(|r| r["method"] == "tools/call")
            .collect::<Vec<_>>();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["params"]["name"], TOOL);
        assert_eq!(calls[0]["params"]["arguments"], json!({"limit":1}));
        assert!(requests.iter().any(|r| r.method.as_str() == "DELETE"));
    }
}

#[tokio::test]
async fn invalid_arguments_mutating_annotations_and_tool_errors_are_not_successes() {
    for (scenario, input, invalid) in [
        (Scenario::Json, json!({"limit":"private-input"}), true),
        (Scenario::Mutating, json!({"limit":1}), false),
        (Scenario::ToolError, json!({"limit":1}), false),
    ] {
        let server = server(scenario).await;
        let error = execute(&server, input, 8, 1024 * 1024).await.unwrap_err();
        if invalid {
            assert!(matches!(error, DatadogError::InvalidInput(_)));
            assert!(!error.to_string().contains("private-input"));
        } else {
            assert!(matches!(error, DatadogError::QueryError(_)));
        }
        let calls = server
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .filter_map(|r| serde_json::from_slice::<Value>(&r.body).ok())
            .filter(|r| r["method"] == "tools/call")
            .count();
        assert_eq!(calls, usize::from(matches!(scenario, Scenario::ToolError)));
    }
}

#[tokio::test]
async fn mcp_denials_do_not_retry_or_change_identity_and_response_budgets_are_global() {
    let denied = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(json!({"errors":["missing mcp_read"]})),
        )
        .expect(1)
        .mount(&denied)
        .await;
    assert!(matches!(
        execute(&denied, json!({"limit":1}), 8, 1024 * 1024)
            .await
            .unwrap_err(),
        DatadogError::PermissionDenied(_)
    ));
    let bounded = server(Scenario::Json).await;
    assert!(matches!(
        execute(&bounded, json!({"limit":1}), 1, 1024 * 1024)
            .await
            .unwrap_err(),
        DatadogError::QueryError(_)
    ));
    assert_eq!(bounded.received_requests().await.unwrap().len(), 1);
    let oversized = server(Scenario::OversizedSse).await;
    assert!(matches!(
        execute(&oversized, json!({"limit":1}), 8, 1024)
            .await
            .unwrap_err(),
        DatadogError::QueryError(_)
    ));
    let cursor = server(Scenario::RepeatedCursor).await;
    assert!(matches!(
        execute(&cursor, json!({"limit":1}), 8, 1024 * 1024)
            .await
            .unwrap_err(),
        DatadogError::DecodeError(_)
    ));
}

#[test]
fn mcp_policy_and_sites_are_explicit_and_discovery_is_bounded() {
    for tool in [
        "execute_code",
        "create_datadog_notebook",
        "run_network_path",
        "datadog_remote_action_restricted_shell_run_command",
        "unknown_read_only_tool",
    ] {
        assert!(PreparedMcp::prepare(tool, Some(json!({}))).is_err());
    }
    for site in [
        "datadoghq.com",
        "us3.datadoghq.com",
        "us5.datadoghq.com",
        "datadoghq.eu",
        "ap1.datadoghq.com",
        "ap2.datadoghq.com",
        "uk1.datadoghq.com",
    ] {
        assert!(
            datadog_cli::mcp::endpoint(site, "dbm")
                .unwrap()
                .starts_with("https://mcp.")
        );
    }
    assert!(datadog_cli::mcp::endpoint("ddog-gov.com", "dbm").is_err());
    assert!(datadog_cli::mcp::endpoint("datadoghq.com", "dbm&arbitrary=1").is_err());
    let list = datadog_cli::mcp::list(Some("profiling"), None, 0, 2).unwrap();
    assert_eq!(list["data"].as_array().unwrap().len(), 2);
    assert_eq!(list["pagination"]["next_offset"], 2);
    assert_eq!(list["pagination"]["total_count"], 12);
}

#[tokio::test]
async fn expired_sessions_are_not_replayed_and_protocol_errors_remain_visible() {
    for scenario in [
        Scenario::Expired,
        Scenario::BadOutput,
        Scenario::ProtocolError,
    ] {
        let server = server(scenario).await;
        let error = execute(&server, json!({"limit":1}), 8, 1024 * 1024)
            .await
            .unwrap_err();
        match scenario {
            Scenario::Expired => {
                assert!(matches!(error, DatadogError::ApiError { status: 404, .. }))
            }
            Scenario::BadOutput => assert!(matches!(error, DatadogError::DecodeError(_))),
            Scenario::ProtocolError => {
                assert!(error.to_string().contains("Invalid database filter"))
            }
            _ => unreachable!(),
        }
        let requests = server.received_requests().await.unwrap();
        for method in ["initialize", "tools/call"] {
            assert_eq!(
                requests
                    .iter()
                    .filter_map(|r| serde_json::from_slice::<Value>(&r.body).ok())
                    .filter(|r| r["method"] == method)
                    .count(),
                1
            );
        }
    }
    let slow = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&slow)
        .await;
    let error = PreparedMcp::prepare(TOOL, None)
        .unwrap()
        .execute_at(
            &config(),
            format!("{}/mcp", slow.uri()),
            Duration::from_millis(50),
            8,
            1024 * 1024,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, DatadogError::TimeoutError));
    assert_eq!(slow.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn server_retry_frames_do_not_resume_response_or_background_streams() {
    for scenario in [
        Scenario::ResumableResponse,
        Scenario::ResumableStateless,
        Scenario::ResumableBackground,
    ] {
        let server = server(scenario).await;
        let result = execute(&server, json!({"limit":1}), 8, 1024 * 1024).await;
        if !matches!(scenario, Scenario::ResumableBackground) {
            assert!(matches!(result, Err(DatadogError::QueryError(_))));
        } else {
            assert!(
                result.is_ok(),
                "a clean optional stream is not a failed tool call: {result:?}"
            );
        }
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.method.as_str() == "GET")
                .count(),
            usize::from(!matches!(scenario, Scenario::ResumableStateless))
        );
        assert!(
            requests
                .iter()
                .all(|request| !request.headers.contains_key("last-event-id"))
        );
        assert_eq!(
            requests
                .iter()
                .filter_map(|request| serde_json::from_slice::<Value>(&request.body).ok())
                .filter(|body| body["method"] == "tools/call")
                .count(),
            1
        );
    }
}
