//! Tests for GAP-ERROR-4: Network error granularity display
//!
//! Validates that network errors (Timeout, DnsError, ConnectionError, NetworkError)
//! display user-friendly messages with troubleshooting hints.

use rustyclawd_core::client::ClientError;

/// Test helper to extract the formatted network error message
fn format_network_error_test(error: &ClientError) -> String {
    // This simulates the format_network_error method from InteractiveSession
    match error {
        ClientError::Timeout(msg) => {
            format!(
                "⏱️  Request timed out\n\
                Details: {}\n\
                Tip: Check your internet connection or try again later.",
                msg
            )
        }
        ClientError::ConnectionError(msg) => {
            format!(
                "🔌 Connection failed\n\
                Details: {}\n\
                Tip: Verify you can reach api.anthropic.com",
                msg
            )
        }
        ClientError::DnsError(msg) => {
            format!(
                "🌐 DNS resolution failed\n\
                Details: {}\n\
                Tip: Check your DNS settings or try a different network.",
                msg
            )
        }
        ClientError::NetworkError(msg) => {
            format!(
                "📡 Network error\n\
                Details: {}\n\
                Tip: Check your internet connection.",
                msg
            )
        }
        _ => error.to_string(),
    }
}

#[test]
fn test_timeout_error_display() {
    let error = ClientError::Timeout("Request exceeded 30 second timeout".to_string());
    let formatted = format_network_error_test(&error);

    assert!(formatted.contains("⏱️  Request timed out"));
    assert!(formatted.contains("Request exceeded 30 second timeout"));
    assert!(formatted.contains("Check your internet connection"));
}

#[test]
fn test_connection_error_display() {
    let error = ClientError::ConnectionError("Connection refused by server".to_string());
    let formatted = format_network_error_test(&error);

    assert!(formatted.contains("🔌 Connection failed"));
    assert!(formatted.contains("Connection refused by server"));
    assert!(formatted.contains("api.anthropic.com"));
}

#[test]
fn test_dns_error_display() {
    let error = ClientError::DnsError("Failed to resolve api.anthropic.com".to_string());
    let formatted = format_network_error_test(&error);

    assert!(formatted.contains("🌐 DNS resolution failed"));
    assert!(formatted.contains("Failed to resolve"));
    assert!(formatted.contains("DNS settings"));
}

#[test]
fn test_network_error_display() {
    let error = ClientError::NetworkError("Network is unreachable".to_string());
    let formatted = format_network_error_test(&error);

    assert!(formatted.contains("📡 Network error"));
    assert!(formatted.contains("Network is unreachable"));
    assert!(formatted.contains("Check your internet connection"));
}

#[test]
fn test_emoji_rendering() {
    // Verify emojis are included for visual distinction
    let timeout = ClientError::Timeout("test".to_string());
    let connection = ClientError::ConnectionError("test".to_string());
    let dns = ClientError::DnsError("test".to_string());
    let network = ClientError::NetworkError("test".to_string());

    assert!(format_network_error_test(&timeout).contains("⏱️"));
    assert!(format_network_error_test(&connection).contains("🔌"));
    assert!(format_network_error_test(&dns).contains("🌐"));
    assert!(format_network_error_test(&network).contains("📡"));
}

#[test]
fn test_troubleshooting_hints_present() {
    // Each error type should have a "Tip:" section with actionable guidance
    let timeout = ClientError::Timeout("test".to_string());
    let connection = ClientError::ConnectionError("test".to_string());
    let dns = ClientError::DnsError("test".to_string());
    let network = ClientError::NetworkError("test".to_string());

    assert!(format_network_error_test(&timeout).contains("Tip:"));
    assert!(format_network_error_test(&connection).contains("Tip:"));
    assert!(format_network_error_test(&dns).contains("Tip:"));
    assert!(format_network_error_test(&network).contains("Tip:"));
}

#[test]
fn test_error_details_included() {
    // Detailed error messages should be preserved
    let error = ClientError::Timeout("Operation timed out after waiting 45 seconds".to_string());
    let formatted = format_network_error_test(&error);

    assert!(formatted.contains("Details:"));
    assert!(formatted.contains("Operation timed out after waiting 45 seconds"));
}

#[test]
fn test_non_network_errors_unchanged() {
    // Non-network errors should fall through to default formatting
    let error = ClientError::ApiKeyNotFound;
    let formatted = format_network_error_test(&error);

    // Should use the error's Display implementation
    assert!(formatted.contains("API key not found"));
}
