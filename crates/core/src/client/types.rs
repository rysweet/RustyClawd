//! Core primitive types for the Anthropic API
//!
//! This module contains the foundational types shared across request and response:
//! `Role`, `Message`, `MessageContent`, `ContentBlock`, `ToolDefinition`,
//! `ExtraToolSchema`, and `ToolChoice`.
//!
//! Request-specific types live in `request.rs`.
//! Response-specific types live in `response.rs`.
//!
//! For backward compatibility, this module also re-exports all request and response
//! types so that existing `client::types::*` import paths continue to work.

use serde::{Deserialize, Serialize};

// Re-export request types for backward compatibility
pub use super::request::{CreateMessageRequest, Metadata, Speed, ThinkingConfig};

// Re-export response types for backward compatibility
pub use super::response::{
    ApiError, ContentBlockStart, ContentDelta, MessageDelta, MessageResponse, MessageStart,
    StreamEvent, Usage,
};

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
}
