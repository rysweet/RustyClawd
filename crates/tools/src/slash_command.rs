//! SlashCommand tool - Execute custom slash commands
//!
//! Demonstrates:
//! - Command expansion and loading
//! - File-based command definitions
//! - Dynamic command discovery

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
            description: "Executes custom slash commands that expand to prompts",
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
                step: format!("Loading command: {}", command_name),
                percentage: Some(40.0),
            };

            // Look for command file in .claude/commands/
            let command_path = PathBuf::from(format!(".claude/commands/{}.md", command_name));

            let prompt_content = match fs::read_to_string(&command_path).await {
                Ok(c) => c,
                Err(_) => {
                    // Command not found, return error
                    yield ToolEvent::Error {
                        message: format!("Command not found: {}", command_name),
                    };
                    return;
                }
            };

            // Parse markdown frontmatter if present
            let expanded_prompt = if let Some(stripped) = prompt_content.strip_prefix("---") {
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
    use std::path::PathBuf;

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

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert_eq!(result.command_name, "review-pr");
        assert!(!result.expanded_prompt.is_empty());
        assert!(result.expanded_prompt.contains("123") || result.expanded_prompt.contains("PR"));

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

        // Should get an error event
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
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

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

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

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

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

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

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

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert!(result
            .expanded_prompt
            .contains("This command takes no arguments"));

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
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
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

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

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

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        // Should still return content even with malformed frontmatter
        assert!(!result.expanded_prompt.is_empty());

        let _ = fs::remove_file(&cmd_path).await;
    }
}
