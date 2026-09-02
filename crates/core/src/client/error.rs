//! Client error types with security-aware error handling
//!
//! This module provides error types for the Anthropic API client with
//! special handling to prevent API key leakage in error messages.

use regex::Regex;
use std::sync::OnceLock;
use std::time::Duration;
use thiserror::Error;

/// Client-specific errors with sanitization to prevent API key leakage
#[derive(Error, Debug)]
pub enum ClientError {
    // HTTP Status Code Errors
    #[error("Unauthorized: {0}\nPlease check your API key. Get a key at: https://console.anthropic.com/settings/keys")]
    Unauthorized(String),

    #[error("Forbidden: {0}\nAccess denied. Check your API key permissions.")]
    Forbidden(String),

    #[error("Bad Request: {0}\nThe request was invalid. Please check your parameters.")]
    BadRequest(String),

    #[error("Not Found: {0}\nThe requested resource was not found.")]
    NotFound(String),

    #[error("Rate Limited: {message}\nToo many requests. Please retry after {retry_after:?}.")]
    RateLimited {
        message: String,
        retry_after: Option<Duration>,
    },

    #[error(
        "Server Error (HTTP {0}): {1}\nThe server encountered an error. Please try again later."
    )]
    ServerError(u16, String),

    #[error("Service Unavailable: {message}\nThe service is temporarily unavailable. Please retry after {retry_after:?}.")]
    ServiceUnavailable {
        message: String,
        retry_after: Option<Duration>,
    },

    // Network/Transport Errors
    #[error(
        "Network Error: {0}\nFailed to connect to the server. Check your internet connection."
    )]
    NetworkError(String),

    #[error("Timeout: {0}\nThe request timed out. Please try again.")]
    Timeout(String),

    #[error("DNS Resolution Failed: {0}\nUnable to resolve the server address. Check your DNS settings and internet connection.")]
    DnsError(String),

    #[error("Connection Failed: {0}\nFailed to establish connection to the server. Check your network and firewall settings.")]
    ConnectionError(String),

    // Existing errors (preserved for backward compatibility)
    #[error("Failed to read API key: {0}")]
    ApiKeyRead(String),

    #[error("Invalid API key format")]
    InvalidApiKey,

    #[error("Anthropic API key not found and ANTHROPIC_AUTH_TOKEN is unset. Configure one of:\n  1. Preferred token: export ANTHROPIC_AUTH_TOKEN='YOUR_SYNTHETIC_GATEWAY_TOKEN'\n  2. Compatible API key: export ANTHROPIC_API_KEY='YOUR_SYNTHETIC_ANTHROPIC_API_KEY'\n  3. .env file: set ANTHROPIC_API_KEY to a compatible API key\n  4. Legacy file: store a compatible API key in ~/.claude-msec-k with mode 600\n\nGet an Anthropic API key: https://console.anthropic.com/settings/keys\n\nOr use GitHub Copilot instead: rusty --provider copilot")]
    ApiKeyNotFound,

    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Invalid SSE format: {0}")]
    InvalidSSE(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    // Catch-all for unknown errors
    #[error("Unknown error: {0}")]
    Unknown(String),
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

pub(crate) fn sanitize_error_with_credential(error: &str, credential: Option<&str>) -> String {
    let Some(credential) = credential.filter(|value| !value.is_empty()) else {
        return sanitize_error(error);
    };

    let json = serde_json::to_string(credential).expect("serializing a string cannot fail");
    let json_escaped = json
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .expect("a JSON string is enclosed in quotes");
    let credential_redacted = error
        .replace(json_escaped, "[REDACTED_API_KEY]")
        .replace(credential, "[REDACTED_API_KEY]");
    sanitize_error(&credential_redacted)
}

impl ClientError {
    /// Create a sanitized error message safe for logging
    pub fn sanitized_message(&self) -> String {
        sanitize_error(&self.to_string())
    }

    /// Create a ClientError from a reqwest::Response with error details
    ///
    /// This function extracts the status code, error message, and Retry-After header
    /// to create a structured error with all relevant context.
    pub async fn from_response(response: reqwest::Response) -> Self {
        Self::from_response_with_credential(response, None).await
    }

    /// Create a response error after redacting the exact active credential.
    ///
    /// The caller supplies the request credential directly so sanitization never
    /// depends on mutable process environment state.
    pub async fn from_response_with_credential(
        response: reqwest::Response,
        credential: Option<&str>,
    ) -> Self {
        let status = response.status();
        let retry_after = parse_retry_after(response.headers().get("retry-after"));

        // Try to extract error message from response body
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());

        let sanitized_body = sanitize_error_with_credential(&error_body, credential);

        match status.as_u16() {
            400 => ClientError::BadRequest(sanitized_body),
            401 => ClientError::Unauthorized(sanitized_body),
            403 => ClientError::Forbidden(sanitized_body),
            404 => ClientError::NotFound(sanitized_body),
            429 => ClientError::RateLimited {
                message: sanitized_body,
                retry_after,
            },
            500..=599 => {
                if status.as_u16() == 503 {
                    ClientError::ServiceUnavailable {
                        message: sanitized_body,
                        retry_after,
                    }
                } else {
                    ClientError::ServerError(status.as_u16(), sanitized_body)
                }
            }
            _ => ClientError::Unknown(format!("HTTP {}: {}", status, sanitized_body)),
        }
    }

    /// Check if this error is retryable (rate limit, service unavailable, or auth errors).
    ///
    /// Auth errors (401 Unauthorized) are retryable because Azure AD tokens
    /// expire hourly — a retry with a fresh token often succeeds.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ClientError::RateLimited { .. }
                | ClientError::ServiceUnavailable { .. }
                | ClientError::ServerError(500..=599, _)
                | ClientError::Timeout(_)
                | ClientError::NetworkError(_)
                | ClientError::DnsError(_)
                | ClientError::ConnectionError(_)
                | ClientError::Unauthorized(_)
        )
    }

    /// Check if this error is an authentication error (401 Unauthorized).
    ///
    /// Callers can use this to invalidate cached credentials before retrying.
    pub fn is_auth_error(&self) -> bool {
        matches!(self, ClientError::Unauthorized(_))
    }

    /// Get the retry delay if this error has one
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            ClientError::RateLimited { retry_after, .. } => *retry_after,
            ClientError::ServiceUnavailable { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Redact an active credential from errors produced while parsing an SSE stream.
    pub(crate) fn sanitize_stream_parser_error(self, credential: &str) -> Self {
        match self {
            ClientError::Stream(message) => {
                ClientError::Stream(sanitize_error_with_credential(&message, Some(credential)))
            }
            ClientError::InvalidSSE(message) => {
                ClientError::InvalidSSE(sanitize_error_with_credential(&message, Some(credential)))
            }
            ClientError::JsonParse(error) => {
                let message = error.to_string();
                let sanitized = sanitize_error_with_credential(&message, Some(credential));
                if sanitized == message {
                    ClientError::JsonParse(error)
                } else {
                    ClientError::InvalidSSE(sanitized)
                }
            }
            other => other,
        }
    }
}

