//! SlashCommand tool - Execute custom slash commands
//!
//! This tool unifies commands and skills, allowing both to be invoked via `/name` syntax.
//! Like Claude Code, skills can be invoked as `/skill-name` in addition to `/command-name`.
//!
//! Resolution order:
//! 1. `.claude/commands/{name}.md` - Traditional commands (highest priority)
//! 2. `.claude/skills/{name}.md` - Direct skill file
//! 3. `.claude/skills/{name}/SKILL.md` - Skill in subdirectory (uppercase SKILL.md)
//! 4. `.claude/skills/{name}/skill.md` - Skill in subdirectory (lowercase skill.md)
//! 5. `.claude/skills/{name}.yaml` - YAML skill file
//!
//! Demonstrates:
//! - Command expansion and loading
//! - File-based command definitions
//! - Dynamic command discovery
//! - Unified skill/command invocation

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Parameters for SlashCommand tool
#[derive(Debug, Deserialize)]
pub struct SlashCommandParams {
    /// The command to execute (e.g., "/review-pr 123")
    pub command: String,
}

/// Output from SlashCommand tool
#[derive(Debug, Serialize)]
pub struct SlashCommandOutput {
    /// The command that was executed
    pub command: String,

    /// Expanded prompt from the command
    pub expanded_prompt: String,

    /// Command name
    pub command_name: String,

    /// Source of the command (command or skill)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<CommandSource>,

    /// Path where the command/skill was found
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Source type for the resolved command
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommandSource {
    /// Traditional command from .claude/commands/
    Command,
    /// Skill from .claude/skills/
    Skill,
}

/// Resolution result containing content, source, and path
struct Resolution {
    content: String,
    source: CommandSource,
    path: String,
}

/// Get search paths for command/skill resolution
fn get_search_paths(name: &str) -> Vec<(PathBuf, CommandSource)> {
    vec![
        // Commands first (backward compatible, highest priority)
        (
            PathBuf::from(format!(".claude/commands/{}.md", name)),
            CommandSource::Command,
        ),
        // Skills - various formats
        (
            PathBuf::from(format!(".claude/skills/{}.md", name)),
            CommandSource::Skill,
        ),
        (
            PathBuf::from(format!(".claude/skills/{}/SKILL.md", name)),
            CommandSource::Skill,
        ),
        (
            PathBuf::from(format!(".claude/skills/{}/skill.md", name)),
            CommandSource::Skill,
        ),
        (
            PathBuf::from(format!(".claude/skills/{}.yaml", name)),
            CommandSource::Skill,
        ),
    ]
}

/// Resolve a command or skill by name
async fn resolve_command_or_skill(name: &str) -> Option<Resolution> {
    let search_paths = get_search_paths(name);

    for (path, source) in search_paths {
        if let Ok(content) = fs::read_to_string(&path).await {
            return Some(Resolution {
                content,
                source,
                path: path.to_string_lossy().to_string(),
            });
        }
    }

    None
}

/// Extract prompt from YAML content
fn extract_yaml_prompt(content: &str) -> String {
    // Try to parse as YAML and extract prompt/instructions field
    if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(content) {
        // Check for common prompt fields in order of preference
        if let Some(prompt) = yaml.get("prompt").and_then(|v| v.as_str()) {
            return prompt.to_string();
        }
        if let Some(instructions) = yaml.get("instructions").and_then(|v| v.as_str()) {
            return instructions.to_string();
        }
        if let Some(description) = yaml.get("description").and_then(|v| v.as_str()) {
            return description.to_string();
        }
    }
    // Fallback: return the content as-is
    content.to_string()
}

/// The SlashCommand tool
pub struct SlashCommandTool;

#[async_trait]
impl crate::Tool for SlashCommandTool {
    type Params = SlashCommandParams;
    type Output = SlashCommandOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "SlashCommand",
            description: "Executes custom slash commands and skills that expand to prompts. Supports both /command and /skill syntax.",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let command = params.command.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: "Parsing command...".to_string(),
                percentage: Some(20.0),
            };

