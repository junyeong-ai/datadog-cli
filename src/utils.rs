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