/// Manual From implementation for reqwest::Error to enable structured error parsing
impl From<reqwest::Error> for ClientError {
    fn from(error: reqwest::Error) -> Self {
        // Check if this is a timeout error
        if error.is_timeout() {
            return ClientError::Timeout(sanitize_error(&error.to_string()));
        }

        // Check if this is a connection error - distinguish DNS from general connection
        if error.is_connect() {
            let error_msg = sanitize_error(&error.to_string());

            // DNS errors typically contain keywords like "dns", "resolve", "lookup", "name resolution"
            let error_lower = error_msg.to_lowercase();
            if error_lower.contains("dns")
                || error_lower.contains("resolve")
                || error_lower.contains("lookup")
                || error_lower.contains("name resolution")
                || error_lower.contains("domain name")
            {
                return ClientError::DnsError(error_msg);
            }

            // Other connection errors (refused, timeout, etc.)
            if error_lower.contains("connection refused")
                || error_lower.contains("connection reset")
                || error_lower.contains("broken pipe")
                || error_lower.contains("connection closed")
            {
                return ClientError::ConnectionError(error_msg);
            }

            // Generic network error for other connection issues
            return ClientError::NetworkError(error_msg);
        }

        // Try to extract status code
        if let Some(status) = error.status() {
            let error_msg = sanitize_error(&error.to_string());

            return match status.as_u16() {
                400 => ClientError::BadRequest(error_msg),
                401 => ClientError::Unauthorized(error_msg),
                403 => ClientError::Forbidden(error_msg),
                404 => ClientError::NotFound(error_msg),
                429 => ClientError::RateLimited {
                    message: error_msg,
                    retry_after: None, // No retry header available in this path
                },
                500..=599 => {
                    if status.as_u16() == 503 {
                        ClientError::ServiceUnavailable {
                            message: error_msg,
                            retry_after: None,
                        }
                    } else {
                        ClientError::ServerError(status.as_u16(), error_msg)
                    }
                }
                _ => ClientError::Unknown(format!("HTTP {}: {}", status, error_msg)),
            };
        }

        // Generic network error
        ClientError::Unknown(sanitize_error(&error.to_string()))
    }
}

