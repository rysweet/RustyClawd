//! Error types for the core crate

use thiserror::Error;

/// Core error type
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Context error: {0}")]
    Context(String),
}

/// Convenience Result type
pub type Result<T> = std::result::Result<T, CoreError>;
