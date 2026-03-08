//! Scheduled task support for the /loop command.
//!
//! Provides interval parsing and a `ScheduledTask` struct that the session
//! loop can use to drive recurring prompt execution.

use std::time::{Duration, Instant};

/// A recurring task created by `/loop`.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    /// How often to run the prompt.
    pub interval: Duration,
    /// The prompt text to execute each cycle.
    pub prompt: String,
    /// When this task was created.
    pub created_at: Instant,
}

/// Parse a human-friendly interval string into a [`Duration`].
///
/// Accepted suffixes:
/// - `s` — seconds (e.g. `30s`)
/// - `m` — minutes (e.g. `5m`)
/// - `h` — hours   (e.g. `1h`)
///
/// A bare number without a suffix is treated as seconds.
///
/// # Errors
///
/// Returns `None` when the string is empty, the numeric part is not a valid
/// positive integer, or the suffix is unrecognised.
pub fn parse_interval(input: &str) -> Option<Duration> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    // Split into numeric prefix and optional suffix character.
    let (num_str, suffix) = if input.ends_with(|c: char| c.is_ascii_alphabetic()) {
        let boundary = input.len() - 1;
        (&input[..boundary], Some(&input[boundary..]))
    } else {
        (input, None)
    };

    let value: u64 = num_str.parse().ok().filter(|&v| v > 0)?;

    let secs = match suffix {
        Some("s") | None => value,
        Some("m") => value.checked_mul(60)?,
        Some("h") => value.checked_mul(3600)?,
        _ => return None,
    };

    Some(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_interval ──────────────────────────────────────────────────

    #[test]
    fn test_parse_seconds() {
        assert_eq!(parse_interval("30s"), Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_parse_minutes() {
        assert_eq!(parse_interval("5m"), Some(Duration::from_secs(300)));
    }

    #[test]
    fn test_parse_hours() {
        assert_eq!(parse_interval("1h"), Some(Duration::from_secs(3600)));
    }

    #[test]
    fn test_parse_bare_number_defaults_to_seconds() {
        assert_eq!(parse_interval("45"), Some(Duration::from_secs(45)));
    }

    #[test]
    fn test_parse_with_whitespace() {
        assert_eq!(parse_interval("  10m  "), Some(Duration::from_secs(600)));
    }

    #[test]
    fn test_parse_zero_returns_none() {
        assert_eq!(parse_interval("0s"), None);
        assert_eq!(parse_interval("0"), None);
    }

    #[test]
    fn test_parse_empty_returns_none() {
        assert_eq!(parse_interval(""), None);
        assert_eq!(parse_interval("   "), None);
    }

    #[test]
    fn test_parse_invalid_suffix() {
        assert_eq!(parse_interval("5x"), None);
        assert_eq!(parse_interval("10d"), None);
    }

    #[test]
    fn test_parse_non_numeric() {
        assert_eq!(parse_interval("abc"), None);
        assert_eq!(parse_interval("m"), None);
    }

    #[test]
    fn test_parse_negative() {
        // u64 parse will reject negative numbers
        assert_eq!(parse_interval("-5m"), None);
    }

    // ── ScheduledTask construction ──────────────────────────────────────

    #[test]
    fn test_scheduled_task_creation() {
        let task = ScheduledTask {
            interval: Duration::from_secs(60),
            prompt: "check status".to_string(),
            created_at: Instant::now(),
        };
        assert_eq!(task.interval, Duration::from_secs(60));
        assert_eq!(task.prompt, "check status");
    }
}
