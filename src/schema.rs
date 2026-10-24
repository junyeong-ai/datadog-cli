use jsonschema::{ValidationError, Validator};
use serde_json::Value;

const MAX_REPORTED_VIOLATIONS: usize = 10;

/// Describes each violation by instance location and schema expectation.
/// Instance values are masked so inputs and responses never reach diagnostics.
pub(crate) fn violations(validator: &Validator, instance: &Value) -> Option<String> {
    let mut errors = validator.iter_errors(instance);
    let mut reported = vec![describe(&errors.next()?)];
    reported.extend(
        errors
            .by_ref()
            .take(MAX_REPORTED_VIOLATIONS - 1)
            .map(|error| describe(&error)),
    );
    let omitted = errors.count();
    if omitted > 0 {
        reported.push(format!("{omitted} more"));
    }
    Some(reported.join("; "))
}

fn describe(error: &ValidationError<'_>) -> String {
    let location = error.instance_path();
    let location = if location.is_empty() {
        "/"
    } else {
        location.as_str()
    };
    format!("{location}: {}", error.masked())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn validator(schema: Value) -> Validator {
        jsonschema::options().build(&schema).unwrap()
    }

    #[test]
    fn violations_name_locations_and_expectations_without_instance_values() {
        let validator = validator(json!({
            "type": "object",
            "required": ["id"],
            "additionalProperties": false,
            "properties": {
                "id": {"type": "string"},
                "limit": {"type": "integer", "maximum": 1000},
                "sort": {"enum": ["timestamp", "-timestamp"]}
            }
        }));
        let message = violations(
            &validator,
            &json!({"limit": 5000, "sort": "secret-sort", "extra": true}),
        )
        .unwrap();
        for expected in [
            "/: \"id\" is a required property",
            "/: Additional properties are not allowed ('extra' was unexpected)",
            "/limit: value is greater than the maximum of 1000",
            "/sort: value is not one of \"timestamp\" or \"-timestamp\"",
        ] {
            assert!(message.contains(expected), "{message}");
        }
        assert!(!message.contains("secret-sort"));
        assert!(!message.contains("5000"));
        assert!(violations(&validator, &json!({"id": "a"})).is_none());
    }

    #[test]
    fn violations_report_a_bounded_number_of_errors() {
        let validator = validator(json!({"type": "array", "items": {"type": "integer"}}));
        let message = violations(&validator, &json!(vec!["x"; 25])).unwrap();
        assert_eq!(
            message.matches("is not of type").count(),
            MAX_REPORTED_VIOLATIONS
        );
        assert!(message.ends_with("; 15 more"), "{message}");
    }
}
