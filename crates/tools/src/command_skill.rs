//! Unified Command/Skill tool - Execute slash commands and load skills
//!
//! This module unifies the SlashCommand and Skill tools into a single
//! interface that handles both:
//! - Slash commands (e.g., "/review-pr 123") from .claude/commands/
//! - Skills (e.g., "code-reviewer") from .claude/skills/
//!
//! The unified tool automatically detects the type based on input format
//! and searches the appropriate locations.

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

/// Type of command/skill being executed
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    /// Slash command from .claude/commands/
    SlashCommand,
    /// Skill from .claude/skills/
    Skill,
}

/// Parameters for the unified Command/Skill tool
#[derive(Debug, Deserialize)]
pub struct CommandSkillParams {
    /// The command or skill to execute
    /// - For commands: "/command-name args..." or just "command-name args..."
    /// - For skills: "skill-name" or "plugin:skill-name"
    pub input: String,

    /// Optional: Force interpretation as skill (skip command lookup)
    #[serde(default)]
    pub force_skill: bool,

    /// Optional: Force interpretation as command (skip skill lookup)
    #[serde(default)]
    pub force_command: bool,
}

/// Metadata extracted from command/skill files
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CommandSkillMetadata {
    /// Description of the command/skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Location type (managed, project, user)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// Arguments specification (for commands)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// Output from the unified Command/Skill tool
#[derive(Debug, Serialize)]
pub struct CommandSkillOutput {
    /// Original input
    pub input: String,

    /// Type of resource that was loaded
    pub command_type: CommandType,

    /// Name of the command/skill (without leading slash or arguments)
    pub name: String,

    /// Arguments passed (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,

    /// The expanded prompt/instructions
    pub prompt: String,

    /// Whether the resource was found
    pub found: bool,

    /// Path where resource was found (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Metadata (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<CommandSkillMetadata>,
}

/// The unified Command/Skill tool
pub struct CommandSkillTool;

#[async_trait]
impl crate::Tool for CommandSkillTool {
    type Params = CommandSkillParams;
    type Output = CommandSkillOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "CommandSkill",
            description: "Unified tool for executing slash commands and loading skills",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let input = params.input.clone();
        let force_skill = params.force_skill;
        let force_command = params.force_command;
        let debug_mode = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Processing: {}", input),
                percentage: Some(10.0),
            };

            // Parse input to determine type
            let (name, args, is_slash_command) = parse_input(&input);

            if debug_mode {
                debug!(
                    input = %input,
                    name = %name,
                    args = ?args,
                    is_slash_command = is_slash_command,
                    force_skill = force_skill,
                    force_command = force_command,
                    "Parsed input"
                );
            }

            let mut found = false;
            let mut prompt = String::new();
            let mut found_path: Option<String> = None;
            let mut metadata: Option<CommandSkillMetadata> = None;
            let mut command_type = if is_slash_command || force_command {
                CommandType::SlashCommand
            } else {
                CommandType::Skill
            };

            // Determine search order based on input format and force flags
            let search_commands_first = (is_slash_command || force_command) && !force_skill;

            yield ToolEvent::Progress {
                step: "Searching for resource...".to_string(),
                percentage: Some(30.0),
            };

            // Try commands first if it looks like a command
            if search_commands_first {
                let command_paths = discover_command_paths(&name);

                if debug_mode {
                    debug!(
                        name = %name,
                        path_count = command_paths.len(),
                        "Searching command locations"
                    );
                }

                for path in &command_paths {
                    if !path.exists() {
                        continue;
                    }

                    match fs::read_to_string(&path).await {
                        Ok(content) => {
                            let parsed = parse_file(&content, path);

                            if !parsed.prompt.is_empty() {
                                found = true;
                                command_type = CommandType::SlashCommand;

                                // Apply argument substitution for commands
                                prompt = apply_argument_substitution(&parsed.prompt, args.as_deref());
                                found_path = Some(path.display().to_string());
                                metadata = parsed.metadata;

                                if debug_mode {
                                    debug!(
                                        name = %name,
                                        path = ?path,
                                        "Command loaded successfully"
                                    );
                                }
                                break;
                            }
                        }
                        Err(e) => {
                            if debug_mode {
                                warn!(path = ?path, error = %e, "Failed to read command file");
                            }
                        }
                    }
                }
            }

            // Try skills if not found as command (or if force_skill)
            if !found && !force_command {
                yield ToolEvent::Progress {
                    step: "Searching skill locations...".to_string(),
                    percentage: Some(60.0),
                };

                let skill_paths = discover_skill_paths(&name);

                if debug_mode {
                    debug!(
                        name = %name,
                        path_count = skill_paths.len(),
                        "Searching skill locations"
                    );
                }

                for path in &skill_paths {
                    if !path.exists() {
                        continue;
                    }

                    match fs::read_to_string(&path).await {
                        Ok(content) => {
                            let parsed = parse_file(&content, path);

                            if !parsed.prompt.is_empty() {
                                found = true;
                                command_type = CommandType::Skill;
                                prompt = parsed.prompt;
                                found_path = Some(path.display().to_string());
                                metadata = parsed.metadata;

                                if debug_mode {
                                    debug!(
                                        name = %name,
                                        path = ?path,
                                        "Skill loaded successfully"
                                    );
                                }
                                break;
                            }
                        }
                        Err(e) => {
                            if debug_mode {
                                warn!(path = ?path, error = %e, "Failed to read skill file");
                            }
                        }
                    }
                }
            }

            yield ToolEvent::Progress {
                step: if found { "Resource loaded successfully" } else { "Resource not found" }.to_string(),
                percentage: Some(100.0),
            };

            // Build "not found" message if needed
            if !found {
                let mut searched_paths = Vec::new();

                if search_commands_first || !force_skill {
                    searched_paths.extend(
                        discover_command_paths(&name)
                            .iter()
                            .map(|p| format!("  - {} (command)", p.display()))
                    );
                }

                if !force_command {
                    searched_paths.extend(
                        discover_skill_paths(&name)
                            .iter()
                            .map(|p| format!("  - {} (skill)", p.display()))
                    );
                }

                prompt = format!(
                    "Command/Skill '{}' not found. Searched in the following locations:\n{}",
                    name,
                    searched_paths.join("\n")
                );
            }

            yield ToolEvent::Result(CommandSkillOutput {
                input: params.input.clone(),
                command_type,
                name: name.to_string(),
                args,
                prompt,
                found,
                path: found_path,
                metadata,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
}

/// Parsed file data
struct ParsedFile {
    prompt: String,
    metadata: Option<CommandSkillMetadata>,
}

/// Parse input to extract name, args, and whether it's a slash command
fn parse_input(input: &str) -> (String, Option<String>, bool) {
    let trimmed = input.trim();

    // Check if it starts with slash (explicit command)
    let is_slash = trimmed.starts_with('/');
    let without_slash = trimmed.trim_start_matches('/');

    // Split into name and args
    let parts: Vec<&str> = without_slash.splitn(2, ' ').collect();
    let name = parts[0].to_string();
    let args = parts.get(1).map(|s| s.to_string());

    (name, args, is_slash)
}

/// Discover command paths for a given name
fn discover_command_paths(name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Support namespaced commands (e.g., "amplihack:analyze")
    let (namespace, cmd_name) = if name.contains(':') {
        let parts: Vec<&str> = name.splitn(2, ':').collect();
        (Some(parts[0]), parts[1])
    } else {
        (None, name)
    };

    // Priority 1: Project-level commands
    if let Some(ns) = namespace {
        paths.push(PathBuf::from(format!(".claude/commands/{}/{}.md", ns, cmd_name)));
    }
    paths.push(PathBuf::from(format!(".claude/commands/{}.md", name)));

    // Priority 2: User-level commands
    if let Some(home) = std::env::var_os("HOME") {
        let home_path = PathBuf::from(home);
        if let Some(ns) = namespace {
            paths.push(home_path.join(format!(".claude/commands/{}/{}.md", ns, cmd_name)));
        }
        paths.push(home_path.join(format!(".claude/commands/{}.md", name)));
    }

    paths
}

/// Discover skill paths for a given name
fn discover_skill_paths(skill_name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Support fully-qualified names (e.g., "plugin-name:skill-name")
    let (plugin, skill) = if skill_name.contains(':') {
        let parts: Vec<&str> = skill_name.splitn(2, ':').collect();
        (Some(parts[0]), parts[1])
    } else {
        (None, skill_name)
    };

    // Priority 1: Project-level skills
    paths.push(PathBuf::from(format!(".claude/skills/{}.md", skill)));
    paths.push(PathBuf::from(format!(".claude/skills/{}/skill.md", skill)));
    paths.push(PathBuf::from(format!(".claude/skills/{}/SKILL.md", skill)));
    paths.push(PathBuf::from(format!(".claude/skills/{}.yaml", skill)));
    paths.push(PathBuf::from(format!(".claude/skills/{}/skill.yaml", skill)));

    // Priority 2: User-level skills
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

    // Priority 4: Example plugins (for development)
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

/// Parse a file and extract prompt and metadata
fn parse_file(content: &str, path: &Path) -> ParsedFile {
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

/// Parse a markdown file with optional YAML frontmatter
fn parse_markdown_file(content: &str) -> ParsedFile {
    if let Some(stripped) = content.strip_prefix("---") {
        // Has YAML frontmatter
        if let Some(end_idx) = stripped.find("---") {
            let frontmatter = &stripped[..end_idx];
            let prompt = stripped[end_idx + 3..].trim().to_string();

            // Parse frontmatter as YAML
            let metadata = serde_yaml::from_str::<CommandSkillMetadata>(frontmatter).ok();

            return ParsedFile { prompt, metadata };
        }
    }

    // No frontmatter, use content as-is
    ParsedFile {
        prompt: content.to_string(),
        metadata: None,
    }
}

/// Parse a YAML file
fn parse_yaml_file(content: &str) -> ParsedFile {
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
        let metadata = serde_yaml::from_value::<CommandSkillMetadata>(yaml_value).ok();

        ParsedFile { prompt, metadata }
    } else {
        ParsedFile {
            prompt: content.to_string(),
            metadata: None,
        }
    }
}

/// Apply argument substitution to a prompt
fn apply_argument_substitution(prompt: &str, args: Option<&str>) -> String {
    if let Some(args_str) = args {
        let mut result = prompt.to_string();

        // Replace {{args}} with full args
        result = result.replace("{{args}}", args_str);

        // Replace {0}, {1}, etc. with individual args
        let arg_parts: Vec<&str> = args_str.split_whitespace().collect();
        for (i, arg) in arg_parts.iter().enumerate() {
            result = result.replace(&format!("{{{}}}", i), arg);
        }

        result
    } else {
        prompt.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;

    /// Helper to collect result from stream
    async fn get_result(tool: &CommandSkillTool, params: CommandSkillParams) -> CommandSkillOutput {
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

    #[test]
    fn test_parse_input_slash_command() {
        let (name, args, is_slash) = parse_input("/review-pr 123");
        assert_eq!(name, "review-pr");
        assert_eq!(args, Some("123".to_string()));
        assert!(is_slash);
    }

    #[test]
    fn test_parse_input_command_without_slash() {
        let (name, args, is_slash) = parse_input("review-pr 123");
        assert_eq!(name, "review-pr");
        assert_eq!(args, Some("123".to_string()));
        assert!(!is_slash);
    }

    #[test]
    fn test_parse_input_skill_name() {
        let (name, args, is_slash) = parse_input("code-reviewer");
        assert_eq!(name, "code-reviewer");
        assert_eq!(args, None);
        assert!(!is_slash);
    }

    #[test]
    fn test_parse_input_namespaced() {
        let (name, args, is_slash) = parse_input("/amplihack:analyze src/");
        assert_eq!(name, "amplihack:analyze");
        assert_eq!(args, Some("src/".to_string()));
        assert!(is_slash);
    }

    #[test]
    fn test_apply_argument_substitution_single_arg() {
        let prompt = "Review PR #{0} for quality.";
        let result = apply_argument_substitution(prompt, Some("123"));
        assert_eq!(result, "Review PR #123 for quality.");
    }

    #[test]
    fn test_apply_argument_substitution_multiple_args() {
        let prompt = "Compare {0} with {1}";
        let result = apply_argument_substitution(prompt, Some("foo bar"));
        assert_eq!(result, "Compare foo with bar");
    }

    #[test]
    fn test_apply_argument_substitution_full_args() {
        let prompt = "Process these: {{args}}";
        let result = apply_argument_substitution(prompt, Some("one two three"));
        assert_eq!(result, "Process these: one two three");
    }

    #[test]
    fn test_apply_argument_substitution_no_args() {
        let prompt = "No args needed.";
        let result = apply_argument_substitution(prompt, None);
        assert_eq!(result, "No args needed.");
    }

    #[test]
    fn test_discover_command_paths() {
        let paths = discover_command_paths("review-pr");
        assert!(paths.iter().any(|p| p.to_str().unwrap().contains(".claude/commands/review-pr.md")));
    }

    #[test]
    fn test_discover_command_paths_namespaced() {
        let paths = discover_command_paths("amplihack:analyze");
        assert!(paths.iter().any(|p| p.to_str().unwrap().contains(".claude/commands/amplihack/analyze.md")));
    }

    #[test]
    fn test_discover_skill_paths() {
        let paths = discover_skill_paths("code-reviewer");
        assert!(paths.iter().any(|p| p.to_str().unwrap().contains(".claude/skills/code-reviewer.md")));
        assert!(paths.iter().any(|p| p.to_str().unwrap().contains(".claude/skills/code-reviewer/skill.md")));
    }

    #[test]
    fn test_discover_skill_paths_with_plugin() {
        let paths = discover_skill_paths("my-plugin:my-skill");
        assert!(paths.iter().any(|p| p.to_str().unwrap().contains(".claude/plugins/my-plugin/skills/my-skill")));
    }

    #[test]
    fn test_parse_markdown_file_no_frontmatter() {
        let content = "# Simple Content\n\nJust text.";
        let parsed = parse_markdown_file(content);
        assert_eq!(parsed.prompt, content);
        assert!(parsed.metadata.is_none());
    }

    #[test]
    fn test_parse_markdown_file_with_frontmatter() {
        let content = r#"---
description: Test command
version: 1.0.0
---

# Test Content

The actual prompt."#;
        let parsed = parse_markdown_file(content);
        assert!(parsed.prompt.contains("Test Content"));
        assert!(!parsed.prompt.contains("---"));
        assert!(parsed.metadata.is_some());
        let meta = parsed.metadata.unwrap();
        assert_eq!(meta.description, Some("Test command".to_string()));
        assert_eq!(meta.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_parse_yaml_file() {
        let content = r#"
description: YAML skill
version: 2.0.0
prompt: |
  This is the prompt content.
"#;
        let parsed = parse_yaml_file(content);
        assert!(parsed.prompt.contains("prompt content"));
        assert!(parsed.metadata.is_some());
    }

    #[tokio::test]
    async fn test_command_skill_tool_metadata() {
        let tool = CommandSkillTool;
        let metadata = tool.metadata();
        assert_eq!(metadata.name, "CommandSkill");
        assert!(metadata.description.contains("command"));
        assert!(metadata.description.contains("skill"));
    }

    #[tokio::test]
    async fn test_command_skill_tool_is_read_only() {
        let tool = CommandSkillTool;
        assert!(tool.is_read_only());
    }

    #[tokio::test]
    async fn test_command_skill_tool_is_concurrency_safe() {
        let tool = CommandSkillTool;
        assert!(tool.is_concurrency_safe());
    }

    #[tokio::test]
    async fn test_load_command() {
        // Create temporary command file
        let cmd_dir = PathBuf::from(".claude/commands");
        let _ = fs::create_dir_all(&cmd_dir).await;

        let cmd_path = cmd_dir.join("test-unified-cmd.md");
        let test_content = r#"---
description: Test unified command
---

Review PR #{0} for code quality."#;
        let _ = fs::write(&cmd_path, test_content).await;

        let tool = CommandSkillTool;
        let params = CommandSkillParams {
            input: "/test-unified-cmd 456".to_string(),
            force_skill: false,
            force_command: false,
        };

        let result = get_result(&tool, params).await;

        assert!(result.found);
        assert_eq!(result.command_type, CommandType::SlashCommand);
        assert_eq!(result.name, "test-unified-cmd");
        assert_eq!(result.args, Some("456".to_string()));
        assert!(result.prompt.contains("456"));
        assert!(result.path.is_some());

        // Clean up
        let _ = fs::remove_file(&cmd_path).await;
    }

    #[tokio::test]
    async fn test_load_skill() {
        // Create temporary skill file
        let skill_dir = PathBuf::from(".claude/skills");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("test-unified-skill.md");
        let test_content = r#"---
description: Test unified skill
---

# Code Review Skill

Instructions for code review."#;
        let _ = fs::write(&skill_path, test_content).await;

        let tool = CommandSkillTool;
        let params = CommandSkillParams {
            input: "test-unified-skill".to_string(),
            force_skill: false,
            force_command: false,
        };

        let result = get_result(&tool, params).await;

        assert!(result.found);
        assert_eq!(result.command_type, CommandType::Skill);
        assert_eq!(result.name, "test-unified-skill");
        assert!(result.prompt.contains("Code Review Skill"));

        // Clean up
        let _ = fs::remove_file(&skill_path).await;
    }

    #[tokio::test]
    async fn test_force_skill_flag() {
        // Create both a command and skill with the same name
        let cmd_dir = PathBuf::from(".claude/commands");
        let skill_dir = PathBuf::from(".claude/skills");
        let _ = fs::create_dir_all(&cmd_dir).await;
        let _ = fs::create_dir_all(&skill_dir).await;

        let cmd_path = cmd_dir.join("dual-test.md");
        let skill_path = skill_dir.join("dual-test.md");

        let _ = fs::write(&cmd_path, "Command content").await;
        let _ = fs::write(&skill_path, "Skill content").await;

        let tool = CommandSkillTool;

        // Without force, slash command should find command first
        let params = CommandSkillParams {
            input: "/dual-test".to_string(),
            force_skill: false,
            force_command: false,
        };
        let result = get_result(&tool, params).await;
        assert!(result.found);
        assert_eq!(result.command_type, CommandType::SlashCommand);
        assert!(result.prompt.contains("Command content"));

        // With force_skill, should find skill
        let params = CommandSkillParams {
            input: "/dual-test".to_string(),
            force_skill: true,
            force_command: false,
        };
        let result = get_result(&tool, params).await;
        assert!(result.found);
        assert_eq!(result.command_type, CommandType::Skill);
        assert!(result.prompt.contains("Skill content"));

        // Clean up
        let _ = fs::remove_file(&cmd_path).await;
        let _ = fs::remove_file(&skill_path).await;
    }

    #[tokio::test]
    async fn test_not_found() {
        let tool = CommandSkillTool;
        let params = CommandSkillParams {
            input: "nonexistent-resource-12345".to_string(),
            force_skill: false,
            force_command: false,
        };

        let result = get_result(&tool, params).await;

        assert!(!result.found);
        assert!(result.prompt.contains("not found"));
        assert!(result.prompt.contains("nonexistent-resource-12345"));
        assert!(result.path.is_none());
    }

    #[tokio::test]
    async fn test_skill_in_subdirectory() {
        let skill_dir = PathBuf::from(".claude/skills/test-subdir-unified");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("skill.md");
        let test_content = "# Subdirectory Skill\n\nContent here.";
        let _ = fs::write(&skill_path, test_content).await;

        let tool = CommandSkillTool;
        let params = CommandSkillParams {
            input: "test-subdir-unified".to_string(),
            force_skill: false,
            force_command: false,
        };

        let result = get_result(&tool, params).await;

        assert!(result.found);
        assert!(result.prompt.contains("Subdirectory Skill"));

        // Clean up
        let _ = fs::remove_file(&skill_path).await;
        let _ = fs::remove_dir(&skill_dir).await;
    }

    #[tokio::test]
    async fn test_yaml_skill() {
        let skill_dir = PathBuf::from(".claude/skills");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("test-yaml-unified.yaml");
        let test_content = r#"
description: YAML based skill
version: 1.0.0
prompt: |
  This is a YAML skill.
  Multi-line content.
"#;
        let _ = fs::write(&skill_path, test_content).await;

        let tool = CommandSkillTool;
        let params = CommandSkillParams {
            input: "test-yaml-unified".to_string(),
            force_skill: false,
            force_command: false,
        };

        let result = get_result(&tool, params).await;

        assert!(result.found);
        assert!(result.prompt.contains("YAML skill"));
        assert!(result.prompt.contains("Multi-line content"));

        // Clean up
        let _ = fs::remove_file(&skill_path).await;
    }
}
