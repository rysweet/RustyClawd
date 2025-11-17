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
    Request(#[from] reqwest::Error),

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
