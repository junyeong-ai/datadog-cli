use chrono::DateTime;
use datadog_cli::catalog::Catalog;
use datadog_cli::query::{PreparedQuery, apply_time_range};
use datadog_cli::time::TimeExpr;
use serde_json::json;

#[test]
fn one_reference_instant_is_encoded_only_at_reviewed_time_fields() {
    let now = DateTime::parse_from_rfc3339("2026-10-19T00:00:00Z")
        .unwrap()
        .to_utc();
    let mut seconds = json!({"query":{"limit":1}});
    let scope = apply_time_range(
        &mut seconds,
        "cloud-network-monitoring",
        "v2.GetAggregatedDns",
        Some(TimeExpr::Ago(3600)),
        None,
        now,
    )
    .unwrap()
    .unwrap();
    assert_eq!(seconds["query"]["from"], now.timestamp() - 3600);
    assert_eq!(seconds["query"]["to"], now.timestamp());
    assert_eq!(scope["encoding"], "unix_seconds");
    let mut milliseconds = json!({"body":{"data":{"type":"timeseries_request","attributes":{"queries":[{"data_source":"metrics","query":"avg:system.cpu.user{*}","name":"a"}]}}}});
    apply_time_range(
        &mut milliseconds,
        "metrics",
        "v2.QueryTimeseriesData",
        Some(TimeExpr::Ago(3600)),
        Some(TimeExpr::Ago(0)),
        now,
    )
    .unwrap();
    assert_eq!(
        milliseconds["body"]["data"]["attributes"]["from"],
        (now.timestamp() - 3600) * 1000
    );
    assert_eq!(
        milliseconds["body"]["data"]["attributes"]["queries"][0]["query"],
        "avg:system.cpu.user{*}"
    );
    PreparedQuery::prepare("metrics", "v2.QueryTimeseriesData", &milliseconds, None).unwrap();
    let mut iso = json!({"body":{"filter":{"query":"service:api"},"page":{"limit":1}}});
    apply_time_range(
        &mut iso,
        "ci-visibility-pipelines",
        "v2.SearchCIAppPipelineEvents",
        Some(TimeExpr::Ago(3600)),
        None,
        now,
    )
    .unwrap();
    assert_eq!(iso["body"]["filter"]["from"], "2026-10-18T23:00:00+00:00");
    assert_eq!(iso["body"]["filter"]["query"], "service:api");
    PreparedQuery::prepare(
        "ci-visibility-pipelines",
        "v2.SearchCIAppPipelineEvents",
        &iso,
        None,
    )
    .unwrap();
}

#[test]
fn range_flags_reject_conflicts_unsupported_mappings_and_reversed_bounds() {
    let now = DateTime::parse_from_rfc3339("2026-10-19T00:00:00Z")
        .unwrap()
        .to_utc();
    let mut input = json!({"query":{"from":1}});
    assert!(
        apply_time_range(
            &mut input,
            "cloud-network-monitoring",
            "v2.GetAggregatedDns",
            Some(TimeExpr::Ago(3600)),
            None,
            now
        )
        .is_err()
    );
    assert!(
        apply_time_range(
            &mut json!({}),
            "teams",
            "v2.ListTeams",
            Some(TimeExpr::Ago(3600)),
            None,
            now
        )
        .is_err()
    );
    assert!(
        apply_time_range(
            &mut json!({}),
            "cloud-network-monitoring",
            "v2.GetAggregatedDns",
            Some(TimeExpr::Ago(0)),
            Some(TimeExpr::Ago(3600)),
            now
        )
        .is_err()
    );
    assert!(
        apply_time_range(
            &mut json!({}),
            "cloud-network-monitoring",
            "v2.GetAggregatedDns",
            None,
            Some(TimeExpr::Ago(0)),
            now
        )
        .is_err()
    );
}

#[test]
fn llm_search_filter_modes_are_enforced_in_both_official_request_envelopes() {
    let catalog = Catalog::load().unwrap();
    let search = catalog
        .query("agent-observability", "v2.SearchLLMObsSpans")
        .unwrap();
    let mut input = json!({"body":{"data":{"type":"spans","attributes":{"filter":{"ml_app":"chat","span_kind":"llm"}}}}});
    assert!(catalog.validate_input(search, &input).is_ok());
    input["body"]["data"]["attributes"]["filter"]["query"] = json!("@ml_app:chat");
    assert!(catalog.validate_input(search, &input).is_err());
    let list = catalog
        .query("agent-observability", "v2.ListLLMObsSpans")
        .unwrap();
    assert!(
        catalog
            .validate_input(
                list,
                &json!({"query":{"filter[query]":"@ml_app:chat","filter[ml_app]":"chat"}})
            )
            .is_err()
    );
}

#[test]
fn capability_discovery_pages_are_bounded_and_have_exact_continuations() {
    let catalog = Catalog::load().unwrap();
    let first = catalog.list(Some("metrics"), false, None, 0, 2).unwrap();
    assert_eq!(first["data"].as_array().unwrap().len(), 2);
    assert_eq!(first["pagination"]["next_offset"], 2);
    let next = catalog.list(Some("metrics"), false, None, 2, 2).unwrap();
    assert_ne!(first["data"][0]["id"], next["data"][0]["id"]);
    let result = catalog
        .list(None, false, Some("SearchCIAppPipelineEvents"), 0, 25)
        .unwrap();
    assert_eq!(result["pagination"]["total_count"], 1);
    assert_eq!(result["data"][0]["id"], "v2.SearchCIAppPipelineEvents");
}
