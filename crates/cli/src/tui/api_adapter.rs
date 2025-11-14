//! TUI API Adapter
//!
//! This module provides the bridge between the TUI's message format and the
//! Claude API client from rustyclawd_core. It handles:
//! - Message conversion between TUI and API formats
//! - Streaming response handling
//! - Error handling and recovery
//!
//! Design follows the Zero-BS philosophy:
//! - Real API calls only (no fake responses)
//! - Working streaming implementation
//! - Comprehensive error handling

use super::{ChatMessage, MessageRole};
use anyhow::{Context as AnyhowContext, Result};
use futures::Stream;
use rustyclawd_core::client::types::MessageContent;
use rustyclawd_core::client::{Client, Config, CreateMessageRequest, Message as ApiMessage, Role};
use std::pin::Pin;

/// TUI API adapter for Claude API integration
pub struct TuiApiAdapter {
    client: Client,
    model: String,
    max_tokens: u32,
}

impl TuiApiAdapter {
    /// Create a new TUI API adapter
    ///
    /// # Arguments
    /// * `api_key` - Claude API key
    /// * `model` - Model name (defaults to claude-sonnet-4-5)
    /// * `max_tokens` - Maximum tokens per response (defaults to 4096)
    ///
    /// # Errors
    /// Returns error if API key is invalid or config cannot be created
    pub fn new(api_key: String, model: Option<String>, max_tokens: Option<u32>) -> Result<Self> {
        use rustyclawd_core::client::ApiKey;

        let api_key = ApiKey::new(api_key).context("Invalid API key format")?;
        let config = Config::new(api_key);

        let client = Client::new(config);

        Ok(Self {
            client,
            model: model.unwrap_or_else(|| "claude-sonnet-4-5-20250929".to_string()),
            max_tokens: max_tokens.unwrap_or(4096),
        })
    }

    /// Create adapter from default config location (~/.claude-msec-k)
    ///
    /// # Errors
    /// Returns error if config file doesn't exist or is invalid
    pub async fn from_default_config(
        model: Option<String>,
        max_tokens: Option<u32>,
    ) -> Result<Self> {
        let config = Config::from_default_location()
            .await
            .context("Failed to load config from ~/.claude-msec-k")?;

        let client = Client::new(config);

        Ok(Self {
            client,
            model: model.unwrap_or_else(|| "claude-sonnet-4-5-20250929".to_string()),
            max_tokens: max_tokens.unwrap_or(4096),
        })
    }

    /// Send a message and get streaming response
    ///
    /// Converts TUI messages to API format, sends request, and returns
    /// a stream of text chunks.
    ///
    /// # Arguments
    /// * `messages` - Conversation history in TUI format
    ///
    /// # Returns
    /// Stream of text chunks as they arrive from the API
    ///
    /// # Errors
    /// Returns error if API call fails or message conversion fails
    pub async fn send_message_stream(
        &self,
        messages: &[ChatMessage],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        use futures::StreamExt;

        // Convert TUI messages to API messages
        let api_messages = self.convert_messages(messages)?;

        // Create streaming request
        let request = CreateMessageRequest::new(self.model.clone(), api_messages, self.max_tokens)
            .with_stream(true);

        // Get text stream from client
        let text_stream = self
            .client
            .stream_text(request)
            .await
            .context("Failed to create message stream")?;

        // Convert ClientResult<String> to Result<String> and box the stream
        let result_stream =
            text_stream.map(|result| result.map_err(|e| anyhow::anyhow!("Stream error: {}", e)));

        Ok(Box::pin(result_stream))
    }