/// Parse Retry-After header from HTTP response
///
/// Supports two formats:
/// 1. Delay-seconds: "120" (number of seconds)
/// 2. HTTP-date: "Wed, 21 Oct 2025 07:28:00 GMT" (absolute time)
fn parse_retry_after(header: Option<&reqwest::header::HeaderValue>) -> Option<Duration> {
    let header_str = header?.to_str().ok()?;

    // Try to parse as seconds first (most common format)
    if let Ok(seconds) = header_str.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }

    // Try to parse as HTTP date
    // Note: For simplicity, we'll use a basic heuristic here
    // In production, you might want to use a proper HTTP date parser
    // For now, if it's not a number, we'll return a default retry delay
    if header_str.contains("GMT") || header_str.contains(",") {
        // Default to 60 seconds for HTTP date format (we'd need chrono to parse properly)
        return Some(Duration::from_secs(60));
    }

    None
}

pub type ClientResult<T> = Result<T, ClientError>;

#[cfg(test)]
mod tests {
    use super::*;

    // Security tests
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
    fn test_sanitize_exact_credential_and_json_escaped_representation() {
        let credential = r#"synthetic-"quoted"\credential"#;
        let json = serde_json::to_string(credential).unwrap();
        let json_escaped = json
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap();
        let error = format!("literal={credential}; json={json_escaped}");

        let sanitized = sanitize_error_with_credential(&error, Some(credential));

        assert_eq!(
            sanitized,
            "literal=[REDACTED_API_KEY]; json=[REDACTED_API_KEY]"
        );
        assert!(!sanitized.contains(credential));
        assert!(!sanitized.contains(json_escaped));
    }

    #[test]
    fn test_empty_exact_credential_does_not_replace_every_boundary() {
        let error = "gateway error";

        assert_eq!(
            sanitize_error_with_credential(error, Some("")),
            "gateway error"
        );
    }

    #[test]
    fn test_stream_parser_error_preserves_unchanged_json_parse_variant() {
        let parse_error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let original_message = parse_error.to_string();

        let sanitized =
            ClientError::JsonParse(parse_error).sanitize_stream_parser_error("unrelated-token");

        match sanitized {
            ClientError::JsonParse(error) => assert_eq!(error.to_string(), original_message),
            other => panic!("expected JsonParse, got {other:?}"),
        }
    }

