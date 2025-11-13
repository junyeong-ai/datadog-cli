use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatadogError {
    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Invalid date format: {0}")]
    DateParseError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Rate limit exceeded")]
    RateLimitError,

    #[error("Timeout occurred")]
    TimeoutError,
}

pub type Result<T> = std::result::Result<T, DatadogError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_display() {
        let error = DatadogError::ApiError("Test error".to_string());
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("API request failed"));
        assert!(error_msg.contains("Test error"));
    }

    #[test]
    fn test_auth_error_display() {
        let error = DatadogError::AuthError("Invalid credentials".to_string());
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Authentication failed"));
        assert!(error_msg.contains("Invalid credentials"));
    }

    #[test]
    fn test_date_parse_error_display() {
        let error = DatadogError::DateParseError("Bad format".to_string());
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Invalid date format"));
    }

    #[test]
    fn test_invalid_input_display() {
        let error = DatadogError::InvalidInput("Missing parameter".to_string());
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Invalid input"));
    }

    #[test]
    fn test_rate_limit_error_display() {
        let error = DatadogError::RateLimitError;
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Rate limit exceeded"));
    }

    #[test]
    fn test_timeout_error_display() {
        let error = DatadogError::TimeoutError;
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Timeout occurred"));
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
        let error = DatadogError::ApiError("test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ApiError"));
    }
}
