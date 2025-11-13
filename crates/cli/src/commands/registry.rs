//! Command registry - discovers and manages available commands

use crate::commands::loader::{CommandLoader, LoadedCommand};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use thiserror::Error;

/// Registry errors
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Registry not initialized")]
    NotInitialized,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid command: {0}")]
    InvalidCommand(String),
}

/// Command scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandScope {
    /// Project-level commands (.claude/commands/)
    Project,
    /// Personal commands (~/.claude/commands/)
    Personal,
}

/// Command with its scope
#[derive(Debug, Clone)]
pub struct ScopedCommand {
    pub command: LoadedCommand,
    pub scope: CommandScope,
}

/// Command registry - manages discovered commands
pub struct Registry {
    /// Loaded commands by name
    commands: HashMap<String, ScopedCommand>,
    /// Project commands directory
    project_dir: PathBuf,
    /// Personal commands directory
    personal_dir: Option<PathBuf>,
    /// Loader instance
    loader: CommandLoader,
}

impl Registry {
    /// Create an empty registry
    pub fn new(project_dir: PathBuf) -> Self {
        let personal_dir = Self::get_personal_dir();
        Self {
            commands: HashMap::new(),
            project_dir,
            personal_dir,
            loader: CommandLoader::new(),
        }
    }

