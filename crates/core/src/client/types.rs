//! Request and response types for the Anthropic API
//!
//! These types match the official Anthropic API specification.

use serde::{Deserialize, Serialize};

/// Message role in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Message content - can be simple string or structured blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text string
    Text(String),
    /// Structured content blocks (for tool use)
    Blocks(Vec<ContentBlock>),
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: MessageContent,
}

/// Tool definition for the API
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: serde_json::Value,
    /// Enable strict schema validation (requires beta header)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl ToolDefinition {
    /// Create a new tool definition
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            strict: None,
        }
    }

    /// Enable strict schema validation
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = Some(strict);
        self
    }
}

/// Server-side tool schema for extra tools like web_search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraToolSchema {
    /// Tool type (e.g., "web_search_20250305")
    #[serde(rename = "type")]
    pub type_field: String,
    /// Tool name (e.g., "web_search")
    pub name: String,
    /// Optional: Allowed domains for web search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    /// Optional: Blocked domains for web search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_domains: Option<Vec<String>>,
    /// Optional: Maximum number of uses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u32>,
}

impl ExtraToolSchema {
    /// Create a web search tool schema
    pub fn web_search(
        allowed_domains: Option<Vec<String>>,
        blocked_domains: Option<Vec<String>>,
        max_uses: Option<u32>,
    ) -> Self {
        Self {
            type_field: "web_search_20250305".to_string(),
            name: "web_search".to_string(),
            allowed_domains,
            blocked_domains,
            max_uses,
        }
    }
}

/// Tool choice configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Auto (default) - model decides whether to use tools
    Auto { r#type: String },
    /// Any - model must use at least one tool
    Any { r#type: String },
    /// Tool - model must use specific tool
    Tool { r#type: String, name: String },
}

impl ToolChoice {
    /// Create auto tool choice (default)
    pub fn auto() -> Self {
        Self::Auto {
            r#type: "auto".to_string(),
        }
    }

    /// Create any tool choice (must use a tool)
    pub fn any() -> Self {
        Self::Any {
            r#type: "any".to_string(),
        }
    }

    /// Create specific tool choice
    pub fn tool(name: impl Into<String>) -> Self {
        Self::Tool {
            r#type: "tool".to_string(),
            name: name.into(),
        }
    }
}

/// Extended thinking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingConfig {
    /// Type of thinking - always "enabled"
    #[serde(rename = "type")]
    pub type_field: String,
    /// Token budget for thinking (1024-16000)
    pub budget_tokens: u32,
}

impl ThinkingConfig {
    /// Create a new thinking configuration with the specified token budget
    pub fn new(budget_tokens: u32) -> Self {
        Self {
            type_field: "enabled".to_string(),
            budget_tokens,
        }
    }
}

/// Request to create a message (non-streaming)
#[derive(Debug, Clone, Serialize)]
pub struct CreateMessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Set to true for streaming responses
    #[serde(default)]
    pub stream: bool,
    /// Tools available for the model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// Tool choice configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Server-side tools (e.g., web_search)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_tool_schemas: Option<Vec<ExtraToolSchema>>,
    /// Extended thinking configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    /// Speed mode: "fast" for fast mode (Opus 4.6 only), or None for standard speed.
    /// Requires beta header `anthropic-beta: fast-mode-2026-02-01` when set to "fast".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
}

/// Request metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Content block in a response or request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Vec<ContentBlock>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// Extended thinking block (chain-of-thought reasoning)
    Thinking {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
}

/// Usage statistics for a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Speed used for the response: "fast" or "standard" (when fast mode beta is active)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
}

/// Complete message response (non-streaming)
#[derive(Debug, Clone, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

/// Streaming event types
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart {
        message: MessageStart,
    },
    ContentBlockStart {
        index: u32,
        content_block: ContentBlockStart,
    },
    ContentBlockDelta {
        index: u32,
        delta: ContentDelta,
    },
    ContentBlockStop {
        index: u32,
    },
    MessageDelta {
        delta: MessageDelta,
        usage: Usage,
    },
    MessageStop,
    Ping,
    Error {
        error: ApiError,
    },
}

/// Message start event data
#[derive(Debug, Clone, Deserialize)]
pub struct MessageStart {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub role: Role,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

/// Content block start event data
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockStart {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
    },
    /// Extended thinking block start
    Thinking,
}

/// Content delta (incremental update)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta {
        text: String,
    },
    InputJsonDelta {
        partial_json: String,
    },
    /// Extended thinking delta
    ThinkingDelta {
        thinking: String,
    },
    /// Signature delta (for thinking block authenticity)
    SignatureDelta {
        signature: String,
    },
}

/// Message delta (final updates)
#[derive(Debug, Clone, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

/// API error response
#[derive(Debug, Clone, Deserialize)]
pub struct ApiError {
    #[serde(rename = "type")]
    pub type_field: String,
    pub message: String,
}

impl CreateMessageRequest {
    /// Create a simple text request
    pub fn new(model: impl Into<String>, messages: Vec<Message>, max_tokens: u32) -> Self {
        Self {
            model: model.into(),
            max_tokens,
            messages,
            system: None,
            temperature: None,
            top_p: None,
            top_k: None,
            metadata: None,
            stop_sequences: None,
            stream: false,
            tools: None,
            tool_choice: None,
            extra_tool_schemas: None,
            thinking: None,
            speed: None,
        }
    }

    /// Builder: Enable streaming
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Builder: Enable extended thinking with specified token budget
    pub fn with_thinking(mut self, budget_tokens: u32) -> Self {
        self.thinking = Some(ThinkingConfig::new(budget_tokens));
        self
    }

