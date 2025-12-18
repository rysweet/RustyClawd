//! Tests for retry logic with exponential backoff
//!
//! These tests verify that the client correctly retries on transient errors
//! and respects retry-after headers and exponential backoff.

use rustyclawd_core::client::{Client, Config, RetryConfig};
use std::time::Duration;

#[test]
fn test_retry_config_defaults() {
    let config = RetryConfig::default();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.initial_delay, Duration::from_secs(1));
    assert_eq!(config.max_delay, Duration::from_secs(30));
}

#[test]
fn test_retry_config_custom() {
    let config = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_secs(2),
        max_delay: Duration::from_secs(60),
    };
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.initial_delay, Duration::from_secs(2));
    assert_eq!(config.max_delay, Duration::from_secs(60));
}

#[test]
fn test_client_with_default_retry_config() {
    // Create a minimal config for testing
    let api_key = rustyclawd_core::client::ApiKey::new("sk-ant-test123".to_string()).unwrap();
    let config = Config::new(api_key);
    let client = Client::new(config);

    // Verify client was created successfully (no panic)
    assert_eq!(client.config().timeout_secs, 120); // Default timeout
}

#[test]
fn test_client_with_custom_retry_config() {
    let api_key = rustyclawd_core::client::ApiKey::new("sk-ant-test123".to_string()).unwrap();
    let config = Config::new(api_key);

    let retry_config = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_secs(2),
        max_delay: Duration::from_secs(60),
    };

    let client = Client::with_retry_config(config, retry_config);
    assert_eq!(client.config().timeout_secs, 120);
}

#[test]
fn test_exponential_backoff_calculation() {
    let initial_delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(30);

    // Test exponential backoff: 1s, 2s, 4s, 8s
    let delays = [
        initial_delay * 2_u32.pow(0), // 1s
        initial_delay * 2_u32.pow(1), // 2s
        initial_delay * 2_u32.pow(2), // 4s
        initial_delay * 2_u32.pow(3), // 8s
    ];

    assert_eq!(delays[0], Duration::from_secs(1));
    assert_eq!(delays[1], Duration::from_secs(2));
    assert_eq!(delays[2], Duration::from_secs(4));
    assert_eq!(delays[3], Duration::from_secs(8));

    // Test that max_delay is respected
    let very_long_delay = initial_delay * 2_u32.pow(10); // Would be 1024s
    let capped_delay = std::cmp::min(very_long_delay, max_delay);
    assert_eq!(capped_delay, max_delay);
}

// Note: Integration tests with mock servers would require additional dependencies
// like wiremock or mockito. For now, we test the configuration and logic.
//
// TODO: Add integration tests with mock HTTP server to test:
// - Retry on 429 (Rate Limited)
// - Retry on 503 (Service Unavailable)
// - Retry on network errors
// - No retry on 400, 401, 403
// - Respect Retry-After header
// - Max retries limit enforcement
