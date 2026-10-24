use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::cli::SpansArgs;
use crate::datadog::{DatadogClient, SearchParams};
use crate::error::Result;
use crate::handlers::common::{
    DEFAULT_STACK_TRACE_LINES, PaginationInfo, ResponseFilter, ResponseFormatter, TagFilter,
};

pub struct SpansHandler;

impl TagFilter for SpansHandler {}
impl ResponseFilter for SpansHandler {}
impl ResponseFormatter for SpansHandler {}

impl SpansHandler {
    pub async fn list(
        client: &DatadogClient,
        args: &SpansArgs,
        now: DateTime<Utc>,
    ) -> Result<Value> {
        let handler = SpansHandler;

        let (from, to) = args.range.resolve(now)?.rfc3339()?;

        let response = client
            .search_spans(&SearchParams {
                query: &args.query,
                from: &from,
                to: &to,
                limit: args.limit,
                cursor: args.cursor.as_deref(),
                sort: args.sort.as_deref(),
            })
            .await?;

        let tag_filter = handler.resolve_tag_filter(args.tag_filter.as_deref(), client);

        let data: Vec<Value> = crate::datadog::models::response_records(&response)?
            .iter()
            .map(|span| {
                let mut span_obj = span.clone();

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

                    if let Some(custom) = attrs_obj.get_mut("custom")
                        && let Some(custom_obj) = custom.as_object_mut()
                        && let Some(error) = custom_obj.get_mut("error")
                        && let Some(error_obj) = error.as_object_mut()
                        && let Some(stack) = error_obj.get_mut("stack")
                        && let Some(stack_str) = stack.as_str()
                        && !args.full_stack_trace
                    {
                        let truncated =
                            handler.truncate_stack_trace(stack_str, DEFAULT_STACK_TRACE_LINES);
                        *stack = Value::String(truncated);
                    }
                }

                span_obj
            })
            .collect();

        let next_cursor = response
            .get("meta")
            .and_then(|m| m.get("page"))
            .and_then(|p| p.get("after"))
            .and_then(|a| a.as_str())
            .map(String::from);

        let pagination = PaginationInfo::from_cursor(data.len(), args.limit as usize, next_cursor);

        Ok(handler.with_api_meta(
            json!({
                "data": data,
                "pagination": pagination
            }),
            response.get("meta").cloned(),
        ))
    }
}
