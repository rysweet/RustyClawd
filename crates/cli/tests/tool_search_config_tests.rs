//! Integration tests for MCP Tool Search auto:N configuration
//!
//! These tests verify the complete flow from configuration parsing
//! through to settings integration.

use std::collections::HashMap;

/// Test that ToolSearchConfig can be parsed and used in settings
#[test]
fn test_tool_search_end_to_end_auto() {
    use rustyclawd::plugins::ToolSearchConfig;
    use rustyclawd::settings::Settings;

    // Parse auto:5 configuration
    let config = ToolSearchConfig::parse("auto:5").expect("Should parse auto:5");

    // Apply to settings
    let settings = Settings::new().with_tool_search(config);

    // Verify it's set correctly
    let retrieved = settings.get_tool_search();
    assert!(retrieved.is_auto());
    assert_eq!(retrieved.threshold_percent(), Some(5));

    // Test the decision logic
    assert!(!retrieved.should_enable_tool_search(4)); // 4% < 5% threshold
    assert!(retrieved.should_enable_tool_search(5)); // 5% = 5% threshold
    assert!(retrieved.should_enable_tool_search(10)); // 10% > 5% threshold
}

/// Test that tool_search config works with environment variable parsing
#[test]
fn test_tool_search_env_variable_integration() {
    use rustyclawd::settings::SettingsLoader;

    let mut overrides = HashMap::new();
    overrides.insert("enable_tool_search".to_string(), "auto:15".to_string());

    let settings = SettingsLoader::parse_env_overrides(&overrides);

    let config = settings.get_tool_search();
    assert!(config.is_auto());
    assert_eq!(config.threshold_percent(), Some(15));
}

/// Test all configuration variants
#[test]
fn test_all_tool_search_variants() {
    use rustyclawd::plugins::ToolSearchConfig;

    // Test "auto" (default 10%)
    let auto = ToolSearchConfig::parse("auto").unwrap();
    assert!(auto.is_auto());
    assert_eq!(auto.threshold_percent(), Some(10));

    // Test "auto:20" (custom threshold)
    let auto20 = ToolSearchConfig::parse("auto:20").unwrap();
    assert!(auto20.is_auto());
    assert_eq!(auto20.threshold_percent(), Some(20));

    // Test "true" (always enabled)
    let enabled = ToolSearchConfig::parse("true").unwrap();
    assert!(enabled.is_always_enabled());
    assert!(enabled.should_enable_tool_search(0)); // Always true

    // Test "false" (disabled)
    let disabled = ToolSearchConfig::parse("false").unwrap();
    assert!(disabled.is_disabled());
    assert!(!disabled.should_enable_tool_search(100)); // Always false
}

/// Test error handling for invalid configurations
#[test]
fn test_invalid_tool_search_configs() {
    use rustyclawd::plugins::ToolSearchConfig;

    // Invalid format
    assert!(ToolSearchConfig::parse("invalid").is_err());

    // Threshold out of range
    assert!(ToolSearchConfig::parse("auto:101").is_err());

    // Invalid threshold (not a number)
    assert!(ToolSearchConfig::parse("auto:abc").is_err());
}

/// Test default configuration behavior
#[test]
fn test_default_tool_search_behavior() {
    use rustyclawd::plugins::ToolSearchConfig;
    use rustyclawd::settings::Settings;

    // Default settings should have no tool_search set
    let settings = Settings::new();
    assert!(settings.tool_search.is_none());

    // But get_tool_search should return default (auto:10)
    let default_config = settings.get_tool_search();
    assert!(default_config.is_auto());
    assert_eq!(default_config.threshold_percent(), Some(10));

    // Verify ToolSearchConfig::default() matches
    let explicit_default = ToolSearchConfig::default();
    assert_eq!(default_config, explicit_default);
}
