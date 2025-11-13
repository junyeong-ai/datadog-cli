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
}

impl PaginationInfo {
    pub fn single_page(result_count: usize, limit: usize) -> Self {
        Self {
            total: result_count,
            page: 0,
            page_size: limit,
            has_next: result_count >= limit,
            next_offset: None,
        }
    }

    pub fn from_offset(total: usize, start: usize, count: usize) -> Self {
        let page = start / count;
        let next_offset = start + count;
        let has_next = next_offset < total;

        Self {
            total,
            page,
            page_size: count,
            has_next,
            next_offset: if has_next { Some(next_offset) } else { None },
        }
    }

    pub fn from_cursor(total: usize, page_size: usize, has_cursor: bool) -> Self {
        Self {
            total,
            page: 0,
            page_size,
            has_next: has_cursor,
            next_offset: None,
        }
    }
}

pub enum TimeParams {
    Timestamp { from: i64, to: i64 },
}

pub trait TimeHandler {
    fn parse_time(&self, params: &Value, _api_version: u8) -> Result<TimeParams> {
        let from_str = params["from"].as_str().unwrap_or("1 hour ago").to_string();
        let to_str = params["to"].as_str().unwrap_or("now").to_string();

        let from = parse_time(&from_str)?;
        let to = parse_time(&to_str)?;
        Ok(TimeParams::Timestamp { from, to })
    }

    fn timestamp_to_iso8601(&self, timestamp: i64) -> Result<String> {
        chrono::DateTime::from_timestamp(timestamp, 0)
            .map(|dt| dt.to_rfc3339())
            .ok_or_else(|| DatadogError::InvalidInput("Invalid timestamp".to_string()))
    }

    fn parse_time_iso8601(&self, params: &Value) -> Result<(String, String)> {
        let time = self.parse_time(params, 1)?;
        let TimeParams::Timestamp { from, to } = time;
        let from_iso = self.timestamp_to_iso8601(from)?;
        let to_iso = self.timestamp_to_iso8601(to)?;
        Ok((from_iso, to_iso))
    }
}

pub trait Paginator {
    fn parse_pagination(&self, params: &Value) -> (usize, usize) {
        let page = params["page"].as_u64().unwrap_or(0) as usize;
        let page_size = params["page_size"].as_u64().unwrap_or(50) as usize;
        (page, page_size)
    }

    fn paginate<'a, T>(&self, data: &'a [T], page: usize, page_size: usize) -> &'a [T] {
        let start = page * page_size;
        let end = std::cmp::min(start + page_size, data.len());

        if start < data.len() {
            &data[start..end]
        } else {
            &data[0..0]
        }
    }
}

pub trait TagFilter {
    fn extract_tag_filter<'a>(
        &self,
        params: &'a Value,
        client: &'a crate::datadog::DatadogClient,
    ) -> &'a str {
        params["tag_filter"]
            .as_str()
            .or_else(|| client.get_tag_filter())
            .unwrap_or("*")
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
    fn should_truncate_stack_trace(&self, params: &Value) -> bool {
        !params
            .get("full_stack_trace")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

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
            s.to_string()
        } else {
            format!("{}...", &s[..max_len])
        }
    }
}

pub trait ParameterParser {
    fn extract_string(&self, params: &Value, key: &str) -> Option<String> {
        params[key].as_str().map(|s| s.to_string())
    }

    fn extract_i32(&self, params: &Value, key: &str, default: i32) -> i32 {
        params[key].as_i64().map(|l| l as i32).unwrap_or(default)
    }

    fn extract_query(&self, params: &Value, default: &str) -> String {
        params["query"].as_str().unwrap_or(default).to_string()
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

    fn format_pagination(&self, page: usize, page_size: usize, total: usize) -> Value {
        json!({
            "page": page,
            "page_size": page_size,
            "total": total,
            "has_next": (page + 1) * page_size < total
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestHandler;
    impl TimeHandler for TestHandler {}
    impl Paginator for TestHandler {}
    impl ResponseFormatter for TestHandler {}

    #[test]
    fn test_time_handler_parse_time() {
        let handler = TestHandler;
        let params = json!({
            "from": "1609459200",
            "to": "1609462800"
        });

        let result = handler.parse_time(&params, 1);
        assert!(result.is_ok());

        if let Ok(TimeParams::Timestamp { from, to }) = result {
            assert!(from > 0);
            assert!(to > from);
        }
    }

    #[test]
    fn test_time_handler_defaults() {
        let handler = TestHandler;
        let params = json!({});
        let result = handler.parse_time(&params, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_paginator_parse() {
        let handler = TestHandler;
        let params = json!({
            "page": 2,
            "page_size": 25
        });

        let (page, page_size) = handler.parse_pagination(&params);
        assert_eq!(page, 2);
        assert_eq!(page_size, 25);
    }

    #[test]
    fn test_paginator_defaults() {
        let handler = TestHandler;
        let params = json!({});

        let (page, page_size) = handler.parse_pagination(&params);
        assert_eq!(page, 0);
        assert_eq!(page_size, 50);
    }

    #[test]
    fn test_paginator_paginate() {
        let handler = TestHandler;
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let page1 = handler.paginate(&data, 0, 3);
        assert_eq!(page1, &[1, 2, 3]);

        let page2 = handler.paginate(&data, 1, 3);
        assert_eq!(page2, &[4, 5, 6]);

        let page4 = handler.paginate(&data, 3, 3);
        assert_eq!(page4, &[10]);

        let page_empty = handler.paginate(&data, 10, 3);
        assert_eq!(page_empty.len(), 0);
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
    fn test_response_formatter_pagination() {
        let handler = TestHandler;

        let pagination = handler.format_pagination(0, 50, 150);
        assert_eq!(pagination["page"], 0);
        assert_eq!(pagination["page_size"], 50);
        assert_eq!(pagination["total"], 150);
        assert_eq!(pagination["has_next"], true);

        let last_page = handler.format_pagination(2, 50, 150);
        assert_eq!(last_page["has_next"], false);

        let mid_page = handler.format_pagination(1, 50, 150);
        assert_eq!(mid_page["has_next"], true);
    }

    #[test]
    fn test_response_formatter_detail() {
        let handler = TestHandler;
        let data = json!({"id": 123, "name": "test"});

        let response = handler.format_detail(data.clone());
        assert_eq!(response["data"], data);
    }
}
