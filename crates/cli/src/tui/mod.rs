//! TUI module
//!
//! Contains TUI-related functionality including input viewport management

pub mod input_viewport;
mod ui;

// Re-export main TUI types and functions
pub use ui::{ChatMessage, MessageRole, TuiState};