            // Parse command (format: "/command-name args...")
            let parts: Vec<&str> = command.trim_start_matches('/').splitn(2, ' ').collect();
            let command_name = parts[0].to_string();
            let args = parts.get(1).map(|s| s.to_string());

            if debug {
                tracing::debug!(
                    command_name = %command_name,
                    args = ?args,
                    "Parsing slash command"
                );
            }

            yield ToolEvent::Progress {
                step: format!("Loading command or skill: {}", command_name),
                percentage: Some(40.0),
            };

            // Resolve command/skill using unified resolution
            let resolution = resolve_command_or_skill(&command_name).await;

            let (prompt_content, source, found_path) = match resolution {
                Some(r) => (r.content, r.source, r.path),
                None => {
                    // Neither command nor skill found
                    let search_paths = get_search_paths(&command_name);
                    yield ToolEvent::Error {
                        message: format!(
                            "Command or skill not found: {}\nSearched in:\n{}",
                            command_name,
                            search_paths.iter()
                                .map(|(p, _)| format!("  - {}", p.display()))
                                .collect::<Vec<_>>()
                                .join("\n")
                        ),
                    };
                    return;
                }
            };

            if debug {
                tracing::debug!(
                    command_name = %command_name,
                    source = ?source,
                    path = %found_path,
                    "Resolved command/skill"
                );
            }

            // Handle YAML files differently
            let is_yaml = found_path.ends_with(".yaml") || found_path.ends_with(".yml");

            // Parse markdown frontmatter if present (or handle YAML)
            let expanded_prompt = if is_yaml {
                // For YAML files, extract the prompt field
                let base_prompt = extract_yaml_prompt(&prompt_content);
                if let Some(args_str) = &args {
                    let mut result = base_prompt;
                    result = result.replace("{{args}}", args_str);
                    let arg_parts: Vec<&str> = args_str.split_whitespace().collect();
                    for (i, arg) in arg_parts.iter().enumerate() {
                        result = result.replace(&format!("{{{}}}", i), arg);
                    }
                    result
                } else {
                    base_prompt
                }
            } else if let Some(stripped) = prompt_content.strip_prefix("---") {
                // Find the end of frontmatter
                if let Some(end_idx) = stripped.find("---") {
                    let frontmatter = &stripped[..end_idx];
                    let content = stripped[end_idx + 3..].trim();

                    // Parse frontmatter as YAML (optional - for future use)
                    if let Ok(meta) = serde_yaml::from_str::<serde_json::Value>(frontmatter) {
                        if debug {
                            tracing::debug!(
                                "Parsed frontmatter: description={:?}",
                                meta.get("description")
                            );
                        }
                    }

                    // Use content after frontmatter
                    if let Some(args_str) = &args {
                        // Simple template substitution: replace {{args}} or {0} style placeholders
                        let mut result = content.to_string();

                        // Replace {{args}} with full args
                        result = result.replace("{{args}}", args_str);

                        // Replace {0}, {1}, etc. with individual args
                        let arg_parts: Vec<&str> = args_str.split_whitespace().collect();
                        for (i, arg) in arg_parts.iter().enumerate() {
                            result = result.replace(&format!("{{{}}}", i), arg);
                        }

                        result
                    } else {
                        content.to_string()
                    }
                } else {
                    // Malformed frontmatter, use as-is
                    prompt_content
                }
            } else {
                // No frontmatter, use content directly
                if let Some(args_str) = &args {
                    format!("{}\n\nArguments: {}", prompt_content, args_str)
                } else {
                    prompt_content
                }
            };

            if debug {
                tracing::debug!(
                    command_name = %command_name,
                    expanded_len = expanded_prompt.len(),
                    "Command expanded"
                );
            }

            yield ToolEvent::Result(SlashCommandOutput {
                command: params.command.clone(),
                expanded_prompt,
                command_name,
                source: Some(source),
                path: Some(found_path),
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true // Loading commands doesn't modify state
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Command loading is independent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;

    // Helper to extract result from events
    fn get_result(events: &[ToolEvent<SlashCommandOutput>]) -> Option<&SlashCommandOutput> {
        events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        })
    }

