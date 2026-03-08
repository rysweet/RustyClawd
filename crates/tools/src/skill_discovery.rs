//! Shared skill discovery and parsing logic
//!
//! This module contains the common path discovery and file parsing functions
//! used by both `skill.rs` (SkillTool) and `command_skill.rs` (CommandSkillTool).
//! Eliminates duplication of ~120 LOC between those two modules.

use std::path::{Path, PathBuf};

/// Result of parsing a skill/command file.
///
/// Uses `serde_yaml::Value` for metadata so consumers can deserialize
/// into their own specific metadata types (e.g., `SkillMetadata` or
/// `CommandSkillMetadata`).
pub struct ParsedFile {
    /// The prompt/instruction content (frontmatter stripped for markdown files)
    pub prompt: String,
    /// Raw YAML metadata value, if frontmatter was found
    pub metadata: Option<serde_yaml::Value>,
}

/// Discover all possible paths where a skill might be located.
///
/// Checks project-level (.claude/skills/), user-level (~/.claude/skills/),
/// plugin-specific, and example plugin directories.
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
    paths.push(PathBuf::from(format!(".claude/skills/{}/SKILL.md", skill)));
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

/// Find and load a skill file by searching through candidate paths.
///
/// Iterates over `skill_paths`, reads the first file that exists and has
/// a non-empty prompt, and returns the prompt, the path where it was found,
/// and the raw YAML metadata (if any).
///
/// This is the shared implementation used by both `SkillTool` and
/// `CommandSkillTool` to avoid duplicated file-loading loops.
pub async fn find_skill_content(
    skill_paths: &[PathBuf],
) -> Option<(String, PathBuf, Option<serde_yaml::Value>)> {
    for path in skill_paths {
        if !path.exists() {
            continue;
        }

        match tokio::fs::read_to_string(path).await {
            Ok(content) => {
                let parsed = parse_file(&content, path);

                if !parsed.prompt.is_empty() {
                    return Some((parsed.prompt, path.clone(), parsed.metadata));
                }
            }
            Err(e) => {
                tracing::warn!(path = ?path, error = %e, "Failed to read skill file");
                continue;
            }
        }
    }

    None
}

/// Parse a file and extract prompt and metadata based on file extension.
///
/// Dispatches to markdown or YAML parsing as appropriate.
pub fn parse_file(content: &str, path: &Path) -> ParsedFile {
    let extension = path.extension().and_then(|s| s.to_str());

    match extension {
        Some("md") => parse_markdown_file(content),
        Some("yaml") | Some("yml") => parse_yaml_file(content),
        _ => ParsedFile {
            prompt: content.to_string(),
            metadata: None,
        },
    }
}

/// Parse a markdown file with optional YAML frontmatter.
///
/// Strips the `---` delimited frontmatter block and returns the body as
/// the prompt. The frontmatter YAML is returned as a raw `serde_yaml::Value`.
pub fn parse_markdown_file(content: &str) -> ParsedFile {
    if let Some(stripped) = content.strip_prefix("---") {
        // Has YAML frontmatter
        if let Some(end_idx) = stripped.find("---") {
            let frontmatter = &stripped[..end_idx];
            let prompt = stripped[end_idx + 3..].trim().to_string();

            // Parse frontmatter as raw YAML value
            let metadata = serde_yaml::from_str::<serde_yaml::Value>(frontmatter).ok();

            return ParsedFile { prompt, metadata };
        }
    }

    // No frontmatter, use content as-is
    ParsedFile {
        prompt: content.to_string(),
        metadata: None,
    }
}

/// Parse a YAML file, extracting the prompt from known fields.
///
/// Looks for `prompt`, `instructions`, or `content` fields for the prompt text.
/// Returns the full YAML value as metadata.
pub fn parse_yaml_file(content: &str) -> ParsedFile {
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

        ParsedFile {
            prompt,
            metadata: Some(yaml_value),
        }
    } else {
        // Failed to parse YAML, use as-is
        ParsedFile {
            prompt: content.to_string(),
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_skill_paths_basic() {
        let paths = discover_skill_paths("my-skill");

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

        // Should have multiple paths
        assert!(paths.len() >= 8);
    }

    #[test]
    fn test_discover_skill_paths_with_plugin() {
        let paths = discover_skill_paths("my-plugin:my-skill");
        assert!(paths.iter().any(|p| p
            .to_str()
            .unwrap()
            .contains(".claude/plugins/my-plugin/skills/my-skill")));
    }

    #[test]
    fn test_parse_markdown_no_frontmatter() {
        let content = "# Simple Content\n\nJust text.";
        let parsed = parse_markdown_file(content);
        assert_eq!(parsed.prompt, content);
        assert!(parsed.metadata.is_none());
    }

    #[test]
    fn test_parse_markdown_with_frontmatter() {
        let content = r#"---
description: Test
version: "1.0"
---

Skill content here."#;
        let parsed = parse_markdown_file(content);

        assert!(parsed.prompt.contains("Skill content"));
        assert!(!parsed.prompt.contains("---"));
        assert!(parsed.metadata.is_some());

        let meta = parsed.metadata.unwrap();
        assert_eq!(
            meta.get("description").and_then(|v| v.as_str()),
            Some("Test")
        );
    }

    #[test]
    fn test_parse_yaml_file_with_prompt() {
        let content = r#"
description: YAML Skill
prompt: This is the prompt content
version: 1.0.0
"#;
        let parsed = parse_yaml_file(content);

        assert!(parsed.prompt.contains("prompt content"));
        assert!(parsed.metadata.is_some());
    }

    #[test]
    fn test_parse_yaml_file_with_instructions() {
        let content = r#"
description: YAML Skill
instructions: Follow these instructions
"#;
        let parsed = parse_yaml_file(content);
        assert!(parsed.prompt.contains("Follow these instructions"));
    }

    #[test]
    fn test_parse_yaml_file_no_known_field() {
        let content = r#"
description: YAML Skill
other_field: something
"#;
        let parsed = parse_yaml_file(content);
        // Falls back to raw content
        assert_eq!(parsed.prompt, content);
    }

    #[test]
    fn test_parse_file_dispatches_by_extension() {
        let md_path = Path::new("test.md");
        let yaml_path = Path::new("test.yaml");
        let yml_path = Path::new("test.yml");
        let txt_path = Path::new("test.txt");

        let md_content = "---\ndescription: Test\n---\nPrompt here.";
        let yaml_content = "prompt: YAML prompt";

        let parsed_md = parse_file(md_content, md_path);
        assert!(parsed_md.prompt.contains("Prompt here"));

        let parsed_yaml = parse_file(yaml_content, yaml_path);
        assert!(parsed_yaml.prompt.contains("YAML prompt"));

        let parsed_yml = parse_file(yaml_content, yml_path);
        assert!(parsed_yml.prompt.contains("YAML prompt"));

        let parsed_txt = parse_file("raw text", txt_path);
        assert_eq!(parsed_txt.prompt, "raw text");
        assert!(parsed_txt.metadata.is_none());
    }

    #[test]
    fn test_discover_skill_paths_includes_uppercase_skill_md() {
        let paths = discover_skill_paths("my-skill");
        assert!(paths.iter().any(|p| p
            .to_str()
            .unwrap()
            .contains(".claude/skills/my-skill/SKILL.md")));
    }
}
