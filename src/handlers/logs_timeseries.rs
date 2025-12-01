use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::{
    DatadogClient,
    models::{LogsCompute, LogsGroupBy},
};
use crate::error::Result;
use crate::handlers::common::{ParameterParser, ResponseFormatter, TimeHandler, TimeParams};

pub struct LogsTimeseriesHandler;

impl TimeHandler for LogsTimeseriesHandler {}
impl ResponseFormatter for LogsTimeseriesHandler {}
impl ParameterParser for LogsTimeseriesHandler {}

impl LogsTimeseriesHandler {
    pub async fn timeseries(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = LogsTimeseriesHandler;

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
        let interval = params["interval"].as_str().unwrap_or("1h");
        let metric = handler.extract_string(params, "metric");
        let aggregation = params["aggregation"].as_str().unwrap_or("count");
        let timezone = params["timezone"].as_str().map(|s| s.to_string());

        // Build meta first to avoid cloning metric and timezone
        let meta_base = json!({
            "query": &query,
            "from": &from,
            "to": &to,
            "interval": interval,
            "aggregation": aggregation,
            "metric": &metric,
            "timezone": &timezone
        });

        // Create timeseries compute with required type field (after meta_base to avoid clone)
        let compute = vec![LogsCompute {
            aggregation: aggregation.to_string(),
            compute_type: Some("timeseries".to_string()),
            interval: Some(interval.to_string()),
            metric,
        }];

        // Parse group_by if provided with required type field
        let group_by = params["group_by"].as_array().map(|group_by_params| {
            group_by_params
                .iter()
                .map(|g| LogsGroupBy {
                    facet: g["facet"].as_str().unwrap_or("status").to_string(),
                    limit: g["limit"].as_i64().map(|l| l as i32),
                    sort: None,
                    group_type: Some(g["type"].as_str().unwrap_or("facet").to_string()),
                })
                .collect::<Vec<_>>()
        });

        let response = client
            .aggregate_logs(&query, &from, &to, Some(compute), group_by, timezone)
            .await?;

        let data = response["data"].clone();
        let buckets_count = data
            .get("buckets")
            .and_then(|b| b.as_array())
            .map(|b| b.len())
            .unwrap_or(0);

        let mut meta = meta_base;
        meta["buckets_count"] = json!(buckets_count);

        Ok(handler.format_list(data, None, Some(meta)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_default_interval() {
        let params = json!({});
        let interval = params["interval"].as_str().unwrap_or("1h");
        assert_eq!(interval, "1h");
    }

    #[test]
    fn test_custom_interval() {
        let params = json!({"interval": "5m"});
        let interval = params["interval"].as_str().unwrap_or("1h");
        assert_eq!(interval, "5m");
    }

    #[test]
    fn test_default_aggregation() {
        let params = json!({});
        let aggregation = params["aggregation"].as_str().unwrap_or("count");
        assert_eq!(aggregation, "count");
    }

    #[test]
    fn test_custom_aggregation() {
        let params = json!({"aggregation": "avg"});
        let aggregation = params["aggregation"].as_str().unwrap_or("count");
        assert_eq!(aggregation, "avg");
    }

    #[test]
    fn test_optional_metric_parameter() {
        let params_with = json!({"metric": "@duration"});
        let params_without = json!({});

        assert_eq!(params_with["metric"].as_str(), Some("@duration"));
        assert_eq!(params_without["metric"].as_str(), None);
    }

    #[test]
    fn test_group_by_parameter() {
        let params = json!({
            "group_by": [
                {
                    "facet": "service",
                    "limit": 5,
                    "type": "facet"
                }
            ]
        });

        let group_by = params["group_by"].as_array();
        assert!(group_by.is_some());
        assert_eq!(group_by.unwrap().len(), 1);
    }

    #[test]
    fn test_timezone_parameter() {
        let params = json!({"timezone": "America/New_York"});
        assert_eq!(params["timezone"].as_str(), Some("America/New_York"));
    }

    #[test]
    fn test_default_query() {
        let params = json!({});
        let query = params["query"].as_str().unwrap_or("*");
        assert_eq!(query, "*");
    }

    #[test]
    fn test_time_handler_trait() {
        let handler = LogsTimeseriesHandler;
        let params = json!({
            "from": "1 hour ago",
            "to": "now"
        });

        let result = handler.parse_time(&params, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_response_formatter_trait() {
        let handler = LogsTimeseriesHandler;
        let data = json!({"buckets": []});
        let meta = json!({"interval": "1h"});

        let response = handler.format_list(data, None, Some(meta));
        assert!(response.get("data").is_some());
        assert!(response.get("meta").is_some());
    }
}