    // Helper to check for error event
    fn has_error(events: &[ToolEvent<SlashCommandOutput>]) -> bool {
        events.iter().any(|e| matches!(e, ToolEvent::Error { .. }))
    }

    // ===== Original Tests (Commands) =====

    #[tokio::test]
    async fn test_slash_command_parsing() {
        // Create temporary command file for testing
        let cmd_dir = PathBuf::from(".claude/commands");
        let _ = fs::create_dir_all(&cmd_dir).await;

        let cmd_path = cmd_dir.join("review-pr.md");
        let test_content = "---\ndescription: Review a pull request\n---\n\nReview PR #{0} for code quality and correctness.\n";
        let _ = fs::write(&cmd_path, test_content).await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/review-pr 123".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        assert_eq!(result.command_name, "review-pr");
        assert!(!result.expanded_prompt.is_empty());
        assert!(result.expanded_prompt.contains("123") || result.expanded_prompt.contains("PR"));
        assert_eq!(result.source, Some(CommandSource::Command));

        // Clean up
        let _ = fs::remove_file(&cmd_path).await;
    }

    #[tokio::test]
    async fn test_command_not_found() {
        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/nonexistent-command".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        assert!(has_error(&events));
    }

    #[tokio::test]
    async fn test_multiple_argument_substitution() {
        let cmd_dir = PathBuf::from(".claude/commands");
        let _ = fs::create_dir_all(&cmd_dir).await;

        let cmd_path = cmd_dir.join("multi-arg.md");
        let test_content =
            "---\ndescription: Test multiple args\n---\n\nFirst: {0}, Second: {1}, Third: {2}\n";
        let _ = fs::write(&cmd_path, test_content).await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/multi-arg foo bar baz".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        assert!(result.expanded_prompt.contains("foo"));
        assert!(result.expanded_prompt.contains("bar"));
        assert!(result.expanded_prompt.contains("baz"));

        let _ = fs::remove_file(&cmd_path).await;
    }

    #[tokio::test]
    async fn test_args_placeholder_substitution() {
        let cmd_dir = PathBuf::from(".claude/commands");
        let _ = fs::create_dir_all(&cmd_dir).await;

        let cmd_path = cmd_dir.join("args-placeholder.md");
        let test_content =
            "---\ndescription: Test {{args}} placeholder\n---\n\nAll arguments: {{args}}\n";
        let _ = fs::write(&cmd_path, test_content).await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/args-placeholder one two three".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        assert!(result.expanded_prompt.contains("one two three"));

        let _ = fs::remove_file(&cmd_path).await;
    }

    #[tokio::test]
    async fn test_no_frontmatter() {
        let cmd_dir = PathBuf::from(".claude/commands");
        let _ = fs::create_dir_all(&cmd_dir).await;

        let cmd_path = cmd_dir.join("no-frontmatter.md");
        let test_content = "Simple command without frontmatter\n";
        let _ = fs::write(&cmd_path, test_content).await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/no-frontmatter".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        assert!(result.expanded_prompt.contains("Simple command"));

        let _ = fs::remove_file(&cmd_path).await;
    }

    #[tokio::test]
    async fn test_command_without_args() {
        let cmd_dir = PathBuf::from(".claude/commands");
        let _ = fs::create_dir_all(&cmd_dir).await;

        let cmd_path = cmd_dir.join("no-args.md");
        let test_content =
            "---\ndescription: Command without args\n---\n\nThis command takes no arguments.\n";
        let _ = fs::write(&cmd_path, test_content).await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/no-args".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        assert!(result.expanded_prompt.contains("This command takes no arguments"));

        let _ = fs::remove_file(&cmd_path).await;
    }

    #[tokio::test]
    async fn test_empty_command_name() {
        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should get an error since command name is empty
        assert!(has_error(&events));
    }

