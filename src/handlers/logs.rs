use serde_json::{Value, json};

use crate::cli::{LogsAggregateArgs, LogsSearchArgs, LogsTimeseriesArgs};
use crate::datadog::models::LogsCompute;
use crate::datadog::{DatadogClient, SearchParams};
use crate::error::Result;
use crate::handlers::common::{PaginationInfo, ResponseFormatter, TagFilter, TimeHandler};

pub struct LogsHandler;

impl TimeHandler for LogsHandler {}
impl TagFilter for LogsHandler {}
impl ResponseFormatter for LogsHandler {}

impl LogsHandler {
    pub async fn search(client: &DatadogClient, args: &LogsSearchArgs) -> Result<Value> {
        let handler = LogsHandler;

        let (from_iso, to_iso) = handler.parse_time_range_iso8601(&args.from, &args.to)?;

        let response = client
            .search_logs(
                &SearchParams {
                    query: &args.query,
                    from: &from_iso,
                    to: &to_iso,
                    limit: args.limit,
                    cursor: args.cursor.as_deref(),
                    sort: args.sort.as_deref(),
                },
                args.storage_tier.as_deref(),
            )
            .await?;

        let tag_filter = handler.resolve_tag_filter(args.tag_filter.as_deref(), client);

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

        let next_cursor = response
            .meta
            .as_ref()
            .and_then(|m| m.page.as_ref())
            .and_then(|p| p.after.clone());

        let pagination = PaginationInfo::from_cursor(logs.len(), args.limit as usize, next_cursor);

        Ok(json!({
            "data": logs,
            "pagination": pagination
        }))
    }

    pub async fn aggregate(client: &DatadogClient, args: &LogsAggregateArgs) -> Result<Value> {
        let handler = LogsHandler;

        let (from_ts, to_ts) = handler.parse_time_range(&args.from, &args.to)?;
        let from = (from_ts * 1000).to_string();
        let to = (to_ts * 1000).to_string();

        let compute = vec![LogsCompute {
            aggregation: "count".to_string(),
            compute_type: Some("total".to_string()),
            interval: None,
            metric: None,
        }];

        let response = client
            .aggregate_logs(&args.query, &from, &to, compute)
            .await?;

        let data = response["data"].clone();
        let buckets_count = data
            .get("buckets")
            .and_then(|b| b.as_array())
            .map(|b| b.len())
            .unwrap_or(0);

        let meta = json!({
            "query": args.query,
            "from": from,
            "to": to,
            "buckets_count": buckets_count
        });

        Ok(handler.format_list(data, None, Some(meta)))
    }

    pub async fn timeseries(client: &DatadogClient, args: &LogsTimeseriesArgs) -> Result<Value> {
        let handler = LogsHandler;

        let (from_ts, to_ts) = handler.parse_time_range(&args.from, &args.to)?;
        let from = (from_ts * 1000).to_string();
        let to = (to_ts * 1000).to_string();

        let compute = vec![LogsCompute {
            aggregation: args.aggregation.clone(),
            compute_type: Some("timeseries".to_string()),
            interval: Some(args.interval.clone()),
            metric: args.metric.clone(),
        }];

        let response = client
            .aggregate_logs(&args.query, &from, &to, compute)
            .await?;

        let data = response["data"].clone();
        let buckets_count = data
            .get("buckets")
            .and_then(|b| b.as_array())
            .map(|b| b.len())
            .unwrap_or(0);

        let meta = json!({
            "query": args.query,
            "from": from,
            "to": to,
            "interval": args.interval,
            "aggregation": args.aggregation,
            "buckets_count": buckets_count
        });

        Ok(handler.format_list(data, None, Some(meta)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
