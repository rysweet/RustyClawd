//! Request types for the Anthropic API
//!
//! Contains `CreateMessageRequest`, builder methods, `Speed`, `ThinkingConfig`, and `Metadata`.

use serde::{Deserialize, Serialize};

use super::types::{ExtraToolSchema, Message, ToolChoice, ToolDefinition};

/// Speed mode for the API request.
///
/// Currently only `Fast` is supported (Opus 4.6 only).
/// Requires beta header `anthropic-beta: fast-mode-2026-02-01`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Speed {
    /// Fast mode - reduced latency with faster output (Opus 4.6 only)
    Fast,
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
    /// Speed mode for the request (Opus 4.6 only).
    /// Requires beta header `anthropic-beta: fast-mode-2026-02-01` when set to `Fast`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<Speed>,
}

/// Request metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
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
    #[must_use]
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Builder: Enable extended thinking with specified token budget
    #[must_use]
    pub fn with_thinking(mut self, budget_tokens: u32) -> Self {
        self.thinking = Some(ThinkingConfig::new(budget_tokens));
        self
    }

    /// Builder: Set system prompt
    #[must_use]
    pub fn with_system(mut self, system: String) -> Self {
        self.system = Some(system);
        self
    }

    /// Builder: Set temperature
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Builder: Set top_p
    #[must_use]
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Builder: Set top_k
    #[must_use]
    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Builder: Set stop_sequences
    #[must_use]
    pub fn with_stop_sequences(mut self, stop_sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(stop_sequences);
        self
    }

    /// Builder: Set tools
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Builder: Set tool choice
    #[must_use]
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Builder: Set extra tool schemas (server-side tools)
    #[must_use]
    pub fn with_extra_tool_schemas(mut self, schemas: Vec<ExtraToolSchema>) -> Self {
        self.extra_tool_schemas = Some(schemas);
        self
    }

    /// Builder: Enable fast mode (Opus 4.6 only)
    ///
    /// Sets `speed` to `Speed::Fast` when enabled, or clears it when disabled.
    /// Model validation happens at send time via `validate()`.
    #[must_use]
    pub fn with_speed(mut self, fast: bool) -> Self {
        if fast {
            self.speed = Some(Speed::Fast);
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
        if self.speed == Some(Speed::Fast) && !self.model.starts_with("claude-opus-4-6") {
            return Err(
                "Fast mode (speed: \"fast\") is only supported on claude-opus-4-6 models"
                    .to_string(),
            );
        }
        Ok(())
    }

    /// Returns true if this request requires the fast mode beta header.
    #[must_use]
    pub fn requires_fast_mode_beta(&self) -> bool {
        self.speed == Some(Speed::Fast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_speed_fast_with_opus_46() {
        let request =
            CreateMessageRequest::new("claude-opus-4-6", vec![Message::user("Test")], 1024)
                .with_speed(true);

        assert_eq!(request.speed, Some(Speed::Fast));
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

        assert_eq!(request.speed, Some(Speed::Fast));
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

        assert_eq!(request.speed, Some(Speed::Fast));
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
}
