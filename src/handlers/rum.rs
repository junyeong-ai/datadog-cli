use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::cli::RumArgs;
use crate::datadog::{DatadogClient, SearchParams};
use crate::error::Result;
use crate::handlers::common::{
    DEFAULT_STACK_TRACE_LINES, ResponseFilter, ResponseFormatter, TagFilter,
};
use crate::pagination::PaginationInfo;

pub struct RumHandler;

impl TagFilter for RumHandler {}
impl ResponseFilter for RumHandler {}
impl ResponseFormatter for RumHandler {}

impl RumHandler {
    pub async fn search_events(
        client: &DatadogClient,
        args: &RumArgs,
        now: DateTime<Utc>,
    ) -> Result<Value> {
        let handler = RumHandler;

        let (from_iso, to_iso) = args.range.resolve(now)?.rfc3339()?;

        let response = client
            .search_rum_events(&SearchParams {
                query: &args.query,
                from: &from_iso,
                to: &to_iso,
                limit: args.limit,
                cursor: args.cursor.as_deref(),
                sort: args.sort.as_deref(),
            })
            .await?;

        let tag_filter = handler.resolve_tag_filter(args.tag_filter.as_deref(), client);

        let events: Vec<Value> = response
            .data
            .unwrap_or_default()
            .iter()
            .map(|event| {
                let attrs = event.attributes.as_ref();

                let tags = attrs
                    .and_then(|a| a.tags.as_ref())
                    .map(|t| handler.filter_tags(t, tag_filter));

                let mut entry = json!({ "id": event.id });

                if let Some(event_type) = &event.event_type {
                    entry["type"] = json!(event_type);
                }

                if let Some(timestamp) = attrs.and_then(|a| a.timestamp.as_ref()) {
                    entry["timestamp"] = json!(timestamp);
                }

                if let Some(service) = attrs.and_then(|a| a.service.as_ref()) {
                    entry["service"] = json!(service);
                }

                if let Some(attributes) = attrs.and_then(|a| a.attributes.as_ref()) {
                    let mut attributes = json!(attributes);
                    if !args.full_stack_trace
                        && let Some(stack) = attributes
                            .get_mut("error")
                            .and_then(|error| error.get_mut("stack"))
                        && let Some(text) = stack.as_str()
                    {
                        *stack =
                            json!(handler.truncate_stack_trace(text, DEFAULT_STACK_TRACE_LINES));
                    }
                    entry["attributes"] = attributes;
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

        let pagination = PaginationInfo::from_cursor(events.len(), next_cursor);

        Ok(handler.with_api_meta(
            json!({
                "data": events,
                "pagination": pagination
            }),
            response.meta.map(serde_json::to_value).transpose()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_filter_trait() {
        let handler = RumHandler;
        let tags = vec!["env:prod".to_string(), "service:web".to_string()];

        assert_eq!(handler.filter_tags(&tags, "*").len(), 2);
        assert_eq!(handler.filter_tags(&tags, "env:").len(), 1);
        assert_eq!(handler.filter_tags(&tags, "").len(), 0);
    }
}
