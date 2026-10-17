use std::time::Duration;

use crate::error::DatadogError;

/// Upper bounds for full-jitter exponential backoff: 2s, 4s, 8s, ...
const BASE_DELAY_SECS: u64 = 2;

/// Server-directed waits beyond this bound return the original error.
const MAX_DELAY: Duration = Duration::from_secs(30);

pub fn next_delay(
    error: &DatadogError,
    attempt: u32,
    max_retries: u32,
    retry_after_secs: Option<u64>,
) -> Option<Duration> {
    if attempt >= max_retries || !is_transient(error) {
        return None;
    }

    let reset_secs = match error {
        DatadogError::RateLimitError { reset_secs } => *reset_secs,
        _ => None,
    };
    match retry_after_secs.into_iter().chain(reset_secs).max() {
        Some(secs) => {
            let delay = Duration::from_secs(secs);
            (delay <= MAX_DELAY).then_some(delay)
        }
        None => Some(backoff(attempt).mul_f64(fastrand::f64())),
    }
}

pub(super) fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let value = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    value.parse().ok().or_else(|| {
        let date = httpdate::parse_http_date(value).ok()?;
        let delay = date
            .duration_since(std::time::SystemTime::now())
            .unwrap_or_default();
        Some(delay.as_secs() + u64::from(delay.subsec_nanos() > 0))
    })
}

/// Transient failures for the read-only requests this client issues:
/// transport errors, timeouts (client-side and HTTP 408), rate limits,
/// and 5xx responses. Everything else fails immediately.
fn is_transient(error: &DatadogError) -> bool {
    match error {
        DatadogError::NetworkError(_)
        | DatadogError::TimeoutError
        | DatadogError::RateLimitError { .. } => true,
        DatadogError::ApiError { status, .. } => (500..600).contains(status) || *status == 408,
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
        for attempt in 0..3 {
            assert!(next_delay(&error, attempt, 3, None).unwrap() <= backoff(attempt));
        }
        assert_eq!(backoff(0), Duration::from_secs(2));
        assert_eq!(backoff(1), Duration::from_secs(4));
        assert_eq!(backoff(2), Duration::from_secs(8));
    }

    #[test]
    fn test_backoff_capped_at_max_delay() {
        assert_eq!(backoff(10), MAX_DELAY);
        assert_eq!(backoff(u32::MAX - 1), MAX_DELAY);
    }

    #[test]
    fn test_retry_budget_exhausted() {
        let error = server_error(503);
        assert_eq!(next_delay(&error, 3, 3, None), None);
        assert_eq!(next_delay(&error, 4, 3, None), None);
        assert_eq!(next_delay(&error, 0, 0, None), None);
    }

    #[test]
    fn test_server_errors_are_transient() {
        for status in [500, 502, 503, 504, 408] {
            assert!(next_delay(&server_error(status), 0, 3, None).is_some());
        }
    }

    #[test]
    fn test_client_errors_fail_immediately() {
        for status in [400, 404, 422, 600] {
            assert_eq!(next_delay(&server_error(status), 0, 3, None), None);
        }
        assert_eq!(
            next_delay(&DatadogError::AuthError("denied".to_string()), 0, 3, None),
            None
        );
        assert_eq!(
            next_delay(
                &DatadogError::DecodeError("bad json".to_string()),
                0,
                3,
                None
            ),
            None
        );
        assert_eq!(
            next_delay(&DatadogError::InvalidInput("bad".to_string()), 0, 3, None),
            None
        );
    }

    #[test]
    fn test_timeout_is_transient() {
        assert!(next_delay(&DatadogError::TimeoutError, 0, 3, None).unwrap() <= backoff(0));
    }

    #[test]
    fn test_rate_limit_waits_for_server_reset() {
        let error = DatadogError::RateLimitError {
            reset_secs: Some(15),
        };
        assert_eq!(
            next_delay(&error, 0, 3, None),
            Some(Duration::from_secs(15))
        );
    }

    #[test]
    fn test_rate_limit_reset_beyond_cap_fails_fast() {
        let error = DatadogError::RateLimitError {
            reset_secs: Some(MAX_DELAY.as_secs() + 1),
        };
        assert_eq!(next_delay(&error, 0, 3, None), None);
    }

    #[test]
    fn test_rate_limit_without_reset_uses_backoff() {
        let error = DatadogError::RateLimitError { reset_secs: None };
        assert!(next_delay(&error, 0, 3, None).unwrap() <= backoff(0));
    }

    #[test]
    fn server_directed_waits_apply_only_to_retryable_errors() {
        let error = server_error(503);
        for seconds in [0, 12, 30] {
            assert_eq!(
                next_delay(&error, 0, 3, Some(seconds)),
                Some(Duration::from_secs(seconds))
            );
        }
        assert_eq!(next_delay(&error, 0, 3, Some(31)), None);
        assert_eq!(next_delay(&error, 3, 3, Some(12)), None);
        assert_eq!(next_delay(&server_error(400), 0, 3, Some(12)), None);
    }

    #[test]
    fn rate_limit_headers_never_shorten_each_others_wait() {
        let error = DatadogError::RateLimitError {
            reset_secs: Some(15),
        };
        assert_eq!(
            next_delay(&error, 0, 3, Some(5)),
            Some(Duration::from_secs(15))
        );
        assert_eq!(
            next_delay(&error, 0, 3, Some(20)),
            Some(Duration::from_secs(20))
        );
        assert_eq!(next_delay(&error, 0, 3, Some(60)), None);
    }
    #[test]
    fn retry_after_supports_http_dates_and_ignores_invalid_values() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "retry-after",
            "Sun, 06 Nov 1994 08:49:37 GMT".parse().unwrap(),
        );
        assert_eq!(retry_after(&headers), Some(0));
        headers.insert("retry-after", "invalid".parse().unwrap());
        assert_eq!(retry_after(&headers), None);
        headers.insert("retry-after", "12".parse().unwrap());
        assert_eq!(retry_after(&headers), Some(12));
    }
}
