use std::time::Duration;

use crate::error::DatadogError;

/// Base for exponential backoff: 2s, 4s, 8s, ...
const BASE_DELAY_SECS: u64 = 2;

/// Upper bound on any single retry delay. Rate-limit resets further away
/// than this are returned to the user instead of silently blocking the CLI.
const MAX_DELAY: Duration = Duration::from_secs(30);

/// Delay to wait before retrying `error`, or `None` when the error is not
/// transient, the retry budget (`attempt >= max_retries`) is spent, or a
/// rate-limit reset exceeds `MAX_DELAY`.
///
/// Rate-limited requests wait for the server-provided reset when available;
/// everything else uses capped exponential backoff.
pub fn next_delay(error: &DatadogError, attempt: u32, max_retries: u32) -> Option<Duration> {
    if attempt >= max_retries || !is_transient(error) {
        return None;
    }

    match error {
        DatadogError::RateLimitError {
            reset_secs: Some(secs),
        } => {
            let delay = Duration::from_secs(*secs);
            (delay <= MAX_DELAY).then_some(delay)
        }
        _ => Some(backoff(attempt)),
    }
}

/// Transient failures for the read-only requests this client issues:
/// transport errors, timeouts (client-side and HTTP 408), rate limits,
/// and 5xx responses. Everything else fails immediately.
fn is_transient(error: &DatadogError) -> bool {
    match error {
        DatadogError::NetworkError(_)
        | DatadogError::TimeoutError
        | DatadogError::RateLimitError { .. } => true,
        DatadogError::ApiError { status, .. } => *status >= 500 || *status == 408,
        _ => false,
    }
}

fn backoff(attempt: u32) -> Duration {
    Duration::from_secs(BASE_DELAY_SECS.saturating_pow(attempt.saturating_add(1))).min(MAX_DELAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server_error(status: u16) -> DatadogError {
        DatadogError::ApiError {
            status,
            message: "error".to_string(),
        }
    }

    #[test]
    fn test_backoff_progression() {
        let error = server_error(503);
        assert_eq!(next_delay(&error, 0, 3), Some(Duration::from_secs(2)));
        assert_eq!(next_delay(&error, 1, 3), Some(Duration::from_secs(4)));
        assert_eq!(next_delay(&error, 2, 3), Some(Duration::from_secs(8)));
    }

    #[test]
    fn test_backoff_capped_at_max_delay() {
        let error = server_error(503);
        assert_eq!(next_delay(&error, 10, 100), Some(MAX_DELAY));
        assert_eq!(next_delay(&error, u32::MAX - 1, u32::MAX), Some(MAX_DELAY));
    }

    #[test]
    fn test_retry_budget_exhausted() {
        let error = server_error(503);
        assert_eq!(next_delay(&error, 3, 3), None);
        assert_eq!(next_delay(&error, 4, 3), None);
        assert_eq!(next_delay(&error, 0, 0), None);
    }

    #[test]
    fn test_server_errors_are_transient() {
        for status in [500, 502, 503, 504, 408] {
            assert!(next_delay(&server_error(status), 0, 3).is_some());
        }
    }

    #[test]
    fn test_client_errors_fail_immediately() {
        for status in [400, 404, 422] {
            assert_eq!(next_delay(&server_error(status), 0, 3), None);
        }
        assert_eq!(
            next_delay(&DatadogError::AuthError("denied".to_string()), 0, 3),
            None
        );
        assert_eq!(
            next_delay(&DatadogError::DecodeError("bad json".to_string()), 0, 3),
            None
        );
        assert_eq!(
            next_delay(&DatadogError::InvalidInput("bad".to_string()), 0, 3),
            None
        );
    }

    #[test]
    fn test_timeout_is_transient() {
        assert_eq!(
            next_delay(&DatadogError::TimeoutError, 0, 3),
            Some(Duration::from_secs(2))
        );
    }

    #[test]
    fn test_rate_limit_waits_for_server_reset() {
        let error = DatadogError::RateLimitError {
            reset_secs: Some(15),
        };
        assert_eq!(next_delay(&error, 0, 3), Some(Duration::from_secs(15)));
    }

    #[test]
    fn test_rate_limit_reset_beyond_cap_fails_fast() {
        let error = DatadogError::RateLimitError {
            reset_secs: Some(MAX_DELAY.as_secs() + 1),
        };
        assert_eq!(next_delay(&error, 0, 3), None);
    }

    #[test]
    fn test_rate_limit_without_reset_uses_backoff() {
        let error = DatadogError::RateLimitError { reset_secs: None };
        assert_eq!(next_delay(&error, 0, 3), Some(Duration::from_secs(2)));
    }
}
