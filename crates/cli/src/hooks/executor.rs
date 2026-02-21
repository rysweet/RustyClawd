//! Hook execution engine with timeout and output capture

use crate::hooks::config::{Hook, HookResult, HookType};
use crate::hooks::context::HookContext;
use crate::hooks::prompt_hook;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Hook executor handles running command and prompt hooks
#[derive(Clone)]
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
            HookType::Prompt => prompt_hook::execute_prompt_hook(hook, context).await,
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

        if let Some(tool_use_id) = &context.tool_use_id {
            cmd.env("CLAUDE_TOOL_USE_ID", tool_use_id);
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
}

impl Default for HookExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::event::HookEvent;

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
    #[ignore] // Requires API configuration at ~/.claude-msec-k; run manually when testing hooks
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

    #[tokio::test]
    async fn test_command_hook_receives_tool_use_id_env_var() {
        // This test verifies that CLAUDE_TOOL_USE_ID is set for tool-related hooks
        let hook = Hook::command("echo $CLAUDE_TOOL_USE_ID".to_string(), Some(5000));
        let context = HookContext::for_tool(
            "test-session".to_string(),
            "/tmp/transcript".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "ask".to_string(),
            HookEvent::PreToolUse,
            "Write".to_string(),
            Some("toolu_test_123".to_string()),
        );

        let executor = HookExecutor::new();
        let result = executor.execute_hooks(&[hook], &context).await.unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].is_success());
        // Verify the tool_use_id was passed through environment variable
        assert!(
            result[0].stdout.trim().contains("toolu_test_123"),
            "Expected CLAUDE_TOOL_USE_ID to be set. Got stdout: {}",
            result[0].stdout
        );
    }

    #[tokio::test]
    async fn test_command_hook_tool_use_id_not_set_when_none() {
        // Verify that CLAUDE_TOOL_USE_ID is NOT set when context has None
        let hook = Hook::command(
            "echo \"ID:$CLAUDE_TOOL_USE_ID:END\"".to_string(),
            Some(5000),
        );
        let context = HookContext::for_tool(
            "test-session".to_string(),
            "/tmp/transcript".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "ask".to_string(),
            HookEvent::PreToolUse,
            "Write".to_string(),
            None, // No tool_use_id
        );

        let executor = HookExecutor::new();
        let result = executor.execute_hooks(&[hook], &context).await.unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].is_success());
        // When tool_use_id is None, the env var should not be set (empty)
        assert!(
            result[0].stdout.trim() == "ID::END",
            "Expected empty CLAUDE_TOOL_USE_ID when None. Got stdout: {}",
            result[0].stdout
        );
    }
}
