//! Claude Code CLI Library - Expose modules for testing and reuse
//!
//! This library exposes the CLI module structure to support both the binary
//! and testing/library use cases.

pub mod checkpoint;
pub mod commands;
pub mod hooks;
pub mod plugins;
pub mod settings;

pub use commands::{
    parser::Command, parser::CommandParser, executor::Executor, registry::Registry,
    CommandResult, SlashCommands,
};