    /// Send a message and get complete response (non-streaming)
    ///
    /// Useful for testing or when streaming is not needed.
    ///
    /// # Arguments
    /// * `messages` - Conversation history in TUI format
    ///
    /// # Returns
    /// Complete response text
    ///
    /// # Errors
    /// Returns error if API call fails or message conversion fails
    pub async fn send_message(&self, messages: &[ChatMessage]) -> Result<String> {
        // Convert TUI messages to API messages
        let api_messages = self.convert_messages(messages)?;

        // Create non-streaming request
        let request = CreateMessageRequest::new(self.model.clone(), api_messages, self.max_tokens);

        // Execute request
        let response = self
            .client
            .create_message(request)
            .await
            .context("Failed to create message")?;

        // Extract text from response
        let text = response
            .content
            .iter()
            .filter_map(|block| {
                if let rustyclawd_core::client::ContentBlock::Text { text } = block {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }

    /// Convert TUI messages to API messages
    ///
    /// Filters out system messages (not supported by API in messages array)
    /// and converts user/assistant messages to API format.
    ///
    /// # Arguments
    /// * `messages` - TUI messages to convert
    ///
    /// # Returns
    /// Vector of API messages
    ///
    /// # Errors
    /// Returns error if message conversion fails
    fn convert_messages(&self, messages: &[ChatMessage]) -> Result<Vec<ApiMessage>> {
        let mut api_messages = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::User => {
                    api_messages.push(ApiMessage {
                        role: Role::User,
                        content: MessageContent::Text(msg.content.clone()),
                    });
                }
                MessageRole::Assistant => {
                    api_messages.push(ApiMessage {
                        role: Role::Assistant,
                        content: MessageContent::Text(msg.content.clone()),
                    });
                }
                MessageRole::System => {
                    // System messages are handled separately via system parameter
                    // For now, we skip them in the messages array
                    // In future, could collect and pass as system prompt
                    continue;
                }
            }
        }

        // Validate we have at least one message
        if api_messages.is_empty() {
            anyhow::bail!(
                "No valid messages to send (need at least one user or assistant message)"
            );
        }

        Ok(api_messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_conversion_user() {
        let adapter = TuiApiAdapter {
            client: create_test_client(),
            model: "test-model".to_string(),
            max_tokens: 1024,
        };

        let messages = vec![ChatMessage::user("Hello, Claude!".to_string())];

        let api_messages = adapter.convert_messages(&messages).unwrap();

        assert_eq!(api_messages.len(), 1);
        assert_eq!(api_messages[0].role, Role::User);
        match &api_messages[0].content {
            MessageContent::Text(text) => assert_eq!(text, "Hello, Claude!"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_message_conversion_assistant() {
        let adapter = TuiApiAdapter {
            client: create_test_client(),
            model: "test-model".to_string(),
            max_tokens: 1024,
        };

        let messages = vec![ChatMessage::assistant("Hello!".to_string())];

        let api_messages = adapter.convert_messages(&messages).unwrap();

        assert_eq!(api_messages.len(), 1);
        assert_eq!(api_messages[0].role, Role::Assistant);
    }

    #[test]
    fn test_message_conversion_skips_system() {
        let adapter = TuiApiAdapter {
            client: create_test_client(),
            model: "test-model".to_string(),
            max_tokens: 1024,
        };

        let messages = vec![
            ChatMessage::system("System message".to_string()),
            ChatMessage::user("User message".to_string()),
        ];

        let api_messages = adapter.convert_messages(&messages).unwrap();

        // System message should be filtered out
        assert_eq!(api_messages.len(), 1);
        assert_eq!(api_messages[0].role, Role::User);
    }

    #[test]
    fn test_message_conversion_mixed() {
        let adapter = TuiApiAdapter {
            client: create_test_client(),
            model: "test-model".to_string(),
            max_tokens: 1024,
        };

        let messages = vec![
            ChatMessage::user("What is Rust?".to_string()),
            ChatMessage::assistant("Rust is a systems programming language.".to_string()),
            ChatMessage::user("Tell me more.".to_string()),
        ];

        let api_messages = adapter.convert_messages(&messages).unwrap();

        assert_eq!(api_messages.len(), 3);
        assert_eq!(api_messages[0].role, Role::User);
        assert_eq!(api_messages[1].role, Role::Assistant);
        assert_eq!(api_messages[2].role, Role::User);
    }

    #[test]
    fn test_message_conversion_empty_fails() {
        let adapter = TuiApiAdapter {
            client: create_test_client(),
            model: "test-model".to_string(),
            max_tokens: 1024,
        };

        let messages: Vec<ChatMessage> = vec![];

        let result = adapter.convert_messages(&messages);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No valid messages"));
    }

    #[test]
    fn test_message_conversion_only_system_fails() {
        let adapter = TuiApiAdapter {
            client: create_test_client(),
            model: "test-model".to_string(),
            max_tokens: 1024,
        };

        let messages = vec![ChatMessage::system("System only".to_string())];

        let result = adapter.convert_messages(&messages);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No valid messages"));
    }

    // Helper to create a test client for unit tests
    fn create_test_client() -> Client {
        use rustyclawd_core::client::ApiKey;

        // Create a test config with dummy key
        let api_key = ApiKey::new("sk-ant-test-key-for-unit-tests-only".to_string())
            .expect("Failed to create test API key");
        let config = Config::new(api_key);

        Client::new(config)
    }
}
