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
mod tests {
    use super::*;

    #[test]
    fn test_valid_semver() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("0.1.0"));
        assert!(is_valid_semver("10.20.30"));
        assert!(!is_valid_semver("1.0"));
        assert!(!is_valid_semver("1.a.0"));
    }

    #[test]
    fn test_validate_manifest_valid() {
        let manifest = PluginManifest {
            id: "com.example.test".to_string(),
            name: "Test".to_string(),
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

        assert!(validate_manifest(&manifest).is_ok());
    }

    #[test]
    fn test_validate_manifest_missing_id() {
        let manifest = PluginManifest {
            id: "".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
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

        let result = validate_manifest(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_mcp_transport_backward_compatibility() {
        // Old format using command/args
        let server = McpServerDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            transport: None,
            command: Some("node".to_string()),
            args: vec!["server.js".to_string()],
            env: HashMap::new(),
            description: None,
        };

        let transport = server.get_transport().unwrap();
        assert!(matches!(transport, McpTransportConfig::Stdio { .. }));
    }

    #[test]
    fn test_mcp_transport_new_stdio_format() {
        // New format using transport field
        let server = McpServerDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            transport: Some(McpTransportConfig::Stdio {
                command: "node".to_string(),
                args: vec!["server.js".to_string()],
            }),
            command: None,
            args: vec![],
            env: HashMap::new(),
            description: None,
        };

        let transport = server.get_transport().unwrap();
        assert!(matches!(transport, McpTransportConfig::Stdio { .. }));
    }

    #[test]
    fn test_mcp_transport_http() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());

        let server = McpServerDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            transport: Some(McpTransportConfig::Http {
                url: "http://localhost:8080/mcp".to_string(),
                headers: Some(headers),
            }),
            command: None,
            args: vec![],
            env: HashMap::new(),
            description: None,
        };

        let transport = server.get_transport().unwrap();
        assert!(matches!(transport, McpTransportConfig::Http { .. }));
    }

    #[test]
    fn test_agent_definition_with_disallowed_tools() {
        let json = r#"{
            "id": "secure-agent",
            "name": "Secure Agent",
            "description": "Read-only agent",
            "path": "agents/secure.md",
            "disallowedTools": ["Write", "Edit", "Bash"]
        }"#;

        let agent: AgentDefinition = serde_json::from_str(json).unwrap();

        assert_eq!(agent.id, "secure-agent");
        assert_eq!(agent.name, "Secure Agent");
        assert_eq!(agent.disallowed_tools, vec!["Write", "Edit", "Bash"]);
    }

    #[test]
    fn test_agent_definition_disallowed_tools_default_empty() {
        let json = r#"{
            "id": "basic-agent",
            "name": "Basic Agent",
            "description": "Basic agent",
            "path": "agents/basic.md"
        }"#;

        let agent: AgentDefinition = serde_json::from_str(json).unwrap();

        // disallowedTools should default to empty vec when not specified
        assert!(agent.disallowed_tools.is_empty());
    }

    #[test]
    fn test_agent_definition_serialization_skips_empty_disallowed_tools() {
        let agent = AgentDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test agent".to_string(),
            path: "test.md".to_string(),
            model: None,
            allowed_tools: None,
            disallowed_tools: vec![],
        };

        let json = serde_json::to_string(&agent).unwrap();
        // Empty disallowed_tools should not appear in serialized JSON
        assert!(!json.contains("disallowedTools"));
    }

    #[test]
    fn test_agent_definition_serialization_includes_non_empty_disallowed_tools() {
        let agent = AgentDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test agent".to_string(),
            path: "test.md".to_string(),
            model: None,
            allowed_tools: None,
            disallowed_tools: vec!["Bash".to_string()],
        };

        let json = serde_json::to_string(&agent).unwrap();
        // Non-empty disallowed_tools should appear in serialized JSON
        assert!(json.contains("disallowedTools"));
        assert!(json.contains("Bash"));
    }

    // Tests for allowed_tools feature

    #[test]
    fn test_agent_definition_with_allowed_tools() {
        let json = r#"{
            "id": "restricted-agent",
            "name": "Restricted Agent",
            "description": "Agent with limited tool access",
            "path": "agents/restricted.md",
            "allowedTools": ["Read", "Grep", "Glob"]
        }"#;

        let agent: AgentDefinition = serde_json::from_str(json).unwrap();

        assert_eq!(agent.id, "restricted-agent");
        assert_eq!(agent.name, "Restricted Agent");
        assert_eq!(
            agent.allowed_tools,
            Some(vec![
                "Read".to_string(),
                "Grep".to_string(),
                "Glob".to_string()
            ])
        );
    }

    #[test]
    fn test_agent_definition_allowed_tools_default_none() {
        let json = r#"{
            "id": "basic-agent",
            "name": "Basic Agent",
            "description": "Basic agent",
            "path": "agents/basic.md"
        }"#;

        let agent: AgentDefinition = serde_json::from_str(json).unwrap();

        // allowedTools should default to None when not specified
        assert!(agent.allowed_tools.is_none());
    }

    #[test]
    fn test_agent_definition_serialization_skips_none_allowed_tools() {
        let agent = AgentDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test agent".to_string(),
            path: "test.md".to_string(),
            model: None,
            disallowed_tools: vec![],
            allowed_tools: None,
        };

        let json = serde_json::to_string(&agent).unwrap();
        // None allowed_tools should not appear in serialized JSON
        assert!(!json.contains("allowedTools"));
    }

    #[test]
    fn test_agent_definition_serialization_includes_non_empty_allowed_tools() {
        let agent = AgentDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test agent".to_string(),
            path: "test.md".to_string(),
            model: None,
            disallowed_tools: vec![],
            allowed_tools: Some(vec!["Read".to_string(), "Grep".to_string()]),
        };

        let json = serde_json::to_string(&agent).unwrap();
        // Non-empty allowed_tools should appear in serialized JSON
        assert!(json.contains("allowedTools"));
        assert!(json.contains("Read"));
        assert!(json.contains("Grep"));
    }

    #[test]
    fn test_agent_definition_with_both_allowed_and_disallowed_tools() {
        let json = r#"{
            "id": "complex-agent",
            "name": "Complex Agent",
            "description": "Agent with both allowed and disallowed tools",
            "path": "agents/complex.md",
            "allowedTools": ["Read", "Write", "Bash"],
            "disallowedTools": ["Bash"]
        }"#;

        let agent: AgentDefinition = serde_json::from_str(json).unwrap();

        assert_eq!(agent.id, "complex-agent");
        // Both fields should be preserved (filtering happens at execution time)
        assert_eq!(
            agent.allowed_tools,
            Some(vec![
                "Read".to_string(),
                "Write".to_string(),
                "Bash".to_string()
            ])
        );
        assert_eq!(agent.disallowed_tools, vec!["Bash".to_string()]);
    }
}
