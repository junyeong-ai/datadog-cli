use serde_json::{json, Value};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{
    DEFAULT_STACK_TRACE_LINES, MAX_STRING_LENGTH, PaginationInfo, ParameterParser, ResponseFilter,
    ResponseFormatter, TagFilter, TimeHandler,
};

pub struct SpansHandler;

impl TimeHandler for SpansHandler {}
impl TagFilter for SpansHandler {}
impl ResponseFilter for SpansHandler {}
impl ResponseFormatter for SpansHandler {}
impl ParameterParser for SpansHandler {}

impl SpansHandler {
    pub async fn list(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = SpansHandler;

        let query = handler.extract_query(params, "*");
        let (from, to) = handler.parse_time_iso8601(params)?;

        let limit = handler.extract_i32(params, "limit", 10);
        let cursor = handler.extract_string(params, "cursor");
        let sort = handler.extract_string(params, "sort");

        let response = client
            .list_spans(&query, &from, &to, limit, cursor, sort)
            .await?;

        let tag_filter = handler.extract_tag_filter(params, &client);

        let data: Vec<Value> = response["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|span| {
                let mut span_obj = span.as_object()?.clone();

                if let Some(attrs) = span_obj.get_mut("attributes")
                    && let Some(attrs_obj) = attrs.as_object_mut()
                {
                    if let Some(tags) = attrs_obj.get("tags")
                        && let Some(tags_arr) = tags.as_array()
                    {
                        let tag_strings: Vec<String> = tags_arr
                            .iter()
                            .filter_map(|t| t.as_str().map(String::from))
                            .collect();

                        let filtered_tags = handler.filter_tags(&tag_strings, tag_filter);

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

                    if let Some(ingestion_reason) = attrs_obj.get("ingestion_reason")
                        && ingestion_reason.as_str().unwrap_or("").is_empty()
                    {
                        attrs_obj.remove("ingestion_reason");
                    }

                    if let Some(custom) = attrs_obj.get_mut("custom")
                        && let Some(custom_obj) = custom.as_object_mut()
                    {
                        if let Some(http) = custom_obj.get_mut("http") {
                            handler.filter_http_verbose_fields(http);
                        }

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
            .collect();

        let has_cursor = response
            .get("meta")
            .and_then(|m| m.get("page"))
            .and_then(|p| p.get("after"))
            .is_some();

        let pagination = PaginationInfo::from_cursor(data.len(), limit as usize, has_cursor);

        Ok(json!({
            "data": data,
            "pagination": pagination
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_parser() {
        let handler = SpansHandler;
        let params = json!({
            "query": "service:web-api",
            "limit": 50,
            "sort": "timestamp",
        });

        assert_eq!(handler.extract_query(&params, "*"), "service:web-api");
        assert_eq!(handler.extract_i32(&params, "limit", 10), 50);
        assert_eq!(handler.extract_string(&params, "sort"), Some("timestamp".to_string()));
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
}
