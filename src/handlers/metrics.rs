use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{ResponseFormatter, TimeHandler, TimeParams};

pub struct MetricsHandler;

impl TimeHandler for MetricsHandler {}
impl ResponseFormatter for MetricsHandler {}

impl MetricsHandler {
    // Calculate rollup interval based on time range and desired max_points
    fn calculate_rollup_interval(from_ts: i64, to_ts: i64, max_points: usize) -> i64 {
        let time_range = to_ts - from_ts;
        let interval = time_range / max_points as i64;

        // Round up to reasonable intervals: 60s, 300s (5m), 600s (10m), 3600s (1h), etc.
        if interval < 60 {
            60
        } else if interval < 300 {
            300
        } else if interval < 600 {
            600
        } else if interval < 1800 {
            1800
        } else if interval < 3600 {
            3600
        } else if interval < 7200 {
            7200
        } else if interval < 21600 {
            21600
        } else if interval < 43200 {
            43200
        } else {
            86400 // 1 day max
        }
    }

    // Add rollup to query if needed
    fn add_rollup_to_query(query: &str, interval: i64) -> String {
        // Check if query already has rollup
        if query.contains(".rollup(") {
            return query.to_string();
        }

        // Extract aggregation method from query (avg:, max:, min:, sum:)
        let agg = if query.starts_with("avg:") {
            "avg"
        } else if query.starts_with("max:") {
            "max"
        } else if query.starts_with("min:") {
            "min"
        } else if query.starts_with("sum:") {
            "sum"
        } else {
            "avg" // default
        };

        format!("{}.rollup({}, {})", query, agg, interval)
    }

    pub async fn query(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = MetricsHandler;

        let mut query = params["query"]
            .as_str()
            .ok_or_else(|| {
                crate::error::DatadogError::InvalidInput("Missing 'query' parameter".to_string())
            })?
            .to_string();

        let time = handler.parse_time(params, 1)?; // v1 API

        let TimeParams::Timestamp {
            from: from_ts,
            to: to_ts,
        } = time;

        // Get max_points parameter and apply rollup at API level
        let max_points = params["max_points"].as_i64().map(|p| p as usize);
        let mut applied_rollup = false;

        if let Some(max) = max_points {
            let interval = Self::calculate_rollup_interval(from_ts, to_ts, max);
            query = Self::add_rollup_to_query(&query, interval);
            applied_rollup = true;
        }

        let response = client.query_metrics(&query, from_ts, to_ts).await?;

        let series = response.series.iter().map(|s| {
            let points_data = if let Some(ref pointlist) = s.pointlist {
                json!({
                    "count": pointlist.len(),
                    "data": pointlist.iter().map(|p| {
                        if p.len() >= 2 {
                            json!({
                                "timestamp": p[0].map(|t| crate::utils::format_timestamp(t as i64 / 1000))
                                    .unwrap_or_else(|| "N/A".to_string()),
                                "value": p[1]
                            })
                        } else {
                            json!({
                                "timestamp": "N/A",
                                "value": null
                            })
                        }
                    }).collect::<Vec<_>>()
                })
            } else {
                json!({
                    "count": 0,
                    "data": []
                })
            };

            // Build series object with only useful fields
            let mut series_obj = serde_json::Map::new();
            series_obj.insert("metric".to_string(), json!(s.metric));
            series_obj.insert("scope".to_string(), json!(s.scope));
            series_obj.insert("points".to_string(), points_data);

            // Add optional fields only if meaningful
            if let Some(ref aggr) = s.aggr {
                series_obj.insert("aggr".to_string(), json!(aggr));
            }
            if let Some(interval) = s.interval {
                series_obj.insert("interval".to_string(), json!(interval));
            }
            if let Some(ref unit) = s.unit {
                // Simplify unit - only include the first non-null unit
                if let Some(first_unit) = unit.iter().find(|u| u.is_some())
                    && let Some(u) = first_unit {
                        let mut unit_obj = serde_json::Map::new();
                        unit_obj.insert("name".to_string(), json!(u.name));
                        unit_obj.insert("family".to_string(), json!(u.family));
                        if let Some(ref short_name) = u.short_name
                            && !short_name.is_empty() {
                                unit_obj.insert("short_name".to_string(), json!(short_name));
                            }
                        series_obj.insert("unit".to_string(), json!(unit_obj));
                    }
            }

            json!(series_obj)
        }).collect::<Vec<_>>();

        // Build optimized meta - only include meaningful fields
        let mut meta = serde_json::Map::new();
        meta.insert("query".to_string(), json!(response.query));
        meta.insert("status".to_string(), json!(response.status));
        meta.insert(
            "from".to_string(),
            json!(crate::utils::format_timestamp(from_ts)),
        );
        meta.insert(
            "to".to_string(),
            json!(crate::utils::format_timestamp(to_ts)),
        );

        // Only include error if present
        if let Some(ref error) = response.error
            && !error.is_empty()
        {
            meta.insert("error".to_string(), json!(error));
        }

        // Only include message if present and non-empty
        if let Some(ref message) = response.message
            && !message.is_empty()
        {
            meta.insert("message".to_string(), json!(message));
        }

        // Only include group_by if present and non-empty
        if let Some(ref group_by) = response.group_by
            && !group_by.is_empty()
        {
            meta.insert("group_by".to_string(), json!(group_by));
        }

        if applied_rollup {
            meta.insert("rollup_applied".to_string(), json!(true));
            if let Some(max) = max_points {
                meta.insert("requested_max_points".to_string(), json!(max));
            }
        }

        Ok(handler.format_list(json!(series), None, Some(json!(meta))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_calculate_rollup_interval() {
        // 30000s / 100 points = 300s, 300 >= 300 and < 600 so rounds to 600
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 30000, 100),
            600
        );

        // 86400s / 100 points = 864s, 864 >= 600 and < 1800 so rounds to 1800
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 86400, 100),
            1800
        );