    #[tokio::test]
    async fn test_command_with_spaces_in_args() {
        let cmd_dir = PathBuf::from(".claude/commands");
        let _ = fs::create_dir_all(&cmd_dir).await;

        let cmd_path = cmd_dir.join("space-args.md");
        let test_content = "---\ndescription: Test args with spaces\n---\n\nArgs: {{args}}\n";
        let _ = fs::write(&cmd_path, test_content).await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/space-args arg with multiple words".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        assert!(result.expanded_prompt.contains("arg with multiple words"));

        let _ = fs::remove_file(&cmd_path).await;
    }

    #[tokio::test]
    async fn test_read_only_flag() {
        let tool = SlashCommandTool;
        assert!(tool.is_read_only());
    }

    #[tokio::test]
    async fn test_concurrency_safe_flag() {
        let tool = SlashCommandTool;
        assert!(tool.is_concurrency_safe());
    }

    #[tokio::test]
    async fn test_malformed_frontmatter() {
        let cmd_dir = PathBuf::from(".claude/commands");
        let _ = fs::create_dir_all(&cmd_dir).await;

        let cmd_path = cmd_dir.join("malformed.md");
        // Frontmatter without closing ---
        let test_content = "---\ndescription: Test\n\nContent here\n";
        let _ = fs::write(&cmd_path, test_content).await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/malformed".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        // Should still return content even with malformed frontmatter
        assert!(!result.expanded_prompt.is_empty());

        let _ = fs::remove_file(&cmd_path).await;
    }

    // ===== New Tests (Skills Resolution) =====

    #[tokio::test]
    async fn test_skill_direct_file_resolution() {
        // Create a skill as direct .md file
        let skill_dir = PathBuf::from(".claude/skills");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("test-skill-direct.md");
        let test_content = "---\ndescription: A test skill\n---\n\nThis is a test skill prompt.\n";
        let _ = fs::write(&skill_path, test_content).await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/test-skill-direct".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        assert_eq!(result.command_name, "test-skill-direct");
        assert!(result.expanded_prompt.contains("test skill prompt"));
        assert_eq!(result.source, Some(CommandSource::Skill));
        assert!(result.path.as_ref().unwrap().contains(".claude/skills/"));

        let _ = fs::remove_file(&skill_path).await;
    }

    #[tokio::test]
    async fn test_skill_subdirectory_uppercase_resolution() {
        // Create a skill in subdirectory with SKILL.md (uppercase)
        let skill_dir = PathBuf::from(".claude/skills/test-skill-upper");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("SKILL.md");
        let test_content = "---\ndescription: Uppercase skill\n---\n\nUppercase SKILL.md content.\n";
        let _ = fs::write(&skill_path, test_content).await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/test-skill-upper".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        assert_eq!(result.command_name, "test-skill-upper");
        assert!(result.expanded_prompt.contains("Uppercase SKILL.md"));
        assert_eq!(result.source, Some(CommandSource::Skill));

        let _ = fs::remove_dir_all(&skill_dir).await;
    }

    #[tokio::test]
    async fn test_skill_yaml_resolution() {
        // Create a YAML skill file
        let skill_dir = PathBuf::from(".claude/skills");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("test-yaml-skill.yaml");
        let test_content = "description: A YAML skill\nprompt: This is a YAML skill prompt with arg {0}.\n";
        let _ = fs::write(&skill_path, test_content).await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/test-yaml-skill myarg".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        assert_eq!(result.command_name, "test-yaml-skill");
        assert!(result.expanded_prompt.contains("YAML skill prompt"));
        assert!(result.expanded_prompt.contains("myarg"));
        assert_eq!(result.source, Some(CommandSource::Skill));

        let _ = fs::remove_file(&skill_path).await;
    }