    /// Get personal commands directory (~/.claude/commands/)
    fn get_personal_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".claude").join("commands"))
    }

    /// Discover and load commands from both project and personal directories
    pub async fn discover(project_dir: PathBuf) -> Result<Self> {
        let mut registry = Self::new(project_dir.clone());

        // Load from personal directory first (lower priority)
        let personal_dir_clone = registry.personal_dir.clone();
        if let Some(personal_dir) = personal_dir_clone {
            registry.load_from_directory(&personal_dir, CommandScope::Personal).await?;
        }

        // Load from project directory (higher priority, can override personal commands)
        let project_dir_clone = registry.project_dir.clone();
        registry.load_from_directory(&project_dir_clone, CommandScope::Project).await?;

        Ok(registry)
    }

    /// Load commands from a specific directory
    async fn load_from_directory(&mut self, dir: &Path, scope: CommandScope) -> Result<()> {
        // Create directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(dir).await {
            tracing::debug!("Could not create commands directory {}: {}", dir.display(), e);
            return Ok(()); // Not an error if personal dir doesn't exist
        }

        // Scan for .md files
        let mut entries = match fs::read_dir(dir).await {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!("Could not read commands directory {}: {}", dir.display(), e);
                return Ok(()); // Not an error if directory isn't readable
            }
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
                match self.loader.load_command(&path).await {
                    Ok(cmd) => {
                        let name = cmd.name.clone();
                        self.commands.insert(
                            name.clone(),
                            ScopedCommand {
                                command: cmd,
                                scope,
                            },
                        );
                        tracing::debug!(
                            "Loaded command '{}' from {:?} scope",
                            name,
                            scope
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to load command from {}: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Register a command in memory (for testing)
    pub fn register(&mut self, cmd: LoadedCommand) -> Result<()> {
        if cmd.name.is_empty() {
            return Err(anyhow!("Command name cannot be empty"));
        }

        let name = cmd.name.clone();
        self.commands.insert(
            name,
            ScopedCommand {
                command: cmd,
                scope: CommandScope::Project,
            },
        );
        Ok(())
    }

    /// Get a command by name
    pub fn get(&self, name: &str) -> Result<&LoadedCommand> {
        self.commands
            .get(name)
            .map(|scoped| &scoped.command)
            .ok_or_else(|| anyhow!(RegistryError::CommandNotFound(name.to_string())))
    }

    /// Get a command with its scope
    pub fn get_scoped(&self, name: &str) -> Result<&ScopedCommand> {
        self.commands
            .get(name)
            .ok_or_else(|| anyhow!(RegistryError::CommandNotFound(name.to_string())))
    }

    /// Check if command exists
    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// List all command names
    pub fn list_commands(&self) -> Vec<String> {
        let mut names: Vec<_> = self.commands.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get command count
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Get info about a specific command
    pub fn get_command_info(&self, name: &str) -> String {
        match self.get_scoped(name) {
            Ok(scoped) => {
                let cmd = &scoped.command;
                let desc = cmd
                    .frontmatter
                    .description
                    .as_deref()
                    .unwrap_or("No description");
                let scope_str = match scoped.scope {
                    CommandScope::Project => "project",
                    CommandScope::Personal => "personal",
                };
                let hint = cmd
                    .frontmatter
                    .argument_hint
                    .as_deref()
                    .map(|h| format!(" {}", h))
                    .unwrap_or_default();

                format!(
                    "/{}{}\n  {}\n  Scope: {}",
                    name, hint, desc, scope_str
                )
            }
            Err(_) => format!("Command '{}' not found", name),
        }
    }

    /// List all commands with info
    pub fn list_all_info(&self) -> String {
        let mut output = String::from("Available Commands:\n\n");

        let commands = self.list_commands();
        if commands.is_empty() {
            output.push_str("No commands found\n");
            output.push_str(&format!("  Project: {}\n", self.project_dir.display()));
            if let Some(personal) = &self.personal_dir {
                output.push_str(&format!("  Personal: {}\n", personal.display()));
            }
        } else {
            for name in commands {
                if let Ok(scoped) = self.get_scoped(&name) {
                    let cmd = &scoped.command;
                    let desc = cmd
                        .frontmatter
                        .description
                        .as_deref()
                        .unwrap_or("No description");
                    let hint = cmd
                        .frontmatter
                        .argument_hint
                        .as_deref()
                        .map(|h| format!(" {}", h))
                        .unwrap_or_default();
                    let scope_badge = match scoped.scope {
                        CommandScope::Project => "[project]",
                        CommandScope::Personal => "[personal]",
                    };

                    output.push_str(&format!(
                        "  /{}{} {}\n    {}\n",
                        name, hint, scope_badge, desc
                    ));
                }
            }
        }

        output
    }

    /// Get tab completion suggestions
    pub fn get_completions(&self, prefix: &str) -> Vec<(String, Option<String>)> {
        self.commands
            .iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .map(|(name, scoped)| {
                let hint = scoped.command.frontmatter.argument_hint.clone();
                (name.clone(), hint)
            })
            .collect()
    }

    /// Search commands by name pattern
    pub fn search(&self, pattern: &str) -> Vec<String> {
        self.commands
            .keys()
            .filter(|name| name.contains(pattern))
            .cloned()
            .collect()
    }

    /// Get loader reference
    pub fn loader(&self) -> &CommandLoader {
        &self.loader
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::loader::FrontMatter;

    #[test]
    fn test_registry_new() {
        let dir = PathBuf::from(".test_commands");
        let registry = Registry::new(dir.clone());

        assert_eq!(registry.project_dir, dir);
        assert_eq!(registry.command_count(), 0);
    }

    #[test]
    fn test_registry_register_command() {
        let mut registry = Registry::new(PathBuf::from(".test"));

        let cmd = LoadedCommand {
            name: "test".to_string(),
            frontmatter: FrontMatter {
                description: Some("Test command".to_string()),
                ..Default::default()
            },
            content: "Content".to_string(),
        };

        registry.register(cmd).unwrap();
        assert_eq!(registry.command_count(), 1);
        assert!(registry.has_command("test"));
    }

    #[test]
    fn test_registry_get_command() {
        let mut registry = Registry::new(PathBuf::from(".test"));

        let cmd = LoadedCommand {
            name: "hello".to_string(),
            frontmatter: FrontMatter::default(),
            content: "Hello world".to_string(),
        };

        registry.register(cmd).unwrap();
        let retrieved = registry.get("hello").unwrap();

        assert_eq!(retrieved.name, "hello");
        assert_eq!(retrieved.content, "Hello world");
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = Registry::new(PathBuf::from(".test"));

        let result = registry.get("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_list_commands() {
        let mut registry = Registry::new(PathBuf::from(".test"));

        for i in 0..3 {
            let cmd = LoadedCommand {
                name: format!("cmd{}", i),
                frontmatter: FrontMatter::default(),
                content: "Content".to_string(),
            };
            registry.register(cmd).unwrap();
        }

        let list = registry.list_commands();
        assert_eq!(list.len(), 3);
        assert!(list.contains(&"cmd0".to_string()));
    }

    #[test]
    fn test_registry_list_commands_sorted() {
        let mut registry = Registry::new(PathBuf::from(".test"));

        for name in &["zebra", "apple", "monkey"] {
            let cmd = LoadedCommand {
                name: name.to_string(),
                frontmatter: FrontMatter::default(),
                content: "Content".to_string(),
            };
            registry.register(cmd).unwrap();
        }

        let list = registry.list_commands();
        assert_eq!(list, vec!["apple", "monkey", "zebra"]);
    }

    #[test]
    fn test_registry_search() {
        let mut registry = Registry::new(PathBuf::from(".test"));

        for name in &["review-pr", "review-issue", "deploy-prod"] {
            let cmd = LoadedCommand {
                name: name.to_string(),
                frontmatter: FrontMatter::default(),
                content: "Content".to_string(),
            };
            registry.register(cmd).unwrap();
        }

        let results = registry.search("review");
        assert_eq!(results.len(), 2);
        assert!(results.contains(&"review-pr".to_string()));
    }

    #[test]
    fn test_registry_register_empty_name() {
        let mut registry = Registry::new(PathBuf::from(".test"));

        let cmd = LoadedCommand {
            name: String::new(),
            frontmatter: FrontMatter::default(),
            content: "Content".to_string(),
        };

        let result = registry.register(cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_command_info() {
        let mut registry = Registry::new(PathBuf::from(".commands"));

        let cmd = LoadedCommand {
            name: "help".to_string(),
            frontmatter: FrontMatter {
                description: Some("Show help".to_string()),
                ..Default::default()
            },
            content: "Help content".to_string(),
        };

        registry.register(cmd).unwrap();
        let info = registry.get_command_info("help");

        assert!(info.contains("help"));
        assert!(info.contains("Show help"));
    }

    #[test]
    fn test_registry_list_all_info() {
        let mut registry = Registry::new(PathBuf::from(".test"));

        let cmd = LoadedCommand {
            name: "test".to_string(),
            frontmatter: FrontMatter {
                description: Some("Test command".to_string()),
                ..Default::default()
            },
            content: "Content".to_string(),
        };

        registry.register(cmd).unwrap();
        let info = registry.list_all_info();

        assert!(info.contains("Available Commands"));
        assert!(info.contains("test"));
        assert!(info.contains("Test command"));
    }
}
