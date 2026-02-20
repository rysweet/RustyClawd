//! Plugin System - Discovery, Loading, and Execution
//!
//! Complete plugin management system for Claude Code CLI.
//!
//! # Features
//! - Plugin discovery from filesystem
//! - Manifest parsing and validation
//! - Plugin loading with file validation
//! - Command and skill execution
//! - API contract enforcement
//!
//! # Example
//! ```ignore
//! use rustyclawd::plugins::*;
//!
//! // Discover plugins
//! let discovery = discovery::PluginDiscovery::new("./plugins");
//! let plugins = discovery.discover_all()?;
//!
//! // Load plugins
//! let mut loader = loader::PluginLoader::new();
//! for plugin in plugins {
//!     loader.register(plugin);
//! }
//! loader.load("com.example.plugin")?;
//! loader.initialize("com.example.plugin")?;
//!
//! // Execute commands
//! let mut executor = executor::PluginExecutor::new();
//! let result = executor.execute_command(
//!     "com.example.plugin",
//!     "mycommand",
//!     serde_json::json!({})
//! )?;
//! ```

pub mod agent_discovery;
pub mod discovery;
pub mod executor;
pub mod frontmatter_substitution;
pub mod hooks_integration;
pub mod loader;
pub mod manager;
pub mod manifest;
pub mod mcp_proxy;
pub mod subprocess;
pub mod tool_search_config;

pub use agent_discovery::{parse_runtime_agents, validate_runtime_agents, RuntimeAgentDefinition};
pub use discovery::PluginDiscovery;
pub use executor::PluginExecutor;
pub use frontmatter_substitution::{Substituter, SubstitutionContext};
pub use loader::PluginLoader;

// Re-exports for library consumers and integration tests.
// These are unused by the binary target but needed for the lib API.
#[allow(unused_imports)]
pub use agent_discovery::AgentDiscovery;
#[allow(unused_imports)]
pub use hooks_integration::PluginHooksIntegrator;
#[allow(unused_imports)]
pub use manager::PluginManager;
#[allow(unused_imports)]
pub use mcp_proxy::{McpNotification, McpNotificationType, McpProxy};
#[allow(unused_imports)]
pub use tool_search_config::ToolSearchConfig;

/// Plugin system version
pub const PLUGIN_VERSION: &str = "1.0.0";

/// Maximum plugin ID length
pub const MAX_PLUGIN_ID_LENGTH: usize = 128;

/// Plugin system error types
#[derive(Debug)]
pub enum PluginError {
    /// Discovery error
    Discovery(String),
    /// Manifest error
    Manifest(String),
    /// Loading error
    Load(String),
    /// Execution error
    Execution(String),
    /// Validation error
    Validation(Vec<String>),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::Discovery(msg) => write!(f, "Discovery error: {}", msg),
            PluginError::Manifest(msg) => write!(f, "Manifest error: {}", msg),
            PluginError::Load(msg) => write!(f, "Load error: {}", msg),
            PluginError::Execution(msg) => write!(f, "Execution error: {}", msg),
            PluginError::Validation(errors) => {
                write!(f, "Validation errors: {}", errors.join(", "))
            }
        }
    }
}

impl std::error::Error for PluginError {}

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

#[cfg(test)]
mod tests {
    use super::discovery::{PluginLoadStatus, PluginMetadata};
    use super::manifest::{CommandDefinition, PluginManifest, SkillDefinition};
    use super::*;
    use std::collections::HashMap;
    use std::fs;