    #[tokio::test]
    async fn test_command_takes_priority_over_skill() {
        // Create both a command and a skill with the same name
        let cmd_dir = PathBuf::from(".claude/commands");
        let skill_dir = PathBuf::from(".claude/skills");
        let _ = fs::create_dir_all(&cmd_dir).await;
        let _ = fs::create_dir_all(&skill_dir).await;

        let cmd_path = cmd_dir.join("priority-test.md");
        let skill_path = skill_dir.join("priority-test.md");

        let _ = fs::write(&cmd_path, "---\ndescription: Command version\n---\n\nThis is the COMMAND.\n").await;
        let _ = fs::write(&skill_path, "---\ndescription: Skill version\n---\n\nThis is the SKILL.\n").await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/priority-test".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        assert!(result.expanded_prompt.contains("COMMAND"));
        assert!(!result.expanded_prompt.contains("SKILL"));
        assert_eq!(result.source, Some(CommandSource::Command));

        let _ = fs::remove_file(&cmd_path).await;
        let _ = fs::remove_file(&skill_path).await;
    }

    #[tokio::test]
    async fn test_skill_with_arguments() {
        let skill_dir = PathBuf::from(".claude/skills");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("skill-with-args.md");
        let test_content = "---\ndescription: Skill with args\n---\n\nProcess: {0} and {1}. All: {{args}}\n";
        let _ = fs::write(&skill_path, test_content).await;

        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/skill-with-args first second".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = get_result(&events).unwrap();
        assert!(result.expanded_prompt.contains("first"));
        assert!(result.expanded_prompt.contains("second"));
        assert!(result.expanded_prompt.contains("first second"));
        assert_eq!(result.source, Some(CommandSource::Skill));

        let _ = fs::remove_file(&skill_path).await;
    }

    #[tokio::test]
    async fn test_error_message_shows_search_paths() {
        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/totally-nonexistent-command".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Find the error event and check its message
        let error_msg = events.iter().find_map(|e| match e {
            ToolEvent::Error { message } => Some(message.clone()),
            _ => None,
        }).expect("Should have error event");

        // Verify error message shows search paths
        assert!(error_msg.contains(".claude/commands/"));
        assert!(error_msg.contains(".claude/skills/"));
        assert!(error_msg.contains("SKILL.md"));
    }

    // ===== Unit Tests for Helper Functions =====

    #[test]
    fn test_get_search_paths() {
        let paths = get_search_paths("my-skill");
        assert_eq!(paths.len(), 5);

        // Verify order - commands first
        assert!(paths[0].0.to_string_lossy().contains("commands"));
        assert_eq!(paths[0].1, CommandSource::Command);

        // Then skills
        assert!(paths[1].0.to_string_lossy().contains("skills"));
        assert_eq!(paths[1].1, CommandSource::Skill);
    }

    #[tokio::test]
    async fn test_resolve_command_or_skill_command() {
        let cmd_dir = PathBuf::from(".claude/commands");
        let _ = fs::create_dir_all(&cmd_dir).await;

        let cmd_path = cmd_dir.join("resolve-test-cmd.md");
        let _ = fs::write(&cmd_path, "test content").await;

        let result = resolve_command_or_skill("resolve-test-cmd").await;
        assert!(result.is_some());
        let resolution = result.unwrap();
        assert_eq!(resolution.source, CommandSource::Command);
        assert!(resolution.path.contains("commands"));

        let _ = fs::remove_file(&cmd_path).await;
    }

    #[tokio::test]
    async fn test_resolve_command_or_skill_not_found() {
        let result = resolve_command_or_skill("definitely-not-exists-xyz").await;
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_yaml_prompt() {
        let yaml = "description: test\nprompt: The actual prompt\n";
        let prompt = extract_yaml_prompt(yaml);
        assert_eq!(prompt, "The actual prompt");
    }

    #[test]
    fn test_extract_yaml_prompt_instructions_field() {
        let yaml = "description: test\ninstructions: Use these instructions\n";
        let prompt = extract_yaml_prompt(yaml);
        assert_eq!(prompt, "Use these instructions");
    }

    #[test]
    fn test_extract_yaml_prompt_fallback() {
        let yaml = "description: Only description here\n";
        let prompt = extract_yaml_prompt(yaml);
        assert_eq!(prompt, "Only description here");
    }
}
