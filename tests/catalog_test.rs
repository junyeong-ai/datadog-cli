use datadog_cli::catalog::{Access, Catalog};
use serde_json::{Value, json};

#[test]
fn query_contracts_compile_offline() {
    let catalog = Catalog::load().unwrap();
    let mut failures = Vec::new();
    for operation in catalog
        .operations
        .values()
        .filter(|operation| operation.access == Access::Query)
    {
        let schema = catalog.input_schema(operation).unwrap();
        if let Err(error) = jsonschema::options().offline().build(&schema) {
            failures.push(format!("{}: {error}", operation.id));
        }
        for (keyword, value) in schema.as_object().unwrap() {
            if keyword != "allOf" {
                composed_closed_objects(value, &operation.id, &mut failures);
            }
        }
        for response in operation.responses.values() {
            if let Some(schema) = &response.schema {
                let schema = catalog.response_schema(operation, schema.clone()).unwrap();
                if let Err(error) = jsonschema::options()
                    .with_draft(jsonschema::Draft::Draft7)
                    .offline()
                    .build(&schema)
                {
                    failures.push(format!("{} response: {error}", operation.id));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Closed request objects reject properties that composed subschemas declare.
fn composed_closed_objects(schema: &Value, operation: &str, failures: &mut Vec<String>) {
    match schema {
        Value::Object(fields) => {
            if fields.contains_key("allOf")
                || fields.contains_key("properties")
                    && (fields.contains_key("anyOf") || fields.contains_key("oneOf"))
            {
                failures.push(format!("{operation}: composed request object"));
            }
            fields
                .values()
                .for_each(|value| composed_closed_objects(value, operation, failures));
        }
        Value::Array(values) => values
            .iter()
            .for_each(|value| composed_closed_objects(value, operation, failures)),
        _ => {}
    }
}

#[test]
fn queries_require_declared_parameters_without_exposing_values() {
    let catalog = Catalog::load().unwrap();
    let operation = catalog.query("apm", "v2.GetServiceList").unwrap();
    assert!(
        catalog
            .validate_input(operation, &json!({"query":{"filter[env]":"prod"}}))
            .is_ok()
    );
    for invalid in [
        json!({}),
        json!({"query":{"filter[env]":false}}),
        json!({"query":{"filter[env]":"prod","extra":"sensitive-value"}}),
    ] {
        let error = catalog.validate_input(operation, &invalid).unwrap_err();
        assert!(!error.to_string().contains("sensitive-value"));
    }
}

#[test]
fn post_searches_are_queries_and_get_actions_are_excluded() {
    let catalog = Catalog::load().unwrap();
    assert!(
        catalog
            .query("ci-visibility-pipelines", "v2.SearchCIAppPipelineEvents")
            .is_ok()
    );
    assert!(
        catalog
            .query("security-monitoring", "v2.SearchSecurityFindings")
            .is_ok()
    );
    for id in [
        "v1.GetGraphSnapshot",
        "v2.GenerateCostTagDescriptionByKey",
        "v2.GetSPARecommendations",
        "v2.CreateOnCallPage",
        "v2.GetAPIKey",
    ] {
        let operation = catalog.operation(id).unwrap();
        assert!(
            catalog
                .query(&datadog_cli::catalog::domain_id(&operation.domains[0]), id)
                .is_err()
        );
        assert_ne!(catalog.operation(id).unwrap().access, Access::Query);
    }
    let error = catalog
        .query("metrics", "v2.SearchCIAppPipelineEvents")
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("its domains: \"ci-visibility-pipelines\"")
    );
}

#[test]
fn numeric_limits_and_required_bodies_follow_the_api_contract() {
    let catalog = Catalog::load().unwrap();
    let teams = catalog.query("teams", "v2.ListTeams").unwrap();
    assert!(
        catalog
            .validate_input(teams, &json!({"query":{"page[size]":1}}))
            .is_ok()
    );
    assert!(
        catalog
            .validate_input(teams, &json!({"query":{"page[size]":101}}))
            .is_err()
    );
    let query = catalog
        .query("ddsql", "v2.ExecuteDdsqlTabularQuery")
        .unwrap();
    assert!(catalog.validate_input(query, &json!({})).is_err());
    let mut input = json!({"body":{"data":{"type":"ddsql_query_request","attributes":{"query":"SELECT 1","time":{"from_timestamp":1,"to_timestamp":2},"row_limit":1}}}});
    assert!(catalog.validate_input(query, &input).is_ok());
    input["body"]["data"]["attributes"]["row_limit"] = json!(10001);
    let error = catalog.validate_input(query, &input).unwrap_err();
    assert!(error.to_string().contains("maximum"));
}

#[test]
fn credential_bearing_contracts_are_excluded_before_request_validation() {
    let catalog = Catalog::load().unwrap();
    for id in [
        "v1.ListAzureIntegration",
        "v1.GetWebhooksIntegration",
        "v2.GetActionConnection",
        "v1.GetAPITest",
        "v1.GetAPITestResult",
        "v1.GetBrowserTest",
        "v1.GetBrowserTestResult",
        "v1.GetTest",
        "v1.ListTests",
        "v1.SearchTests",
        "v2.GetSyntheticsTestVersion",
    ] {
        let operation = catalog.operation(id).unwrap();
        assert_eq!(operation.access, Access::CredentialMaterial);
        let domain = datadog_cli::catalog::domain_id(&operation.domains[0]);
        assert!(
            datadog_cli::query::PreparedQuery::prepare(&domain, id, &json!({}), None)
                .err()
                .unwrap()
                .to_string()
                .contains("credential-bearing")
        );
    }
    for id in [
        "v1.GetAPITestLatestResults",
        "v1.GetBrowserTestLatestResults",
        "v2.GetSyntheticsTestResult",
        "v2.GetSyntheticsBrowserTestResult",
        "v2.ListSyntheticsTestLatestResults",
    ] {
        assert!(catalog.query("synthetics", id).is_ok());
    }
}

#[test]
fn monitor_validation_does_not_require_response_only_fields() {
    let catalog = Catalog::load().unwrap();
    let mut input = json!({"body":{"type":"query alert","query":"avg(last_5m):avg:system.cpu.user{*}>90","matching_downtimes":[{}]}});
    catalog
        .validate_input(
            catalog.query("monitors", "v1.ValidateMonitor").unwrap(),
            &input,
        )
        .unwrap();
    input["path"] = json!({"monitor_id":1});
    catalog
        .validate_input(
            catalog
                .query("monitors", "v1.ValidateExistingMonitor")
                .unwrap(),
            &input,
        )
        .unwrap();
    input["body"].as_object_mut().unwrap().remove("query");
    assert!(
        catalog
            .validate_input(
                catalog
                    .query("monitors", "v1.ValidateExistingMonitor")
                    .unwrap(),
                &input
            )
            .is_err()
    );
}

#[test]
fn domain_counts_are_disjoint_and_hide_query_free_domains_by_default() {
    let catalog = Catalog::load().unwrap();
    let all = catalog.domains(true)["data"].as_array().unwrap().clone();
    let reviewed = catalog.domains(false)["data"].as_array().unwrap().clone();
    let operations = catalog.operations.values();
    let total = |key: &str| {
        all.iter()
            .map(|domain| domain[key].as_u64().unwrap())
            .sum::<u64>()
    };
    assert_eq!(
        total("queries") + total("excluded"),
        operations
            .map(|operation| operation.domains.len() as u64)
            .sum::<u64>()
    );
    assert!(
        reviewed
            .iter()
            .all(|domain| domain["queries"].as_u64() > Some(0))
    );
    assert_eq!(
        reviewed.len(),
        all.iter().filter(|domain| domain["queries"] != 0).count()
    );
    for domain in &all {
        let mut keys = domain.as_object().unwrap().keys().collect::<Vec<_>>();
        keys.sort();
        assert_eq!(keys, ["excluded", "id", "preview_queries", "queries"]);
    }
}
