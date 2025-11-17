//! Custom assertions for TUI testing
//!
//! Provides high-level assertions for common TUI test patterns

/// Assert that buffer contains text
#[macro_export]
macro_rules! assert_buffer_contains {
    ($harness:expr, $text:expr) => {
        assert!(
            $harness.contains($text),
            "Buffer does not contain expected text: '{}'",
            $text
        );
    };
}

/// Assert that buffer does not contain text
#[macro_export]
macro_rules! assert_buffer_not_contains {
    ($harness:expr, $text:expr) => {
        assert!(
            !$harness.contains($text),
            "Buffer unexpectedly contains text: '{}'",
            $text
        );
    };
}

/// Assert that specific line contains text
#[macro_export]
macro_rules! assert_line_contains {
    ($harness:expr, $line:expr, $text:expr) => {
        let line_content = $harness.get_line($line);
        assert!(
            line_content.contains($text),
            "Line {} does not contain expected text: '{}'\nLine content: '{}'",
            $line,
            $text,
            line_content
        );
    };
}

/// Assert message count
#[macro_export]
macro_rules! assert_message_count {
    ($messages:expr, $expected:expr) => {
        assert_eq!(
            $messages.len(),
            $expected,
            "Expected {} messages, but found {}",
            $expected,
            $messages.len()
        );
    };
}

/// Test utilities
pub struct TestAssertions;

impl TestAssertions {
    /// Check if string contains all of the given substrings
    pub fn contains_all(haystack: &str, needles: &[&str]) -> bool {
        needles.iter().all(|needle| haystack.contains(needle))
    }

    /// Check if string contains any of the given substrings
    pub fn contains_any(haystack: &str, needles: &[&str]) -> bool {
        needles.iter().any(|needle| haystack.contains(needle))
    }

    /// Check if string matches pattern approximately (ignoring whitespace differences)
    pub fn matches_approximately(text: &str, pattern: &str) -> bool {
        let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        normalize(text).contains(&normalize(pattern))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_all() {
        assert!(TestAssertions::contains_all(
            "hello world test",
            &["hello", "world"]
        ));
        assert!(!TestAssertions::contains_all(
            "hello world test",
            &["hello", "missing"]
        ));
    }

    #[test]
    fn test_contains_any() {
        assert!(TestAssertions::contains_any(
            "hello world test",
            &["missing", "world"]
        ));
        assert!(!TestAssertions::contains_any(
            "hello world test",
            &["missing", "absent"]
        ));
    }

    #[test]
    fn test_matches_approximately() {
        assert!(TestAssertions::matches_approximately(
            "hello    world",
            "hello world"
        ));
        assert!(TestAssertions::matches_approximately(
            "hello\n  world\n test",
            "hello world test"
        ));
        assert!(!TestAssertions::matches_approximately(
            "hello world",
            "hello test"
        ));
    }
}
