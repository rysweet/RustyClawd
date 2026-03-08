//! Agent frontmatter parsing - extracts metadata from agent definition YAML headers.

use crate::agent_memory::MemoryScope;

/// Agent isolation mode parsed from frontmatter.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentIsolation {
    /// Run the agent in an isolated git worktree
    Worktree,
}

/// Metadata extracted from agent definition frontmatter (YAML between `---` delimiters).
///
/// Example agent.md:
/// ```markdown
/// ---
/// background: true
/// memory: project
/// isolation: worktree
/// ---
/// # Agent Name
/// You are a specialized agent...
/// ```
#[derive(Debug, Clone, Default)]
pub struct AgentFrontmatter {
    /// If true, this agent should always run in the background
    pub background: bool,
    /// Memory scope for this agent's memory operations (user, project, local)
    pub memory_scope: Option<MemoryScope>,
    /// Isolation mode for agent execution
    pub isolation: Option<AgentIsolation>,
}

impl AgentFrontmatter {
    /// Parse frontmatter from agent markdown content.
    ///
    /// Looks for YAML frontmatter between `---` delimiters at the start of the file.
    /// Returns the parsed frontmatter and the remaining content (the system prompt).
    pub fn parse(content: &str) -> (Self, String) {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return (Self::default(), content.to_string());
        }

        // Find the closing `---`
        let after_first = &trimmed[3..];
        let closing = after_first.find("---");
        match closing {
            Some(end_pos) => {
                let yaml_block = &after_first[..end_pos];
                let rest = &after_first[end_pos + 3..];

                let mut frontmatter = Self::default();

                // Simple line-by-line key: value parsing (avoids YAML dependency)
                for line in yaml_block.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once(':') {
                        let key = key.trim();
                        let value = value.trim();
                        match key {
                            "background" => {
                                frontmatter.background = value == "true";
                            }
                            "memory" | "memory_scope" => {
                                frontmatter.memory_scope = match value {
                                    "user" => Some(MemoryScope::User),
                                    "project" => Some(MemoryScope::Project),
                                    "local" => Some(MemoryScope::Local),
                                    _ => None,
                                };
                            }
                            "isolation" => {
                                frontmatter.isolation = match value {
                                    "worktree" => Some(AgentIsolation::Worktree),
                                    _ => None,
                                };
                            }
                            _ => {
                                // Ignore unknown frontmatter keys
                            }
                        }
                    }
                }

                (frontmatter, rest.to_string())
            }
            None => {
                // No closing `---`, treat entire content as prompt
                (Self::default(), content.to_string())
            }
        }
    }
}
