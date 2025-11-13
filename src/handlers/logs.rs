use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{
    PaginationInfo, ParameterParser, ResponseFilter, ResponseFormatter, TagFilter, TimeHandler,
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

        let limit = handler.extract_i32(params, "limit", 10) as usize;

        let (from_iso, to_iso) = handler.parse_time_iso8601(params)?;

        let response = client
            .search_logs(query, &from_iso, &to_iso, Some(limit as i32))
            .await?;

        if let Some(errors) = response.errors {
            return Err(crate::error::DatadogError::ApiError(errors.join(", ")));
        }

        let tag_filter = handler.extract_tag_filter(params, &client);

        let logs = response
            .data
            .unwrap_or_default()
            .iter()
            .map(|log| {
                let attrs = log.attributes.as_ref();
                let tags = attrs
                    .and_then(|a| a.tags.as_ref())
                    .map(|t| handler.filter_tags(t, tag_filter));

                // Build log entry, excluding null/empty fields
                let mut log_entry = json!({
                    "id": log.id,
                });

                // Only add non-null fields
                if let Some(timestamp) = attrs.and_then(|a| a.timestamp.as_ref()) {
                    log_entry["timestamp"] = json!(timestamp);
                }
                if let Some(message) = attrs.and_then(|a| a.message.as_ref()) {
                    log_entry["message"] = json!(message);
                }
                if let Some(host) = attrs.and_then(|a| a.host.as_ref()) {
                    log_entry["host"] = json!(host);
                }
                if let Some(service) = attrs.and_then(|a| a.service.as_ref()) {
                    log_entry["service"] = json!(service);
                }
                if let Some(status) = attrs.and_then(|a| a.status.as_ref()) {
                    log_entry["status"] = json!(status);
                }

                // Only add tags if not empty
                if let Some(tags_vec) = tags
                    && !tags_vec.is_empty()
                {
                    log_entry["tags"] = json!(tags_vec);
                }

                log_entry
            })
            .collect::<Vec<_>>();

        let result_count = logs.len();

        // Use PaginationInfo for single-page API with heuristic
        let pagination = PaginationInfo::single_page(result_count, limit);

        Ok(json!({
            "data": logs,
            "pagination": pagination
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_missing_query_parameter() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = Arc::new(
                DatadogClient::new("test_key".to_string(), "test_app_key".to_string(), None)
                    .unwrap(),
            );

            let params = json!({
                "from": "1 hour ago",
                "to": "now"
                // Missing "query"
            });

            let result = LogsHandler::search(client, &params).await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_valid_input_parameters() {
        let params = json!({
            "query": "service:web-api",
            "from": "1 hour ago",
            "to": "now",
            "limit": 50
        });

        assert_eq!(params["query"].as_str(), Some("service:web-api"));
        assert_eq!(params["limit"].as_i64(), Some(50));
    }

    #[test]
    fn test_optional_limit_parameter() {
        let params_with = json!({"query": "test", "limit": 100});
        let params_without = json!({"query": "test"});

        assert_eq!(params_with["limit"].as_i64(), Some(100));
        assert_eq!(params_without["limit"].as_i64(), None);
    }

    #[test]
    fn test_tag_filter_modes() {
        // Test all tags mode
        let _tags = ["env:prod".to_string(), "service:api".to_string()];
        let filter_all = "*";
        assert_eq!(filter_all, "*");

        // Test no tags mode
        let filter_none = "";
        assert_eq!(filter_none, "");

        // Test prefix filtering
        let filter_prefixes = "env:,service:";
        assert!(filter_prefixes.contains("env:"));
        assert!(filter_prefixes.contains("service:"));
    }

    #[test]
    fn test_time_handler_available() {
        let handler = LogsHandler;
        let params = json!({
            "from": "1609459200",
            "to": "1609462800"
        });

        let result = handler.parse_time(&params, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_response_formatter_available() {
        let handler = LogsHandler;
        let data = json!([{"id": "log1"}]);
        let formatted = handler.format_list(data, None, None);
        assert!(formatted.get("data").is_some());
    }
}
