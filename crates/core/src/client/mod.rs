//! Anthropic API client with streaming support
//!
//! This module provides a complete, production-ready client for the Anthropic API
//! with the following features:
//!
//! - Secure API key handling with zeroization
//! - HTTP/2 streaming via Server-Sent Events (SSE)
//! - Real-time message streaming
//! - Comprehensive error handling with structured error types
//! - Automatic retry logic with exponential backoff
//! - Request timeout management
//!
//! # Example
//!
//! ```no_run
//! use rustyclawd_core::client::{Client, Config, Message, CreateMessageRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load config from ~/.claude-msec-k
//!     let config = Config::from_default_location().await?;
//!     let client = Client::new(config)?;
//!
//!     // Create a simple request
//!     let request = CreateMessageRequest::new(
//!         "claude-3-5-sonnet-20241022",
//!         vec![Message::user("Hello, Claude!")],
//!         1024,
//!     );
//!
//!     // Non-streaming request
//!     let response = client.create_message(request).await?;
//!     println!("{:?}", response);
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod copilot;
pub mod error;
pub mod request;
pub mod response;
pub mod retry;
pub mod stream;
mod tool_loop;
pub mod types;

pub use tool_loop::ToolLoopEvent;

use futures::stream::BoxStream;
use futures::Stream;
use futures::StreamExt;
use reqwest::Client as HttpClient;
use secrecy::ExposeSecret;
use std::future::Future;
use std::time::Duration;

pub use config::{ApiKey, Backend, Config};
pub use copilot::{CopilotAuth, CopilotModel};
pub use error::{ClientError, ClientResult};
pub use request::{CreateMessageRequest, Metadata, Speed, ThinkingConfig};
pub use response::{
    ApiError, ContentBlockStart, ContentDelta, MessageDelta, MessageResponse, MessageStart,
    StreamEvent, Usage,
};
pub use retry::RetryConfig;
pub use stream::{EventStream, SseEvent, SseStream};
pub use types::{
    ContentBlock, ExtraToolSchema, Message, MessageContent, Role, ToolChoice, ToolDefinition,
};

/// API client supporting Anthropic and GitHub Copilot backends.
#[derive(Clone)]
pub struct Client {
    config: Config,
    http_client: HttpClient,
    retry_config: RetryConfig,
    /// Copilot authentication state (only present when backend is Copilot)
    copilot_auth: Option<CopilotAuth>,
}

