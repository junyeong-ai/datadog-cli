use serde_json::{Value, json};
use std::sync::Arc;

use crate::datadog::DatadogClient;
use crate::error::Result;
use crate::handlers::common::{
    DEFAULT_STACK_TRACE_LINES, PaginationInfo, ParameterParser, ResponseFilter, ResponseFormatter,
    TagFilter, TimeHandler,
};

pub struct RumHandler;

impl TimeHandler for RumHandler {}
impl TagFilter for RumHandler {}
impl ResponseFilter for RumHandler {}
impl ResponseFormatter for RumHandler {}
impl ParameterParser for RumHandler {}

impl RumHandler {
    pub async fn search_events(client: Arc<DatadogClient>, params: &Value) -> Result<Value> {
        let handler = RumHandler;

        let query = handler.extract_query(params, "*");

        let (from_iso, to_iso) = handler.parse_time_iso8601(params)?;

        let limit = handler.extract_i32(params, "limit", 10);
        let cursor = handler.extract_string(params, "cursor");
        let sort = handler.extract_string(params, "sort");

        let response = client
            .search_rum_events(&query, &from_iso, &to_iso, Some(limit), cursor, sort)
            .await?;

        let tag_filter = handler.extract_tag_filter(params, &client);

        // Process RUM events with aggressive optimization - only meaningful data
        let events = response
            .data
            .unwrap_or_default()
            .iter()
            .map(|event| {
                let attrs = event.attributes.as_ref();

                // Apply tag filtering
                let tags = attrs
                    .and_then(|a| a.tags.as_ref())
                    .map(|t| handler.filter_tags(t, tag_filter));

                // Build minimal event entry - only meaningful fields
                let mut event_entry = json!({
                    "id": event.id,
                });

                // Core fields only (timestamp, type)
                if let Some(event_type) = &event.event_type {
                    event_entry["type"] = json!(event_type);
                }

                if let Some(timestamp) = attrs.and_then(|a| a.timestamp.as_ref()) {
                    event_entry["timestamp"] = json!(timestamp);
                }

                if let Some(service) = attrs.and_then(|a| a.service.as_ref()) {
                    event_entry["service"] = json!(service);
                }

                // Application - only essential fields (name)
                if let Some(app) = attrs.and_then(|a| a.application.as_ref())
                    && let Some(name) = &app.name
                {
                    event_entry["application"] = json!({ "name": name });
                }

                // View - only performance-critical fields
                if let Some(view) = attrs.and_then(|a| a.view.as_ref()) {
                    let mut view_obj = json!({});

                    if let Some(name) = &view.name {
                        view_obj["name"] = json!(name);
                    }
                    if let Some(url_path) = &view.url_path {
                        view_obj["url_path"] = json!(url_path);
                    }
                    // Performance metrics are valuable
                    if let Some(loading_time) = view.loading_time {
                        view_obj["loading_time"] = json!(loading_time);
                    }
                    if let Some(time_spent) = view.time_spent {
                        view_obj["time_spent"] = json!(time_spent);
                    }

                    if let Some(obj) = view_obj.as_object()
                        && !obj.is_empty()
                    {
                        event_entry["view"] = view_obj;
                    }
                }

                // Session - minimal but critical for tracking
                if let Some(session) = attrs.and_then(|a| a.session.as_ref()) {
                    let mut session_obj = json!({});

                    if let Some(id) = &session.id {
                        session_obj["id"] = json!(id);
                    }
                    if let Some(session_type) = &session.session_type {
                        session_obj["type"] = json!(session_type);
                    }
                    if let Some(has_replay) = session.has_replay {
                        // Only include if true - valuable for debugging
                        if has_replay {
                            session_obj["has_replay"] = json!(true);
                        }
                    }

                    if let Some(obj) = session_obj.as_object()
                        && !obj.is_empty()
                    {
                        event_entry["session"] = session_obj;
                    }
                }

                // Action - essential action tracking
                if let Some(action) = attrs.and_then(|a| a.action.as_ref()) {
                    let mut action_obj = json!({});

                    if let Some(name) = &action.name {
                        action_obj["name"] = json!(name);
                    }
                    if let Some(action_type) = &action.action_type {
                        action_obj["type"] = json!(action_type);
                    }
                    // Loading time is performance-critical
                    if let Some(loading_time) = action.loading_time {
                        action_obj["loading_time"] = json!(loading_time);
                    }

                    if let Some(obj) = action_obj.as_object()
                        && !obj.is_empty()
                    {
                        event_entry["action"] = action_obj;
                    }
                }

                // Resource - performance and error tracking
                if let Some(resource) = attrs.and_then(|a| a.resource.as_ref()) {
                    let mut resource_obj = json!({});

                    if let Some(url) = &resource.url {
                        resource_obj["url"] = json!(url);
                    }
                    if let Some(method) = &resource.method {
                        resource_obj["method"] = json!(method);
                    }
                    // Status code is critical for error detection
                    if let Some(status_code) = resource.status_code {
                        resource_obj["status_code"] = json!(status_code);
                    }
                    // Performance metrics
                    if let Some(duration) = resource.duration {
                        resource_obj["duration"] = json!(duration);
                    }

                    if let Some(obj) = resource_obj.as_object()
                        && !obj.is_empty()
                    {
                        event_entry["resource"] = resource_obj;
                    }
                }

                // Error - critical for debugging (with stack trace truncation)
                if let Some(error) = attrs.and_then(|a| a.error.as_ref()) {
                    let mut error_obj = json!({});

                    if let Some(message) = &error.message {
                        error_obj["message"] = json!(message);
                    }
                    if let Some(source) = &error.source {
                        error_obj["source"] = json!(source);
                    }
                    if let Some(error_type) = &error.error_type {
                        error_obj["type"] = json!(error_type);
                    }

                    // Truncate stack trace for token efficiency
                    if let Some(stack) = &error.stack {
                        let stack_str = if handler.should_truncate_stack_trace(params) {
                            handler.truncate_stack_trace(stack, DEFAULT_STACK_TRACE_LINES)
                        } else {
                            stack.clone()
                        };
                        error_obj["stack"] = json!(stack_str);
                    }

                    // is_crash is critical information
                    if let Some(is_crash) = error.is_crash
                        && is_crash
                    {
                        error_obj["is_crash"] = json!(true);
                    }

                    if let Some(obj) = error_obj.as_object()
                        && !obj.is_empty()
                    {
                        event_entry["error"] = error_obj;
                    }
                }

                // Only add tags if not empty
                if let Some(tags_vec) = tags
                    && !tags_vec.is_empty()
                {
                    event_entry["tags"] = json!(tags_vec);
                }

                event_entry
            })
            .collect::<Vec<_>>();

        let events_count = events.len();

        // Use PaginationInfo for cursor-based pagination
        let has_cursor = response
            .meta
            .as_ref()
            .and_then(|m| m.page.as_ref())
            .and_then(|p| p.after.as_ref())
            .is_some();

        let pagination = PaginationInfo::from_cursor(events_count, limit as usize, has_cursor);

        Ok(json!({
            "data": events,
            "pagination": pagination
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_default_query_parameter() {
        let params = json!({});
        let query = params["query"].as_str().unwrap_or("*");
        assert_eq!(query, "*");
    }

    #[test]
    fn test_custom_query_parameter() {
        let params = json!({"query": "@type:session AND @session.type:user"});
        assert_eq!(
            params["query"].as_str(),
            Some("@type:session AND @session.type:user")
        );
    }

    #[test]
    fn test_optional_limit() {
        let params = json!({"limit": 25});
        let limit = params["limit"].as_i64().unwrap_or(10);
        assert_eq!(limit, 25);
    }

    #[test]
    fn test_time_handler_trait() {
        let handler = RumHandler;
        let params = json!({
            "from": "1 hour ago",
            "to": "now"
        });

        let result = handler.parse_time(&params, 2);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tag_filter_trait() {
        let handler = RumHandler;
        let tags = vec!["env:prod".to_string(), "service:web".to_string()];

        // Test wildcard filter
        let filtered = handler.filter_tags(&tags, "*");
        assert_eq!(filtered.len(), 2);

        // Test prefix filter
        let filtered = handler.filter_tags(&tags, "env:");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0], "env:prod");

        // Test empty filter
        let filtered = handler.filter_tags(&tags, "");
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_response_filter_trait() {
        let handler = RumHandler;

        // Test with full_stack_trace = false (default)
        let params = json!({});
        assert!(handler.should_truncate_stack_trace(&params));

        // Test with full_stack_trace = true
        let params = json!({"full_stack_trace": true});
        assert!(!handler.should_truncate_stack_trace(&params));
    }

    #[test]
    fn test_response_formatter_trait() {
        let handler = RumHandler;
        let data = json!([{"id": "event1"}]);
        let pagination = json!({"page": 0});

        let response = handler.format_list(data, Some(pagination), None);
        assert!(response.get("data").is_some());
        assert!(response.get("pagination").is_some());
    }
}
