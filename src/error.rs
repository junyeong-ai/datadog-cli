use thiserror::Error;

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum ErrorFormat {
    Text,
    Json,
}

#[derive(Error, Debug)]
pub enum DatadogError {
    #[error("API request failed (HTTP {status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Query failed: {0}")]
    QueryError(String),

    #[error("Interrupted")]
    Interrupted,

    #[error("Credentials unavailable: {0}")]
    CredentialError(String),

    #[error("Authentication failed (HTTP 401): {0}")]
    Unauthorized(String),

    #[error("Permission denied (HTTP 403): {0}")]
    PermissionDenied(String),

    #[error("Network error: {0}")]
    NetworkError(reqwest::Error),

    #[error("Unexpected response format: {0}")]
    DecodeError(String),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Rate limit exceeded{}", .reset_secs.map(|s| format!(" (resets in {s}s)")).unwrap_or_default())]
    RateLimitError { reset_secs: Option<u64> },

    #[error("Request timed out")]
    TimeoutError,
}

impl DatadogError {
    /// Process exit code for this error. Exit code 2 is left to clap's
    /// usage errors; codes 3-7 distinguish failure classes for scripting.
    pub fn exit_code(&self) -> i32 {
        match self {
            DatadogError::CredentialError(_)
            | DatadogError::Unauthorized(_)
            | DatadogError::PermissionDenied(_) => 3,
            DatadogError::ApiError { .. } | DatadogError::QueryError(_) => 4,
            DatadogError::Interrupted => 130,
            DatadogError::RateLimitError { .. } => 5,
            DatadogError::NetworkError(_) | DatadogError::TimeoutError => 6,
            DatadogError::DecodeError(_) => 7,
            DatadogError::InvalidInput(_)
            | DatadogError::JsonError(_)
            | DatadogError::IoError(_) => 1,
        }
    }

    pub fn diagnostic(&self) -> serde_json::Value {
        let (kind, status) = match self {
            Self::ApiError { status, .. } => ("api", Some(*status)),
            Self::CredentialError(_) => ("credentials", None),
            Self::Unauthorized(_) => ("authentication", Some(401)),
            Self::PermissionDenied(_) => ("permission_denied", Some(403)),
            Self::QueryError(_) => ("query", None),
            Self::Interrupted => ("interrupted", None),
            Self::NetworkError(_) => ("network", None),
            Self::DecodeError(_) => ("decode", None),
            Self::JsonError(_) => ("json", None),
            Self::IoError(_) => ("io", None),
            Self::InvalidInput(_) => ("invalid_input", None),
            Self::RateLimitError { .. } => ("rate_limit", Some(429)),
            Self::TimeoutError => ("timeout", None),
        };
        let mut error = serde_json::json!({"kind":kind,"exit_code":self.exit_code(),"message":self.to_string()});
        if let Some(status) = status {
            error["http_status"] = serde_json::json!(status);
        }
        if let Self::RateLimitError {
            reset_secs: Some(seconds),
        } = self
        {
            error["retry_after_seconds"] = serde_json::json!(seconds);
        }
        serde_json::json!({"error":error})
    }
}

pub type Result<T> = std::result::Result<T, DatadogError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_display() {
        let error = DatadogError::ApiError {
            status: 503,
            message: "Service unavailable".to_string(),
        };
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("API request failed"));
        assert!(error_msg.contains("HTTP 503"));
        assert!(error_msg.contains("Service unavailable"));
    }

    #[test]
    fn credential_failures_and_rejected_credentials_have_distinct_kinds() {
        for (error, kind, status) in [
            (
                DatadogError::CredentialError("missing".into()),
                "credentials",
                None,
            ),
            (
                DatadogError::Unauthorized("denied".into()),
                "authentication",
                Some(401),
            ),
            (
                DatadogError::PermissionDenied("denied".into()),
                "permission_denied",
                Some(403),
            ),
        ] {
            assert_eq!(error.exit_code(), 3);
            let diagnostic = error.diagnostic();
            assert_eq!(diagnostic["error"]["kind"], kind);
            assert_eq!(diagnostic["error"]["http_status"].as_u64(), status);
        }
        assert_eq!(
            DatadogError::Unauthorized("denied".into()).to_string(),
            "Authentication failed (HTTP 401): denied"
        );
    }

    #[test]
    fn test_decode_error_display() {
        let error = DatadogError::DecodeError("missing field `data`".to_string());
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Unexpected response format"));
        assert!(error_msg.contains("missing field `data`"));
    }

    #[test]
    fn test_invalid_input_display() {
        let error = DatadogError::InvalidInput("Missing parameter".to_string());
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Invalid input"));
    }

    #[test]
    fn test_rate_limit_error_display_without_reset() {
        let error = DatadogError::RateLimitError { reset_secs: None };
        assert_eq!(format!("{}", error), "Rate limit exceeded");
    }

    #[test]
    fn test_rate_limit_error_display_with_reset() {
        let error = DatadogError::RateLimitError {
            reset_secs: Some(42),
        };
        assert_eq!(format!("{}", error), "Rate limit exceeded (resets in 42s)");
    }

    #[test]
    fn test_timeout_error_display() {
        let error = DatadogError::TimeoutError;
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Request timed out"));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_str = "invalid json {";
        let result: serde_json::Result<serde_json::Value> = serde_json::from_str(json_str);
        let error = result.map_err(DatadogError::from).unwrap_err();

        match error {
            DatadogError::JsonError(_) => {}
            _ => panic!("Expected JsonError"),
        }
    }

    #[test]
    fn test_error_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DatadogError>();
    }

    #[test]
    fn test_error_debug_format() {
        let error = DatadogError::ApiError {
            status: 400,
            message: "test".to_string(),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ApiError"));
    }
}
