//! Plugin Manifest Parsing and Validation
//!
//! Handles `plugin.json` manifest parsing, validation, and schema enforcement.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Plugin manifest as defined in plugin.json
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    /// Unique plugin identifier (e.g., "com.example.myplugin")
    pub id: String,
    /// Human-readable plugin name
    pub name: String,
    /// Semantic version (e.g., "1.0.0")
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin author name
    pub author: String,
    /// License identifier (e.g., "MIT", "Apache-2.0")
    pub license: String,
    /// Entry point file path relative to plugin root
    pub main: String,
    /// Command definitions
    #[serde(default)]
    pub commands: Vec<CommandDefinition>,
    /// Skill definitions
    #[serde(default)]
    pub skills: Vec<SkillDefinition>,
    /// Hook definitions
    #[serde(default)]
    pub hooks: Vec<HookDefinition>,
    /// Agent definitions
    #[serde(default)]
    pub agents: Vec<AgentDefinition>,
    /// MCP server definitions
    #[serde(default)]
    pub mcp_servers: Vec<McpServerDefinition>,
    /// Runtime dependencies
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    /// Configuration schema in JSON Schema format
    #[serde(default)]
    pub config_schema: serde_json::Value,
}

/// Command plugin definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandDefinition {
    /// Command name (unique within plugin)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// File path relative to plugin root
    pub path: String,
    /// JSON Schema for command arguments
    pub args_schema: serde_json::Value,
}

/// Skill plugin definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillDefinition {
    /// Unique skill identifier
    pub id: String,
    /// Human-readable skill name
    pub name: String,
    /// Skill description
    pub description: String,
    /// File path to skill documentation/implementation
    pub path: String,
    /// Tools that are explicitly allowed for this skill (empty/None means all tools allowed)
    #[serde(
        default,
        rename = "allowedTools",
        skip_serializing_if = "Option::is_none"
    )]
    pub allowed_tools: Option<Vec<String>>,
}

/// Hook definition for lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookDefinition {
    /// Event type (e.g., "onLoad", "onUnload", "onError")
    pub event: String,
    /// Handler function name or file path
    pub handler: String,
    /// Optional matcher pattern for filtering hooks
    /// If None, defaults to "*" (match all)
    /// Can be exact match ("Write") or regex pattern ("Write|Edit|Read")
    #[serde(default)]
    pub matcher: Option<String>,
}

/// Agent plugin definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentDefinition {
    /// Unique agent identifier
    pub id: String,
    /// Human-readable agent name
    pub name: String,
    /// Agent description
    pub description: String,
    /// File path to agent prompt/configuration
    pub path: String,
    /// Optional model override for agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Tools that are explicitly allowed for this agent (empty/None means all tools allowed)
    #[serde(
        default,
        rename = "allowedTools",
        skip_serializing_if = "Option::is_none"
    )]
    pub allowed_tools: Option<Vec<String>>,
    /// Tools that are explicitly blocked for this agent
    #[serde(
        default,
        rename = "disallowedTools",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub disallowed_tools: Vec<String>,
}

/// MCP server transport type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpTransportConfig {
    /// Standard I/O transport (default)
    Stdio {
        /// Command to start the MCP server
        command: String,
        /// Command arguments
        #[serde(default)]
        args: Vec<String>,
    },
    /// HTTP/SSE transport
    Http {
        /// HTTP URL for MCP server
        url: String,
        /// Optional HTTP headers
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServerDefinition {
    /// Server identifier (used in tool names as mcp__{server_id}__{tool_name})
    pub id: String,
    /// Human-readable server name
    pub name: String,
    /// Transport configuration (defaults to stdio for backward compatibility)
    #[serde(flatten)]
    pub transport: Option<McpTransportConfig>,
    /// Command to start the MCP server (deprecated - use transport.stdio.command)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Command arguments (deprecated - use transport.stdio.args)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Environment variables for the server (stdio only)
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Server description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl McpServerDefinition {
    /// Get the effective transport configuration, handling backward compatibility
    pub fn get_transport(&self) -> Result<McpTransportConfig, String> {
        if let Some(transport) = &self.transport {
            return Ok(transport.clone());
        }

        // Backward compatibility: use command/args fields if transport not specified
        if let Some(command) = &self.command {
            return Ok(McpTransportConfig::Stdio {
                command: command.clone(),
                args: self.args.clone(),
            });
        }

        Err("No transport configuration found".to_string())
    }
}

/// Parse manifest from plugin.json file
pub fn parse_manifest(plugin_path: &Path) -> Result<PluginManifest, String> {
    let manifest_path = plugin_path.join("plugin.json");

    if !manifest_path.exists() {
        return Err("Missing plugin.json".to_string());
    }

    let content = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;

    serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {}", e))
}

/// Validate manifest against required schema
pub fn validate_manifest(manifest: &PluginManifest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Required fields
    if manifest.id.is_empty() {
        errors.push("Field 'id' is required".to_string());
    }
    if manifest.name.is_empty() {
        errors.push("Field 'name' is required".to_string());
    }
    if manifest.version.is_empty() {
        errors.push("Field 'version' is required".to_string());
    }
    if manifest.main.is_empty() {
        errors.push("Field 'main' is required".to_string());
    }
    if manifest.author.is_empty() {
        errors.push("Field 'author' is required".to_string());
    }
    if manifest.license.is_empty() {
        errors.push("Field 'license' is required".to_string());
    }

    // Validation rules
    if !manifest.id.contains('.') {
        errors.push("Plugin ID must be a dotted identifier (e.g., com.example.plugin)".to_string());
    }

    if !is_valid_semver(&manifest.version) {
        errors.push(format!("Invalid semantic version: {}", manifest.version));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Check if a string is valid semantic versioning
fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 3 {
        return false;
    }
    parts.iter().take(3).all(|p| p.parse::<u32>().is_ok())
}

/// Validate that all referenced files exist
pub fn validate_references(
    manifest: &PluginManifest,
    plugin_path: &Path,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Validate main entry point
    let main_path = plugin_path.join(&manifest.main);
    if !main_path.exists() {
        errors.push(format!("Main entry point not found: {}", manifest.main));
    }

    // Validate command paths
    for cmd in &manifest.commands {
        let cmd_path = plugin_path.join(&cmd.path);
        if !cmd_path.exists() {
            errors.push(format!("Command file not found: {}", cmd.path));
        }
    }

    // Validate skill paths
    for skill in &manifest.skills {
        let skill_path = plugin_path.join(&skill.path);
        if !skill_path.exists() {
            errors.push(format!("Skill file not found: {}", skill.path));
        }
    }

    // Validate agent paths
    for agent in &manifest.agents {
        let agent_path = plugin_path.join(&agent.path);
        if !agent_path.exists() {
            errors.push(format!("Agent file not found: {}", agent.path));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
