//! Skill tool - Load and execute skills
//!
//! Demonstrates:
//! - Dynamic skill loading
//! - Skill registry management
//! - Multiple skill file format support (markdown, YAML)
//! - Skill discovery from filesystem

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

/// Parameters for Skill tool
#[derive(Debug, Deserialize)]
pub struct SkillParams {
    /// Skill name to execute
    pub skill: String,
}

/// Output from Skill tool
#[derive(Debug, Serialize)]
pub struct SkillOutput {
    /// Skill that was loaded
    pub skill: String,

    /// Skill prompt/instructions
    pub prompt: String,

    /// Whether skill was found
    pub found: bool,

    /// Path where skill was found (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Skill metadata (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SkillMetadata>,
}

/// Skill metadata extracted from frontmatter
#[derive(Debug, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Skill description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Skill version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Skill author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Skill location type (managed, project, user)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// The Skill tool
pub struct SkillTool;

#[async_trait]
impl crate::Tool for SkillTool {
    type Params = SkillParams;
    type Output = SkillOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "Skill",
            description: "Loads and executes skills from the skill registry",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let skill = params.skill.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Loading skill: {}", skill),
                percentage: Some(25.0),
            };

            // Discover all possible skill locations
            let skill_paths = discover_skill_paths(&skill);

            if debug {
                debug!(skill = %skill, path_count = skill_paths.len(), "Searching for skill in {} locations", skill_paths.len());
            }

            yield ToolEvent::Progress {
                step: "Searching skill locations...".to_string(),
                percentage: Some(50.0),
            };

            let mut found = false;
            let mut prompt = String::new();
            let mut found_path: Option<String> = None;
            let mut metadata: Option<SkillMetadata> = None;

            // Try each path
            for path in &skill_paths {
                if !path.exists() {
                    continue;
                }

                match fs::read_to_string(&path).await {
                    Ok(content) => {
                        if debug {
                            debug!(path = ?path, content_len = content.len(), "Found skill file");
                        }

                        // Parse the skill file
                        let parsed = parse_skill_file(&content, path);

                        if !parsed.prompt.is_empty() {
                            found = true;
                            prompt = parsed.prompt;
                            found_path = Some(path.display().to_string());
                            metadata = parsed.metadata;

                            if debug {
                                debug!(
                                    skill = %skill,
                                    path = ?path,
                                    prompt_len = prompt.len(),
                                    has_metadata = metadata.is_some(),
                                    "Skill loaded successfully"
                                );
                            }
                            break;
                        }
                    }
                    Err(e) => {
                        if debug {
                            warn!(path = ?path, error = %e, "Failed to read skill file");
                        }
                        continue;
                    }
                }
            }

            yield ToolEvent::Progress {
                step: if found { "Skill loaded successfully" } else { "Skill not found" }.to_string(),
                percentage: Some(100.0),
            };

            if !found {
                prompt = format!(
                    "Skill '{}' not found. Searched in the following locations:\n{}",
                    skill,
                    skill_paths.iter()
                        .map(|p| format!("  - {}", p.display()))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }

            if debug {
                debug!(
                    skill = %skill,
                    found = found,
                    path = ?found_path,
                    "Skill loading complete"
                );
            }

            yield ToolEvent::Result(SkillOutput {
                skill: params.skill.clone(),
                prompt,
                found,
                path: found_path,
                metadata,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true // Loading skills doesn't modify state
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
}

/// Parsed skill file data
struct ParsedSkill {
    prompt: String,
    metadata: Option<SkillMetadata>,
}

/// Load skill content by name, searching all standard skill locations.
///
/// Returns `Some(content)` if a skill with the given name is found, `None` otherwise.
/// The returned content has YAML frontmatter stripped (only the prompt body is returned).
///
/// This is the public entry point used by the slash command system to invoke skills
/// as `/skill-name` commands.
pub async fn load_skill_content(skill_name: &str) -> Option<String> {
    let paths = discover_skill_paths(skill_name);

    for path in &paths {
        if !path.exists() {
            continue;
        }

        match fs::read_to_string(path).await {
            Ok(content) => {
                let parsed = parse_skill_file(&content, path);
                if !parsed.prompt.is_empty() {
                    return Some(parsed.prompt);
                }
            }
            Err(e) => {
                warn!(path = ?path, error = %e, "Failed to read skill file");
            }
        }
    }

    None
}

/// List all skill names available in the standard skill directories.
///
/// Scans `.claude/skills/` (project-level) and `~/.claude/skills/` (user-level)
/// and returns the names of all skills found. Each name corresponds to a directory
/// or file stem that can be invoked as `/skill-name`.
pub async fn list_available_skills() -> Vec<String> {
    let mut skill_names = std::collections::BTreeSet::new();

    let mut dirs_to_scan: Vec<PathBuf> = vec![PathBuf::from(".claude/skills")];

    if let Some(home) = std::env::var_os("HOME") {
        dirs_to_scan.push(PathBuf::from(home).join(".claude/skills"));
    }

    for dir in dirs_to_scan {
        if let Ok(mut entries) = fs::read_dir(&dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();

                if path.is_file() {
                    // e.g. .claude/skills/review.md -> "review"
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if path
                            .extension()
                            .map(|e| e == "md" || e == "yaml" || e == "yml")
                            .unwrap_or(false)
                        {
                            skill_names.insert(stem.to_string());
                        }
                    }
                } else if path.is_dir() {
                    // e.g. .claude/skills/review/skill.md -> "review"
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if !dir_name.starts_with('.') {
                            // Check if it contains a skill file
                            let has_skill = path.join("skill.md").exists()
                                || path.join("skill.yaml").exists()
                                || path.join("skill.yml").exists();
                            if has_skill {
                                skill_names.insert(dir_name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    skill_names.into_iter().collect()
}

/// Discover all possible paths where a skill might be located
pub fn discover_skill_paths(skill_name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Support fully-qualified names (e.g., "plugin-name:skill-name")
    let (plugin, skill) = if skill_name.contains(':') {
        let parts: Vec<&str> = skill_name.splitn(2, ':').collect();
        (Some(parts[0]), parts[1])
    } else {
        (None, skill_name)
    };

    // Priority 1: Project-level skills in .claude/skills/
    paths.push(PathBuf::from(format!(".claude/skills/{}.md", skill)));
    paths.push(PathBuf::from(format!(".claude/skills/{}/skill.md", skill)));
    paths.push(PathBuf::from(format!(".claude/skills/{}.yaml", skill)));
    paths.push(PathBuf::from(format!(
        ".claude/skills/{}/skill.yaml",
        skill
    )));

    // Priority 2: User-level skills in ~/.claude/skills/
    if let Some(home) = std::env::var_os("HOME") {
        let home_path = PathBuf::from(home);
        paths.push(home_path.join(format!(".claude/skills/{}.md", skill)));
        paths.push(home_path.join(format!(".claude/skills/{}/skill.md", skill)));
        paths.push(home_path.join(format!(".claude/skills/{}.yaml", skill)));
        paths.push(home_path.join(format!(".claude/skills/{}/skill.yaml", skill)));
    }

    // Priority 3: Plugin-specific skills
    if let Some(plugin_name) = plugin {
        paths.push(PathBuf::from(format!(
            ".claude/plugins/{}/skills/{}.md",
            plugin_name, skill
        )));
        paths.push(PathBuf::from(format!(
            ".claude/plugins/{}/skills/{}/skill.md",
            plugin_name, skill
        )));
        paths.push(PathBuf::from(format!(
            ".claude/plugins/{}/skills/{}.yaml",
            plugin_name, skill
        )));
    }

    // Priority 4: Example plugins (for testing/development)
    paths.push(PathBuf::from(format!(
        "examples/plugins/example-plugin/skills/{}.md",
        skill
    )));
    paths.push(PathBuf::from(format!(
        "examples/plugins/example-plugin/skills/{}/skill.md",
        skill
    )));

    paths
}

/// Parse a skill file and extract prompt and metadata
fn parse_skill_file(content: &str, path: &Path) -> ParsedSkill {
    let extension = path.extension().and_then(|s| s.to_str());

    match extension {
        Some("md") => parse_markdown_skill(content),
        Some("yaml") | Some("yml") => parse_yaml_skill(content),
        _ => ParsedSkill {
            prompt: content.to_string(),
            metadata: None,
        },
    }
}

/// Parse a markdown skill file with optional YAML frontmatter
fn parse_markdown_skill(content: &str) -> ParsedSkill {
    if let Some(stripped) = content.strip_prefix("---") {
        // Has YAML frontmatter
        if let Some(end_idx) = stripped.find("---") {
            let frontmatter = &stripped[..end_idx];
            let prompt = stripped[end_idx + 3..].trim().to_string();

            // Parse frontmatter as YAML
            let metadata = serde_yaml::from_str::<SkillMetadata>(frontmatter).ok();

            return ParsedSkill { prompt, metadata };
        }
    }

    // No frontmatter, use content as-is
    ParsedSkill {
        prompt: content.to_string(),
        metadata: None,
    }
}

/// Parse a YAML skill file
fn parse_yaml_skill(content: &str) -> ParsedSkill {
    // First, try to parse as generic YAML value
    if let Ok(yaml_value) = serde_yaml::from_str::<serde_yaml::Value>(content) {
        // Extract prompt from various possible fields
        let prompt = if let Some(prompt_val) = yaml_value
            .get("prompt")
            .or_else(|| yaml_value.get("instructions"))
            .or_else(|| yaml_value.get("content"))
        {
            prompt_val.as_str().unwrap_or(content).to_string()
        } else {
            content.to_string()
        };

        // Try to parse metadata from the YAML
        let metadata = serde_yaml::from_value::<SkillMetadata>(yaml_value).ok();

        ParsedSkill { prompt, metadata }
    } else {
        // Failed to parse YAML, use as-is
        ParsedSkill {
            prompt: content.to_string(),
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;
    use std::path::PathBuf;

    /// Helper to collect result from stream
    async fn get_result(tool: &SkillTool, params: SkillParams) -> SkillOutput {
        let ctx = ToolContext::default();
        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        events
            .into_iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .expect("No result event found")
    }

    #[tokio::test]
    async fn test_skill_loading_simple_markdown() {
        // Create temporary skill file for testing
        let skill_dir = PathBuf::from(".claude/skills");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("test-simple.md");
        let test_content = "# Test Skill\n\nThis is a test skill for verifying skill loading.\n";
        let _ = fs::write(&skill_path, test_content).await;

        let tool = SkillTool;
        let params = SkillParams {
            skill: "test-simple".to_string(),
        };

        let result = get_result(&tool, params).await;

        assert_eq!(result.skill, "test-simple");
        assert!(result.found);
        assert!(result.prompt.contains("Test Skill"));
        assert!(result.path.is_some());
        assert!(result.path.unwrap().contains("test-simple.md"));

        // Clean up
        let _ = fs::remove_file(&skill_path).await;
    }

    #[tokio::test]
    async fn test_skill_with_frontmatter() {
        let skill_dir = PathBuf::from(".claude/skills");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("test-frontmatter.md");
        let test_content = r#"---
description: A test skill with metadata
version: 1.0.0
author: Test Author
location: project
---

# Test Skill with Frontmatter

This skill has YAML frontmatter for metadata.
"#;
        let _ = fs::write(&skill_path, test_content).await;

        let tool = SkillTool;
        let params = SkillParams {
            skill: "test-frontmatter".to_string(),
        };

        let result = get_result(&tool, params).await;

        assert!(result.found);
        assert!(result.prompt.contains("Test Skill with Frontmatter"));
        assert!(!result.prompt.contains("---")); // Frontmatter should be stripped
        assert!(result.metadata.is_some());

        let metadata = result.metadata.unwrap();
        assert_eq!(
            metadata.description,
            Some("A test skill with metadata".to_string())
        );
        assert_eq!(metadata.version, Some("1.0.0".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.location, Some("project".to_string()));

        // Clean up
        let _ = fs::remove_file(&skill_path).await;
    }

    #[tokio::test]
    async fn test_skill_yaml_format() {
        let skill_dir = PathBuf::from(".claude/skills");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("test-yaml.yaml");
        let test_content = r#"
description: A YAML skill
version: 2.0.0
prompt: |
  This is a skill defined in YAML format.
  It can have multi-line prompts.
"#;
        let _ = fs::write(&skill_path, test_content).await;

        let tool = SkillTool;
        let params = SkillParams {
            skill: "test-yaml".to_string(),
        };

        let result = get_result(&tool, params).await;

        assert!(result.found);
        assert!(result.prompt.contains("YAML format"));
        assert!(result.prompt.contains("multi-line prompts"));

        // Clean up
        let _ = fs::remove_file(&skill_path).await;
    }

    #[tokio::test]
    async fn test_skill_not_found() {
        let tool = SkillTool;
        let params = SkillParams {
            skill: "nonexistent-skill-12345".to_string(),
        };

        let result = get_result(&tool, params).await;

        assert!(!result.found);
        assert!(result.prompt.contains("not found"));
        assert!(result.prompt.contains("nonexistent-skill-12345"));
        assert!(result.path.is_none());
    }

    #[tokio::test]
    async fn test_skill_subdirectory() {
        let skill_dir = PathBuf::from(".claude/skills/test-subdir");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("skill.md");
        let test_content = "# Subdirectory Skill\n\nSkill in subdirectory.\n";
        let _ = fs::write(&skill_path, test_content).await;

        let tool = SkillTool;
        let params = SkillParams {
            skill: "test-subdir".to_string(),
        };

        let result = get_result(&tool, params).await;

        assert!(result.found);
        assert!(result.prompt.contains("Subdirectory Skill"));

        // Clean up
        let _ = fs::remove_file(&skill_path).await;
        let _ = fs::remove_dir(&skill_dir).await;
    }

    #[tokio::test]
    async fn test_skill_discovery_paths() {
        let paths = discover_skill_paths("my-skill");

        // Should include project-level paths
        assert!(paths
            .iter()
            .any(|p| p.to_str().unwrap().contains(".claude/skills/my-skill.md")));
        assert!(paths.iter().any(|p| p
            .to_str()
            .unwrap()
            .contains(".claude/skills/my-skill/skill.md")));
        assert!(paths
            .iter()
            .any(|p| p.to_str().unwrap().contains(".claude/skills/my-skill.yaml")));

        // Should include home directory paths
        assert!(paths
            .iter()
            .any(|p| p.to_str().unwrap().contains(".claude/skills/my-skill")));

        // Should have multiple paths
        assert!(paths.len() >= 8);
    }

    #[tokio::test]
    async fn test_skill_with_plugin_prefix() {
        let paths = discover_skill_paths("example-plugin:code-reviewer");

        // Should include plugin-specific paths
        assert!(paths.iter().any(|p| p
            .to_str()
            .unwrap()
            .contains(".claude/plugins/example-plugin/skills/code-reviewer")));
    }

    #[tokio::test]
    async fn test_parse_markdown_skill_no_frontmatter() {
        let content = "# Simple Skill\n\nJust content, no frontmatter.";
        let parsed = parse_markdown_skill(content);

        assert_eq!(parsed.prompt, content);
        assert!(parsed.metadata.is_none());
    }

    #[tokio::test]
    async fn test_parse_markdown_skill_with_frontmatter() {
        let content = r#"---
description: Test
version: 1.0
---

Skill content here."#;
        let parsed = parse_markdown_skill(content);

        assert!(parsed.prompt.contains("Skill content"));
        assert!(!parsed.prompt.contains("---"));
        assert!(parsed.metadata.is_some());
    }

    #[tokio::test]
    async fn test_parse_yaml_skill() {
        let content = r#"
description: YAML Skill
prompt: This is the prompt content
version: 1.0.0
"#;
        let parsed = parse_yaml_skill(content);

        assert!(parsed.prompt.contains("prompt content"));
        assert!(parsed.metadata.is_some());
    }

    #[tokio::test]
    async fn test_skill_tool_is_read_only() {
        let tool = SkillTool;
        assert!(tool.is_read_only());
    }

    #[tokio::test]
    async fn test_skill_tool_is_concurrency_safe() {
        let tool = SkillTool;
        assert!(tool.is_concurrency_safe());
    }

    #[tokio::test]
    async fn test_skill_tool_metadata() {
        let tool = SkillTool;
        let metadata = tool.metadata();

        assert_eq!(metadata.name, "Skill");
        assert!(metadata.description.contains("skill"));
    }

    #[tokio::test]
    async fn test_real_example_skill() {
        // Test with real example skills if they exist
        let example_path = PathBuf::from("examples/plugins/example-plugin/skills/code-reviewer.md");

        if example_path.exists() {
            let tool = SkillTool;
            let params = SkillParams {
                skill: "code-reviewer".to_string(),
            };

            let result = get_result(&tool, params).await;

            assert!(result.found);
            assert!(!result.prompt.is_empty());
            assert!(result.prompt.contains("code") || result.prompt.contains("review"));
        }
    }
}
