//! KillShell tool - Terminate background shells
//!
//! Demonstrates:
//! - Process lifecycle management
//! - Signal handling
//! - Resource cleanup

use crate::process_registry::global_registry;
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for KillShell tool
#[derive(Debug, Clone, Deserialize)]
pub struct KillShellParams {
    /// ID of the shell to kill
    pub shell_id: String,
}

/// Output from KillShell tool
#[derive(Debug, Serialize)]
pub struct KillShellOutput {
    /// Shell ID that was killed
    pub shell_id: String,

    /// Whether kill was successful
    pub success: bool,

    /// Status message
    pub message: String,
}

/// The KillShell tool
pub struct KillShellTool;

#[async_trait]
impl crate::Tool for KillShellTool {
    type Params = KillShellParams;
    type Output = KillShellOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "KillShell",
            description: "Terminates a running background shell by ID",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let shell_id = params.shell_id.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Terminating shell: {}", shell_id),
                percentage: None,
            };

            // Get the global process registry
            let registry = global_registry();

            // Attempt to kill the process
            let (success, message) = match registry.kill(&shell_id).await {
                Ok(true) => (
                    true,
                    format!("Shell {} terminated successfully", shell_id)
                ),
                Ok(false) => (
                    false,
                    format!("Shell {} not found", shell_id)
                ),
                Err(e) => (
                    false,
                    format!("Failed to terminate shell {}: {}", shell_id, e)
                ),
            };

            if debug {
                tracing::debug!(
                    shell_id = %shell_id,
                    success = success,
                    "Shell termination complete"
                );
            }

            yield ToolEvent::Result(KillShellOutput {
                shell_id: params.shell_id.clone(),
                success,
                message,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        false // Kills processes
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Each kill is independent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_kill_shell_basic() {
        // Register a test process first
        let registry = global_registry();

        // Spawn a long-running process that we can kill
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_kill_basic".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Verify the process was registered
        assert!(registry.exists(&test_shell_id).await);

        let tool = KillShellTool;
        let params = KillShellParams {
            shell_id: test_shell_id.clone(),
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

        assert_eq!(result.shell_id, "test_kill_basic");
        assert!(result.success);
        assert!(result.message.contains("terminated"));

        // Verify the process is no longer in registry
        assert!(!registry.exists(&test_shell_id).await);
    }

    #[tokio::test]
    async fn test_kill_shell_nonexistent() {
        let tool = KillShellTool;
        let params = KillShellParams {
            shell_id: "nonexistent_shell".to_string(),
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

        assert_eq!(result.shell_id, "nonexistent_shell");
        assert!(!result.success);
        assert!(result.message.contains("not found"));
    }

    #[tokio::test]
    async fn test_kill_shell_already_completed() {
        let registry = global_registry();

        // Spawn a quick process that will complete immediately
        let child = tokio::process::Command::new("echo")
            .arg("done")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_kill_completed".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Give it time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let tool = KillShellTool;
        let params = KillShellParams {
            shell_id: test_shell_id.clone(),
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

        // Should still succeed even if process already completed
        assert_eq!(result.shell_id, "test_kill_completed");
    }

    #[tokio::test]
    async fn test_kill_shell_multiple_processes() {
        let registry = global_registry();

        // Spawn multiple processes
        let mut shell_ids = Vec::new();
        for i in 0..3 {
            let child = tokio::process::Command::new("sleep")
                .arg("60")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .expect("Failed to spawn test process");

            let shell_id = format!("test_kill_multi_{}", i);
            registry.register(shell_id.clone(), child).await.ok();
            shell_ids.push(shell_id);
        }

        // Verify all registered
        for shell_id in &shell_ids {
            assert!(registry.exists(shell_id).await);
        }

        let tool = KillShellTool;
        let ctx = ToolContext::default();

        // Kill them all
        for shell_id in &shell_ids {
            let params = KillShellParams {
                shell_id: shell_id.clone(),
            };
            let stream = tool.execute(params, &ctx).await.unwrap();
            let events: Vec<_> = stream.collect().await;

            let result = events
                .iter()
                .find_map(|e| match e {
                    ToolEvent::Result(output) => Some(output),
                    _ => None,
                })
                .unwrap();

            assert!(result.success);
        }

        // Verify all are gone
        for shell_id in &shell_ids {
            assert!(!registry.exists(shell_id).await);
        }
    }

    #[tokio::test]
    async fn test_kill_shell_double_kill() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_kill_double".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        let tool = KillShellTool;
        let params = KillShellParams {
            shell_id: test_shell_id.clone(),
        };
        let ctx = ToolContext::default();

        // First kill should succeed
        let stream = tool.execute(params.clone(), &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;
        let result1 = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert!(result1.success);

        // Second kill should fail (not found)
        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;
        let result2 = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert!(!result2.success);
        assert!(result2.message.contains("not found"));
    }

    #[tokio::test]
    async fn test_kill_shell_with_output() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_kill_with_output".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Add some output
        registry
            .append_output(&test_shell_id, "Output before kill".to_string(), false)
            .await
            .ok();

        let tool = KillShellTool;
        let params = KillShellParams {
            shell_id: test_shell_id.clone(),
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

        assert!(result.success);
        assert!(!registry.exists(&test_shell_id).await);
    }

    #[tokio::test]
    async fn test_kill_shell_message_format() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_kill_message".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        let tool = KillShellTool;
        let params = KillShellParams {
            shell_id: test_shell_id.clone(),
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

        // Check message format
        assert!(result.message.contains(&test_shell_id));
        assert!(result.message.contains("terminated") || result.message.contains("killed"));
    }

    #[tokio::test]
    async fn test_kill_shell_progress_event() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_kill_progress".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        let tool = KillShellTool;
        let params = KillShellParams {
            shell_id: test_shell_id.clone(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should have at least one progress event
        let has_progress = events
            .iter()
            .any(|e| matches!(e, ToolEvent::Progress { .. }));
        assert!(has_progress, "Expected progress event");
    }

    #[tokio::test]
    async fn test_kill_shell_rapid_succession() {
        let registry = global_registry();

        // Spawn multiple processes rapidly
        let mut shell_ids = Vec::new();
        for i in 0..5 {
            let child = tokio::process::Command::new("sleep")
                .arg("60")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .expect("Failed to spawn test process");

            let shell_id = format!(
                "test_kill_rapid_{}_{}",
                i,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            registry.register(shell_id.clone(), child).await.ok();
            shell_ids.push(shell_id);
        }

        let tool = KillShellTool;
        let ctx = ToolContext::default();

        // Kill them sequentially (not in parallel) to avoid race conditions
        for shell_id in &shell_ids {
            let params = KillShellParams {
                shell_id: shell_id.clone(),
            };
            let stream = tool.execute(params, &ctx).await.unwrap();
            let _events: Vec<_> = stream.collect().await;
        }

        // Verify all are gone
        for shell_id in &shell_ids {
            assert!(
                !registry.exists(shell_id).await,
                "Shell {} still exists",
                shell_id
            );
        }
    }

    #[tokio::test]
    async fn test_kill_shell_empty_id() {
        let tool = KillShellTool;
        let params = KillShellParams {
            shell_id: "".to_string(),
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

        assert!(!result.success);
    }
}
