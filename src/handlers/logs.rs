use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::datadog::models::{LogsCompute, LogsGroupBy, LogsGroupBySort};
use crate::error::Result;
use crate::handlers::common::{
    PaginationInfo, ParameterParser, ResponseFilter, ResponseFormatter, TagFilter, TimeHandler,
    TimeParams,
};

pub struct LogsHandler;

impl TimeHandler for LogsHandler {}
impl TagFilter for LogsHandler {}
impl ResponseFilter for LogsHandler {}
impl ResponseFormatter for LogsHandler {}
impl ParameterParser for LogsHandler {}

impl LogsHandler {
    pub async fn search(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = LogsHandler;

        let query = params["query"].as_str().ok_or_else(|| {
            crate::error::DatadogError::InvalidInput("Missing 'query' parameter".to_string())
        })?;

        let limit = handler.extract_i32(params, "limit", 10);
        let cursor = handler.extract_string(params, "cursor");
        let sort = handler.extract_string(params, "sort");

        let (from_iso, to_iso) = handler.parse_time_iso8601(params)?;

        let response = client
            .search_logs(query, &from_iso, &to_iso, limit, cursor, sort)
            .await?;

        if let Some(errors) = response.errors {
            return Err(crate::error::DatadogError::ApiError(errors.join(", ")));
        }

        let tag_filter = handler.extract_tag_filter(params, &client);

        let logs: Vec<Value> = response
            .data
            .unwrap_or_default()
            .iter()
            .map(|log| {
                let attrs = log.attributes.as_ref();
                let tags = attrs
                    .and_then(|a| a.tags.as_ref())
                    .map(|t| handler.filter_tags(t, tag_filter));

                let mut entry = json!({ "id": log.id });

                if let Some(timestamp) = attrs.and_then(|a| a.timestamp.as_ref()) {
                    entry["timestamp"] = json!(timestamp);
                }
                if let Some(message) = attrs.and_then(|a| a.message.as_ref()) {
                    entry["message"] = json!(message);
                }
                if let Some(host) = attrs.and_then(|a| a.host.as_ref()) {
                    entry["host"] = json!(host);
                }
                if let Some(service) = attrs.and_then(|a| a.service.as_ref()) {
                    entry["service"] = json!(service);
                }
                if let Some(status) = attrs.and_then(|a| a.status.as_ref()) {
                    entry["status"] = json!(status);
                }
                if let Some(tags_vec) = tags
                    && !tags_vec.is_empty()
                {
                    entry["tags"] = json!(tags_vec);
                }

                entry
            })
            .collect();

        let has_cursor = response
            .meta
            .as_ref()
            .and_then(|m| m.page.as_ref())
            .and_then(|p| p.after.as_ref())
            .is_some();

        let pagination = PaginationInfo::from_cursor(logs.len(), limit as usize, has_cursor);

        Ok(json!({
            "data": logs,
            "pagination": pagination
        }))
    }

    pub async fn aggregate(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = LogsHandler;

        let time = handler.parse_time(params, 1)?;
        let TimeParams::Timestamp {
            from: from_ts,
            to: to_ts,
        } = time;

        let from = (from_ts * 1000).to_string();
        let to = (to_ts * 1000).to_string();

        let query = handler.extract_query(params, "*");

        let compute = if let Some(compute_params) = params["compute"].as_array() {
            if compute_params.is_empty() {
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
                        .collect(),
                )
            }
        } else {
            Some(vec![LogsCompute {
                aggregation: "count".to_string(),
                compute_type: Some("total".to_string()),
                interval: None,
                metric: None,
            }])
        };

        let group_by = params["group_by"].as_array().map(|arr| {
            arr.iter()
                .map(|g| {
                    let sort = g["sort"].as_object().map(|s| LogsGroupBySort {
                        order: s["order"].as_str().map(|v| v.to_string()),
                        sort_type: Some(s["type"].as_str().unwrap_or("measure").to_string()),
                        aggregation: s["aggregation"].as_str().map(|v| v.to_string()),
                        metric: s["metric"].as_str().map(|v| v.to_string()),
                    });

                    LogsGroupBy {
                        facet: g["facet"].as_str().unwrap_or("status").to_string(),
                        limit: g["limit"].as_i64().map(|l| l as i32),
                        sort,
                        group_type: Some(g["type"].as_str().unwrap_or("facet").to_string()),
                    }
                })
                .collect()
        });

        let timezone = params["timezone"].as_str().map(|s| s.to_string());

        let response = client
            .aggregate_logs(&query, &from, &to, compute, group_by, timezone.clone())
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
            "timezone": timezone,
            "buckets_count": buckets_count
        });

        Ok(handler.format_list(data, None, Some(meta)))
    }

    pub async fn timeseries(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = LogsHandler;

        let time = handler.parse_time(params, 1)?;
        let TimeParams::Timestamp {
            from: from_ts,
            to: to_ts,
        } = time;

        let from = (from_ts * 1000).to_string();
        let to = (to_ts * 1000).to_string();

        let query = handler.extract_query(params, "*");
        let interval = params["interval"].as_str().unwrap_or("1h");
        let metric = handler.extract_string(params, "metric");
        let aggregation = params["aggregation"].as_str().unwrap_or("count");
        let timezone = params["timezone"].as_str().map(|s| s.to_string());

        let compute = vec![LogsCompute {
            aggregation: aggregation.to_string(),
            compute_type: Some("timeseries".to_string()),
            interval: Some(interval.to_string()),
            metric,
        }];

        let group_by = params["group_by"].as_array().map(|arr| {
            arr.iter()
                .map(|g| LogsGroupBy {
                    facet: g["facet"].as_str().unwrap_or("status").to_string(),
                    limit: g["limit"].as_i64().map(|l| l as i32),
                    sort: None,
                    group_type: Some(g["type"].as_str().unwrap_or("facet").to_string()),
                })
                .collect()
        });

        let response = client
            .aggregate_logs(
                &query,
                &from,
                &to,
                Some(compute),
                group_by,
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
            "interval": interval,
            "aggregation": aggregation,
            "timezone": timezone,
            "buckets_count": buckets_count
        });

        Ok(handler.format_list(data, None, Some(meta)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_handler_trait() {
        let handler = LogsHandler;
        let params = json!({
            "from": "1609459200",
            "to": "1609462800"
        });

        let result = handler.parse_time(&params, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_response_formatter_trait() {
        let handler = LogsHandler;
        let data = json!([{"id": "log1"}]);
        let formatted = handler.format_list(data, None, None);
        assert!(formatted.get("data").is_some());
    }

    #[test]
    fn test_tag_filter_modes() {
        let handler = LogsHandler;
        let tags = vec!["env:prod".to_string(), "service:api".to_string()];

        assert_eq!(handler.filter_tags(&tags, "*").len(), 2);
        assert_eq!(handler.filter_tags(&tags, "env:").len(), 1);
        assert_eq!(handler.filter_tags(&tags, "").len(), 0);
    }
}
