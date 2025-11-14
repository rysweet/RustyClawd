//! Command executor - handles command execution pipeline

use crate::commands::{
    builtins::BuiltinCommands, loader::CommandLoader, parser::Command, registry::Registry,
    CommandResult, MAX_EXPANDED_CHARS,
};
use anyhow::{anyhow, Result};
use thiserror::Error;

/// Executor errors
#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Expanded prompt exceeds character limit: {0}/{1}")]
    CharacterLimitExceeded(usize, usize),

    #[error("Expansion failed: {0}")]
    ExpansionFailed(String),

    #[error("Template error: {0}")]
    TemplateError(String),
}

/// Command executor
pub struct Executor {
    loader: CommandLoader,
    working_dir: std::path::PathBuf,
}

impl Executor {
    /// Create a new executor with default working directory
    pub fn new() -> Self {
        Self {
            loader: CommandLoader::new(),
            working_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        }
    }

    /// Create executor with specific working directory
    pub fn with_working_dir(working_dir: std::path::PathBuf) -> Self {
        Self {
            loader: CommandLoader::new(),
            working_dir,
        }
    }

    /// Execute a command
    pub async fn execute(&self, cmd: &Command, registry: &Registry) -> Result<CommandResult> {
        // Check for built-in commands first
        if BuiltinCommands::is_builtin(&cmd.name) {
            return self.execute_builtin(cmd);
        }

        // Look up in registry
        let loaded_cmd = registry
            .get(&cmd.name)
            .map_err(|_| anyhow!(ExecutorError::CommandNotFound(cmd.name.clone())))?;

        // Expand template with all features (bash, file refs, arguments)
        let expanded = self
            .loader
            .expand_full(&loaded_cmd.content, &cmd.args, &self.working_dir)
            .await
            .map_err(|e| anyhow!(ExecutorError::ExpansionFailed(e.to_string())))?;

        // Check character limit
        if expanded.len() > MAX_EXPANDED_CHARS {
            return Err(anyhow!(ExecutorError::CharacterLimitExceeded(
                expanded.len(),
                MAX_EXPANDED_CHARS
            )));
        }

        Ok(CommandResult {
            command_name: cmd.name.clone(),
            expanded_prompt: expanded,
            is_builtin: false,
            arguments: cmd.args.clone(),
        })
    }

    /// Execute a built-in command
    fn execute_builtin(&self, cmd: &Command) -> Result<CommandResult> {
        let output = BuiltinCommands::execute(cmd)
            .ok_or_else(|| anyhow!(ExecutorError::CommandNotFound(cmd.name.clone())))?;

        Ok(CommandResult {
            command_name: cmd.name.clone(),
            expanded_prompt: output,
            is_builtin: true,
            arguments: cmd.args.clone(),
        })
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::loader::{FrontMatter, LoadedCommand};

    #[tokio::test]
    async fn test_execute_builtin_help() {
        let executor = Executor::new();
        let cmd = Command::new("help".to_string(), None);
        let registry = Registry::new(std::path::PathBuf::from(".test"));

        let result = executor.execute(&cmd, &registry).await.unwrap();

        assert!(result.is_builtin);
        assert_eq!(result.command_name, "help");
        assert!(!result.expanded_prompt.is_empty());
    }

    #[tokio::test]
    async fn test_execute_builtin_clear() {
        let executor = Executor::new();
        let cmd = Command::new("clear".to_string(), None);
        let registry = Registry::new(std::path::PathBuf::from(".test"));

        let result = executor.execute(&cmd, &registry).await.unwrap();

        assert!(result.is_builtin);
        assert!(result.expanded_prompt.contains("cleared"));
    }

    #[tokio::test]
    async fn test_execute_custom_command() {
        let executor = Executor::new();
        let mut registry = Registry::new(std::path::PathBuf::from(".test"));

        let cmd_obj = LoadedCommand {
            name: "review-pr".to_string(),
            frontmatter: FrontMatter::default(),
            content: "Review PR #{0}".to_string(),
        };

        registry.register(cmd_obj).unwrap();

        let cmd = Command::new("review-pr".to_string(), Some("123".to_string()));
        let result = executor.execute(&cmd, &registry).await.unwrap();

        assert!(!result.is_builtin);
        assert_eq!(result.command_name, "review-pr");
        assert_eq!(result.expanded_prompt, "Review PR #123");
    }

    #[tokio::test]
    async fn test_execute_command_not_found() {
        let executor = Executor::new();
        let registry = Registry::new(std::path::PathBuf::from(".test"));
        let cmd = Command::new("nonexistent".to_string(), None);

        let result = executor.execute(&cmd, &registry).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_execute_with_multiple_args() {
        let executor = Executor::new();
        let mut registry = Registry::new(std::path::PathBuf::from(".test"));

        let cmd_obj = LoadedCommand {
            name: "analyze".to_string(),
            frontmatter: FrontMatter::default(),
            content: "PR {0} priority {1} reviewer {2}".to_string(),
        };

        registry.register(cmd_obj).unwrap();

        let cmd = Command::new("analyze".to_string(), Some("456 high alice".to_string()));
        let result = executor.execute(&cmd, &registry).await.unwrap();

        assert_eq!(
            result.expanded_prompt,
            "PR 456 priority high reviewer alice"
        );
        assert_eq!(result.arguments.len(), 3);
    }

    #[tokio::test]
    async fn test_execute_exceeds_character_limit() {
        let executor = Executor::new();
        let mut registry = Registry::new(std::path::PathBuf::from(".test"));

        let cmd_obj = LoadedCommand {
            name: "huge".to_string(),
            frontmatter: FrontMatter::default(),
            content: "x".repeat(MAX_EXPANDED_CHARS + 1),
        };

        registry.register(cmd_obj).unwrap();

        let cmd = Command::new("huge".to_string(), None);
        let result = executor.execute(&cmd, &registry).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("character limit"));
    }

    #[tokio::test]
    async fn test_execute_within_character_limit() {
        let executor = Executor::new();
        let mut registry = Registry::new(std::path::PathBuf::from(".test"));

        let cmd_obj = LoadedCommand {
            name: "bigcmd".to_string(),
            frontmatter: FrontMatter::default(),
            content: "x".repeat(10_000),
        };

        registry.register(cmd_obj).unwrap();

        let cmd = Command::new("bigcmd".to_string(), None);
        let result = executor.execute(&cmd, &registry).await.unwrap();

        assert!(result.is_within_budget());
        assert_eq!(result.char_count(), 10_000);
    }

    #[test]
    fn test_executor_new() {
        let _executor = Executor::new();
        let _executor2 = Executor::default();

        // Both should be created successfully
        // This is just a compile test to ensure they can be instantiated
    }
}