    #[test]
    fn test_stream_parser_error_redacts_credential_exposed_by_json_parser() {
        #[derive(Debug, serde::Deserialize)]
        enum ExpectedValue {
            Allowed,
        }

        let credential = "synthetic-gateway-secret";
        let parse_error =
            serde_json::from_str::<ExpectedValue>(&format!(r#""{credential}""#)).unwrap_err();
        assert!(parse_error.to_string().contains(credential));

        let sanitized =
            ClientError::JsonParse(parse_error).sanitize_stream_parser_error(credential);

        match sanitized {
            ClientError::InvalidSSE(message) => {
                assert!(message.contains("[REDACTED_API_KEY]"));
                assert!(!message.contains(credential));
            }
            other => panic!("expected InvalidSSE, got {other:?}"),
        }
    }

    #[test]
    fn test_no_key_unchanged() {
        let error = "Connection timeout";
        let sanitized = sanitize_error(error);
        assert_eq!(sanitized, "Connection timeout");
    }

    // Retry-After header parsing tests
    #[test]
    fn test_parse_retry_after_seconds() {
        let header_value = reqwest::header::HeaderValue::from_str("120").unwrap();
        let duration = parse_retry_after(Some(&header_value));
        assert_eq!(duration, Some(Duration::from_secs(120)));
    }

    #[test]
    fn test_parse_retry_after_http_date() {
        let header_value =
            reqwest::header::HeaderValue::from_str("Wed, 21 Oct 2025 07:28:00 GMT").unwrap();
        let duration = parse_retry_after(Some(&header_value));
        // Should return default 60 seconds for HTTP date format
        assert_eq!(duration, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_parse_retry_after_none() {
        let duration = parse_retry_after(None);
        assert_eq!(duration, None);
    }

    #[test]
    fn test_parse_retry_after_invalid() {
        let header_value = reqwest::header::HeaderValue::from_str("invalid").unwrap();
        let duration = parse_retry_after(Some(&header_value));
        assert_eq!(duration, None);
    }

    // Error classification tests
    #[test]
    fn test_is_retryable() {
        // Retryable errors
        assert!(ClientError::RateLimited {
            message: "Too many requests".to_string(),
            retry_after: Some(Duration::from_secs(60)),
        }
        .is_retryable());

        assert!(ClientError::ServiceUnavailable {
            message: "Service down".to_string(),
            retry_after: Some(Duration::from_secs(30)),
        }
        .is_retryable());

        assert!(ClientError::ServerError(500, "Internal error".to_string()).is_retryable());
        assert!(ClientError::Timeout("Request timeout".to_string()).is_retryable());
        assert!(ClientError::NetworkError("Network failure".to_string()).is_retryable());
        assert!(ClientError::DnsError("DNS lookup failed".to_string()).is_retryable());
        assert!(ClientError::ConnectionError("Connection refused".to_string()).is_retryable());

        // Auth errors are retryable (expired Azure tokens)
        assert!(ClientError::Unauthorized("Token expired".to_string()).is_retryable());

        // Non-retryable errors
        assert!(!ClientError::BadRequest("Invalid request".to_string()).is_retryable());
        assert!(!ClientError::Forbidden("Access denied".to_string()).is_retryable());
        assert!(!ClientError::NotFound("Not found".to_string()).is_retryable());
    }

    #[test]
    fn test_is_auth_error() {
        assert!(ClientError::Unauthorized("Token expired".to_string()).is_auth_error());
        assert!(!ClientError::Forbidden("Access denied".to_string()).is_auth_error());
        assert!(!ClientError::BadRequest("Bad request".to_string()).is_auth_error());
        assert!(!ClientError::Timeout("Timeout".to_string()).is_auth_error());
    }

    #[test]
    fn test_retry_after() {
        let error = ClientError::RateLimited {
            message: "Too many requests".to_string(),
            retry_after: Some(Duration::from_secs(120)),
        };
        assert_eq!(error.retry_after(), Some(Duration::from_secs(120)));

        let error = ClientError::ServiceUnavailable {
            message: "Service down".to_string(),
            retry_after: Some(Duration::from_secs(30)),
        };
        assert_eq!(error.retry_after(), Some(Duration::from_secs(30)));

        let error = ClientError::BadRequest("Invalid".to_string());
        assert_eq!(error.retry_after(), None);
    }

    // Error message tests
    #[test]
    fn test_error_messages() {
        let error = ClientError::Unauthorized("Invalid API key".to_string());
        let msg = error.to_string();
        assert!(msg.contains("Unauthorized"));
        assert!(msg.contains("Invalid API key"));
        assert!(msg.contains("console.anthropic.com"));

        let error = ClientError::RateLimited {
            message: "Rate limit exceeded".to_string(),
            retry_after: Some(Duration::from_secs(60)),
        };
        let msg = error.to_string();
        assert!(msg.contains("Rate Limited"));
        assert!(msg.contains("Too many requests"));

        let error = ClientError::ServerError(500, "Internal error".to_string());
        let msg = error.to_string();
        assert!(msg.contains("Server Error"));
        assert!(msg.contains("500"));
    }

    #[test]
    fn test_sanitized_message() {
        let error = ClientError::Unauthorized("Failed with key sk-ant-test123".to_string());
        let sanitized = error.sanitized_message();
        assert!(!sanitized.contains("test123"));
        assert!(sanitized.contains("[REDACTED_API_KEY]"));
    }

    // Network error variant tests
    #[test]
    fn test_dns_error_message() {
        let error = ClientError::DnsError("Failed to resolve api.anthropic.com".to_string());
        let msg = error.to_string();
        assert!(msg.contains("DNS Resolution Failed"));
        assert!(msg.contains("DNS settings"));
    }

    #[test]
    fn test_connection_error_message() {
        let error = ClientError::ConnectionError("Connection refused by server".to_string());
        let msg = error.to_string();
        assert!(msg.contains("Connection Failed"));
        assert!(msg.contains("network and firewall settings"));
    }

    #[test]
    fn test_network_error_message() {
        let error = ClientError::NetworkError("Network is unreachable".to_string());
        let msg = error.to_string();
        assert!(msg.contains("Network Error"));
        assert!(msg.contains("Check your internet connection"));
    }
}
