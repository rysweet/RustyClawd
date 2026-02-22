//! Agent Discovery - Auto-discovers agents from .claude/agents/ directory
//!
//! Scans the .claude/agents/ directory for agent definition files and makes them
//! available to the plugin system. Also supports runtime agent definitions via
//! the `--agents` CLI flag.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::plugins::manifest::AgentDefinition;

/// Runtime agent definition from --agents CLI JSON flag
///
/// Format: `--agents '{"name": {"description":"...", "prompt":"...", "tools":["Read"], "allowedTools":["Read","Grep"], "disallowedTools":["Bash"], "model":"sonnet"}}'`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeAgentDefinition {
    /// Agent description
    pub description: String,
    /// System prompt for the agent
    pub prompt: String,
    /// Tools available to this agent (e.g., ["Read", "Write", "Bash"])
    #[serde(default)]
    pub tools: Vec<String>,
    /// Tools that are explicitly allowed for this agent (empty means all tools allowed)
    #[serde(default, rename = "allowedTools")]
    pub allowed_tools: Vec<String>,
    /// Tools that are explicitly blocked for this agent
    #[serde(default, rename = "disallowedTools")]
    pub disallowed_tools: Vec<String>,
    /// Model to use (e.g., "sonnet", "opus", "haiku", or full model ID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Parse runtime agents from JSON string
///
/// Expected format: `{"agent_name": {"description": "...", "prompt": "...", "tools": [...], "model": "..."}}`
///
/// # Errors
/// Returns error if JSON is malformed or doesn't match expected schema
pub fn parse_runtime_agents(
    json_str: &str,
) -> Result<HashMap<String, RuntimeAgentDefinition>, String> {
    serde_json::from_str(json_str).map_err(|e| format!("Invalid agents JSON: {}", e))
}

