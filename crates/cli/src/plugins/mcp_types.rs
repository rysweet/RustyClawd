//! MCP Type Definitions - Data structures for MCP protocol messages and server state
//!
//! All type definitions used by the MCP proxy system:
//! structs, enums, and their trait implementations.

use serde::{Deserialize, Serialize};
use tokio::process::Child as TokioChild;

use crate::plugins::manifest::McpServerDefinition;

/// MCP connection type
#[derive(Debug)]
pub enum McpConnection {
    /// Standard I/O connection with child process
    Stdio {
        process: TokioChild,
        /// Handle to notification listener task (if started)
        notification_task: Option<tokio::task::JoinHandle<()>>,
    },
    /// HTTP connection with reqwest client
    Http {
        client: reqwest::Client,
        url: String,
    },
}

/// MCP server instance state
#[derive(Debug)]
pub struct McpServerInstance {
    /// Server definition
    pub definition: McpServerDefinition,
    /// Active connection (if started)
    pub connection: Option<McpConnection>,
    /// Server capabilities discovered at startup
    pub capabilities: Option<McpCapabilities>,
    /// Available tools from this server
    pub tools: Vec<McpToolDefinition>,
    /// Available resources from this server
    pub resources: Vec<Resource>,
    /// Available prompts from this server
    pub prompts: Vec<McpPromptDefinition>,
}

/// MCP server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCapabilities {
    /// Tools capability
    #[serde(default)]
    pub tools: bool,
    /// Resources capability
    #[serde(default)]
    pub resources: bool,
    /// Prompts capability
    #[serde(default)]
    pub prompts: bool,
}

/// MCP tool definition from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    /// Tool name (will be prefixed with mcp__{server_id}__)
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for input
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// MCP CallToolResult per MCP spec (2025-11-25)
///
/// Represents the result of executing a tool via tools/call.
/// Includes both human-readable content and optional structured JSON data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallToolResult {
    /// Array of content blocks (text, images, etc.) for human-readable output
    pub content: Vec<serde_json::Value>,
    /// Optional structured JSON result matching the tool's declared outputSchema.
    /// Use this when returning typed data that callers can parse programmatically.
    #[serde(rename = "structuredContent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<serde_json::Value>,
    /// Whether this is an error response
    #[serde(rename = "isError")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// MCP resource definition from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Resource URI (e.g., file:///path/to/file, http://example.com/resource)
    pub uri: String,
    /// Human-readable resource name
    pub name: String,
    /// Resource description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type of the resource
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP resource contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContents {
    /// Resource URI
    pub uri: String,
    /// MIME type of the contents
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Text contents (for text-based resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Binary contents (base64 encoded, for binary resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// MCP prompt definition from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptDefinition {
    /// Prompt name (unique identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Optional list of arguments
    #[serde(default)]
    pub arguments: Vec<McpPromptArgument>,
}

/// Argument for MCP prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    /// Argument name
    pub name: String,
    /// Argument description
    pub description: String,
    /// Whether this argument is required
    pub required: bool,
}

/// Message in a prompt response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptMessage {
    /// Message role (user, assistant, system)
    pub role: String,
    /// Message content (structured JSON)
    pub content: serde_json::Value,
}

/// Result from prompts/get
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptResult {
    /// Prompt description
    #[serde(default)]
    pub description: Option<String>,
    /// List of messages
    pub messages: Vec<McpPromptMessage>,
}

/// MCP request message
#[derive(Debug, Serialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

/// MCP response message
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields populated by JSON deserialization
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<McpError>,
}

/// MCP notification message (no id field)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC message (can be response or notification)
#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)] // Used by serde for untagged deserialization dispatch
pub enum JsonRpcMessage {
    Response(McpResponse),
    Notification(McpNotification),
}

/// Types of MCP notifications we handle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpNotificationType {
    ToolsListChanged,
    ResourcesListChanged,
    PromptsListChanged,
    Unknown(String),
}

impl McpNotificationType {
    /// Parse notification type from method string
    pub fn from_method(method: &str) -> Self {
        match method {
            "notifications/tools/list_changed" => Self::ToolsListChanged,
            "notifications/resources/list_changed" => Self::ResourcesListChanged,
            "notifications/prompts/list_changed" => Self::PromptsListChanged,
            _ => Self::Unknown(method.to_string()),
        }
    }

    /// Convert to method string
    pub fn to_method(&self) -> String {
        match self {
            Self::ToolsListChanged => "notifications/tools/list_changed".to_string(),
            Self::ResourcesListChanged => "notifications/resources/list_changed".to_string(),
            Self::PromptsListChanged => "notifications/prompts/list_changed".to_string(),
            Self::Unknown(method) => method.clone(),
        }
    }
}

/// MCP error structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields populated by JSON deserialization
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}
