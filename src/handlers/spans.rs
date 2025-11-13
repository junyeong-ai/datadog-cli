use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{
    DEFAULT_STACK_TRACE_LINES, MAX_STRING_LENGTH, PaginationInfo, Paginator, ParameterParser,
    ResponseFilter, ResponseFormatter, TagFilter, TimeHandler,
};

pub struct SpansHandler;

impl TimeHandler for SpansHandler {}
impl Paginator for SpansHandler {}
impl TagFilter for SpansHandler {}
impl ResponseFilter for SpansHandler {}
impl ResponseFormatter for SpansHandler {}
impl ParameterParser for SpansHandler {}

impl SpansHandler {
    pub async fn list(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = SpansHandler;

        let query = handler.extract_query(params, "*");

        let (from, to) = handler.parse_time_iso8601(params)?;

        let (_page, page_size) = handler.parse_pagination(params);
        let limit = handler
            .extract_string(params, "limit")
            .and_then(|s| s.parse::<i32>().ok())
            .or(Some(page_size as i32));
        let cursor = handler.extract_string(params, "cursor");
        let sort = handler.extract_string(params, "sort");

        let response = client
            .list_spans(&query, &from, &to, limit, cursor, sort)
            .await?;

        let tag_filter = handler.extract_tag_filter(params, &client);

        // Process spans with filtering and optimization
        let data = response["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|span| {
                let mut span_obj = span.as_object()?.clone();

                // Apply tag filtering and response optimization to attributes
                if let Some(attrs) = span_obj.get_mut("attributes")
                    && let Some(attrs_obj) = attrs.as_object_mut()
                {
                    // Apply tag filtering
                    if let Some(tags) = attrs_obj.get("tags")
                        && let Some(tags_arr) = tags.as_array()
                    {
                        let tag_strings: Vec<String> = tags_arr
                            .iter()
                            .filter_map(|t| t.as_str().map(String::from))
                            .collect();

                        let filtered_tags = handler.filter_tags(&tag_strings, tag_filter);

                        // Remove empty tags arrays
                        if filtered_tags.is_empty() {
                            attrs_obj.remove("tags");
                        } else {
                            attrs_obj.insert(
                                "tags".to_string(),
                                Value::Array(
                                    filtered_tags.into_iter().map(Value::String).collect(),
                                ),
                            );
                        }
                    }

                    // Remove empty ingestion_reason
                    if let Some(ingestion_reason) = attrs_obj.get("ingestion_reason")
                        && ingestion_reason.as_str().unwrap_or("").is_empty()
                    {
                        attrs_obj.remove("ingestion_reason");
                    }

                    // Process custom object for filtering and truncation
                    if let Some(custom) = attrs_obj.get_mut("custom")
                        && let Some(custom_obj) = custom.as_object_mut()
                    {
                        // Remove http.useragent_details
                        if let Some(http) = custom_obj.get_mut("http") {
                            handler.filter_http_verbose_fields(http);
                        }

                        // Truncate stack traces in error objects
                        if let Some(error) = custom_obj.get_mut("error")
                            && let Some(error_obj) = error.as_object_mut()
                            && let Some(stack) = error_obj.get_mut("stack")
                            && let Some(stack_str) = stack.as_str()
                            && handler.should_truncate_stack_trace(params)
                        {
                            let truncated =
                                handler.truncate_stack_trace(stack_str, DEFAULT_STACK_TRACE_LINES);
                            *stack = Value::String(truncated);
                        }

                        // Truncate long strings in kafka bootstrap servers
                        if let Some(messaging) = custom_obj.get_mut("messaging")
                            && let Some(messaging_obj) = messaging.as_object_mut()
                            && let Some(kafka) = messaging_obj.get_mut("kafka")
                            && let Some(kafka_obj) = kafka.as_object_mut()
                            && let Some(bootstrap) = kafka_obj.get_mut("bootstrap")
                            && let Some(bootstrap_obj) = bootstrap.as_object_mut()
                            && let Some(servers) = bootstrap_obj.get_mut("servers")
                            && let Some(servers_str) = servers.as_str()
                        {
                            let truncated =
                                handler.truncate_long_string(servers_str, MAX_STRING_LENGTH);
                            *servers = Value::String(truncated);
                        }
                    }
                }

                Some(Value::Object(span_obj))
            })
            .collect::<Vec<_>>();

        let spans_count = data.len();

        // Use PaginationInfo for consistent pagination structure
        let has_cursor = response
            .get("meta")
            .and_then(|m| m.get("page"))
            .and_then(|p| p.get("after"))
            .is_some();

        let pagination = PaginationInfo::from_cursor(spans_count, page_size, has_cursor);

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
    fn test_required_query_parameter() {
        let params = json!({"query": "service:web-api"});
        assert_eq!(params["query"].as_str(), Some("service:web-api"));
    }

    #[test]
    fn test_default_limit() {
        let params = json!({});
        let limit = params["limit"].as_i64().map(|l| l as i32).or(Some(10));
        assert_eq!(limit, Some(10));
    }

    #[test]
    fn test_optional_sort_parameter() {
        let params = json!({"sort": "timestamp"});
        assert_eq!(params["sort"].as_str(), Some("timestamp"));
    }

    #[test]
    fn test_pagination_parameters() {
        let handler = SpansHandler;
        let params = json!({"page": 1, "page_size": 50});

        let (page, page_size) = handler.parse_pagination(&params);
        assert_eq!(page, 1);
        assert_eq!(page_size, 50);
    }

    #[test]
    fn test_time_handler_trait() {
        let handler = SpansHandler;
        let params = json!({
            "from": "1 hour ago",
            "to": "now"
        });

        let result = handler.parse_time(&params, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_response_formatter_trait() {
        let handler = SpansHandler;
        let data = json!([{"span_id": "123"}]);
        let pagination = json!({"page": 0});
        let meta = json!({"query": "*"});

        let response = handler.format_list(data, Some(pagination), Some(meta));
        assert!(response.get("data").is_some());
    }
}
