//! Error types for tools

use thiserror::Error;

/// Tool execution errors
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Process error: {0}")]
    ProcessError(String),
}

/// Convenience Result type for tools
pub type ToolResult<T> = std::result::Result<T, ToolError>;

/// Errors for agent memory operations
#[derive(Error, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AgentMemoryError {
    #[error("project ID is required for project scope operations")]
    ProjectIdRequired,

    #[error("entry value exceeds maximum size of {max_bytes} bytes (got {actual_bytes} bytes)")]
    EntrySizeLimitExceeded {
        max_bytes: usize,
        actual_bytes: usize,
    },
}

/// Errors for agent registry operations
#[derive(Error, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AgentRegistryError {
    #[error("agent not found: {0}")]
    AgentNotFound(String),
}
