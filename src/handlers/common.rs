use crate::error::{DatadogError, Result};
use crate::utils::parse_time;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

pub const DEFAULT_STACK_TRACE_LINES: usize = 10;
pub const MAX_STRING_LENGTH: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaginationInfo {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub has_next: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl PaginationInfo {
    pub fn single_page(result_count: usize, limit: usize) -> Self {
        Self {
            total: result_count,
            page: 0,
            page_size: limit,
            has_next: result_count >= limit,
            next_offset: None,
            next_cursor: None,
        }
    }

    pub fn from_offset(total: usize, start: usize, count: usize) -> Self {
        debug_assert!(count > 0);
        let page = start / count;
        let next_offset = start + count;
        let has_next = next_offset < total;

        Self {
            total,
            page,
            page_size: count,
            has_next,
            next_offset: if has_next { Some(next_offset) } else { None },
            next_cursor: None,
        }
    }

    /// Offset pagination for APIs that report no total: a full page implies
    /// more results may exist (the boundary case yields one extra empty page).
    pub fn from_offset_without_total(returned: usize, start: usize, count: usize) -> Self {
        debug_assert!(count > 0);
        let has_next = returned >= count;

        Self {
            total: returned,
            page: start / count,
            page_size: count,
            has_next,
            next_offset: has_next.then_some(start + returned),
            next_cursor: None,
        }
    }

    /// Page-number pagination for APIs that report no total: a full page
    /// implies more results may exist.
    pub fn from_page_number(returned: usize, page: usize, page_size: usize) -> Self {
        Self {
            total: returned,
            page,
            page_size,
            has_next: returned >= page_size,
            next_offset: None,
            next_cursor: None,
        }
    }

    pub fn from_cursor(total: usize, page_size: usize, next_cursor: Option<String>) -> Self {
        Self {
            total,
            page: 0,
            page_size,
            has_next: next_cursor.is_some(),
            next_offset: None,
            next_cursor,
        }
    }
}

pub trait TimeHandler {
    fn parse_time_range(&self, from: &str, to: &str) -> Result<(i64, i64)> {
        Ok((parse_time(from)?, parse_time(to)?))
    }

    fn timestamp_to_iso8601(&self, timestamp: i64) -> Result<String> {
        chrono::DateTime::from_timestamp(timestamp, 0)
            .map(|dt| dt.to_rfc3339())
            .ok_or_else(|| DatadogError::InvalidInput("Invalid timestamp".to_string()))
    }

    fn parse_time_range_iso8601(&self, from: &str, to: &str) -> Result<(String, String)> {
        let (from_ts, to_ts) = self.parse_time_range(from, to)?;
        Ok((
            self.timestamp_to_iso8601(from_ts)?,
            self.timestamp_to_iso8601(to_ts)?,
        ))
    }
}

pub trait TagFilter {
    /// The effective filter: explicit argument, then the config default
    /// carried by the client, then `"*"` (keep everything).
    fn resolve_tag_filter<'a>(
        &self,
        arg: Option<&'a str>,
        client: &'a crate::datadog::DatadogClient,
    ) -> &'a str {
        arg.or_else(|| client.get_tag_filter()).unwrap_or("*")
    }

    fn filter_tags(&self, tags: &[String], filter: &str) -> Vec<String> {
        match filter {
            "*" => tags.to_vec(),
            "" => Vec::new(),
            filter => {
                let prefixes: Vec<&str> = filter.split(',').map(str::trim).collect();
                tags.iter()
                    .filter(|tag| prefixes.iter().any(|p| tag.starts_with(p)))
                    .cloned()
                    .collect()
            }
        }
    }

    fn filter_tags_map(
        &self,
        tags_map: Option<&HashMap<String, Vec<String>>>,
        filter: &str,
    ) -> Option<HashMap<String, Vec<String>>> {
        match filter {
            "*" => tags_map.cloned(),
            "" => None,
            filter => tags_map.map(|map| {
                let prefixes: Vec<&str> = filter.split(',').map(str::trim).collect();
                let mut filtered_map = HashMap::new();

                for (source, tags) in map.iter() {
                    let filtered_tags: Vec<String> = tags
                        .iter()
                        .filter(|tag| prefixes.iter().any(|p| tag.starts_with(p)))
                        .cloned()
                        .collect();

                    if !filtered_tags.is_empty() {
                        filtered_map.insert(source.clone(), filtered_tags);
                    }
                }

                filtered_map
            }),
        }
    }
}

