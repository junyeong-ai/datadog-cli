use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::error::{DatadogError, Result};

/// Latest accepted Unix timestamp and offset: 9999-12-31T23:59:59Z. Larger
/// values are almost always milliseconds pasted where seconds are expected,
/// and the bound keeps resolved times convertible to milliseconds.
const MAX_UNIX_SECONDS: i64 = 253_402_300_799;

const EXPECTED_FORMS: &str = "expected now, a past offset such as 15m, 1h, 7d, now-1h or \
    '2 hours ago', RFC3339 such as 2024-01-01T00:00:00Z, or Unix seconds";

/// A time argument resolved against the single reference instant of a command.
/// Offsets always point into the past.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeExpr {
    Ago(i64),
    At(i64),
}

impl FromStr for TimeExpr {
    type Err = String;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        let input = input.trim();
        if input == "now" {
            return Ok(Self::Ago(0));
        }
        if !input.is_empty() && input.bytes().all(|byte| byte.is_ascii_digit()) {
            return input
                .parse()
                .ok()
                .filter(|seconds| *seconds <= MAX_UNIX_SECONDS)
                .map(Self::At)
                .ok_or_else(|| {
                    "Unix timestamp out of range (expected seconds; is this milliseconds?)".into()
                });
        }
        if let Ok(time) = DateTime::parse_from_rfc3339(input) {
            return Ok(Self::At(time.timestamp()));
        }
        let offset = input
            .strip_prefix("now-")
            .or_else(|| input.strip_suffix(" ago"))
            .unwrap_or(input);
        offset_seconds(offset)
            .map(Self::Ago)
            .ok_or_else(|| EXPECTED_FORMS.into())
    }
}

impl TimeExpr {
    pub fn resolve(self, now: DateTime<Utc>) -> i64 {
        match self {
            Self::Ago(seconds) => now.timestamp().saturating_sub(seconds),
            Self::At(seconds) => seconds,
        }
    }
}

fn offset_seconds(offset: &str) -> Option<i64> {
    let unit_start = offset.find(|character: char| !character.is_ascii_digit())?;
    let amount = offset[..unit_start].parse::<i64>().ok()?;
    let unit = match offset[unit_start..].trim() {
        "s" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3_600,
        "d" | "day" | "days" => 86_400,
        "w" | "week" | "weeks" => 604_800,
        _ => return None,
    };
    amount
        .checked_mul(unit)
        .filter(|seconds| *seconds <= MAX_UNIX_SECONDS)
}

/// A resolved, ordered time range in Unix seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    pub from: i64,
    pub to: i64,
}

impl TimeRange {
    pub fn resolve(from: TimeExpr, to: TimeExpr, now: DateTime<Utc>) -> Result<Self> {
        let (from, to) = (from.resolve(now), to.resolve(now));
        if from > to {
            return Err(DatadogError::InvalidInput(
                "Start time must not be after end time".into(),
            ));
        }
        Ok(Self { from, to })
    }

    pub fn rfc3339(&self) -> Result<(String, String)> {
        Ok((rfc3339(self.from)?, rfc3339(self.to)?))
    }
}

pub fn rfc3339(timestamp: i64) -> Result<String> {
    DateTime::from_timestamp(timestamp, 0)
        .map(|time| time.to_rfc3339())
        .ok_or_else(|| DatadogError::InvalidInput("Time is outside the supported range".into()))
}

pub fn format_timestamp(timestamp: i64) -> String {
    DateTime::from_timestamp(timestamp, 0)
        .map(|time| time.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| format!("Invalid timestamp: {timestamp}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_704_067_200, 0).unwrap()
    }

    fn resolve(input: &str) -> std::result::Result<i64, String> {
        input.parse::<TimeExpr>().map(|time| time.resolve(now()))
    }

    #[test]
    fn relative_forms_point_into_the_past() {
        let hour_ago = 1_704_067_200 - 3_600;
        for input in [
            "1h",
            "60m",
            "3600s",
            "now-1h",
            "1 hour ago",
            "1hour",
            " 1 hr ",
            "60 minutes ago",
        ] {
            assert_eq!(resolve(input), Ok(hour_ago), "{input}");
        }
        assert_eq!(resolve("7d"), Ok(1_704_067_200 - 604_800));
        assert_eq!(resolve("1w"), resolve("7 days"));
        assert_eq!(resolve("now"), Ok(1_704_067_200));
        assert_eq!(resolve("0m"), Ok(1_704_067_200));
    }

    #[test]
    fn absolute_forms_ignore_the_reference_instant() {
        assert_eq!(resolve("2024-01-01T00:00:00Z"), Ok(1_704_067_200));
        assert_eq!(resolve("2024-01-01T09:00:00+09:00"), Ok(1_704_067_200));
        assert_eq!(resolve("1609459200"), Ok(1_609_459_200));
        assert_eq!(resolve("253402300799"), Ok(MAX_UNIX_SECONDS));
    }

    #[test]
    fn unlisted_forms_are_rejected_instead_of_guessed() {
        for input in [
            "",
            "Now",
            "monday",
            "yesterday",
            "1 month ago",
            "1.5h",
            "-1h",
            "+1h",
            "in 1h",
            "now+1h",
            "now-1h ago",
            "-3600",
            "2024-01-01",
            "h",
        ] {
            assert_eq!(resolve(input), Err(EXPECTED_FORMS.into()), "{input}");
        }
        for input in [
            "1704067200000",
            &i64::MAX.to_string(),
            "99999999999999999999",
        ] {
            assert!(
                resolve(input).unwrap_err().contains("milliseconds"),
                "{input}"
            );
        }
        assert!(resolve(&format!("{}s", MAX_UNIX_SECONDS + 1)).is_err());
        assert!(resolve(&format!("{}w", i64::MAX)).is_err());
    }

    #[test]
    fn ranges_share_one_instant_and_reject_reversed_bounds() {
        let range = |from: &str, to: &str| {
            TimeRange::resolve(from.parse().unwrap(), to.parse().unwrap(), now())
        };
        assert_eq!(
            range("1h", "now").unwrap(),
            TimeRange {
                from: 1_704_063_600,
                to: 1_704_067_200
            }
        );
        assert!(range("now", "now").is_ok());
        assert!(range("now", "1h").is_err());
        assert!(range("2024-01-02T00:00:00Z", "2024-01-01T00:00:00Z").is_err());
        assert_eq!(
            range("1609459200", "1609462800")
                .unwrap()
                .rfc3339()
                .unwrap(),
            (
                "2021-01-01T00:00:00+00:00".into(),
                "2021-01-01T01:00:00+00:00".into()
            )
        );
    }

    #[test]
    fn formatted_timestamps_report_unrepresentable_values() {
        assert_eq!(format_timestamp(1_704_067_200), "2024-01-01 00:00:00 UTC");
        assert!(format_timestamp(i64::MAX).contains("Invalid timestamp"));
    }
}