    fn create_test_plugin_dir(name: &str) -> std::path::PathBuf {
        let test_dir = std::env::temp_dir().join(format!("plugin-test-{}", name));
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).expect("Failed to create test directory");
        test_dir
    }

    #[test]
    fn test_discovery_and_loading() {
        let test_dir = create_test_plugin_dir("discovery_loading");
        let plugin_dir = test_dir.join("test-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        // Create manifest
        let manifest = PluginManifest {
            id: "com.test.e2e".to_string(),
            name: "E2E Test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test plugin".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            main: "index.js".to_string(),
            commands: vec![],
            skills: vec![],
            hooks: vec![],
            agents: vec![],
            mcp_servers: vec![],
            dependencies: HashMap::new(),
            config_schema: serde_json::json!({}),
        };

        fs::write(
            plugin_dir.join("plugin.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(plugin_dir.join("index.js"), "// Plugin code").unwrap();

        // Discover
        let discovery = PluginDiscovery::new(&test_dir);
        let plugins = discovery.discover_all().unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, "com.test.e2e");

        // Load
        let mut loader = PluginLoader::new();
        loader.register(plugins[0].clone());
        assert!(loader.load("com.test.e2e").is_ok());
        assert!(loader.is_loaded("com.test.e2e"));
    }

    #[tokio::test]
    async fn test_command_execution() {
        // Create a temp directory with a real cmd.js script so the executor
        // can detect its interpreter and run it via subprocess.
        let test_dir = create_test_plugin_dir("command_execution");
        fs::write(
            test_dir.join("cmd.js"),
            "console.log(JSON.stringify({command: 'test-cmd', status: 'ok'}));\n",
        )
        .expect("Failed to write cmd.js");

        let manifest = PluginManifest {
            id: "com.test.cmd".to_string(),
            name: "Command Test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            main: "index.js".to_string(),
            commands: vec![CommandDefinition {
                name: "test-cmd".to_string(),
                description: "Test".to_string(),
                path: "cmd.js".to_string(),
                args_schema: serde_json::json!({}),
            }],
            skills: vec![],
            hooks: vec![],
            agents: vec![],
            mcp_servers: vec![],
            dependencies: HashMap::new(),
            config_schema: serde_json::json!({}),
        };

        let metadata = PluginMetadata {
            id: "com.test.cmd".to_string(),
            path: test_dir.clone(),
            manifest,
            enabled: true,
            load_status: PluginLoadStatus::Loaded,
        };

        let mut executor = PluginExecutor::new();
        executor.register(metadata);

        let result = executor
            .execute_command("com.test.cmd", "test-cmd", serde_json::json!({}))
            .await;

        // The command should either succeed (if node is available) or return
        // a meaningful error. It must not panic.
        match result {
            Ok(exec_result) => {
                assert!(exec_result.success);
                assert!(exec_result.output.contains("test-cmd"));
            }
            Err(e) => {
                // If node is not installed, the subprocess will fail to spawn.
                // That is acceptable in CI environments without node - the key
                // is that it returns an error rather than panicking.
                assert!(
                    e.contains("Failed to execute") || e.contains("Failed to spawn"),
                    "Unexpected error: {}",
                    e
                );
            }
        }

        // Cleanup
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_skill_execution() {
        let test_dir = create_test_plugin_dir("skill_execution");
        let plugin_dir = test_dir.join("test-skill-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        // Create skill file
        fs::write(
            plugin_dir.join("skill.md"),
            "# Test Skill\n\nThis is a test skill for skill-1",
        )
        .unwrap();

        let manifest = PluginManifest {
            id: "com.test.skill".to_string(),
            name: "Skill Test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            main: "index.js".to_string(),
            commands: vec![],
            skills: vec![SkillDefinition {
                id: "skill-1".to_string(),
                name: "Test Skill".to_string(),
                description: "Test".to_string(),
                path: "skill.md".to_string(),
                allowed_tools: None,
            }],
            hooks: vec![],
            agents: vec![],
            mcp_servers: vec![],
            dependencies: HashMap::new(),
            config_schema: serde_json::json!({}),
        };

        let metadata = PluginMetadata {
            id: "com.test.skill".to_string(),
            path: plugin_dir,
            manifest,
            enabled: true,
            load_status: PluginLoadStatus::Loaded,
        };

        let mut executor = PluginExecutor::new();
        executor.register(metadata);

        let result = executor.execute_skill("com.test.skill", "skill-1").unwrap();

        assert!(result.success);
        assert!(result.output.contains("skill-1"));
    }
}
