use crate::error::{DatadogError, Result};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

pub const DEFAULT_STACK_TRACE_LINES: usize = 10;
pub const MAX_STRING_LENGTH: usize = 100;

pub(crate) fn api_count(value: Option<&Value>) -> Result<Option<usize>> {
    value
        .filter(|value| !value.is_null())
        .map(|value| {
            usize::deserialize(value).map_err(|error| DatadogError::DecodeError(error.to_string()))
        })
        .transpose()
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
                let prefixes: Vec<&str> = filter
                    .split(',')
                    .map(str::trim)
                    .filter(|prefix| !prefix.is_empty())
                    .collect();
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
                let prefixes: Vec<&str> = filter
                    .split(',')
                    .map(str::trim)
                    .filter(|prefix| !prefix.is_empty())
                    .collect();
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
    fn with_api_meta(&self, mut output: Value, meta: Option<Value>) -> Value {
        if let Some(meta) = meta.filter(|meta| !meta.is_null()) {
            output["meta"]["api"] = meta;
        }
        output
    }

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
    impl ResponseFormatter for TestHandler {}
    impl TagFilter for TestHandler {}

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
    #[test]
    fn empty_tag_prefixes_do_not_match_every_tag() {
        let tags = vec!["env:prod".into(), "service:api".into()];
        assert_eq!(TestHandler.filter_tags(&tags, "env:,"), vec!["env:prod"]);
        assert!(TestHandler.filter_tags(&tags, ", ,").is_empty());
    }
}
