use clap::Parser;
use datadog_cli::cli::{Cli, Command, LogsAction};
use datadog_cli::config::{Config, Credentials, Defaults, Network};
use datadog_cli::datadog::DatadogClient;
use datadog_cli::handlers::{
    logs::LogsHandler, metrics::MetricsHandler, monitors::MonitorsHandler, spans::SpansHandler,
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