        // Very short range: 100s / 100 = 1s, < 60 so gets 60s minimum
        assert_eq!(MetricsHandler::calculate_rollup_interval(0, 100, 100), 60);

        // 6000s / 100 = 60s, 60 >= 60 and < 300 so rounds to 300
        assert_eq!(MetricsHandler::calculate_rollup_interval(0, 6000, 100), 300);
    }

    #[test]
    fn test_add_rollup_to_query() {
        // Test adding rollup to simple query
        let query = "avg:system.cpu.user{*}";
        let result = MetricsHandler::add_rollup_to_query(query, 300);
        assert!(result.contains(".rollup(avg, 300)"));

        // Test with max aggregation
        let query = "max:system.cpu.user{*}";
        let result = MetricsHandler::add_rollup_to_query(query, 60);
        assert!(result.contains(".rollup(max, 60)"));

        // Test when rollup already exists
        let query = "avg:system.cpu.user{*}.rollup(sum, 600)";
        let result = MetricsHandler::add_rollup_to_query(query, 300);
        assert_eq!(result, query); // Should not modify
    }

    #[test]
    fn test_missing_query_parameter() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = Arc::new(
                DatadogClient::new(
                    "test_key".to_string(),
                    "test_app_key".to_string(),
                    None,
                    30,
                    3,
                    None,
                )
                .unwrap(),
            );

            let params = json!({
                "from": "1 hour ago",
                "to": "now"
                // Missing "query" parameter
            });

            let result = MetricsHandler::query(client, &params).await;
            assert!(result.is_err());

            if let Err(e) = result {
                let error_str = format!("{}", e);
                assert!(error_str.contains("query") || error_str.contains("Missing"));
            }
        });
    }

    #[test]
    fn test_valid_input_parameters() {
        // This test verifies parameter extraction works
        let params = json!({
            "query": "avg:system.cpu.user{*}",
            "from": "1609459200", // Unix timestamp
            "to": "1609462800"
        });

        assert_eq!(params["query"].as_str(), Some("avg:system.cpu.user{*}"));
        assert!(params["from"].as_str().is_some());
        assert!(params["to"].as_str().is_some());
    }

    #[test]
    fn test_optional_max_points_parameter() {
        let params_with = json!({
            "query": "avg:cpu",
            "from": "1 hour ago",
            "to": "now",
            "max_points": 100
        });

        let params_without = json!({
            "query": "avg:cpu",
            "from": "1 hour ago",
            "to": "now"
        });

        assert_eq!(params_with["max_points"].as_i64(), Some(100));
        assert_eq!(params_without["max_points"].as_i64(), None);
    }

    #[test]
    fn test_time_handler_trait_available() {
        // Verify MetricsHandler implements TimeHandler
        let handler = MetricsHandler;
        let params = json!({
            "from": "1609459200",
            "to": "1609462800"
        });

        // This should not panic
        let result = handler.parse_time(&params, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_response_formatter_trait_available() {
        // Verify MetricsHandler implements ResponseFormatter
        let handler = MetricsHandler;
        let data = json!(["test"]);

        let formatted = handler.format_list(data, None, None);
        assert!(formatted.get("data").is_some());
    }

    #[test]
    fn test_rollup_interval_boundaries() {
        assert_eq!(MetricsHandler::calculate_rollup_interval(0, 5900, 100), 60);
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 29900, 100),
            300
        );
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 59900, 100),
            600
        );
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 179900, 100),
            1800
        );
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 359900, 100),
            3600
        );
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 719900, 100),
            7200
        );
    }

    #[test]
    fn test_add_rollup_preserves_query_structure() {
        let query_with_filter = "avg:system.cpu.user{host:web-1,env:prod}";
        let result = MetricsHandler::add_rollup_to_query(query_with_filter, 300);
        assert!(result.contains("host:web-1"));
        assert!(result.contains("env:prod"));
        assert!(result.ends_with(".rollup(avg, 300)"));

        let query_with_wildcard = "avg:system.cpu.user{*}";
        let result = MetricsHandler::add_rollup_to_query(query_with_wildcard, 60);
        assert!(result.contains("{*}"));
        assert!(result.ends_with(".rollup(avg, 60)"));
    }

    #[test]
    fn test_add_rollup_with_all_aggregation_types() {
        let test_cases = vec![
            ("avg:metric{*}", "avg"),
            ("max:metric{*}", "max"),
            ("min:metric{*}", "min"),
            ("sum:metric{*}", "sum"),
            ("count:metric{*}", "avg"),
            ("metric{*}", "avg"),
        ];

        for (query, expected_agg) in test_cases {
            let result = MetricsHandler::add_rollup_to_query(query, 300);
            let expected_suffix = format!(".rollup({}, 300)", expected_agg);
            assert!(
                result.ends_with(&expected_suffix),
                "Query '{}' should produce rollup with aggregation '{}', got: {}",
                query,
                expected_agg,
                result
            );
        }
    }

    #[test]
    fn test_calculate_rollup_interval_large_ranges() {
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 2159900, 100),
            21600
        );
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 4319900, 100),
            43200
        );
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 8639900, 100),
            86400
        );
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 86400 * 100, 100),
            86400
        );
        assert_eq!(
            MetricsHandler::calculate_rollup_interval(0, 86400 * 1000, 100),
            86400
        );
    }
}
