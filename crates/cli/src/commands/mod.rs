//! Slash Command Execution System
//!
//! Provides:
//! - Command discovery from `.claude/commands/` directory
//! - File loading with YAML frontmatter parsing
//! - Template expansion with argument substitution
//! - Built-in commands (/help, /exit, /clear)
//! - Full command lifecycle management

pub mod parser;
pub mod loader;
pub mod registry;
pub mod executor;
pub mod builtins;

pub use self::{
    executor::Executor,
    parser::CommandParser,
    registry::Registry,
};

use anyhow::Result;
use std::path::PathBuf;

/// Default commands directory
pub const DEFAULT_COMMANDS_DIR: &str = ".claude/commands";

/// Maximum expanded prompt character budget
pub const MAX_EXPANDED_CHARS: usize = 15_000;

/// Command execution result
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub command_name: String,
    pub expanded_prompt: String,
    pub is_builtin: bool,
    pub arguments: Vec<String>,
}

impl CommandResult {
    /// Check if expanded prompt is within character budget
    pub fn is_within_budget(&self) -> bool {
        self.expanded_prompt.len() <= MAX_EXPANDED_CHARS
    }

    /// Get number of characters used
    pub fn char_count(&self) -> usize {
        self.expanded_prompt.len()
    }

    /// Get percentage of character budget used
    pub fn budget_percentage(&self) -> f32 {
        (self.char_count() as f32 / MAX_EXPANDED_CHARS as f32) * 100.0
    }
}

/// Public API for slash command execution
pub struct SlashCommands {
    parser: CommandParser,
    registry: Registry,
    executor: Executor,
}

impl SlashCommands {
    /// Create a new slash command system with default configuration
    pub async fn new() -> Result<Self> {
        let registry = Registry::discover(PathBuf::from(DEFAULT_COMMANDS_DIR)).await?;
        let executor = Executor::new();
        let parser = CommandParser::new();

        Ok(Self {
            parser,
            registry,
            executor,
        })
    }

    /// Create with custom commands directory
    pub async fn with_commands_dir(dir: PathBuf) -> Result<Self> {
        let registry = Registry::discover(dir).await?;
        let executor = Executor::new();
        let parser = CommandParser::new();

        Ok(Self {
            parser,
            registry,
            executor,
        })
    }

    /// Execute a slash command
    pub async fn execute(&self, input: &str) -> Result<CommandResult> {
        let cmd = self.parser.parse(input)?;
        self.executor.execute(&cmd, &self.registry).await
    }

    /// Get all available commands
    pub fn list_commands(&self) -> Vec<String> {
        self.registry.list_commands()
    }

    /// Check if a command exists
    pub fn has_command(&self, name: &str) -> bool {
        self.registry.has_command(name)
    }

    /// Get command help information
    pub fn get_help(&self, command_name: Option<&str>) -> String {
        if let Some(name) = command_name {
            self.registry.get_command_info(name)
        } else {
            self.registry.list_all_info()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_result_budget_check() {
        let result = CommandResult {
            command_name: "test".to_string(),
            expanded_prompt: "x".repeat(5000),
            is_builtin: false,
            arguments: vec![],
        };

        assert!(result.is_within_budget());
        assert_eq!(result.char_count(), 5000);
        assert!(result.budget_percentage() < 50.0);
    }

    #[test]
    fn test_command_result_exceeds_budget() {
        let result = CommandResult {
            command_name: "test".to_string(),
            expanded_prompt: "x".repeat(15_001),
            is_builtin: false,
            arguments: vec![],
        };

        assert!(!result.is_within_budget());
    }

    #[test]
    fn test_default_commands_dir_constant() {
        assert_eq!(DEFAULT_COMMANDS_DIR, ".claude/commands");
    }

    #[test]
    fn test_max_expanded_chars_constant() {
        assert_eq!(MAX_EXPANDED_CHARS, 15_000);
    }
}
