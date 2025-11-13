//! Agent Discovery - Auto-discovers agents from .claude/agents/ directory
//!
//! Scans the .claude/agents/ directory for agent definition files and makes them
//! available to the plugin system.

use std::fs;
use std::path::{Path, PathBuf};

use crate::plugins::manifest::AgentDefinition;

/// Agent discovery from filesystem
pub struct AgentDiscovery {
    agents_dir: PathBuf,
}

impl AgentDiscovery {
    /// Create new agent discovery for a directory
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            agents_dir: project_root.as_ref().join(".claude").join("agents"),
        }
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
            model: None, // Use default model
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
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_agents_dir() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let agents_dir = temp_dir.path().join(".claude").join("agents");
        fs::create_dir_all(&agents_dir).unwrap();
        temp_dir
    }

    #[test]
    fn test_discover_empty_directory() {
        let temp_dir = setup_test_agents_dir();
        let discovery = AgentDiscovery::new(temp_dir.path());

        let agents = discovery.discover_all().unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[test]
    fn test_discover_single_agent() {
        let temp_dir = setup_test_agents_dir();
        let agents_dir = temp_dir.path().join(".claude").join("agents");

        fs::write(
            agents_dir.join("builder.md"),
            "# Builder Agent\n\nBuilds code from specifications.",
        )
        .unwrap();

        let discovery = AgentDiscovery::new(temp_dir.path());
        let agents = discovery.discover_all().unwrap();

        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, "builder");
        assert_eq!(agents[0].name, "Builder Agent");
        assert_eq!(agents[0].description, "Builds code from specifications.");
    }

    #[test]
    fn test_discover_multiple_agents() {
        let temp_dir = setup_test_agents_dir();
        let agents_dir = temp_dir.path().join(".claude").join("agents");

        fs::write(
            agents_dir.join("builder.md"),
            "# Builder\n\nBuilds things.",
        )
        .unwrap();

        fs::write(agents_dir.join("tester.md"), "# Tester\n\nTests things.").unwrap();

        fs::write(agents_dir.join("reviewer.md"), "# Reviewer\n\nReviews things.").unwrap();

        let discovery = AgentDiscovery::new(temp_dir.path());
        let agents = discovery.discover_all().unwrap();

        assert_eq!(agents.len(), 3);

        let ids: Vec<_> = agents.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"builder"));
        assert!(ids.contains(&"tester"));
        assert!(ids.contains(&"reviewer"));
    }

    #[test]
    fn test_extract_metadata_with_heading() {
        let content = "# My Agent\n\nThis is a test agent.\n\nMore content here.";
        let (name, desc) = AgentDiscovery::extract_metadata(content, "fallback");

        assert_eq!(name, "My Agent");
        assert_eq!(desc, "This is a test agent.");
    }

    #[test]
    fn test_extract_metadata_without_heading() {
        let content = "Just some content without a heading.";
        let (name, desc) = AgentDiscovery::extract_metadata(content, "fallback");

        assert_eq!(name, "fallback");
        assert_eq!(desc, "Agent: fallback");
    }

    #[test]
    fn test_has_agent() {
        let temp_dir = setup_test_agents_dir();
        let agents_dir = temp_dir.path().join(".claude").join("agents");

        fs::write(agents_dir.join("builder.md"), "# Builder\n\nBuilds.").unwrap();

        let discovery = AgentDiscovery::new(temp_dir.path());

        assert!(discovery.has_agent("builder"));
        assert!(!discovery.has_agent("nonexistent"));
    }

    #[test]
    fn test_get_agent() {
        let temp_dir = setup_test_agents_dir();
        let agents_dir = temp_dir.path().join(".claude").join("agents");

        fs::write(
            agents_dir.join("builder.md"),
            "# Builder Agent\n\nBuilds code.",
        )
        .unwrap();

        let discovery = AgentDiscovery::new(temp_dir.path());

        let agent = discovery.get_agent("builder").unwrap().unwrap();
        assert_eq!(agent.id, "builder");
        assert_eq!(agent.name, "Builder Agent");

        assert!(discovery.get_agent("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_list_agent_ids() {
        let temp_dir = setup_test_agents_dir();
        let agents_dir = temp_dir.path().join(".claude").join("agents");

        fs::write(agents_dir.join("a.md"), "# A\n\nAgent A.").unwrap();
        fs::write(agents_dir.join("b.md"), "# B\n\nAgent B.").unwrap();
        fs::write(agents_dir.join("c.md"), "# C\n\nAgent C.").unwrap();

        let discovery = AgentDiscovery::new(temp_dir.path());
        let ids = discovery.list_agent_ids().unwrap();

        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"a".to_string()));
        assert!(ids.contains(&"b".to_string()));
        assert!(ids.contains(&"c".to_string()));
    }
}
