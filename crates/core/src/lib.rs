//! Core types and traits for Claude Code Rust translation
//!
//! This crate provides the fundamental types used throughout the application:
//! - Messages (User, Assistant, System, ToolResult)
//! - Context (conversation state)
//! - Error types
//! - Anthropic API client with streaming support

pub mod client;
pub mod context;
pub mod error;
pub mod message;

pub use context::Context;
pub use error::{CoreError, Result};
pub use message::{Message, MessageRole};
