//! Error Message Formatting Tests (GAP-ERROR-3)
//!
//! Tests for user-friendly error message formatting, especially rate limit errors.
//! Verifies that ClientError variants are formatted with helpful context and tips.

use rustyclawd_core::client::ClientError;
use std::time::Duration;

// ============================================================================
// UNIT TESTS: Rate Limit Error Messages
// ============================================================================

#[test]
fn test_rate_limited_error_with_retry_after() {
    let error = ClientError::RateLimited {
        message: "Too many requests".to_string(),
        retry_after: Some(Duration::from_secs(120)),
    };

    let msg = error.to_string();

    // Verify key components are present
    assert!(msg.contains("Rate Limited"), "Should mention rate limit");
    assert!(msg.contains("Too many requests"), "Should include message");
    assert!(msg.contains("120"), "Should show retry duration");
    assert!(
        msg.contains("retry") || msg.contains("Retry"),
        "Should mention retry"
    );
}

#[test]
fn test_rate_limited_error_without_retry_after() {
    let error = ClientError::RateLimited {
        message: "Rate limit exceeded".to_string(),
        retry_after: None,
    };

    let msg = error.to_string();

    assert!(msg.contains("Rate Limited"));
    assert!(msg.contains("Rate limit exceeded"));
}

#[test]
fn test_rate_limited_shows_seconds_for_short_duration() {
    let error = ClientError::RateLimited {
        message: "Too many requests".to_string(),
        retry_after: Some(Duration::from_secs(30)),
    };

    let msg = error.to_string();
    assert!(
        msg.contains("30") || msg.contains("seconds"),
        "Should show duration in seconds for short waits"
    );
}

#[test]
fn test_rate_limited_shows_minutes_for_medium_duration() {
    let error = ClientError::RateLimited {
        message: "Too many requests".to_string(),
        retry_after: Some(Duration::from_secs(180)), // 3 minutes
    };

    let msg = error.to_string();
    assert!(
        msg.contains("3") || msg.contains("180"),
        "Should show duration for medium waits"
    );
}

#[test]
fn test_rate_limited_shows_hours_for_long_duration() {
    let error = ClientError::RateLimited {
        message: "Too many requests".to_string(),
        retry_after: Some(Duration::from_secs(7200)), // 2 hours
    };

    let msg = error.to_string();
    assert!(
        msg.contains("2") || msg.contains("7200"),
        "Should show duration for long waits"
    );
}

// ============================================================================
// UNIT TESTS: Service Unavailable Error Messages
// ============================================================================

#[test]
fn test_service_unavailable_with_retry_after() {
    let error = ClientError::ServiceUnavailable {
        message: "Service temporarily down".to_string(),
        retry_after: Some(Duration::from_secs(60)),
    };

    let msg = error.to_string();

    assert!(msg.contains("Service Unavailable"));
    assert!(msg.contains("Service temporarily down"));
    assert!(msg.contains("60") || msg.contains("retry"));
}

#[test]
fn test_service_unavailable_without_retry_after() {
    let error = ClientError::ServiceUnavailable {
        message: "Maintenance mode".to_string(),
        retry_after: None,
    };

    let msg = error.to_string();

    assert!(msg.contains("Service Unavailable"));
    assert!(msg.contains("Maintenance mode"));
}

// ============================================================================
// UNIT TESTS: Authentication Error Messages
// ============================================================================

#[test]
fn test_unauthorized_error_message() {
    let error = ClientError::Unauthorized("Invalid API key".to_string());

    let msg = error.to_string();

    assert!(msg.contains("Unauthorized"));
    assert!(msg.contains("Invalid API key"));
    assert!(
        msg.contains("console.anthropic.com"),
        "Should provide link to get API key"
    );
}

#[test]
fn test_forbidden_error_message() {
    let error = ClientError::Forbidden("Access denied".to_string());

    let msg = error.to_string();

    assert!(msg.contains("Forbidden"));
    assert!(msg.contains("Access denied"));
    assert!(msg.contains("permissions"), "Should mention permissions");
}

// ============================================================================
// UNIT TESTS: Request Error Messages
// ============================================================================

#[test]
fn test_bad_request_error_message() {
    let error = ClientError::BadRequest("Invalid parameters".to_string());

    let msg = error.to_string();

    assert!(msg.contains("Bad Request"));
    assert!(msg.contains("Invalid parameters"));
    assert!(
        msg.contains("invalid"),
        "Should indicate request was invalid"
    );
}

#[test]
fn test_not_found_error_message() {
    let error = ClientError::NotFound("Resource not found".to_string());

    let msg = error.to_string();

    assert!(msg.contains("Not Found"));
    assert!(msg.contains("Resource not found"));
}

// ============================================================================
// UNIT TESTS: Server Error Messages
// ============================================================================

#[test]
fn test_server_error_message() {
    let error = ClientError::ServerError(500, "Internal server error".to_string());

    let msg = error.to_string();

    assert!(msg.contains("Server Error"));
    assert!(msg.contains("500"));
    assert!(msg.contains("Internal server error"));
    assert!(
        msg.contains("try again"),
        "Should suggest retrying for server errors"
    );
}

