//! Hook execution engine with timeout and output capture

use crate::hooks::types::{Hook, HookContext, HookResult, HookType};
use anyhow::{Context, Result};
use rustyclawd_core::client::{Client as AnthropicClient, Config, CreateMessageRequest, Message};
use std::collections::HashSet;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Hook executor handles running command and prompt hooks
pub struct HookExecutor {
    /// Environment file path for persistence
    env_file: Option<String>,
}

impl HookExecutor {
    /// Create a new hook executor
    pub fn new() -> Self {
        Self {
            env_file: std::env::var("CLAUDE_ENV_FILE").ok(),
        }
    }

    /// Execute multiple hooks in parallel with deduplication
    pub async fn execute_hooks(
        &self,
        hooks: &[Hook],
        context: &HookContext,
    ) -> Result<Vec<HookResult>> {
        // Deduplicate hooks by command
        let unique_hooks = self.deduplicate_hooks(hooks);

        // Execute all hooks in parallel
        let mut tasks = Vec::new();
        for hook in unique_hooks {
            let hook = hook.clone();
            let context = context.clone();
            let env_file = self.env_file.clone();

            tasks.push(tokio::spawn(async move {
                Self::execute_single_hook(&hook, &context, env_file).await
            }));
        }

        // Wait for all hooks to complete
        let mut results = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => {
                    // Hook execution error - return as failed result
                    results.push(HookResult {
                        exit_code: 1,
                        stdout: String::new(),
                        stderr: format!("Hook execution error: {}", e),
                    });
                }
                Err(e) => {
                    // Task join error
                    results.push(HookResult {
                        exit_code: 1,
                        stdout: String::new(),
                        stderr: format!("Task join error: {}", e),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Deduplicate hooks by their command/type
    fn deduplicate_hooks<'a>(&self, hooks: &'a [Hook]) -> Vec<&'a Hook> {
        let mut seen = HashSet::new();
        let mut unique = Vec::new();

        for hook in hooks {
            let key = match &hook.hook_type {
                HookType::Command => hook.command.as_deref().unwrap_or(""),
                HookType::Prompt => "prompt",
            };

            if seen.insert(key) {
                unique.push(hook);
            }
        }

        unique
    }

    /// Execute a single hook
    async fn execute_single_hook(
        hook: &Hook,
        context: &HookContext,
        env_file: Option<String>,
    ) -> Result<HookResult> {
        match hook.hook_type {
            HookType::Command => Self::execute_command_hook(hook, context, env_file).await,
            HookType::Prompt => Self::execute_prompt_hook(hook, context).await,
        }
    }

    /// Execute a command (bash) hook
    async fn execute_command_hook(
        hook: &Hook,
        context: &HookContext,
        env_file: Option<String>,
    ) -> Result<HookResult> {
        let command = hook
            .command
            .as_ref()
            .context("Command hook must have command")?;

        // Build the command with environment
        let mut cmd = Command::new("bash");
        cmd.arg("-c")
            .arg(command)
            .current_dir(&context.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables
        cmd.env("CLAUDE_SESSION_ID", &context.session_id)
            .env("CLAUDE_TRANSCRIPT_PATH", &context.transcript_path)
            .env("CLAUDE_CWD", &context.cwd)
            .env("CLAUDE_PERMISSION_MODE", &context.permission_mode)
            .env("CLAUDE_HOOK_EVENT", &context.hook_event_name);

        // Set CLAUDE_PROJECT_DIR - use cwd as project dir
        cmd.env("CLAUDE_PROJECT_DIR", &context.cwd);

        // Set CLAUDE_CODE_REMOTE if running in web environment
        // This would be set by the web environment, but we can check for it
        if std::env::var("CLAUDE_CODE_REMOTE").is_ok() {
            cmd.env("CLAUDE_CODE_REMOTE", "true");
        }

        if let Some(tool_name) = &context.tool_name {
            cmd.env("CLAUDE_TOOL_NAME", tool_name);
        }

        if let Some(env_file) = env_file {
            cmd.env("CLAUDE_ENV_FILE", env_file);
        }

        // Execute with timeout
        let timeout_duration = Duration::from_millis(hook.effective_timeout() as u64);
        let child = cmd.spawn().context("Failed to spawn hook command")?;

        match timeout(timeout_duration, child.wait_with_output()).await {
            Ok(Ok(output)) => Ok(HookResult {
                exit_code: output.status.code().unwrap_or(1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            }),
            Ok(Err(e)) => Ok(HookResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!("Command execution failed: {}", e),
            }),
            Err(_) => Ok(HookResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!("Hook timed out after {}ms", hook.effective_timeout()),
            }),
        }
    }

    /// Execute a prompt (LLM) hook
    async fn execute_prompt_hook(hook: &Hook, context: &HookContext) -> Result<HookResult> {
        // Load API configuration and create client
        let config = Config::from_default_location()
            .await
            .context("Failed to load API configuration")?;
        let client = AnthropicClient::new(config);

        // Build the prompt with context information
        let prompt = Self::build_hook_prompt(hook, context);

        // Create the LLM request
        // Use claude-3-haiku-20240307 which is a fast, available model for hooks
        let request =
            CreateMessageRequest::new("claude-3-haiku-20240307", vec![Message::user(prompt)], 1024)
                .with_temperature(0.0); // Use deterministic responses for hooks

        // Execute with timeout
        let timeout_duration = Duration::from_millis(hook.effective_timeout() as u64);

        match timeout(timeout_duration, client.create_message(request)).await {
            Ok(Ok(response)) => {
                // Extract text from response
                let text = response
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        rustyclawd_core::client::ContentBlock::Text { text } => Some(text.as_str()),
                        rustyclawd_core::client::ContentBlock::ToolUse { .. } => None,
                        rustyclawd_core::client::ContentBlock::ToolResult { .. } => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                // Try to parse as JSON decision
                let exit_code = match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(json) => {
                        // Check for "continue" field
                        if let Some(continue_val) = json.get("continue") {
                            if continue_val.as_bool() == Some(true) {
                                0 // Success - continue
                            } else {
                                2 // Blocking - do not continue
                            }
                        } else if let Some(decision) = json.get("decision") {
                            // Check for "decision" field with "approve"/"block"
                            if decision.as_str() == Some("approve") {
                                0
                            } else {
                                2
                            }
                        } else if let Some(permission) = json.get("permissionDecision") {
                            // Check for "permissionDecision" field
                            if permission.as_str() == Some("allow") {
                                0
                            } else {
                                2
                            }
                        } else {
                            // No recognized decision field - default to success
                            0
                        }
                    }
                    Err(_) => {
                        // Not valid JSON - return as-is with success code
                        0
                    }
                };

                Ok(HookResult {
                    exit_code,
                    stdout: text,
                    stderr: String::new(),
                })
            }
            Ok(Err(e)) => Ok(HookResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!("LLM request failed: {}", e),
            }),
            Err(_) => Ok(HookResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!("Hook timed out after {}ms", hook.effective_timeout()),
            }),
        }
    }

    /// Find the project directory (directory with .claude folder)
    fn find_project_dir(start_dir: &str) -> Result<String> {
        let mut current = std::path::PathBuf::from(start_dir);
        loop {
            let claude_dir = current.join(".claude");
            if claude_dir.exists() && claude_dir.is_dir() {
                return Ok(current.to_string_lossy().to_string());
            }
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => return Err(anyhow::anyhow!("Project root not found")),
            }
        }
    }

    /// Build a prompt for the LLM hook with context information
    fn build_hook_prompt(hook: &Hook, context: &HookContext) -> String {
        let context_json =
            serde_json::to_string_pretty(context).unwrap_or_else(|_| "{}".to_string());

        // If hook has custom prompt, use it and replace $ARGUMENTS
        if let Some(custom_prompt) = &hook.prompt {
            return custom_prompt.replace("$ARGUMENTS", &context_json);
        }

        // Default prompt based on event type
        format!(
            r#"You are a hook execution assistant for Claude Code CLI.

Event: {}
Tool: {}

Context:
{}

Please analyze this event and respond with a JSON decision in one of these formats:

For Stop/SubagentStop events:
{{"decision": "approve"}} or {{"decision": "block", "reason": "explanation"}}

For PreToolUse events:
{{"permissionDecision": "allow"}} or {{"permissionDecision": "deny"}} or {{"permissionDecision": "ask"}}

For PostToolUse events:
{{"decision": "block", "additionalContext": "context"}} or {{"continue": true}}

For UserPromptSubmit events:
{{"decision": "block", "additionalContext": "context"}} or {{"continue": true}}

For other events:
{{"continue": true}} or {{"continue": false, "stopReason": "reason"}}

You can also include optional fields:
- "systemMessage": "warning to show user"
- "suppressOutput": true (to hide from transcript)

Respond ONLY with the JSON decision, no other text."#,
            context.hook_event_name,
            context.tool_name.as_deref().unwrap_or("N/A"),
            context_json
        )
    }
}

