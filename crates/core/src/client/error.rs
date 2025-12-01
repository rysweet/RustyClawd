//! Client error types with security-aware error handling
//!
//! This module provides error types for the Anthropic API client with
//! special handling to prevent API key leakage in error messages.

use regex::Regex;
use std::sync::OnceLock;
use thiserror::Error;

/// Client-specific errors with sanitization to prevent API key leakage
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Request(String),

    #[error("Failed to read API key: {0}")]
    ApiKeyRead(String),

    #[error("Invalid API key format")]
    InvalidApiKey,

    #[error("API key not found. Please set via one of:\n  1. Environment: export ANTHROPIC_API_KEY='sk-ant-...'\n  2. .env file: echo 'ANTHROPIC_API_KEY=\"sk-ant-...\"' > .env\n  3. Legacy file: echo 'sk-ant-...' > ~/.claude-msec-k && chmod 600 ~/.claude-msec-k\n\nGet your API key: https://console.anthropic.com/settings/keys")]
    ApiKeyNotFound,

    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Invalid SSE format: {0}")]
    InvalidSSE(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // Network error types for granular error handling (GAP-ERROR-4)
    #[error("Request timeout: {0}")]
    Timeout(String),

    #[error("DNS resolution failed: {0}")]
    DnsError(String),

    #[error("Connection failed: {0}")]
    ConnectionError(String),

    #[error("Network error: {0}")]
    NetworkError(String),
}

/// Pattern to detect API keys in error messages (sk-ant-...)
static API_KEY_PATTERN: OnceLock<Regex> = OnceLock::new();

/// Get or initialize the API key detection pattern
fn api_key_pattern() -> &'static Regex {
    API_KEY_PATTERN
        .get_or_init(|| Regex::new(r"sk-ant-[a-zA-Z0-9_-]+").expect("Invalid regex pattern"))
}

/// Sanitize error messages to remove any API keys
pub fn sanitize_error(error: &str) -> String {
    let pattern = api_key_pattern();
    pattern.replace_all(error, "[REDACTED_API_KEY]").to_string()
}

impl ClientError {
    /// Create a sanitized error message safe for logging
    pub fn sanitized_message(&self) -> String {
        sanitize_error(&self.to_string())
    }
}

/// Convert reqwest::Error into specific ClientError types for better error handling
impl From<reqwest::Error> for ClientError {
    fn from(err: reqwest::Error) -> Self {
        let error_msg = err.to_string();

        // Check for timeout errors
        if err.is_timeout() {
            return ClientError::Timeout(error_msg);
        }

        // Check for connection errors
        if err.is_connect() {
            // Check if it's a DNS error specifically
            if error_msg.contains("dns") || error_msg.contains("name resolution") {
                return ClientError::DnsError(error_msg);
            }
            return ClientError::ConnectionError(error_msg);
        }

        // Check for other request errors (network issues)
        if err.is_request() {
            return ClientError::NetworkError(error_msg);
        }

        // For other errors, use the generic Request variant
        ClientError::Request(error_msg)
    }
}

pub type ClientResult<T> = Result<T, ClientError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_error() {
        let error = "Failed to authenticate with key sk-ant-abc123xyz";
        let sanitized = sanitize_error(error);
        assert_eq!(
            sanitized,
            "Failed to authenticate with key [REDACTED_API_KEY]"
        );
    }

    #[test]
    fn test_no_key_unchanged() {
        let error = "Connection timeout";
        let sanitized = sanitize_error(error);
        assert_eq!(sanitized, "Connection timeout");
    }
}
