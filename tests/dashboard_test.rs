use datadog_cli::cli::DashboardsListArgs;
use datadog_cli::config::{Config, Credentials, Defaults, Network};
use datadog_cli::datadog::DatadogClient;
use datadog_cli::error::DatadogError;
use datadog_cli::handlers::dashboards::DashboardsHandler;
use serde_json::{Value, json};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client_for(server: &MockServer) -> DatadogClient {
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

async fn get_dashboard(body: &Value) -> Value {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/dashboard/dashboard-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;
    DashboardsHandler::get(&client_for(&server), "dashboard-1")
        .await
        .unwrap()["data"]
        .clone()
}

fn assert_fields_preserved(original: &Value, output: &Value) {
    for (key, value) in original.as_object().unwrap() {
        assert_eq!(output.get(key), Some(value), "dashboard field {key}");
    }
}

#[tokio::test]
async fn dashboard_details_preserve_modern_settings_and_nested_extensions() {
    let body = json!({
        "id": "dashboard-1",
        "title": "Checkout",
        "layout_type": "ordered",
        "url": "/dashboard/dashboard-1",
        "author_handle": "fixture@example.test",
        "author_name": "Fixture Author",
        "created_at": "2026-10-17T00:00:00Z",
        "modified_at": "2026-10-17T01:00:00Z",
        "reflow_type": "fixed",
        "default_timeframe": {"type": "live", "value": "1h"},
        "tags": ["team:checkout"],
        "template_variables": [{
            "name": "env", "prefix": "env", "defaults": ["prod", "staging"],
            "type": "filter", "extension": {"retained": [false, null, 1]}
        }],
        "template_variable_presets": [{"name": "production", "template_variables": [{"name": "env", "values": ["prod"]}]}],
        "widgets": [{
            "id": 9007199254740993u64,
            "definition": {
                "type": "group", "layout_type": "ordered",
                "widgets": [{"definition": {"type": "note", "content": "hello", "extension": null}}],
                "extension": {"value": true}
            },
            "layout": {"x": 0, "y": 0, "width": 4, "height": 2, "is_column_break": true, "extension": false},
            "extension": [1, null]
        }],
        "extension": {"nested": [false, null, {"retained": true}]}
    });
    let result = get_dashboard(&body).await;
    assert_fields_preserved(&body, &result);
    assert_eq!(result["created"], body["created_at"]);
    assert_eq!(result["modified"], body["modified_at"]);
    assert_eq!(
        result["author"],
        json!({"name": "Fixture Author", "handle": "fixture@example.test"})
    );
    assert!(result["template_variables"][0].get("default").is_none());
}

#[tokio::test]
async fn dashboard_optional_settings_preserve_null_and_absence() {
    for body in [
        json!({"title": "Minimal", "layout_type": "ordered", "widgets": []}),
        json!({
            "title": "Legacy", "layout_type": "ordered",
            "author_name": null, "description": null, "tags": null,
            "template_variables": [{"name": "env", "default": "prod", "prefix": null, "available_values": null}],
            "widgets": [{"definition": {"type": "note", "content": "hello", "title": null}, "layout": null}]
        }),
        json!({"title": "Null variables", "layout_type": "free", "widgets": [], "template_variables": null}),
    ] {
        let result = get_dashboard(&body).await;
        assert_fields_preserved(&body, &result);
        for field in [
            "url",
            "id",
            "description",
            "template_variables",
            "author_name",
        ] {
            assert_eq!(result.get(field).is_some(), body.get(field).is_some());
        }
        if let Some(variable) = result["template_variables"].get(0) {
            assert!(variable.get("defaults").is_none());
            assert!(variable.get("type").is_none());
        }
    }
}

#[tokio::test]
async fn dashboard_compatibility_aliases_do_not_replace_api_extensions() {
    let body = json!({
        "title": "Future fields", "layout_type": "ordered", "widgets": [],
        "created_at": "2026-10-17T00:00:00Z", "created": {"retained": true},
        "modified_at": "2026-10-17T01:00:00Z", "modified": null,
        "author_handle": "fixture@example.test", "author": {"retained": true}
    });
    assert_fields_preserved(&body, &get_dashboard(&body).await);
}

#[tokio::test]
async fn dashboard_lists_preserve_summary_fields_and_pagination() {
    let server = MockServer::start().await;
    let summaries = json!([
        {"id": "dashboard-1", "title": "Checkout", "url": "/dashboard/dashboard-1", "author_handle": "fixture@example.test", "created_at": "2026-10-17T00:00:00Z", "description": null, "extension": {"retained": true}},
        {}
    ]);
    Mock::given(method("GET"))
        .and(path("/api/v1/dashboard"))
        .and(query_param("count", "2"))
        .and(query_param("start", "4"))
        .and(query_param("filter[shared]", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"dashboards": summaries})))
        .expect(1)
        .mount(&server)
        .await;
    let output = DashboardsHandler::list(
        &client_for(&server),
        &DashboardsListArgs {
            count: 2,
            start: 4,
            filter_shared: true,
            filter_deleted: false,
        },
    )
    .await
    .unwrap();
    for (original, actual) in summaries
        .as_array()
        .unwrap()
        .iter()
        .zip(output["data"].as_array().unwrap())
    {
        assert_fields_preserved(original, actual);
    }
    assert_eq!(output["data"][0]["created"], summaries[0]["created_at"]);
    assert!(output["data"][1].get("id").is_none());
    assert_eq!(output["pagination"]["next_offset"], 6);
}

#[tokio::test]
async fn dashboard_required_core_fields_still_reject_malformed_responses() {
    for body in [
        json!({"layout_type": "ordered", "widgets": []}),
        json!({"title": "Missing widgets", "layout_type": "ordered"}),
        json!({"title": "Missing widget definition type", "layout_type": "ordered", "widgets": [{"definition": {}}]}),
    ] {
        let server = MockServer::start().await;
        Mock::given(path("/api/v1/dashboard/dashboard-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            client_for(&server).get_dashboard("dashboard-1").await,
            Err(DatadogError::DecodeError(_))
        ));
    }
}
