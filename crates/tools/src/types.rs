//! Common types used by tools

use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// Stream of tool events (progress, results, errors)
pub type ToolStream<T> = Pin<Box<dyn Stream<Item = ToolEvent<T>> + Send>>;

/// Events emitted by tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolEvent<T> {
    /// Progress update during execution
    Progress {
        step: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        percentage: Option<f32>,
    },

    /// Final result
    Result(T),

    /// Error occurred
    Error { message: String },
}

/// Metadata describing a tool
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub name: &'static str,
    pub description: &'static str,
}

/// Execution context for tools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionContext {
    /// Running in TUI/interactive mode - process isolation needed
    Tui,
    /// Running in non-interactive mode - no isolation needed
    #[default]
    NonInteractive,
}

/// Context passed to tools during execution
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Current working directory
    pub cwd: std::path::PathBuf,

    /// Debug mode enabled
    pub debug: bool,

    /// Additional context data
    pub metadata: serde_json::Value,

    /// Execution context (TUI vs non-interactive)
    pub execution_context: ExecutionContext,

    /// List of tools that are explicitly allowed (empty means all tools allowed)
    /// When non-empty, only tools in this list can be executed
    pub allowed_tools: Vec<String>,

    /// List of tools that are explicitly disallowed
    /// Takes precedence over allowed_tools
    pub disallowed_tools: Vec<String>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_default(),
            debug: false,
            metadata: serde_json::Value::Null,
            execution_context: ExecutionContext::default(),
            allowed_tools: vec![],
            disallowed_tools: vec![],
        }
    }
}