/// Validate runtime agent definitions
///
/// Checks:
/// - Agent names are non-empty
/// - Descriptions are non-empty
/// - Prompts are non-empty
pub fn validate_runtime_agents(
    agents: &HashMap<String, RuntimeAgentDefinition>,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    for (name, agent) in agents {
        if name.is_empty() {
            errors.push("Agent name cannot be empty".to_string());
        }
        if agent.description.is_empty() {
            errors.push(format!("Agent '{}' has empty description", name));
        }
        if agent.prompt.is_empty() {
            errors.push(format!("Agent '{}' has empty prompt", name));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Agent discovery from filesystem with optional runtime agents
pub struct AgentDiscovery {
    agents_dir: PathBuf,
    /// Runtime agents defined via --agents CLI flag
    runtime_agents: HashMap<String, RuntimeAgentDefinition>,
}

impl AgentDiscovery {
    /// Create new agent discovery for a directory
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            agents_dir: project_root.as_ref().join(".claude").join("agents"),
            runtime_agents: HashMap::new(),
        }
    }

    /// Add runtime agents from --agents CLI flag
    ///
    /// Runtime agents take precedence over file-based agents with the same ID.
    pub fn with_runtime_agents(mut self, agents: HashMap<String, RuntimeAgentDefinition>) -> Self {
        self.runtime_agents = agents;
        self
    }

    /// Add a single runtime agent
    pub fn add_runtime_agent(&mut self, id: String, agent: RuntimeAgentDefinition) {
        self.runtime_agents.insert(id, agent);
    }

    /// Get runtime agent by ID
    pub fn get_runtime_agent(&self, agent_id: &str) -> Option<&RuntimeAgentDefinition> {
        self.runtime_agents.get(agent_id)
    }

    /// List all runtime agent IDs
    pub fn runtime_agent_ids(&self) -> Vec<String> {
        self.runtime_agents.keys().cloned().collect()
    }

    /// Get all agents (both file-based and runtime)
    ///
    /// Returns AgentDefinition for file-based agents.
    /// Runtime agents have their prompt stored in the path field for compatibility.
    pub fn all_agents(&self) -> Result<Vec<AgentDefinition>, String> {
        let mut agents = self.discover_all()?;

        // Convert runtime agents to AgentDefinition format
        // Runtime agents use a special path format: "runtime:<agent_id>"
        for (id, runtime_agent) in &self.runtime_agents {
            agents.push(AgentDefinition {
                id: id.clone(),
                name: id.clone(), // Use ID as name for runtime agents
                description: runtime_agent.description.clone(),
                path: format!("runtime:{}", id), // Special marker for runtime agents
                model: runtime_agent.model.clone(),
                allowed_tools: if runtime_agent.allowed_tools.is_empty() {
                    None
                } else {
                    Some(runtime_agent.allowed_tools.clone())
                },
                disallowed_tools: runtime_agent.disallowed_tools.clone(),
            });
        }

        Ok(agents)
    }

    /// Check if agent is a runtime agent
    pub fn is_runtime_agent(&self, agent_id: &str) -> bool {
        self.runtime_agents.contains_key(agent_id)
    }

    /// Discover all agents in the .claude/agents/ directory
    ///
    /// Scans for .md files and creates AgentDefinition for each.
    /// Agent ID is derived from filename (e.g., "builder.md" -> "builder")
    pub fn discover_all(&self) -> Result<Vec<AgentDefinition>, String> {
        if !self.agents_dir.exists() {
            return Ok(Vec::new());
        }

        let mut agents = Vec::new();

        for entry in fs::read_dir(&self.agents_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(agent) = self.load_agent_from_file(&path)? {
                    agents.push(agent);
                }
            }
        }

        Ok(agents)
    }

    /// Load agent definition from a markdown file
    fn load_agent_from_file(&self, path: &Path) -> Result<Option<AgentDefinition>, String> {
        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "Invalid filename".to_string())?;

        // Read first few lines to extract name and description
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let (name, description) = Self::extract_metadata(&content, filename);

        // Get relative path from .claude/agents/
        let relative_path = path
            .strip_prefix(self.agents_dir.parent().unwrap())
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .to_string();

        Ok(Some(AgentDefinition {
            id: filename.to_string(),
            name,
            description,
            path: relative_path,
            model: None,              // Use default model
            allowed_tools: None, // No allowed tools restriction by default for file-based agents
            disallowed_tools: vec![], // No disallowed tools by default for file-based agents
        }))
    }

    /// Extract name and description from agent file content
    ///
    /// Looks for:
    /// - First H1 heading (# Name) as name
    /// - First paragraph after heading as description
    fn extract_metadata(content: &str, fallback_name: &str) -> (String, String) {
        let lines: Vec<&str> = content.lines().collect();
        let mut name = fallback_name.to_string();
        let mut description = String::new();

        let mut found_heading = false;
        for line in lines {
            let trimmed = line.trim();

            // Look for H1 heading
            if trimmed.starts_with("# ") && !found_heading {
                name = trimmed.trim_start_matches("# ").trim().to_string();
                found_heading = true;
                continue;
            }

            // After finding heading, look for first non-empty line as description
            if found_heading && !trimmed.is_empty() && !trimmed.starts_with('#') {
                description = trimmed.to_string();
                break;
            }
        }

        // If no description found, use a default
        if description.is_empty() {
            description = format!("Agent: {}", name);
        }

        (name, description)
    }

    /// Get agent definition by ID
    pub fn get_agent(&self, agent_id: &str) -> Result<Option<AgentDefinition>, String> {
        let agent_path = self.agents_dir.join(format!("{}.md", agent_id));

        if !agent_path.exists() {
            return Ok(None);
        }

        self.load_agent_from_file(&agent_path)
    }

    /// Check if an agent exists
    pub fn has_agent(&self, agent_id: &str) -> bool {
        self.agents_dir.join(format!("{}.md", agent_id)).exists()
    }

    /// List all agent IDs
    pub fn list_agent_ids(&self) -> Result<Vec<String>, String> {
        let agents = self.discover_all()?;
        Ok(agents.into_iter().map(|a| a.id).collect())
    }
}

#[cfg(test)]
#[path = "agent_discovery_tests.rs"]
mod tests;
