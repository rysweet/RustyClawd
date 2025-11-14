//! Plugin Manager - Unified orchestration of the complete plugin system
//!
//! Manages discovery, loading, execution, agents, MCP servers, and hooks integration.

use std::path::{Path, PathBuf};

use crate::hooks::registry::HookRegistry;
use crate::plugins::{
    agent_discovery::AgentDiscovery,
    discovery::{PluginDiscovery, PluginMetadata},
    executor::PluginExecutor,
    loader::PluginLoader,
    mcp_proxy::McpProxy,
};

/// Complete plugin system manager
pub struct PluginManager {
    /// Plugin loader
    pub loader: PluginLoader,
    /// Plugin executor
    pub executor: PluginExecutor,
    /// MCP server proxy
    pub mcp_proxy: McpProxy,
    /// Agent discovery
    pub agent_discovery: Option<AgentDiscovery>,
    /// Base plugin directory
    plugin_dir: PathBuf,
    /// Project root for agent discovery
    project_root: Option<PathBuf>,
}

impl PluginManager {
    /// Create new plugin manager
    pub fn new(plugin_dir: impl AsRef<Path>) -> Self {
        Self {
            loader: PluginLoader::new(),
            executor: PluginExecutor::new(),
            mcp_proxy: McpProxy::new(),
            agent_discovery: None,
            plugin_dir: plugin_dir.as_ref().to_path_buf(),
            project_root: None,
        }
    }

    /// Set project root for agent discovery
    pub fn with_project_root(mut self, project_root: impl AsRef<Path>) -> Self {
        let root = project_root.as_ref().to_path_buf();
        self.agent_discovery = Some(AgentDiscovery::new(&root));
        self.project_root = Some(root);
        self
    }

    /// Discover and load all plugins
    pub async fn discover_and_load_all(&mut self) -> Result<Vec<String>, String> {
        let discovery = PluginDiscovery::new(&self.plugin_dir);
        let plugins = discovery.discover_all()?;

        let mut loaded_ids = Vec::new();

        for plugin in plugins {
            let plugin_id = plugin.id.clone();

            // Register with loader
            self.loader.register(plugin.clone());

            // Load the plugin
            if let Err(e) = self.loader.load(&plugin_id) {
                eprintln!("Failed to load plugin {}: {}", plugin_id, e);
                continue;
            }

            // Initialize the plugin
            if let Err(e) = self.loader.initialize(&plugin_id) {
                eprintln!("Failed to initialize plugin {}: {}", plugin_id, e);
                continue;
            }

            // Register with executor
            self.executor.register(plugin.clone());

            // Register MCP servers
            for mcp_server in &plugin.manifest.mcp_servers {
                self.mcp_proxy.register_server(mcp_server.clone());
            }

            loaded_ids.push(plugin_id);
        }

        Ok(loaded_ids)
    }

    /// Register plugin hooks with the hooks system
    pub fn register_all_hooks(&self, hooks_registry: &mut HookRegistry) -> Result<(), String> {
        let all_plugins = self.loader.all_plugins();

        let plugin_hooks: Vec<_> = all_plugins
            .iter()
            .map(|p| (p.id.clone(), p.path.clone(), p.manifest.hooks.clone()))
            .collect();

        crate::plugins::hooks_integration::register_plugin_hooks(&plugin_hooks, hooks_registry)?;

        Ok(())
    }

    /// Start an MCP server
    pub async fn start_mcp_server(&mut self, server_id: &str) -> Result<(), String> {
        self.mcp_proxy.start_server(server_id).await
    }

    /// Stop an MCP server
    pub async fn stop_mcp_server(&mut self, server_id: &str) -> Result<(), String> {
        self.mcp_proxy.stop_server(server_id).await
    }

    /// Start all MCP servers
    pub async fn start_all_mcp_servers(&mut self) -> Result<Vec<String>, String> {
        let server_ids = self.mcp_proxy.list_servers();
        let mut started = Vec::new();

        for server_id in server_ids {
            if let Err(e) = self.mcp_proxy.start_server(&server_id).await {
                eprintln!("Failed to start MCP server {}: {}", server_id, e);
                continue;
            }
            started.push(server_id);
        }

        Ok(started)
    }

    /// Discover agents from .claude/agents/
    pub fn discover_agents(&self) -> Result<Vec<String>, String> {
        if let Some(discovery) = &self.agent_discovery {
            discovery.list_agent_ids()
        } else {
            Ok(Vec::new())
        }
    }

    /// Get all loaded plugin IDs
    pub fn loaded_plugin_ids(&self) -> Vec<String> {
        // Include both Loaded and Initialized plugins
        self.loader
            .all_plugins()
            .into_iter()
            .filter(|p| {
                p.load_status == crate::plugins::discovery::PluginLoadStatus::Loaded
                    || p.load_status == crate::plugins::discovery::PluginLoadStatus::Initialized
            })
            .map(|p| p.id)
            .collect()
    }

