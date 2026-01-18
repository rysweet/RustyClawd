//! MCP Tool Search Configuration
//!
//! Handles the `auto:N` syntax for configuring MCP tool search thresholds.
//! Tool search dynamically loads MCP tools on-demand to preserve context window space.
//!
//! ## Configuration Values
//!
//! - `auto` - Auto-enable at 10% threshold (default)
//! - `auto:<N>` - Auto-enable at N% threshold (0-100)
//! - `true` - Always enabled
//! - `false` - Disabled (load all tools upfront)
//!
//! ## Usage
//!
//! ```ignore
//! use rustyclawd_cli::plugins::tool_search_config::ToolSearchConfig;
//!
//! let config = ToolSearchConfig::parse("auto:5").unwrap();
//! assert!(config.should_enable_tool_search(15)); // 15% > 5% threshold
//! ```

use std::fmt;
use std::str::FromStr;

/// Default threshold percentage for auto mode
pub const DEFAULT_THRESHOLD_PERCENT: u8 = 10;

/// Configuration for MCP tool search behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSearchConfig {
    /// Disabled - load all MCP tools upfront
    Disabled,
    /// Always enabled - use tool search regardless of context usage
    Enabled,
    /// Auto-enable when tool definitions exceed threshold percentage of context
    Auto {
        /// Threshold as percentage (0-100)
        threshold_percent: u8,
    },
}

impl Default for ToolSearchConfig {
    fn default() -> Self {
        ToolSearchConfig::Auto {
            threshold_percent: DEFAULT_THRESHOLD_PERCENT,
        }
    }
}

impl ToolSearchConfig {
    /// Parse a tool search configuration string
    ///
    /// Accepts:
    /// - "auto" - Auto-enable at 10% threshold
    /// - "auto:N" - Auto-enable at N% threshold
    /// - "true" - Always enabled
    /// - "false" - Disabled
    pub fn parse(s: &str) -> Result<Self, ToolSearchConfigError> {
        let s = s.trim().to_lowercase();

        match s.as_str() {
            "true" | "1" | "yes" | "enabled" => Ok(ToolSearchConfig::Enabled),
            "false" | "0" | "no" | "disabled" => Ok(ToolSearchConfig::Disabled),
            "auto" => Ok(ToolSearchConfig::Auto {
                threshold_percent: DEFAULT_THRESHOLD_PERCENT,
            }),
            _ if s.starts_with("auto:") => {
                let threshold_str = s.strip_prefix("auto:").unwrap();
                let threshold: u8 = threshold_str.parse().map_err(|_| {
                    ToolSearchConfigError::InvalidThreshold(threshold_str.to_string())
                })?;

                if threshold > 100 {
                    return Err(ToolSearchConfigError::ThresholdOutOfRange(threshold));
                }

                Ok(ToolSearchConfig::Auto {
                    threshold_percent: threshold,
                })
            }
            _ => Err(ToolSearchConfigError::InvalidFormat(s)),
        }
    }

    /// Determine if tool search should be enabled based on current context usage
    ///
    /// # Arguments
    /// * `tool_context_percent` - Percentage of context window used by tool definitions
    ///
    /// # Returns
    /// * `true` if tool search should be enabled
    pub fn should_enable_tool_search(&self, tool_context_percent: u8) -> bool {
        match self {
            ToolSearchConfig::Disabled => false,
            ToolSearchConfig::Enabled => true,
            ToolSearchConfig::Auto { threshold_percent } => {
                tool_context_percent >= *threshold_percent
            }
        }
    }

    /// Get the threshold percentage if in auto mode
    pub fn threshold_percent(&self) -> Option<u8> {
        match self {
            ToolSearchConfig::Auto { threshold_percent } => Some(*threshold_percent),
            _ => None,
        }
    }

    /// Check if tool search is explicitly disabled
    pub fn is_disabled(&self) -> bool {
        matches!(self, ToolSearchConfig::Disabled)
    }

    /// Check if tool search is always enabled
    pub fn is_always_enabled(&self) -> bool {
        matches!(self, ToolSearchConfig::Enabled)
    }

    /// Check if tool search uses auto mode
    pub fn is_auto(&self) -> bool {
        matches!(self, ToolSearchConfig::Auto { .. })
    }
}

impl FromStr for ToolSearchConfig {
    type Err = ToolSearchConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for ToolSearchConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolSearchConfig::Disabled => write!(f, "false"),
            ToolSearchConfig::Enabled => write!(f, "true"),
            ToolSearchConfig::Auto { threshold_percent } => {
                if *threshold_percent == DEFAULT_THRESHOLD_PERCENT {
                    write!(f, "auto")
                } else {
                    write!(f, "auto:{}", threshold_percent)
                }
            }
        }
    }
}

