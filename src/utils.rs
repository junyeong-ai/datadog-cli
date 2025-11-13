use crate::error::{DatadogError, Result};
use chrono::{DateTime, Utc};
use interim::{Dialect, parse_date_string};

pub fn parse_time(input: &str) -> Result<i64> {
    if input.trim().to_lowercase() == "now" {
        return Ok(Utc::now().timestamp());
    }

    if let Ok(timestamp) = input.parse::<i64>() {
        return Ok(timestamp);
    }

    if let Ok(dt) = parse_date_string(input, Utc::now(), Dialect::Us) {
        return Ok(dt.timestamp());
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        return Ok(dt.timestamp());
    }

    Err(DatadogError::DateParseError(format!(
        "Unable to parse time: '{}'",
        input
    )))
}

pub fn format_timestamp(timestamp: i64) -> String {
    DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| format!("Invalid timestamp: {}", timestamp))
}

pub fn truncate_stack_trace(stack: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = stack.lines().collect();

    if lines.len() <= max_lines {
        stack.to_string()
    } else {
        let truncated: Vec<&str> = lines.iter().take(max_lines).copied().collect();
        format!(
            "{}\n... ({} more lines)",
            truncated.join("\n"),
            lines.len() - max_lines
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_now() {
        let result = parse_time("now");
        assert!(result.is_ok());
        let timestamp = result.unwrap();
        assert!(timestamp > 0);
    }

    #[test]
    fn test_parse_time_unix_timestamp() {
        let result = parse_time("1704067200");
        assert_eq!(result.unwrap(), 1704067200);
    }

    #[test]
    fn test_parse_time_natural_language() {
        let result = parse_time("1 hour ago");
        assert!(result.is_ok());
        let now = Utc::now().timestamp();
        let parsed = result.unwrap();
        assert!(parsed < now);
        assert!(parsed > now - 7200);
    }

    #[test]
    fn test_parse_time_iso8601() {
        let result = parse_time("2024-01-01T00:00:00Z");
        assert_eq!(result.unwrap(), 1704067200);
    }

    #[test]
    fn test_parse_time_invalid() {
        let result = parse_time("invalid time string");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_timestamp() {
        let formatted = format_timestamp(1704067200);
        assert!(formatted.contains("2024-01-01"));
        assert!(formatted.contains("00:00:00 UTC"));
    }

    #[test]
    fn test_format_invalid_timestamp() {
        let formatted = format_timestamp(i64::MAX);
        assert!(formatted.contains("Invalid timestamp"));
    }

    #[test]
    fn test_truncate_stack_trace_short() {
        let stack = "Line 1\nLine 2\nLine 3";
        let result = truncate_stack_trace(stack, 5);
        assert_eq!(result, stack);
    }

    #[test]
    fn test_truncate_stack_trace_long() {
        let stack = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6";
        let result = truncate_stack_trace(stack, 3);
        assert!(result.contains("Line 1"));
        assert!(result.contains("Line 2"));
        assert!(result.contains("Line 3"));
        assert!(result.contains("... (3 more lines)"));
        assert!(!result.contains("Line 4"));
    }
}
