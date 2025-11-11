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
                    // Command not found, return simple expansion
                    format!("Execute command: {}", command)
                }
            };

            // In a full implementation, would:
            // 1. Parse command file (markdown with frontmatter)
            // 2. Substitute arguments into template
            // 3. Return expanded prompt

            let expanded_prompt = if let Some(args_str) = args {
                format!("{}\n\nArguments: {}", prompt_content, args_str)
            } else {
                prompt_content
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

    #[tokio::test]
    async fn test_slash_command_parsing() {
        let tool = SlashCommandTool;
        let params = SlashCommandParams {
            command: "/review-pr 123".to_string(),
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        }).unwrap();

        assert_eq!(result.command_name, "review-pr");
        assert!(!result.expanded_prompt.is_empty());
    }
}