    /// Builder: Set system prompt
    pub fn with_system(mut self, system: String) -> Self {
        self.system = Some(system);
        self
    }

    /// Builder: Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Builder: Set top_p
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Builder: Set top_k
    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Builder: Set stop_sequences
    pub fn with_stop_sequences(mut self, stop_sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(stop_sequences);
        self
    }

    /// Builder: Set tools
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Builder: Set tool choice
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Builder: Set extra tool schemas (server-side tools)
    pub fn with_extra_tool_schemas(mut self, schemas: Vec<ExtraToolSchema>) -> Self {
        self.extra_tool_schemas = Some(schemas);
        self
    }

    /// Builder: Enable fast mode (Opus 4.6 only)
    ///
    /// Sets `speed` to `"fast"` when enabled, or clears it when disabled.
    /// Model validation happens at send time via `validate()`.
    pub fn with_speed(mut self, fast: bool) -> Self {
        if fast {
            self.speed = Some("fast".to_string());
        } else {
            self.speed = None;
        }
        self
    }

    /// Validate the request before sending.
    ///
    /// Returns an error message if the request is invalid.
    /// Called automatically by the client before making API requests.
    pub fn validate(&self) -> Result<(), String> {
        // Fast mode is only supported on Opus 4.6 models
        if self.speed.as_deref() == Some("fast") && !self.model.starts_with("claude-opus-4-6") {
            return Err(
                "Fast mode (speed: \"fast\") is only supported on claude-opus-4-6 models"
                    .to_string(),
            );
        }
        Ok(())
    }

    /// Returns true if this request requires the fast mode beta header.
    pub fn requires_fast_mode_beta(&self) -> bool {
        self.speed.as_deref() == Some("fast")
    }
}

impl Message {
    /// Create a user message with text content
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create an assistant message with text content
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a message with structured content blocks
    pub fn with_blocks(role: Role, blocks: Vec<ContentBlock>) -> Self {
        Self {
            role,
            content: MessageContent::Blocks(blocks),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, Role::User);
        assert!(matches!(user_msg.content, MessageContent::Text(_)));

        let assistant_msg = Message::assistant("Hi there");
        assert_eq!(assistant_msg.role, Role::Assistant);
        assert!(matches!(assistant_msg.content, MessageContent::Text(_)));
    }

    #[test]
    fn test_request_builder() {
        let request = CreateMessageRequest::new(
            "claude-3-5-sonnet-20241022",
            vec![Message::user("Test")],
            1024,
        )
        .with_stream(true)
        .with_temperature(0.7);

        assert_eq!(request.model, "claude-3-5-sonnet-20241022");
        assert_eq!(request.max_tokens, 1024);
        assert!(request.stream);
        assert_eq!(request.temperature, Some(0.7));
    }
}

#[cfg(test)]
mod fast_mode_tests {
    use super::*;
    use crate::client::{ApiKey, Config};

    #[test]
    fn test_speed_fast_with_opus_46() {
        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Test")], 1024)
                .with_speed(true);

        assert_eq!(request.speed, Some("fast".to_string()));
        assert!(request.validate().is_ok());
        assert!(request.requires_fast_mode_beta());
    }

    #[test]
    fn test_speed_fast_with_dated_opus_46_model() {
        // Model IDs may include dates, e.g. "claude-opus-4-6-20260201"
        let request = CreateMessageRequest::new(
            "claude-opus-4-6-20260201",
            vec![Message::user("Test")],
            1024,
        )
        .with_speed(true);

        assert_eq!(request.speed, Some("fast".to_string()));
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_speed_fast_with_non_opus_fails_validation() {
        let request = CreateMessageRequest::new(
            "claude-3-5-sonnet-20241022",
            vec![Message::user("Test")],
            1024,
        )
        .with_speed(true);

        let result = request.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("only supported on claude-opus-4-6"));
    }

    #[test]
    fn test_speed_disabled_works_with_any_model() {
        let request = CreateMessageRequest::new(
            "claude-3-5-sonnet-20241022",
            vec![Message::user("Test")],
            1024,
        )
        .with_speed(false);

        assert_eq!(request.speed, None);
        assert!(request.validate().is_ok());
        assert!(!request.requires_fast_mode_beta());
    }

    #[test]
    fn test_speed_fast_builder_chains() {
        // Verify with_speed returns Self and can be chained
        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Test")], 1024)
                .with_speed(true)
                .with_stream(true)
                .with_temperature(0.7);

        assert_eq!(request.speed, Some("fast".to_string()));
        assert!(request.stream);
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_speed_serialization() {
        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Test")], 1024)
                .with_speed(true);

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(r#""speed":"fast""#));
        assert!(!json.contains("fast_mode"));
    }

    #[test]
    fn test_speed_none_not_serialized() {
        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Test")], 1024);

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("speed"));
    }

    #[test]
    fn test_model_matching_rejects_substring() {
        // "my-custom-claude-opus-4-6-proxy" should NOT pass because starts_with is used
        let request = CreateMessageRequest::new(
            "my-custom-claude-opus-4-6-proxy",
            vec![Message::user("Test")],
            1024,
        )
        .with_speed(true);

        let result = request.validate();
        assert!(result.is_err(), "Substring model name should be rejected");
    }

    #[test]
    fn test_config_with_fast_mode() {
        let api_key = ApiKey::new("sk-ant-test123".to_string()).unwrap();
        let config = Config::new(api_key).with_fast_mode(true);

        assert!(config.fast_mode_enabled);
    }

    #[test]
    fn test_config_default_fast_mode_false() {
        let api_key = ApiKey::new("sk-ant-test123".to_string()).unwrap();
        let config = Config::new(api_key);

        assert!(!config.fast_mode_enabled);
    }
}
