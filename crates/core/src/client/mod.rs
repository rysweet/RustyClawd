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
//!     let client = Client::new(config);
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
pub mod error;
pub mod stream;
pub mod types;

use futures::Stream;
use reqwest::Client as HttpClient;
use secrecy::ExposeSecret;
use std::time::Duration;

pub use config::{ApiKey, Config};
pub use error::{ClientError, ClientResult};
pub use stream::{EventStream, SseEvent, SseStream};
pub use types::{
    ContentBlock, CreateMessageRequest, ExtraToolSchema, Message, MessageResponse, Role,
    StreamEvent, ToolChoice, ToolDefinition, Usage,
};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries (default: 3)
    pub max_retries: u32,
    /// Initial delay before first retry (default: 1s)
    pub initial_delay: Duration,
    /// Maximum delay between retries (default: 30s)
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Anthropic API client
pub struct Client {
    config: Config,
    http_client: HttpClient,
    retry_config: RetryConfig,
}

impl Client {
    /// Create a new client with the given configuration and default retry settings
    pub fn new(config: Config) -> Self {
        let timeout = Duration::from_secs(config.timeout_secs);

        let http_client = HttpClient::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            config,
            http_client,
            retry_config: RetryConfig::default(),
        }
    }

    /// Create a new client with custom retry configuration
    pub fn with_retry_config(config: Config, retry_config: RetryConfig) -> Self {
        let timeout = Duration::from_secs(config.timeout_secs);

        let http_client = HttpClient::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            config,
            http_client,
            retry_config,
        }
    }

    /// Create a message (non-streaming) with automatic retry logic
    pub async fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> ClientResult<MessageResponse> {
        self.create_message_with_retry(request).await
    }

    /// Internal method to create a message with retry logic
    async fn create_message_with_retry(
        &self,
        request: CreateMessageRequest,
    ) -> ClientResult<MessageResponse> {
        let mut retries = 0;

        loop {
            match self.create_message_internal(&request).await {
                Ok(message) => return Ok(message),
                Err(e) if e.is_retryable() && retries < self.retry_config.max_retries => {
                    // Calculate delay with exponential backoff
                    let base_delay = self.retry_config.initial_delay * 2_u32.pow(retries);
                    let delay = std::cmp::min(base_delay, self.retry_config.max_delay);

                    // Use Retry-After if provided, otherwise use calculated delay
                    let actual_delay = e.retry_after().unwrap_or(delay);

                    eprintln!(
                        "Retrying request after {}s (attempt {}/{})",
                        actual_delay.as_secs(),
                        retries + 1,
                        self.retry_config.max_retries
                    );

                    tokio::time::sleep(actual_delay).await;
                    retries += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Internal method to execute a single message request without retry
    async fn create_message_internal(
        &self,
        request: &CreateMessageRequest,
    ) -> ClientResult<MessageResponse> {
        let url = format!("{}/v1/messages", self.config.api_url);

        let response = self
            .http_client
            .post(&url)
            .header("x-api-key", self.config.api_key.expose_secret().expose())
            .header("anthropic-version", &self.config.api_version)
            .header("content-type", "application/json")
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

    /// Create a message with streaming
    pub async fn create_message_stream(
        &self,
        mut request: CreateMessageRequest,
    ) -> ClientResult<impl Stream<Item = ClientResult<StreamEvent>>> {
        // Ensure streaming is enabled
        request.stream = true;

        let url = format!("{}/v1/messages", self.config.api_url);

        let response = self
            .http_client
            .post(&url)
            .header("x-api-key", self.config.api_key.expose_secret().expose())
            .header("anthropic-version", &self.config.api_version)
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&request)
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
                            delta: types::ContentDelta::TextDelta { text },
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

    /// Execute a message with automatic tool calling loop
    ///
    /// This method handles the full tool use protocol:
    /// 1. Sends initial request with tools
    /// 2. If Claude returns tool_use blocks, executes them
    /// 3. Sends tool results back to Claude
    /// 4. Repeats until Claude returns a text response
    ///
    /// # Arguments
    ///
    /// * `request` - Initial request with tools configured
    /// * `tool_executor` - Callback to execute tool calls
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rustyclawd_core::client::{Client, Config, CreateMessageRequest, Message};
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = Config::from_default_location().await?;
    ///     let client = Client::new(config);
    ///
    ///     let request = CreateMessageRequest::new(
    ///         "claude-sonnet-4-5-20250929",
    ///         vec![Message::user("Run ls command")],
    ///         4096,
    ///     );
    ///
    ///     let response = client.execute_with_tools(request, |tool_name, tool_input| async move {
    ///         // Execute tool and return result as JSON
    ///         Ok(serde_json::json!({"output": "file1.txt\nfile2.txt"}))
    ///     }).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute_with_tools<F, Fut>(
        &self,
        mut request: CreateMessageRequest,
        tool_executor: F,
    ) -> ClientResult<MessageResponse>
    where
        F: Fn(String, serde_json::Value) -> Fut,
        Fut: std::future::Future<Output = ClientResult<serde_json::Value>>,
    {
        // High limit for complex agentic workflows
        const MAX_ITERATIONS: usize = 10_000;
        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration > MAX_ITERATIONS {
                return Err(ClientError::Unknown(
                    "Tool execution exceeded maximum iterations".to_string(),
                ));
            }

            // Execute the request
            let response = self.create_message(request.clone()).await?;

            // Check if response contains tool use
            let mut has_tool_use = false;
            let mut tool_result_blocks = Vec::new();

            for block in &response.content {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    has_tool_use = true;

                    // Print tool invocation details
                    eprintln!("\n[Tool: {}]", name);
                    if let Ok(pretty_input) = serde_json::to_string_pretty(input) {
                        eprintln!("Input: {}", pretty_input);
                    }
                    eprintln!();

                    // Execute the tool
                    match tool_executor(name.clone(), input.clone()).await {
                        Ok(result) => {
                            // Print tool result
                            eprintln!("[Tool Result: {}]", name);
                            if let Ok(pretty_result) = serde_json::to_string_pretty(&result) {
                                eprintln!("{}", pretty_result);
                            }
                            eprintln!();

                            tool_result_blocks.push(ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: result.to_string(),
                                is_error: None,
                            });
                        }
                        Err(e) => {
                            // Print tool error
                            eprintln!("[Tool Error: {}]", name);
                            eprintln!("Error: {}", e);
                            eprintln!();

                            tool_result_blocks.push(ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: format!("Tool execution error: {}", e),
                                is_error: Some(true),
                            });
                        }
                    }
                }
            }

            // If no tool use, we're done
            if !has_tool_use {
                return Ok(response);
            }

            // Build the next request with tool results
            // First, add the assistant's response with tool_use blocks to conversation
            request.messages.push(Message::with_blocks(
                Role::Assistant,
                response.content.clone(),
            ));

            // Then add tool results as user message with tool_result blocks
            request
                .messages
                .push(Message::with_blocks(Role::User, tool_result_blocks));
        }
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

/// Sanitize error text to remove any API keys
#[allow(dead_code)]
fn sanitize_error_text(text: &str) -> String {
    error::sanitize_error(text)
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
        let sanitized = sanitize_error_text(error);
        assert!(!sanitized.contains("test123"));
        assert!(sanitized.contains("[REDACTED_API_KEY]"));
    }

    #[test]
    fn test_client_no_leak_in_debug() {
        let key = ApiKey::new("sk-ant-secret123".to_string()).unwrap();
        let config = Config::new(key);
        let client = Client::new(config);
        let debug_str = format!("{:?}", client);
        assert!(!debug_str.contains("secret123"));
    }
}