impl Default for HookExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::types::HookEvent;

    #[tokio::test]
    async fn test_execute_command_hook_success() {
        let hook = Hook::command("echo 'test'".to_string(), Some(5000));
        let context = HookContext::for_session(
            "test-session".to_string(),
            "/tmp/transcript".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "auto".to_string(),
            HookEvent::SessionStart,
        );

        let executor = HookExecutor::new();
        let result = executor.execute_hooks(&[hook], &context).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].exit_code, 0);
        assert!(result[0].stdout.contains("test"));
    }

    #[tokio::test]
    async fn test_execute_command_hook_error() {
        let hook = Hook::command("exit 2".to_string(), Some(5000));
        let context = HookContext::for_session(
            "test-session".to_string(),
            "/tmp/transcript".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "auto".to_string(),
            HookEvent::SessionStart,
        );

        let executor = HookExecutor::new();
        let result = executor.execute_hooks(&[hook], &context).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].exit_code, 2);
    }

    #[tokio::test]
    async fn test_execute_multiple_hooks() {
        let hooks = vec![
            Hook::command("echo 'hook1'".to_string(), Some(5000)),
            Hook::command("echo 'hook2'".to_string(), Some(5000)),
        ];
        let context = HookContext::for_session(
            "test-session".to_string(),
            "/tmp/transcript".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "auto".to_string(),
            HookEvent::SessionStart,
        );

        let executor = HookExecutor::new();
        let results = executor.execute_hooks(&hooks, &context).await.unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].is_success());
        assert!(results[1].is_success());
    }

    #[tokio::test]
    async fn test_deduplicate_hooks() {
        let hooks = vec![
            Hook::command("echo duplicate".to_string(), Some(5000)),
            Hook::command("echo duplicate".to_string(), Some(5000)),
            Hook::command("echo unique".to_string(), Some(5000)),
        ];

        let executor = HookExecutor::new();
        let unique = executor.deduplicate_hooks(&hooks);

        // Should deduplicate to 2 unique hooks
        assert_eq!(unique.len(), 2);
    }

    #[tokio::test]
    async fn test_execute_prompt_hook() {
        let hook = Hook::prompt(None, Some(60000));
        let context = HookContext::for_session(
            "test-session".to_string(),
            "/tmp/transcript".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "auto".to_string(),
            HookEvent::Stop,
        );

        let executor = HookExecutor::new();
        let result = executor.execute_hooks(&[hook], &context).await.unwrap();

        assert_eq!(result.len(), 1);

        // Print debug info if test fails
        if !result[0].is_success() {
            eprintln!("Exit code: {}", result[0].exit_code);
            eprintln!("Stdout: {}", result[0].stdout);
            eprintln!("Stderr: {}", result[0].stderr);
        }

        assert!(result[0].is_success());
    }
}