/// Errors that can occur when parsing tool search configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolSearchConfigError {
    /// Invalid format - not a recognized configuration value
    InvalidFormat(String),
    /// Invalid threshold value - not a valid number
    InvalidThreshold(String),
    /// Threshold out of range (must be 0-100)
    ThresholdOutOfRange(u8),
}

impl fmt::Display for ToolSearchConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolSearchConfigError::InvalidFormat(s) => {
                write!(
                    f,
                    "Invalid tool search config '{}'. Expected: auto, auto:<N>, true, or false",
                    s
                )
            }
            ToolSearchConfigError::InvalidThreshold(s) => {
                write!(
                    f,
                    "Invalid threshold '{}'. Expected a number between 0 and 100",
                    s
                )
            }
            ToolSearchConfigError::ThresholdOutOfRange(n) => {
                write!(
                    f,
                    "Threshold {} is out of range. Must be between 0 and 100",
                    n
                )
            }
        }
    }
}

impl std::error::Error for ToolSearchConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================
    // Parsing Tests
    // ==========================================

    #[test]
    fn test_parse_auto_default() {
        let config = ToolSearchConfig::parse("auto").unwrap();
        assert_eq!(
            config,
            ToolSearchConfig::Auto {
                threshold_percent: 10
            }
        );
    }

    #[test]
    fn test_parse_auto_with_threshold() {
        let config = ToolSearchConfig::parse("auto:5").unwrap();
        assert_eq!(
            config,
            ToolSearchConfig::Auto {
                threshold_percent: 5
            }
        );
    }

    #[test]
    fn test_parse_auto_zero_threshold() {
        let config = ToolSearchConfig::parse("auto:0").unwrap();
        assert_eq!(
            config,
            ToolSearchConfig::Auto {
                threshold_percent: 0
            }
        );
    }

    #[test]
    fn test_parse_auto_max_threshold() {
        let config = ToolSearchConfig::parse("auto:100").unwrap();
        assert_eq!(
            config,
            ToolSearchConfig::Auto {
                threshold_percent: 100
            }
        );
    }

    #[test]
    fn test_parse_auto_threshold_out_of_range() {
        let result = ToolSearchConfig::parse("auto:101");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ToolSearchConfigError::ThresholdOutOfRange(101)
        ));
    }

    #[test]
    fn test_parse_auto_invalid_threshold() {
        let result = ToolSearchConfig::parse("auto:abc");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ToolSearchConfigError::InvalidThreshold(_)
        ));
    }

    #[test]
    fn test_parse_true_variants() {
        for input in &["true", "1", "yes", "enabled", "TRUE", "True"] {
            let config = ToolSearchConfig::parse(input).unwrap();
            assert_eq!(
                config,
                ToolSearchConfig::Enabled,
                "Failed for input: {}",
                input
            );
        }
    }

    #[test]
    fn test_parse_false_variants() {
        for input in &["false", "0", "no", "disabled", "FALSE", "False"] {
            let config = ToolSearchConfig::parse(input).unwrap();
            assert_eq!(
                config,
                ToolSearchConfig::Disabled,
                "Failed for input: {}",
                input
            );
        }
    }

    #[test]
    fn test_parse_invalid_format() {
        let result = ToolSearchConfig::parse("invalid");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ToolSearchConfigError::InvalidFormat(_)
        ));
    }

    #[test]
    fn test_parse_with_whitespace() {
        let config = ToolSearchConfig::parse("  auto:15  ").unwrap();
        assert_eq!(
            config,
            ToolSearchConfig::Auto {
                threshold_percent: 15
            }
        );
    }

    #[test]
    fn test_parse_case_insensitive() {
        let config = ToolSearchConfig::parse("AUTO:20").unwrap();
        assert_eq!(
            config,
            ToolSearchConfig::Auto {
                threshold_percent: 20
            }
        );
    }

    // ==========================================
    // Behavior Tests
    // ==========================================

    #[test]
    fn test_should_enable_disabled() {
        let config = ToolSearchConfig::Disabled;
        assert!(!config.should_enable_tool_search(0));
        assert!(!config.should_enable_tool_search(50));
        assert!(!config.should_enable_tool_search(100));
    }

    #[test]
    fn test_should_enable_enabled() {
        let config = ToolSearchConfig::Enabled;
        assert!(config.should_enable_tool_search(0));
        assert!(config.should_enable_tool_search(50));
        assert!(config.should_enable_tool_search(100));
    }

    #[test]
    fn test_should_enable_auto_below_threshold() {
        let config = ToolSearchConfig::Auto {
            threshold_percent: 10,
        };
        assert!(!config.should_enable_tool_search(5));
        assert!(!config.should_enable_tool_search(9));
    }

    #[test]
    fn test_should_enable_auto_at_threshold() {
        let config = ToolSearchConfig::Auto {
            threshold_percent: 10,
        };
        assert!(config.should_enable_tool_search(10));
    }

    #[test]
    fn test_should_enable_auto_above_threshold() {
        let config = ToolSearchConfig::Auto {
            threshold_percent: 10,
        };
        assert!(config.should_enable_tool_search(15));
        assert!(config.should_enable_tool_search(100));
    }

    #[test]
    fn test_should_enable_auto_zero_threshold() {
        let config = ToolSearchConfig::Auto {
            threshold_percent: 0,
        };
        // Should always enable when threshold is 0
        assert!(config.should_enable_tool_search(0));
        assert!(config.should_enable_tool_search(1));
    }

    // ==========================================
    // Accessor Tests
    // ==========================================

    #[test]
    fn test_threshold_percent() {
        assert_eq!(
            ToolSearchConfig::Auto {
                threshold_percent: 15
            }
            .threshold_percent(),
            Some(15)
        );
        assert_eq!(ToolSearchConfig::Enabled.threshold_percent(), None);
        assert_eq!(ToolSearchConfig::Disabled.threshold_percent(), None);
    }

    #[test]
    fn test_is_disabled() {
        assert!(ToolSearchConfig::Disabled.is_disabled());
        assert!(!ToolSearchConfig::Enabled.is_disabled());
        assert!(!ToolSearchConfig::Auto {
            threshold_percent: 10
        }
        .is_disabled());
    }

    #[test]
    fn test_is_always_enabled() {
        assert!(ToolSearchConfig::Enabled.is_always_enabled());
        assert!(!ToolSearchConfig::Disabled.is_always_enabled());
        assert!(!ToolSearchConfig::Auto {
            threshold_percent: 10
        }
        .is_always_enabled());
    }

    #[test]
    fn test_is_auto() {
        assert!(ToolSearchConfig::Auto {
            threshold_percent: 10
        }
        .is_auto());
        assert!(!ToolSearchConfig::Enabled.is_auto());
        assert!(!ToolSearchConfig::Disabled.is_auto());
    }

    // ==========================================
    // Display Tests
    // ==========================================

    #[test]
    fn test_display_disabled() {
        assert_eq!(ToolSearchConfig::Disabled.to_string(), "false");
    }

    #[test]
    fn test_display_enabled() {
        assert_eq!(ToolSearchConfig::Enabled.to_string(), "true");
    }

    #[test]
    fn test_display_auto_default() {
        let config = ToolSearchConfig::Auto {
            threshold_percent: 10,
        };
        assert_eq!(config.to_string(), "auto");
    }

    #[test]
    fn test_display_auto_custom() {
        let config = ToolSearchConfig::Auto {
            threshold_percent: 5,
        };
        assert_eq!(config.to_string(), "auto:5");
    }

    // ==========================================
    // FromStr Tests
    // ==========================================

    #[test]
    fn test_from_str() {
        let config: ToolSearchConfig = "auto:25".parse().unwrap();
        assert_eq!(
            config,
            ToolSearchConfig::Auto {
                threshold_percent: 25
            }
        );
    }

    // ==========================================
    // Default Tests
    // ==========================================

    #[test]
    fn test_default() {
        let config = ToolSearchConfig::default();
        assert_eq!(
            config,
            ToolSearchConfig::Auto {
                threshold_percent: 10
            }
        );
    }

    // ==========================================
    // Error Display Tests
    // ==========================================

    #[test]
    fn test_error_display_invalid_format() {
        let err = ToolSearchConfigError::InvalidFormat("xyz".to_string());
        let msg = err.to_string();
        assert!(msg.contains("xyz"));
        assert!(msg.contains("auto"));
    }

    #[test]
    fn test_error_display_invalid_threshold() {
        let err = ToolSearchConfigError::InvalidThreshold("abc".to_string());
        let msg = err.to_string();
        assert!(msg.contains("abc"));
        assert!(msg.contains("0 and 100"));
    }

    #[test]
    fn test_error_display_out_of_range() {
        let err = ToolSearchConfigError::ThresholdOutOfRange(150);
        let msg = err.to_string();
        assert!(msg.contains("150"));
        assert!(msg.contains("out of range"));
    }
}
