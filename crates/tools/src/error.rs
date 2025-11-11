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
