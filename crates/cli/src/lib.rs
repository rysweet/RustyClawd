//! RustyClawd CLI library
//!
//! Infrastructure modules for the Claude Code CLI.

pub mod checkpoint;
pub mod commands;
pub mod hooks;
pub mod interactive;
pub mod plugins;
pub mod session;
pub mod settings;
pub mod terminal_guard;
pub mod tool_definitions;
pub mod tool_executor;
pub mod tui;

// Public exports
pub use session::{SessionState, SessionStats};
