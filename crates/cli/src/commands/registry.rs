//! Command registry - discovers and manages available commands

use crate::commands::loader::{CommandLoader, LoadedCommand};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
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

/// Command registry - manages discovered commands
pub struct Registry {
    /// Loaded commands by name
    commands: HashMap<String, LoadedCommand>,
    /// Commands directory
    commands_dir: PathBuf,
    /// Loader instance
    loader: CommandLoader,
}

impl Registry {
    /// Create an empty registry
    pub fn new(commands_dir: PathBuf) -> Self {
        Self {
            commands: HashMap::new(),
            commands_dir,
            loader: CommandLoader::new(),
        }
    }

    /// Discover and load commands from directory
    pub async fn discover(commands_dir: PathBuf) -> Result<Self> {
        let mut registry = Self::new(commands_dir.clone());

        // Create directory if it doesn't exist
        fs::create_dir_all(&registry.commands_dir)
            .await
            .context("Failed to create commands directory")?;

        // Scan for .md files
        let mut entries = fs::read_dir(&registry.commands_dir)
            .await
            .context("Failed to read commands directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
                match registry.loader.load_command(&path).await {
                    Ok(cmd) => {
                        registry.commands.insert(cmd.name.clone(), cmd);
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

        Ok(registry)
    }

    /// Register a command in memory (for testing)
    pub fn register(&mut self, cmd: LoadedCommand) -> Result<()> {
        if cmd.name.is_empty() {
            return Err(anyhow!("Command name cannot be empty"));
        }

        self.commands.insert(cmd.name.clone(), cmd);
        Ok(())
    }

    /// Get a command by name
    pub fn get(&self, name: &str) -> Result<&LoadedCommand> {
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
        match self.get(name) {
            Ok(cmd) => {
                let desc = cmd
                    .frontmatter
                    .description
                    .as_deref()
                    .unwrap_or("No description");
                format!("{}:\n  {}\n  Location: {}/{}",
                    name,
                    desc,
                    self.commands_dir.display(),
                    name)
            }
            Err(_) => format!("Command '{}' not found", name),
        }
    }

    /// List all commands with info
    pub fn list_all_info(&self) -> String {
        let mut output = String::from("Available Commands:\n\n");

        let commands = self.list_commands();
        if commands.is_empty() {
            output.push_str("No commands found in ");
            output.push_str(&self.commands_dir.display().to_string());
        } else {
            for name in commands {
                if let Ok(cmd) = self.get(&name) {
                    let desc = cmd
                        .frontmatter
                        .description
                        .as_deref()
                        .unwrap_or("No description");
                    output.push_str(&format!("  /{}\n    {}\n", name, desc));
                }
            }
        }

        output
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

        assert_eq!(registry.commands_dir, dir);
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