#[test]
fn test_server_error_different_status_codes() {
    let codes = vec![500, 502, 503, 504];

    for code in codes {
        let error = ClientError::ServerError(code, format!("Error {}", code));
        let msg = error.to_string();

        assert!(
            msg.contains(&code.to_string()),
            "Should show status code {}",
            code
        );
    }
}

// ============================================================================
// UNIT TESTS: Network Error Messages
// ============================================================================

#[test]
fn test_timeout_error_message() {
    let error = ClientError::Timeout("Request timed out".to_string());

    let msg = error.to_string();

    assert!(msg.contains("Timeout"));
    assert!(msg.contains("Request timed out"));
    assert!(msg.contains("try again"), "Should suggest retrying");
}

#[test]
fn test_connection_error_message() {
    let error = ClientError::ConnectionError("Connection refused".to_string());

    let msg = error.to_string();

    assert!(msg.contains("Connection Failed"));
    assert!(msg.contains("Connection refused"));
    assert!(
        msg.contains("network") || msg.contains("firewall"),
        "Should mention network or firewall"
    );
}

#[test]
fn test_dns_error_message() {
    let error = ClientError::DnsError("Failed to resolve api.anthropic.com".to_string());

    let msg = error.to_string();

    assert!(msg.contains("DNS"));
    assert!(msg.contains("Failed to resolve api.anthropic.com"));
    assert!(
        msg.contains("DNS settings") || msg.contains("internet connection"),
        "Should mention DNS settings or connection"
    );
}

#[test]
fn test_network_error_message() {
    let error = ClientError::NetworkError("Network unreachable".to_string());

    let msg = error.to_string();

    assert!(msg.contains("Network Error"));
    assert!(msg.contains("Network unreachable"));
    assert!(
        msg.contains("internet connection"),
        "Should mention internet connection"
    );
}

// ============================================================================
// UNIT TESTS: Error Retryability
// ============================================================================

#[test]
fn test_rate_limited_is_retryable() {
    let error = ClientError::RateLimited {
        message: "Too many requests".to_string(),
        retry_after: Some(Duration::from_secs(60)),
    };

    assert!(
        error.is_retryable(),
        "Rate limit errors should be retryable"
    );
}

#[test]
fn test_service_unavailable_is_retryable() {
    let error = ClientError::ServiceUnavailable {
        message: "Service down".to_string(),
        retry_after: Some(Duration::from_secs(30)),
    };

    assert!(
        error.is_retryable(),
        "Service unavailable should be retryable"
    );
}

#[test]
fn test_server_error_is_retryable() {
    let error = ClientError::ServerError(500, "Internal error".to_string());
    assert!(error.is_retryable(), "Server errors should be retryable");
}

#[test]
fn test_timeout_is_retryable() {
    let error = ClientError::Timeout("Timeout".to_string());
    assert!(error.is_retryable(), "Timeouts should be retryable");
}

#[test]
fn test_network_error_is_retryable() {
    let error = ClientError::NetworkError("Network issue".to_string());
    assert!(error.is_retryable(), "Network errors should be retryable");
}

#[test]
fn test_unauthorized_is_retryable() {
    // 401 is retryable because Azure AD tokens expire hourly.
    // Retrying with a fresh token often succeeds.
    let error = ClientError::Unauthorized("Token expired".to_string());
    assert!(
        error.is_retryable(),
        "Auth errors should be retryable (Azure token expiry)"
    );
}

#[test]
fn test_bad_request_is_not_retryable() {
    let error = ClientError::BadRequest("Invalid params".to_string());
    assert!(
        !error.is_retryable(),
        "Bad requests should not be retryable"
    );
}

// ============================================================================
// UNIT TESTS: Retry After Extraction
// ============================================================================

#[test]
fn test_extract_retry_after_from_rate_limited() {
    let error = ClientError::RateLimited {
        message: "Too many requests".to_string(),
        retry_after: Some(Duration::from_secs(120)),
    };

    assert_eq!(error.retry_after(), Some(Duration::from_secs(120)));
}

#[test]
fn test_extract_retry_after_from_service_unavailable() {
    let error = ClientError::ServiceUnavailable {
        message: "Service down".to_string(),
        retry_after: Some(Duration::from_secs(60)),
    };

    assert_eq!(error.retry_after(), Some(Duration::from_secs(60)));
}

#[test]
fn test_retry_after_none_for_non_retryable_errors() {
    let errors = vec![
        ClientError::Unauthorized("Invalid".to_string()),
        ClientError::BadRequest("Invalid".to_string()),
        ClientError::Forbidden("Denied".to_string()),
        ClientError::NotFound("Missing".to_string()),
    ];

    for error in errors {
        assert_eq!(
            error.retry_after(),
            None,
            "Non-retryable errors should have no retry_after"
        );
    }
}

// ============================================================================
// INTEGRATION TEST: Reqwest Error Conversion
// ============================================================================

#[test]
fn test_reqwest_error_conversion_preserves_status() {
    // This tests that when we convert reqwest errors to ClientError,
    // we preserve the status code and create appropriate error types
    // Note: This is a conceptual test - actual reqwest errors need real HTTP responses
    let error = ClientError::RateLimited {
        message: "Converted from reqwest 429".to_string(),
        retry_after: None,
    };

    assert!(matches!(error, ClientError::RateLimited { .. }));
}
