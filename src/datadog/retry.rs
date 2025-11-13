use std::time::Duration;

/// Maximum number of retry attempts for failed API requests
pub const MAX_RETRIES: u32 = 3;

/// Calculate exponential backoff duration for a given retry attempt
///
/// Returns: Duration = 2^retry_count seconds
/// - Retry 1: 2 seconds
/// - Retry 2: 4 seconds
/// - Retry 3: 8 seconds
pub fn calculate_backoff(retry_count: u32) -> Duration {
    Duration::from_secs(2_u64.pow(retry_count))
}

/// Check if another retry should be attempted
pub fn should_retry(current_retry: u32) -> bool {
    current_retry < MAX_RETRIES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_backoff_progression() {
        assert_eq!(calculate_backoff(0), Duration::from_secs(1));
        assert_eq!(calculate_backoff(1), Duration::from_secs(2));
        assert_eq!(calculate_backoff(2), Duration::from_secs(4));
        assert_eq!(calculate_backoff(3), Duration::from_secs(8));
        assert_eq!(calculate_backoff(4), Duration::from_secs(16));
    }

    #[test]
    fn test_calculate_backoff_edge_cases() {
        assert_eq!(calculate_backoff(0), Duration::from_secs(1));
        assert_eq!(calculate_backoff(10), Duration::from_secs(1024));
    }

    #[test]
    fn test_should_retry_under_limit() {
        assert!(should_retry(0));
        assert!(should_retry(1));
        assert!(should_retry(2));
    }

    #[test]
    fn test_should_retry_at_limit() {
        assert!(!should_retry(3));
    }

    #[test]
    fn test_should_retry_over_limit() {
        assert!(!should_retry(4));
        assert!(!should_retry(5));
        assert!(!should_retry(100));
    }

    #[test]
    fn test_max_retries_constant() {
        assert_eq!(MAX_RETRIES, 3);
    }
}