impl Client {
    /// Create a new client with the given configuration and default retry settings
    pub fn new(config: Config) -> ClientResult<Self> {
        let timeout = Duration::from_secs(config.timeout_secs);

        let http_client = HttpClient::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ClientError::Unknown(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            copilot_auth: None,
            config,
            http_client,
            retry_config: RetryConfig::default(),
        })
    }

    /// Create a new client with custom retry configuration
    pub fn with_retry_config(config: Config, retry_config: RetryConfig) -> ClientResult<Self> {
        let timeout = Duration::from_secs(config.timeout_secs);

        let http_client = HttpClient::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ClientError::Unknown(format!("Failed to build HTTP client: {}", e)))?;

        Ok(Self {
            copilot_auth: None,
            config,
            http_client,
            retry_config,
        })
    }

    /// Create a client configured for the GitHub Copilot backend.
    pub async fn new_copilot() -> ClientResult<Self> {
        let github_token = copilot::get_github_token().await?;
        let config = Config::new_copilot();
        let timeout = Duration::from_secs(config.timeout_secs);

        let http_client = HttpClient::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ClientError::Unknown(format!("Failed to build HTTP client: {}", e)))?;

        let auth = CopilotAuth::connect(github_token, http_client.clone()).await?;

        Ok(Self {
            copilot_auth: Some(auth),
            config,
            http_client,
            retry_config: RetryConfig::default(),
        })
    }

    /// Attach Copilot authentication to an existing client.
    pub fn with_copilot_auth(mut self, auth: CopilotAuth) -> Self {
        self.copilot_auth = Some(auth);
        self
    }

    /// Get a reference to the Copilot auth, if configured.
    pub fn copilot_auth(&self) -> Option<&CopilotAuth> {
        self.copilot_auth.as_ref()
    }

    /// Get the active backend.
    pub fn backend(&self) -> config::Backend {
        self.config.backend
    }

    /// Generic retry helper with exponential backoff
    ///
    /// Executes the given operation, retrying on retryable errors up to
    /// `max_retries` times. Respects `Retry-After` headers when present,
    /// otherwise uses exponential backoff with jitter.
    async fn with_retry<T, F, Fut>(&self, label: &str, operation: F) -> ClientResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = ClientResult<T>>,
    {
        let mut retries = 0;

        loop {
            match operation().await {
                Ok(value) => return Ok(value),
                Err(e) if e.is_retryable() && retries < self.retry_config.max_retries => {
                    // Calculate delay with exponential backoff and jitter
                    let calculated_delay = self.retry_config.calculate_delay(retries);

                    // Use Retry-After if provided, otherwise use calculated delay
                    let actual_delay = e.retry_after().unwrap_or(calculated_delay);

                    tracing::warn!(
                        delay_secs = actual_delay.as_secs_f64(),
                        attempt = retries + 1,
                        max_retries = self.retry_config.max_retries,
                        "Retrying {label}"
                    );

                    tokio::time::sleep(actual_delay).await;
                    retries += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Create a message (non-streaming) with automatic retry logic.
    ///
    /// Dispatches to the appropriate backend (Anthropic or Copilot).
    pub async fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> ClientResult<MessageResponse> {
        match self.config.backend {
            config::Backend::Copilot => {
                let auth = self.copilot_auth.as_ref().ok_or_else(|| {
                    ClientError::Unknown(
                        "Copilot backend requires authentication. Use Client::new_copilot()."
                            .to_string(),
                    )
                })?;
                self.with_retry("copilot request", || {
                    copilot::create_message(&self.http_client, auth, &request)
                })
                .await
            }
            config::Backend::Anthropic => {
                self.with_retry("request", || self.create_message_internal(&request))
                    .await
            }
        }
    }

    /// Build common headers for API requests, including conditional beta headers.
    fn build_request_headers(
        &self,
        request: &CreateMessageRequest,
    ) -> ClientResult<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            self.config
                .api_key
                .expose_secret()
                .expose()
                .parse()
                .map_err(|_: reqwest::header::InvalidHeaderValue| ClientError::InvalidApiKey)?,
        );
        headers.insert(
            "anthropic-version",
            self.config
                .api_version
                .parse()
                .map_err(|e: reqwest::header::InvalidHeaderValue| {
                    ClientError::Unknown(format!("Invalid API version header: {}", e))
                })?,
        );
        headers.insert("content-type", "application/json".parse().unwrap());

        // Add fast mode beta header when speed is set to "fast"
        if request.requires_fast_mode_beta() {
            headers.insert("anthropic-beta", "fast-mode-2026-02-01".parse().unwrap());
        }

        Ok(headers)
    }

    /// Internal method to execute a single message request without retry
    async fn create_message_internal(
        &self,
        request: &CreateMessageRequest,
    ) -> ClientResult<MessageResponse> {
        // Validate request before sending
        request.validate().map_err(ClientError::Unknown)?;

        let url = format!("{}/v1/messages", self.config.api_url);
        let headers = self.build_request_headers(request)?;

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(request)
            .send()
            .await?;

        // Check for HTTP errors and create structured error
        if !response.status().is_success() {
            return Err(ClientError::from_response(response).await);
        }

        let message_response: MessageResponse = response.json().await?;
        Ok(message_response)
    }

    /// Create a message with streaming and automatic retry logic.
    ///
    /// Dispatches to the appropriate backend (Anthropic or Copilot).
    /// Returns a boxed stream to unify the different backend stream types.
    pub async fn create_message_stream(
        &self,
        mut request: CreateMessageRequest,
    ) -> ClientResult<BoxStream<'static, ClientResult<StreamEvent>>> {
        // Ensure streaming is enabled
        request.stream = true;

        match self.config.backend {
            config::Backend::Copilot => {
                let auth = self.copilot_auth.as_ref().ok_or_else(|| {
                    ClientError::Unknown(
                        "Copilot backend requires authentication. Use Client::new_copilot()."
                            .to_string(),
                    )
                })?;
                let stream =
                    copilot::create_message_stream(&self.http_client, auth, &request).await?;
                Ok(stream.boxed())
            }
            config::Backend::Anthropic => {
                let stream = self
                    .with_retry("streaming request", || {
                        self.create_message_stream_internal(&request)
                    })
                    .await?;
                Ok(stream.boxed())
            }
        }
    }

    /// Internal method to create a streaming message request without retry
    async fn create_message_stream_internal(
        &self,
        request: &CreateMessageRequest,
    ) -> ClientResult<impl Stream<Item = ClientResult<StreamEvent>>> {
        // Validate request before sending
        request.validate().map_err(ClientError::Unknown)?;

        let url = format!("{}/v1/messages", self.config.api_url);
        let mut headers = self.build_request_headers(request)?;
        headers.insert("accept", "text/event-stream".parse().unwrap());

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(request)
            .send()
            .await?;

        // Check for HTTP errors and create structured error
        if !response.status().is_success() {
            return Err(ClientError::from_response(response).await);
        }

        // Convert response body into a byte stream
        let byte_stream = response.bytes_stream();

        // Wrap in our SSE parser
        let event_stream = EventStream::new(byte_stream);

        Ok(event_stream)
    }

    /// Helper to extract just the text from a streaming response
    pub async fn stream_text(
        &self,
        request: CreateMessageRequest,
    ) -> ClientResult<impl Stream<Item = ClientResult<String>>> {
        let event_stream = self.create_message_stream(request).await?;

        Ok(futures::stream::unfold(
            event_stream,
            |mut stream| async move {
                use futures::StreamExt;

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(StreamEvent::ContentBlockDelta {
                            delta: ContentDelta::TextDelta { text },
                            ..
                        }) => {
                            return Some((Ok(text), stream));
                        }
                        Ok(StreamEvent::Error { error }) => {
                            return Some((Err(ClientError::Unknown(error.message)), stream));
                        }
                        Err(e) => {
                            return Some((Err(e), stream));
                        }
                        _ => {
                            // Continue to next event for other types
                        }
                    }
                }

                None
            },
        ))
    }

    /// Get the API URL for custom request handling
    pub fn api_url(&self) -> &str {
        &self.config.api_url
    }

    /// Get the API version for custom request handling
    pub fn api_version(&self) -> &str {
        &self.config.api_version
    }

    /// Get the HTTP client for custom request handling
    pub fn http_client(&self) -> &HttpClient {
        &self.http_client
    }

    /// Get a reference to the config
    pub fn config(&self) -> &Config {
        &self.config
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_error_text() {
        let error = "Failed with key sk-ant-test123";
        let sanitized = error::sanitize_error(error);
        assert!(!sanitized.contains("test123"));
        assert!(sanitized.contains("[REDACTED_API_KEY]"));
    }

    #[test]
    fn test_client_no_leak_in_debug() {
        let key = ApiKey::new("sk-ant-secret123".to_string()).unwrap();
        let config = Config::new(key);
        let client = Client::new(config).unwrap();
        let debug_str = format!("{:?}", client);
        assert!(!debug_str.contains("secret123"));
    }

    /// Test that build_request_headers includes the fast-mode beta header
    /// when speed is set to Fast.
    #[test]
    fn test_build_request_headers_includes_fast_mode_beta() {
        let key = ApiKey::new("sk-ant-test123".to_string()).unwrap();
        let config = Config::new(key);
        let client = Client::new(config).unwrap();

        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Test")], 1024)
                .with_speed(true);

        let headers = client.build_request_headers(&request).unwrap();

        assert_eq!(
            headers.get("anthropic-beta").map(|v| v.to_str().unwrap()),
            Some("fast-mode-2026-02-01"),
            "Beta header must be present when speed is Fast"
        );
        // Standard headers should still be present
        assert!(headers.get("x-api-key").is_some());
        assert!(headers.get("anthropic-version").is_some());
        assert_eq!(
            headers.get("content-type").map(|v| v.to_str().unwrap()),
            Some("application/json")
        );
    }

    /// Test that build_request_headers omits the fast-mode beta header
    /// when speed is not set.
    #[test]
    fn test_build_request_headers_omits_beta_without_fast_mode() {
        let key = ApiKey::new("sk-ant-test123".to_string()).unwrap();
        let config = Config::new(key);
        let client = Client::new(config).unwrap();

        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Test")], 1024);

        let headers = client.build_request_headers(&request).unwrap();

        assert!(
            headers.get("anthropic-beta").is_none(),
            "Beta header must NOT be present when speed is None"
        );
    }

    /// Test requires_fast_mode_beta returns true only for Speed::Fast.
    #[test]
    fn test_requires_fast_mode_beta_true_when_fast() {
        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Test")], 1024)
                .with_speed(true);

        assert!(request.requires_fast_mode_beta());
    }

    /// Test requires_fast_mode_beta returns false when speed is None.
    #[test]
    fn test_requires_fast_mode_beta_false_when_none() {
        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Test")], 1024);

        assert!(!request.requires_fast_mode_beta());
    }

    /// Test requires_fast_mode_beta returns false after disabling speed.
    #[test]
    fn test_requires_fast_mode_beta_false_after_disable() {
        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Test")], 1024)
                .with_speed(true)
                .with_speed(false);

        assert!(!request.requires_fast_mode_beta());
    }

    /// Integration test: construct a full request with speed=Fast,
    /// verify the serialized JSON contains "speed":"fast" AND
    /// the headers contain the beta header.
    #[test]
    fn test_fast_mode_full_request_json_and_headers() {
        // 1. Build the request
        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Hello")], 2048)
                .with_speed(true)
                .with_stream(true);

        // 2. Validate the request passes validation
        assert!(request.validate().is_ok());

        // 3. Verify serialized JSON contains "speed":"fast"
        let json = serde_json::to_string(&request).unwrap();
        assert!(
            json.contains(r#""speed":"fast""#),
            "Serialized JSON must contain '\"speed\":\"fast\"', got: {}",
            json
        );
        // Ensure it does NOT contain the old field name
        assert!(
            !json.contains("fast_mode"),
            "JSON must not contain legacy 'fast_mode' field"
        );

        // 4. Verify headers contain the beta header
        let key = ApiKey::new("sk-ant-test123".to_string()).unwrap();
        let config = Config::new(key);
        let client = Client::new(config).unwrap();
        let headers = client.build_request_headers(&request).unwrap();

        assert_eq!(
            headers.get("anthropic-beta").map(|v| v.to_str().unwrap()),
            Some("fast-mode-2026-02-01"),
            "Headers must contain fast-mode beta header"
        );
    }

    /// Test that a malformed API key (containing invalid header characters)
    /// returns an error instead of panicking.
    #[test]
    fn test_build_request_headers_returns_error_on_malformed_api_key() {
        // A key with a newline passes the sk-ant- prefix check but is invalid
        // as an HTTP header value.
        let key = ApiKey::new("sk-ant-bad\nkey".to_string()).unwrap();
        let config = Config::new(key);
        let client = Client::new(config).unwrap();

        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Test")], 1024);

        let result = client.build_request_headers(&request);
        assert!(
            result.is_err(),
            "build_request_headers must return Err for malformed API key, not panic"
        );
        assert!(
            matches!(result.unwrap_err(), ClientError::InvalidApiKey),
            "Error must be ClientError::InvalidApiKey"
        );
    }
}
