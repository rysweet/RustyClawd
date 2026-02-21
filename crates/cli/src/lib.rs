//! RustyClawd CLI library
//!
//! Public API modules for the CLI crate. These modules expose types and
//! functions for external consumers and the binary target.

pub mod checkpoint;
pub mod command_handlers;
pub mod commands;
pub mod conversation;
pub mod hooks;
pub mod interactive;
pub mod mcp_commands;
pub mod notification;
pub mod permission_mode;
pub mod plugins;
pub mod schema_validator;
pub mod session;
pub mod session_graph;
pub mod session_index;
pub mod session_persistence;
pub mod settings;
pub mod streaming;
pub mod terminal_guard;
pub mod tool_definitions;
pub mod tool_orchestrator;
// TODO: Migrate from ClientError::Api to specific error types (BadRequest, Unknown, etc.)
#[allow(deprecated)]
pub mod tool_executor;
pub mod tool_formatter;
pub mod tui;
pub mod update;
