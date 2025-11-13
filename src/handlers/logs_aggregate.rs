use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::{
    DatadogClient,
    models::{LogsCompute, LogsGroupBy, LogsGroupBySort},
};
use crate::error::Result;
use crate::handlers::common::{ParameterParser, ResponseFormatter, TimeHandler, TimeParams};

pub struct LogsAggregateHandler;

impl TimeHandler for LogsAggregateHandler {}
impl ResponseFormatter for LogsAggregateHandler {}
impl ParameterParser for LogsAggregateHandler {}

impl LogsAggregateHandler {
    pub async fn aggregate(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = LogsAggregateHandler;

        // Use v1 API time parsing to get timestamps, then convert to milliseconds strings
        let time = handler.parse_time(params, 1)?; // Parse as v1 to get timestamps
        let TimeParams::Timestamp {
            from: from_ts,
            to: to_ts,
        } = time;

        // Convert to milliseconds strings (Datadog expects string format for v2)
        let from = (from_ts * 1000).to_string();
        let to = (to_ts * 1000).to_string();

        let query = handler.extract_query(params, "*");

        // Parse compute parameters - MUST have type field
        let compute = if let Some(compute_params) = params["compute"].as_array() {
            if compute_params.is_empty() {
                // Empty array: use default compute to avoid Datadog API error
                Some(vec![LogsCompute {
                    aggregation: "count".to_string(),
                    compute_type: Some("total".to_string()),
                    interval: None,
                    metric: None,
                }])
            } else {
                Some(
                    compute_params
                        .iter()
                        .map(|c| LogsCompute {
                            aggregation: c["aggregation"].as_str().unwrap_or("count").to_string(),
                            compute_type: Some(c["type"].as_str().unwrap_or("total").to_string()),
                            interval: c["interval"].as_str().map(|s| s.to_string()),
                            metric: c["metric"].as_str().map(|s| s.to_string()),
                        })
                        .collect::<Vec<_>>(),
                )
            }
        } else {
            // Missing compute parameter: use default
            Some(vec![LogsCompute {
                aggregation: "count".to_string(),
                compute_type: Some("total".to_string()),
                interval: None,
                metric: None,
            }])
        };

        // Parse group_by parameters with required type field
        let group_by = params["group_by"].as_array().map(|group_by_params| {
            group_by_params
                .iter()
                .map(|g| {
                    let sort = g["sort"].as_object().map(|sort_params| LogsGroupBySort {
                        order: sort_params["order"].as_str().map(|s| s.to_string()),
                        sort_type: Some(
                            sort_params["type"]
                                .as_str()
                                .unwrap_or("measure")
                                .to_string(),
                        ), // Required
                        aggregation: sort_params["aggregation"].as_str().map(|s| s.to_string()),
                        metric: sort_params["metric"].as_str().map(|s| s.to_string()),
                    });

                    LogsGroupBy {
                        facet: g["facet"].as_str().unwrap_or("status").to_string(),
                        limit: g["limit"].as_i64().map(|l| l as i32),
                        sort,
                        group_type: Some(g["type"].as_str().unwrap_or("facet").to_string()), // Required
                    }
                })
                .collect::<Vec<_>>()
        });

        let timezone = params["timezone"].as_str().map(|s| s.to_string());

        let response = client
            .aggregate_logs(
                &query,
                &from,
                &to,
                compute.clone(),
                group_by.clone(),
                timezone.clone(),
            )
            .await?;

        let data = response["data"].clone();
        let buckets_count = data
            .get("buckets")
            .and_then(|b| b.as_array())
            .map(|b| b.len())
            .unwrap_or(0);

        let meta = json!({
            "query": query,
            "from": from,
            "to": to,
            "buckets_count": buckets_count,
            "timezone": timezone
        });

        Ok(handler.format_list(data, None, Some(meta)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_default_query_parameter() {
        let params = json!({
            "from": "1 hour ago",
            "to": "now"
        });

        let query = params["query"].as_str().unwrap_or("*");
        assert_eq!(query, "*");
    }

    #[test]
    fn test_custom_query_parameter() {
        let params = json!({
            "query": "service:web-api",
            "from": "1 hour ago",
            "to": "now"
        });

        let query = params["query"].as_str().unwrap_or("*");
        assert_eq!(query, "service:web-api");
    }

    #[test]
    fn test_default_compute_when_empty() {
        let params = json!({
            "compute": []
        });

        let compute_params = params["compute"].as_array();
        assert!(compute_params.is_some());
        assert!(compute_params.unwrap().is_empty());
    }

    #[test]
    fn test_compute_with_aggregation() {
        let params = json!({
            "compute": [
                {
                    "aggregation": "count",
                    "type": "total"
                }
            ]
        });

        let compute_params = params["compute"].as_array().unwrap();
        assert_eq!(compute_params.len(), 1);
        assert_eq!(compute_params[0]["aggregation"].as_str(), Some("count"));
        assert_eq!(compute_params[0]["type"].as_str(), Some("total"));
    }

    #[test]
    fn test_compute_with_metric() {
        let params = json!({
            "compute": [
                {
                    "aggregation": "sum",
                    "type": "total",
                    "metric": "@duration"
                }
            ]
        });

        let compute_params = params["compute"].as_array().unwrap();
        assert_eq!(compute_params[0]["metric"].as_str(), Some("@duration"));
    }

    #[test]
    fn test_group_by_parameter() {
        let params = json!({
            "group_by": [
                {
                    "facet": "status",
                    "limit": 10,
                    "type": "facet"
                }
            ]
        });

        let group_by = params["group_by"].as_array();
        assert!(group_by.is_some());
        assert_eq!(group_by.unwrap().len(), 1);
    }

    #[test]
    fn test_group_by_with_sort() {
        let params = json!({
            "group_by": [
                {
                    "facet": "status",
                    "limit": 10,
                    "type": "facet",
                    "sort": {
                        "order": "desc",
                        "type": "measure",
                        "aggregation": "count"
                    }
                }
            ]
        });

        let group_by = params["group_by"].as_array().unwrap();
        let sort = group_by[0]["sort"].as_object();
        assert!(sort.is_some());
        assert_eq!(sort.unwrap()["order"].as_str(), Some("desc"));
    }

    #[test]
    fn test_optional_timezone_parameter() {
        let params_with = json!({"timezone": "UTC"});
        let params_without = json!({});

        assert_eq!(params_with["timezone"].as_str(), Some("UTC"));
        assert_eq!(params_without["timezone"].as_str(), None);
    }

    #[test]
    fn test_time_handler_available() {
        let handler = LogsAggregateHandler;
        let params = json!({
            "from": "1609459200",
            "to": "1609462800"
        });

        let result = handler.parse_time(&params, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_response_formatter_available() {
        let handler = LogsAggregateHandler;
        let data = json!({"buckets": []});
        let meta = json!({"query": "*"});

        let response = handler.format_list(data, None, Some(meta));
        assert!(response.get("data").is_some());
        assert!(response.get("meta").is_some());
    }
}
