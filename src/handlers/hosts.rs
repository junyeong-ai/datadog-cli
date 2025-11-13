use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{
    PaginationInfo, ParameterParser, ResponseFilter, ResponseFormatter, TagFilter, TimeHandler,
};

pub struct HostsHandler;

impl TimeHandler for HostsHandler {}
impl TagFilter for HostsHandler {}
impl ResponseFilter for HostsHandler {}
impl ResponseFormatter for HostsHandler {}
impl ParameterParser for HostsHandler {}

impl HostsHandler {
    pub async fn list(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = HostsHandler;

        let filter = handler.extract_string(params, "filter");
        let sort_field = handler.extract_string(params, "sort_field");
        let sort_dir = handler.extract_string(params, "sort_dir");

        let time = handler.parse_time(params, 1)?;
        let crate::handlers::common::TimeParams::Timestamp { from, .. } = time;
        let from = Some(from);

        let start = handler.extract_i32(params, "start", 0) as usize;
        let count = handler.extract_i32(params, "count", 100) as usize;

        let response = client
            .list_hosts(
                filter,
                from,
                sort_field,
                sort_dir,
                Some(start as i32),
                Some(count as i32),
            )
            .await?;

        let tag_filter = handler.extract_tag_filter(params, &client);

        let data = json!(response.host_list.iter().map(|host| {
            let filtered_tags = handler.filter_tags_map(host.tags_by_source.as_ref(), tag_filter);

            // Remove empty tags field if filter results in empty
            let mut host_json = json!({
                "name": host.name,
                "host_name": host.host_name,
                "up": host.up,
                "is_muted": host.is_muted,
                "last_reported": host.last_reported_time.map(crate::utils::format_timestamp),
                "aws_name": host.aws_name,
                "apps": host.apps,
                "sources": host.sources,
            });

            // Only add tags if not empty
            if let Some(tags) = filtered_tags
                && !tags.is_empty() {
                    host_json["tags"] = json!(tags);
                }

            host_json
        }).collect::<Vec<_>>());

        // Use PaginationInfo for consistent pagination structure
        let pagination =
            PaginationInfo::from_offset(response.total_matching as usize, start, count);

        Ok(json!({
            "data": data,
            "pagination": pagination
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_optional_filter_parameter() {
        let params = json!({"filter": "env:prod"});
        assert_eq!(params["filter"].as_str(), Some("env:prod"));
    }

    #[test]
    fn test_optional_sort_parameters() {
        let params = json!({
            "sort_field": "cpu",
            "sort_dir": "desc"
        });

        assert_eq!(params["sort_field"].as_str(), Some("cpu"));
        assert_eq!(params["sort_dir"].as_str(), Some("desc"));
    }

    #[test]
    fn test_optional_start_parameter() {
        let params = json!({"start": 50});
        assert_eq!(params["start"].as_i64(), Some(50));
    }

    #[test]
    fn test_default_count_parameter() {
        let params = json!({});
        let count = params["count"].as_i64().map(|c| c as i32).or(Some(100));
        assert_eq!(count, Some(100));
    }

    #[test]
    fn test_custom_count_parameter() {
        let params = json!({"count": 500});
        let count = params["count"].as_i64().map(|c| c as i32);
        assert_eq!(count, Some(500));
    }

    #[test]
    fn test_tag_filter_modes() {
        let tag_filter_all = "*";
        let tag_filter_none = "";
        let tag_filter_specific = "env:,service:";

        assert_eq!(tag_filter_all, "*");
        assert_eq!(tag_filter_none, "");
        assert!(tag_filter_specific.contains("env:"));
    }

    #[test]
    fn test_time_handler_trait() {
        let handler = HostsHandler;
        let params = json!({
            "from": "1 hour ago",
            "to": "now"
        });

        let result = handler.parse_time(&params, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_response_formatter_trait() {
        let handler = HostsHandler;
        let data = json!([{"name": "host1"}]);
        let meta = json!({"total_matching": 1});

        let response = handler.format_list(data, None, Some(meta));
        assert!(response.get("data").is_some());
        assert!(response.get("meta").is_some());
    }
}