    /// Get plugin metadata
    pub fn get_plugin(&self, plugin_id: &str) -> Option<PluginMetadata> {
        self.loader.get(plugin_id)
    }

    /// Execute a plugin command
    pub async fn execute_command(
        &self,
        plugin_id: &str,
        command_name: &str,
        args: serde_json::Value,
    ) -> Result<crate::plugins::executor::PluginExecutionResult, String> {
        self.executor.execute_command(plugin_id, command_name, args)
    }

    /// Execute a plugin skill
    pub async fn execute_skill(
        &self,
        plugin_id: &str,
        skill_id: &str,
    ) -> Result<crate::plugins::executor::PluginExecutionResult, String> {
        self.executor.execute_skill(plugin_id, skill_id)
    }

    /// List all available commands from all plugins
    pub fn list_all_commands(&self) -> Vec<(String, Vec<String>)> {
        let mut result = Vec::new();

        for plugin_id in self.loaded_plugin_ids() {
            if let Ok(commands) = self.executor.get_commands(&plugin_id) {
                if !commands.is_empty() {
                    result.push((plugin_id, commands));
                }
            }
        }

        result
    }

    /// List all available skills from all plugins
    pub fn list_all_skills(&self) -> Vec<(String, Vec<String>)> {
        let mut result = Vec::new();

        for plugin_id in self.loaded_plugin_ids() {
            if let Ok(skills) = self.executor.get_skills(&plugin_id) {
                if !skills.is_empty() {
                    result.push((plugin_id, skills));
                }
            }
        }

        result
    }

    /// Get summary of loaded plugins
    pub fn summary(&self) -> PluginSystemSummary {
        let plugins = self.loader.all_plugins();
        let total_commands: usize = plugins.iter().map(|p| p.manifest.commands.len()).sum();
        let total_skills: usize = plugins.iter().map(|p| p.manifest.skills.len()).sum();
        let total_hooks: usize = plugins.iter().map(|p| p.manifest.hooks.len()).sum();
        let total_agents: usize = plugins.iter().map(|p| p.manifest.agents.len()).sum();
        let total_mcp_servers: usize = plugins.iter().map(|p| p.manifest.mcp_servers.len()).sum();

        let discovered_agents = self.discover_agents().unwrap_or_default().len();

        PluginSystemSummary {
            total_plugins: plugins.len(),
            loaded_plugins: self.loaded_plugin_ids().len(),
            total_commands,
            total_skills,
            total_hooks,
            total_agents: total_agents + discovered_agents,
            total_mcp_servers,
            running_mcp_servers: self
                .mcp_proxy
                .list_servers()
                .iter()
                .filter(|s| self.mcp_proxy.is_server_running(s))
                .count(),
        }
    }

    /// Shutdown the plugin system
    pub async fn shutdown(&mut self) -> Result<(), String> {
        // Stop all MCP servers
        self.mcp_proxy.stop_all().await?;

        Ok(())
    }
}

/// Summary statistics for the plugin system
#[derive(Debug, Clone)]
pub struct PluginSystemSummary {
    pub total_plugins: usize,
    pub loaded_plugins: usize,
    pub total_commands: usize,
    pub total_skills: usize,
    pub total_hooks: usize,
    pub total_agents: usize,
    pub total_mcp_servers: usize,
    pub running_mcp_servers: usize,
}

impl std::fmt::Display for PluginSystemSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Plugin System Summary:")?;
        writeln!(
            f,
            "  Plugins: {}/{} loaded",
            self.loaded_plugins, self.total_plugins
        )?;
        writeln!(f, "  Commands: {}", self.total_commands)?;
        writeln!(f, "  Skills: {}", self.total_skills)?;
        writeln!(f, "  Hooks: {}", self.total_hooks)?;
        writeln!(f, "  Agents: {}", self.total_agents)?;
        writeln!(
            f,
            "  MCP Servers: {}/{} running",
            self.running_mcp_servers, self.total_mcp_servers
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PluginManager::new(temp_dir.path());

        assert_eq!(manager.loaded_plugin_ids().len(), 0);
    }

    #[tokio::test]
    async fn test_plugin_manager_with_project_root() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PluginManager::new(temp_dir.path()).with_project_root(temp_dir.path());

        assert!(manager.agent_discovery.is_some());
        assert!(manager.project_root.is_some());
    }

    #[tokio::test]
    async fn test_discover_agents() {
        let temp_dir = TempDir::new().unwrap();
        let agents_dir = temp_dir.path().join(".claude").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();

        fs::write(agents_dir.join("test.md"), "# Test Agent\n\nTest").unwrap();

        let manager = PluginManager::new(temp_dir.path()).with_project_root(temp_dir.path());

        let agents = manager.discover_agents().unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], "test");
    }

    #[tokio::test]
    async fn test_summary_empty() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PluginManager::new(temp_dir.path());

        let summary = manager.summary();
        assert_eq!(summary.total_plugins, 0);
        assert_eq!(summary.loaded_plugins, 0);
        assert_eq!(summary.total_commands, 0);
    }
}
