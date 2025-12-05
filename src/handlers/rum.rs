use serde_json::{json, Value};
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
            .search_rum_events(&query, &from_iso, &to_iso, limit, cursor, sort)
            .await?;

        let tag_filter = handler.extract_tag_filter(params, &client);

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

                if let Some(app) = attrs.and_then(|a| a.application.as_ref())
                    && let Some(name) = &app.name
                {
                    entry["application"] = json!({ "name": name });
                }

                if let Some(view) = attrs.and_then(|a| a.view.as_ref()) {
                    let mut v = json!({});
                    if let Some(name) = &view.name {
                        v["name"] = json!(name);
                    }
                    if let Some(url_path) = &view.url_path {
                        v["url_path"] = json!(url_path);
                    }
                    if let Some(loading_time) = view.loading_time {
                        v["loading_time"] = json!(loading_time);
                    }
                    if let Some(time_spent) = view.time_spent {
                        v["time_spent"] = json!(time_spent);
                    }
                    if let Some(obj) = v.as_object()
                        && !obj.is_empty()
                    {
                        entry["view"] = v;
                    }
                }

                if let Some(session) = attrs.and_then(|a| a.session.as_ref()) {
                    let mut s = json!({});
                    if let Some(id) = &session.id {
                        s["id"] = json!(id);
                    }
                    if let Some(session_type) = &session.session_type {
                        s["type"] = json!(session_type);
                    }
                    if let Some(true) = session.has_replay {
                        s["has_replay"] = json!(true);
                    }
                    if let Some(obj) = s.as_object()
                        && !obj.is_empty()
                    {
                        entry["session"] = s;
                    }
                }

                if let Some(action) = attrs.and_then(|a| a.action.as_ref()) {
                    let mut a_obj = json!({});
                    if let Some(name) = &action.name {
                        a_obj["name"] = json!(name);
                    }
                    if let Some(action_type) = &action.action_type {
                        a_obj["type"] = json!(action_type);
                    }
                    if let Some(loading_time) = action.loading_time {
                        a_obj["loading_time"] = json!(loading_time);
                    }
                    if let Some(obj) = a_obj.as_object()
                        && !obj.is_empty()
                    {
                        entry["action"] = a_obj;
                    }
                }

                if let Some(resource) = attrs.and_then(|a| a.resource.as_ref()) {
                    let mut r = json!({});
                    if let Some(url) = &resource.url {
                        r["url"] = json!(url);
                    }
                    if let Some(method) = &resource.method {
                        r["method"] = json!(method);
                    }
                    if let Some(status_code) = resource.status_code {
                        r["status_code"] = json!(status_code);
                    }
                    if let Some(duration) = resource.duration {
                        r["duration"] = json!(duration);
                    }
                    if let Some(obj) = r.as_object()
                        && !obj.is_empty()
                    {
                        entry["resource"] = r;
                    }
                }

                if let Some(error) = attrs.and_then(|a| a.error.as_ref()) {
                    let mut e = json!({});
                    if let Some(message) = &error.message {
                        e["message"] = json!(message);
                    }
                    if let Some(source) = &error.source {
                        e["source"] = json!(source);
                    }
                    if let Some(error_type) = &error.error_type {
                        e["type"] = json!(error_type);
                    }
                    if let Some(stack) = &error.stack {
                        let stack_str = if handler.should_truncate_stack_trace(params) {
                            handler.truncate_stack_trace(stack, DEFAULT_STACK_TRACE_LINES)
                        } else {
                            stack.clone()
                        };
                        e["stack"] = json!(stack_str);
                    }
                    if let Some(true) = error.is_crash {
                        e["is_crash"] = json!(true);
                    }
                    if let Some(obj) = e.as_object()
                        && !obj.is_empty()
                    {
                        entry["error"] = e;
                    }
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

        let pagination = PaginationInfo::from_cursor(events.len(), limit as usize, has_cursor);

        Ok(json!({
            "data": events,
            "pagination": pagination
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_parser() {
        let handler = RumHandler;
        let params = json!({
            "query": "@type:session",
            "limit": 25,
            "sort": "timestamp",
        });

        assert_eq!(handler.extract_query(&params, "*"), "@type:session");
        assert_eq!(handler.extract_i32(&params, "limit", 10), 25);
        assert_eq!(handler.extract_string(&params, "sort"), Some("timestamp".to_string()));
    }

    #[test]
    fn test_tag_filter_trait() {
        let handler = RumHandler;
        let tags = vec!["env:prod".to_string(), "service:web".to_string()];

        assert_eq!(handler.filter_tags(&tags, "*").len(), 2);
        assert_eq!(handler.filter_tags(&tags, "env:").len(), 1);
        assert_eq!(handler.filter_tags(&tags, "").len(), 0);
    }

    #[test]
    fn test_response_filter_trait() {
        let handler = RumHandler;

        let params = json!({});
        assert!(handler.should_truncate_stack_trace(&params));

        let params = json!({"full_stack_trace": true});
        assert!(!handler.should_truncate_stack_trace(&params));
    }
}
