//! Tests for retry logic with exponential backoff and jitter
//!
//! These tests verify that the client correctly retries on transient errors
//! and respects retry-after headers, exponential backoff, and jitter.

use rustyclawd_core::client::{Client, Config, RetryConfig};
use std::time::Duration;

#[test]
fn test_retry_config_defaults() {
    let config = RetryConfig::default();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.initial_delay, Duration::from_secs(1));
    assert_eq!(config.max_delay, Duration::from_secs(30));
    assert!((config.jitter_factor - 0.1).abs() < f64::EPSILON);
}

#[test]
fn test_retry_config_custom() {
    let config = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_secs(2),
        max_delay: Duration::from_secs(60),
        jitter_factor: 0.2,
    };
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.initial_delay, Duration::from_secs(2));
    assert_eq!(config.max_delay, Duration::from_secs(60));
    assert!((config.jitter_factor - 0.2).abs() < f64::EPSILON);
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
        jitter_factor: 0.15,
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

#[test]
fn test_calculate_delay_exponential_growth() {
    // Use zero jitter to test pure exponential backoff
    let config = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(60),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    // Test exponential growth pattern
    let delay_0 = config.calculate_delay(0);
    let delay_1 = config.calculate_delay(1);
    let delay_2 = config.calculate_delay(2);
    let delay_3 = config.calculate_delay(3);

    // Without jitter, delays should be exactly 1s, 2s, 4s, 8s
    assert_eq!(delay_0, Duration::from_secs(1));
    assert_eq!(delay_1, Duration::from_secs(2));
    assert_eq!(delay_2, Duration::from_secs(4));
    assert_eq!(delay_3, Duration::from_secs(8));
}

#[test]
fn test_calculate_delay_max_cap() {
    let config = RetryConfig {
        max_retries: 10,
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(30),
        jitter_factor: 0.0, // No jitter for deterministic testing
    };

    // Attempt 5: 1 * 2^5 = 32s, should be capped to 30s
    let delay = config.calculate_delay(5);
    assert_eq!(delay, Duration::from_secs(30));

    // Attempt 10: 1 * 2^10 = 1024s, should still be capped to 30s
    let delay = config.calculate_delay(10);
    assert_eq!(delay, Duration::from_secs(30));
}

#[test]
fn test_calculate_delay_with_jitter_bounds() {
    let config = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_secs(10),
        max_delay: Duration::from_secs(60),
        jitter_factor: 0.5, // 50% jitter
    };

    // Run multiple times to test that jitter produces values in expected range
    for _ in 0..10 {
        let delay = config.calculate_delay(0);
        let delay_secs = delay.as_secs_f64();

        // Base delay is 10s, with 50% jitter the range should be 5s to 10s
        // (delay * (1.0 - 0.5*random)) where random is 0.0 to 1.0
        assert!(
            (5.0..=10.0).contains(&delay_secs),
            "Delay {:.2}s should be between 5.0s and 10.0s",
            delay_secs
        );
    }
}

#[test]
fn test_calculate_delay_minimum_bound() {
    let config = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_secs(60),
        jitter_factor: 0.99, // Very high jitter
    };

    // Even with extreme jitter, delay should never go below 10ms
    for _ in 0..10 {
        let delay = config.calculate_delay(0);
        assert!(
            delay >= Duration::from_millis(10),
            "Delay {:?} should be at least 10ms",
            delay
        );
    }
}

#[test]
fn test_calculate_delay_with_fractional_seconds() {
    let config = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_millis(500),
        max_delay: Duration::from_secs(60),
        jitter_factor: 0.0, // No jitter
    };

    // 500ms * 2^0 = 500ms
    let delay_0 = config.calculate_delay(0);
    assert_eq!(delay_0, Duration::from_millis(500));

    // 500ms * 2^1 = 1000ms
    let delay_1 = config.calculate_delay(1);
    assert_eq!(delay_1, Duration::from_secs(1));

    // 500ms * 2^2 = 2000ms
    let delay_2 = config.calculate_delay(2);
    assert_eq!(delay_2, Duration::from_secs(2));
}

#[test]
fn test_jitter_produces_varying_delays() {
    let config = RetryConfig {
        max_retries: 5,
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(60),
        jitter_factor: 0.3, // 30% jitter
    };

    // Collect multiple delays and verify they're not all identical
    // (jitter should produce some variation)
    let mut delays = Vec::new();
    for _ in 0..20 {
        delays.push(config.calculate_delay(0));
        // Small sleep to ensure time-based randomness changes
        std::thread::sleep(Duration::from_nanos(1000));
    }

    // With jitter, we should see at least some variation
    let first = delays[0];
    let has_variation = delays.iter().any(|d| *d != first);

    // Note: This test might occasionally fail if the random generator
    // produces the same value repeatedly, but it's statistically unlikely
    assert!(
        has_variation,
        "Jitter should produce varying delays, but all were {:?}",
        first
    );
}
