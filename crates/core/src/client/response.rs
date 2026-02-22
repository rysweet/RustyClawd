//! Response types for the Anthropic API
//!
//! Contains `MessageResponse`, `StreamEvent`, and related streaming event types.

use serde::Deserialize;

use super::types::{ContentBlock, Role};

/// Usage statistics for a request
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test Usage.speed deserialization when speed field is present with "fast"
    #[test]
    fn test_usage_speed_deserialization_fast() {
        let json = r#"{"input_tokens": 100, "output_tokens": 50, "speed": "fast"}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.speed, Some("fast".to_string()));
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }

    /// Test Usage.speed deserialization when speed field is present with "standard"
    #[test]
    fn test_usage_speed_deserialization_standard() {
        let json = r#"{"input_tokens": 100, "output_tokens": 50, "speed": "standard"}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.speed, Some("standard".to_string()));
    }

    /// Test Usage.speed deserialization backward compatibility - speed field missing
    #[test]
    fn test_usage_speed_deserialization_missing() {
        let json = r#"{"input_tokens": 100, "output_tokens": 50}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.speed, None);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }
}
