//! Slash Command Execution System
//!
//! Provides:
//! - Command discovery from `.claude/commands/` directory
//! - File loading with YAML frontmatter parsing
//! - Template expansion with argument substitution
//! - Built-in commands (/help, /exit, /clear)
//! - Full command lifecycle management

pub mod builtins;
pub mod executor;
pub mod loader;
pub mod parser;
pub mod registry;

pub use self::{executor::Executor, parser::CommandParser, registry::Registry};

use anyhow::Result;
use std::path::PathBuf;

/// Default commands directory
pub const DEFAULT_COMMANDS_DIR: &str = ".claude/commands";

/// Maximum expanded prompt character budget
pub const MAX_EXPANDED_CHARS: usize = 15_000;

/// Maximum recursion depth for command discovery
pub const MAX_RECURSION_DEPTH: usize = 3;

/// Namespace separator for nested commands
pub const NAMESPACE_SEPARATOR: char = ':';

/// Lightweight metadata for command introspection
#[derive(Debug, Clone)]
pub struct CommandMetadata {
    pub name: String,
    pub description: Option<String>,
    pub disable_model_invocation: bool,
}

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

    /// Get tab completion suggestions for a command prefix
    /// Returns list of (command_name, optional_argument_hint) tuples
    pub fn get_completions(&self, prefix: &str) -> Vec<(String, Option<String>)> {
        self.registry.get_completions(prefix)
    }

    /// Get command metadata for a specific command
    ///
    /// Returns None if the command does not exist in the registry.
    /// For existing commands, returns metadata including the disable_model_invocation flag,
    /// which defaults to true (local execution) for backward compatibility.
    pub fn get_command_metadata(&self, name: &str) -> Option<CommandMetadata> {
        let cmd = self.registry.get(name).ok()?;
        Some(CommandMetadata {
            name: cmd.name.clone(),
            description: cmd.frontmatter.description.clone(),
            disable_model_invocation: cmd.frontmatter.disable_model_invocation.unwrap_or(true),
        })
    }

    /// Check if command should be intercepted and executed locally
    ///
    /// Returns true if the command should be intercepted locally (default behavior).
    /// Returns false if the command should be passed through to Claude for model invocation.
    ///
    /// For unknown commands, returns false (pass through to model) to allow Claude
    /// to handle them or provide appropriate error messages.
    pub fn should_intercept_locally(&self, command_name: &str) -> bool {
        let cmd = match self.registry.get(command_name) {
            Ok(cmd) => cmd,
            Err(_) => return false, // Unknown commands pass through to model
        };
        cmd.frontmatter.disable_model_invocation.unwrap_or(true)
    }
}

#[cfg(test)]
mod tests_model_invocation;

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

    #[tokio::test]
    async fn test_get_completions_empty_prefix() {
        let slash_commands = SlashCommands::new().await.unwrap();
        let completions = slash_commands.get_completions("");

        // With empty prefix, should return all commands
        assert!(!completions.is_empty());
    }

    #[tokio::test]
    async fn test_get_completions_with_prefix() {
        let slash_commands = SlashCommands::new().await.unwrap();

        // Test with a prefix that matches some commands
        let completions = slash_commands.get_completions("an");

        // Should return commands starting with "an" (like "analyze")
        // The exact result depends on available commands
        for (cmd, _) in &completions {
            assert!(cmd.starts_with("an"));
        }
    }

    #[tokio::test]
    async fn test_get_completions_no_match() {
        let slash_commands = SlashCommands::new().await.unwrap();

        // Test with a prefix that matches no commands
        let completions = slash_commands.get_completions("zzz_no_match");

        assert!(completions.is_empty());
    }

    #[tokio::test]
    async fn test_get_completions_includes_hints() {
        let slash_commands = SlashCommands::new().await.unwrap();

        // Get all completions
        let completions = slash_commands.get_completions("");

        // Some commands should have hints (argument_hint in frontmatter)
        // We just verify the structure is correct
        for (_cmd, hint) in &completions {
            // hint is Option<String>, verify it can be None or Some
            if let Some(h) = hint {
                assert!(!h.is_empty())
            }
        }
    }
}
