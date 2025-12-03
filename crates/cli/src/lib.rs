//! RustyClawd CLI library
//!
//! Infrastructure modules that aren't yet fully integrated.
//! Allow dead code temporarily while features are being completed.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(deprecated)] // TODO: Migrate from ClientError::Api to specific error types

pub mod checkpoint;
pub mod commands;
pub mod hooks;
pub mod interactive;
pub mod mcp_commands;
pub mod notification;
pub mod plugins;
pub mod session;
pub mod session_persistence;
pub mod settings;
pub mod terminal_guard;
pub mod tool_definitions;
pub mod tool_executor;
pub mod tool_formatter;
pub mod tui;
pub mod update;