pub trait ResponseFilter {
    fn truncate_stack_trace(&self, stack: &str, max_lines: usize) -> String {
        crate::utils::truncate_stack_trace(stack, max_lines)
    }

    fn filter_http_verbose_fields(&self, http: &mut Value) {
        if let Some(obj) = http.as_object_mut() {
            obj.remove("useragent_details");
        }
    }

    fn truncate_long_string(&self, s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            return s.to_string();
        }

        // API data may contain multi-byte characters; cut at the nearest
        // char boundary at or below max_len so the slice cannot panic.
        let mut end = max_len;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

pub trait ResponseFormatter {
    fn format_list(&self, data: Value, pagination: Option<Value>, meta: Option<Value>) -> Value {
        let mut response = json!({ "data": data });

        if let Some(p) = pagination {
            response["pagination"] = p;
        }

        if let Some(m) = meta {
            response["meta"] = m;
        }

        response
    }

    fn format_detail(&self, data: Value) -> Value {
        json!({ "data": data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestHandler;
    impl TimeHandler for TestHandler {}
    impl ResponseFormatter for TestHandler {}

    #[test]
    fn test_time_handler_parse_time_range() {
        let handler = TestHandler;
        let (from, to) = handler
            .parse_time_range("1609459200", "1609462800")
            .unwrap();
        assert_eq!(from, 1609459200);
        assert_eq!(to, 1609462800);
    }

    #[test]
    fn test_time_handler_parse_time_range_iso8601() {
        let handler = TestHandler;
        let (from, to) = handler
            .parse_time_range_iso8601("1609459200", "1609462800")
            .unwrap();
        assert!(from.starts_with("2021-01-01T00:00:00"));
        assert!(to.starts_with("2021-01-01T01:00:00"));
    }

    #[test]
    fn test_from_offset_without_total_full_page() {
        let pagination = PaginationInfo::from_offset_without_total(100, 0, 100);
        assert!(pagination.has_next);
        assert_eq!(pagination.next_offset, Some(100));
        assert_eq!(pagination.page, 0);

        let next = PaginationInfo::from_offset_without_total(100, 100, 100);
        assert!(next.has_next);
        assert_eq!(next.next_offset, Some(200));
        assert_eq!(next.page, 1);
    }

    #[test]
    fn test_from_offset_without_total_partial_page() {
        let pagination = PaginationInfo::from_offset_without_total(37, 200, 100);
        assert!(!pagination.has_next);
        assert_eq!(pagination.next_offset, None);
        assert_eq!(pagination.page, 2);
    }

    #[test]
    fn test_response_formatter_list() {
        let handler = TestHandler;
        let data = json!(["item1", "item2"]);

        let response = handler.format_list(data.clone(), None, None);
        assert_eq!(response["data"], data);
        assert!(response["pagination"].is_null());
        assert!(response["meta"].is_null());
    }

    #[test]
    fn test_response_formatter_with_meta() {
        let handler = TestHandler;
        let data = json!(["item1"]);
        let meta = json!({"count": 1});

        let response = handler.format_list(data.clone(), None, Some(meta.clone()));
        assert_eq!(response["data"], data);
        assert_eq!(response["meta"], meta);
    }

    #[test]
    fn test_from_page_number() {
        let full = PaginationInfo::from_page_number(50, 2, 50);
        assert!(full.has_next);
        assert_eq!(full.page, 2);

        let partial = PaginationInfo::from_page_number(10, 3, 50);
        assert!(!partial.has_next);
    }

    #[test]
    fn test_truncate_long_string_multibyte_boundary() {
        struct Filter;
        impl ResponseFilter for Filter {}

        // 99 ASCII bytes followed by a 3-byte char: byte 100 is not a char
        // boundary, so truncation must back up instead of panicking.
        let s = format!("{}\u{3042}\u{3042}", "a".repeat(99));
        let truncated = Filter.truncate_long_string(&s, 100);
        assert_eq!(truncated, format!("{}...", "a".repeat(99)));

        let short = "짧은 문자열";
        assert_eq!(Filter.truncate_long_string(short, 100), short);
    }

    #[test]
    fn test_response_formatter_detail() {
        let handler = TestHandler;
        let data = json!({"id": 123, "name": "test"});

        let response = handler.format_detail(data.clone());
        assert_eq!(response["data"], data);
    }
}
