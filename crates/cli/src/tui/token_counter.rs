//! Token counter with human-readable formatting
//!
//! Provides reusable token counting and formatting for display throughout TUI.

use std::fmt;

/// Token count with human-readable formatting
#[derive(Debug, Clone, Copy, Default)]
pub struct TokenCount {
    pub input: u32,
    pub output: u32,
}

impl TokenCount {
    /// Create a new token count
    pub fn new(input: u32, output: u32) -> Self {
        Self { input, output }
    }

    /// Total tokens (input + output)
    pub fn total(&self) -> u32 {
        self.input.saturating_add(self.output)
    }

    /// Update token counts (additive)
    pub fn add(&mut self, input: u32, output: u32) {
        self.input = self.input.saturating_add(input);
        self.output = self.output.saturating_add(output);
    }

    /// Format a token count with K/M suffixes
    ///
    /// Examples:
    /// - 0-999: "42"
    /// - 1,000-999,999: "1.2K"
    /// - 1,000,000+: "2.1M"
    pub fn format_count(count: u32) -> String {
        match count {
            0..=999 => count.to_string(),
            1_000..=999_999 => {
                let k = count as f32 / 1_000.0;
                if k >= 100.0 {
                    // No decimals for 100K+
                    format!("{}K", k as u32)
                } else if k >= 10.0 {
                    // One decimal for 10K-99K
                    format!("{:.1}K", k)
                } else {
                    // One decimal for 1K-9.9K
                    format!("{:.1}K", k)
                }
            }
            1_000_000.. => {
                let m = count as f32 / 1_000_000.0;
                if m >= 100.0 {
                    // No decimals for 100M+
                    format!("{}M", m as u32)
                } else if m >= 10.0 {
                    // One decimal for 10M-99M
                    format!("{:.1}M", m)
                } else {
                    // One decimal for 1M-9.9M
                    format!("{:.1}M", m)
                }
            }
        }
    }

    /// Format input tokens
    pub fn format_input(&self) -> String {
        Self::format_count(self.input)
    }

    /// Format output tokens
    pub fn format_output(&self) -> String {
        Self::format_count(self.output)
    }

    /// Format total tokens
    pub fn format_total(&self) -> String {
        Self::format_count(self.total())
    }

    /// Format as "input → output" (compact)
    pub fn format_compact(&self) -> String {
        format!("{} → {}", self.format_input(), self.format_output())
    }

    /// Format as "total (input+output)" (verbose)
    pub fn format_verbose(&self) -> String {
        format!(
            "{} ({}+{})",
            self.format_total(),
            self.format_input(),
            self.format_output()
        )
    }
}

impl fmt::Display for TokenCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_compact())
    }
}

impl From<(u32, u32)> for TokenCount {
    fn from((input, output): (u32, u32)) -> Self {
        Self::new(input, output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_count() {
        assert_eq!(TokenCount::format_count(0), "0");
        assert_eq!(TokenCount::format_count(42), "42");
        assert_eq!(TokenCount::format_count(999), "999");
        assert_eq!(TokenCount::format_count(1_000), "1.0K");
        assert_eq!(TokenCount::format_count(1_234), "1.2K");
        assert_eq!(TokenCount::format_count(9_999), "10.0K");
        assert_eq!(TokenCount::format_count(10_000), "10.0K");
        assert_eq!(TokenCount::format_count(99_999), "100K");
        assert_eq!(TokenCount::format_count(100_000), "100K");
        assert_eq!(TokenCount::format_count(999_999), "1000K");
        assert_eq!(TokenCount::format_count(1_000_000), "1.0M");
        assert_eq!(TokenCount::format_count(1_234_567), "1.2M");
        assert_eq!(TokenCount::format_count(10_000_000), "10.0M");
        assert_eq!(TokenCount::format_count(100_000_000), "100M");
    }

    #[test]
    fn test_token_count_operations() {
        let mut count = TokenCount::new(100, 200);
        assert_eq!(count.total(), 300);
        assert_eq!(count.format_input(), "100");
        assert_eq!(count.format_output(), "200");
        assert_eq!(count.format_total(), "300");

        count.add(50, 100);
        assert_eq!(count.input, 150);
        assert_eq!(count.output, 300);
        assert_eq!(count.total(), 450);
    }

    #[test]
    fn test_format_compact() {
        let count = TokenCount::new(1_234, 5_678);
        assert_eq!(count.format_compact(), "1.2K → 5.7K");
    }

    #[test]
    fn test_format_verbose() {
        let count = TokenCount::new(1_234, 5_678);
        assert_eq!(count.format_verbose(), "6.9K (1.2K+5.7K)");
    }

    #[test]
    fn test_display_trait() {
        let count = TokenCount::new(100, 200);
        assert_eq!(format!("{}", count), "100 → 200");
    }

    #[test]
    fn test_from_tuple() {
        let count: TokenCount = (100, 200).into();
        assert_eq!(count.input, 100);
        assert_eq!(count.output, 200);
    }

    #[test]
    fn test_saturation() {
        let mut count = TokenCount::new(u32::MAX - 10, 0);
        count.add(20, 0);  // Should saturate at u32::MAX
        assert_eq!(count.input, u32::MAX);
    }
}
