//! Bash tool - Execute shell commands
//!
//! This tool executes bash commands and streams their output.
//! It demonstrates:
//! - Async process spawning with Tokio
//! - Streaming stdout/stderr
//! - Timeout handling
//! - Error propagation

use crate::process_isolation::{apply_isolation, ProcessSpawnConfig};
use crate::process_registry::{global_registry, ProcessRegistry};
use crate::{ExecutionContext, ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Parameters for the Bash tool
#[derive(Debug, Deserialize)]
pub struct BashParams {
    /// The command to execute
    pub command: String,

    /// Optional timeout in milliseconds (max 600000 = 10 minutes)
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Optional description of what the command does
    #[serde(default)]
    pub description: Option<String>,

    /// Run the command in the background
    #[serde(default)]
    pub run_in_background: bool,
}

fn default_timeout() -> u64 {
    120_000 // 2 minutes default
}

/// Output from the Bash tool
#[derive(Debug, Serialize)]
pub struct BashOutput {
    /// Stdout from the command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,

    /// Stderr from the command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,

    /// Exit code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,

    /// Whether the command succeeded (exit code 0)
    pub success: bool,

    /// Shell ID (for background processes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_id: Option<String>,
}

/// The Bash tool
pub struct BashTool;

#[async_trait]
impl crate::Tool for BashTool {
    type Params = BashParams;
    type Output = BashOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "Bash",
            description: "Executes bash commands and returns output",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let command = params.command.clone();
        let timeout_duration = Duration::from_millis(params.timeout);
        let cwd = ctx.cwd.clone();
        let debug = ctx.debug;
        let run_in_background = params.run_in_background;
        let execution_context = ctx.execution_context;

        Ok(Box::pin(stream! {
            // Progress: Starting execution
            yield ToolEvent::Progress {
                step: format!("Executing: {}", command),
                percentage: None,
            };

            if debug {
                tracing::debug!(
                    command = %command,
                    timeout_ms = params.timeout,
                    cwd = ?cwd,
                    run_in_background = run_in_background,
                    "Executing bash command"
                );
            }

            // Determine if we need process isolation
            let isolation_config = if execution_context == ExecutionContext::Tui {
                ProcessSpawnConfig::with_isolation()
            } else {
                ProcessSpawnConfig::without_isolation()
            };

            // Spawn bash process with optional isolation
            let mut cmd = Command::new("bash");
            cmd.arg("-c")
                .arg(&command)
                .current_dir(&cwd)
                .stdin(std::process::Stdio::null())   // Isolate stdin - prevent terminal access
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            // Apply process isolation to prevent terminal corruption
            let mut cmd = apply_isolation(cmd, &isolation_config);

            let mut child = match cmd.spawn() {
                Ok(child) => child,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to spawn bash: {}", e),
                    };
                    return;
                }
            };

            // Handle background vs foreground execution
            if run_in_background {
                // Background mode: register and return immediately
                let registry = global_registry();
                let shell_id = ProcessRegistry::generate_id();

                // Register the process (we need to move child into registry)
                if let Err(e) = registry.register(shell_id.clone(), child).await {
                    yield ToolEvent::Error {
                        message: format!("Failed to register background process: {}", e),
                    };
                    return;
                }

                // Background process registered - output will be captured by
                // BashOutput tool when requested

                if debug {
                    tracing::debug!(
                        shell_id = %shell_id,
                        "Background process registered"
                    );
                }

                yield ToolEvent::Result(BashOutput {
                    stdout: None,
                    stderr: None,
                    exit_code: None,
                    success: true,
                    shell_id: Some(shell_id),
                });
                return;
            }

            // Get stdout and stderr
            let stdout = child.stdout.take().expect("stdout not captured");
            let stderr = child.stderr.take().expect("stderr not captured");

            // Execute with timeout
            let result = timeout(timeout_duration, async {
                // Read stdout and stderr concurrently
                let stdout_handle = tokio::spawn(async move {
                    let mut reader = BufReader::new(stdout);
                    let mut output = String::new();
                    reader.read_to_string(&mut output).await?;
                    Ok::<_, std::io::Error>(output)
                });

                let stderr_handle = tokio::spawn(async move {
                    let mut reader = BufReader::new(stderr);
                    let mut output = String::new();
                    reader.read_to_string(&mut output).await?;
                    Ok::<_, std::io::Error>(output)
                });

                // Wait for process to complete
                let status = child.wait().await?;

                // Collect outputs
                let stdout_output = stdout_handle.await
                    .map_err(std::io::Error::other)??;
                let stderr_output = stderr_handle.await
                    .map_err(std::io::Error::other)??;

                Ok::<_, std::io::Error>(BashOutput {
                    stdout: Some(stdout_output),
                    stderr: Some(stderr_output),
                    exit_code: Some(status.code().unwrap_or(-1)),
                    success: status.success(),
                    shell_id: None,
                })
            }).await;

            // Handle timeout
            let output = match result {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => {
                    yield ToolEvent::Error {
                        message: format!("Command execution failed: {}", e),
                    };
                    return;
                }
                Err(_) => {
                    yield ToolEvent::Error {
                        message: format!("Command timed out after {}ms", params.timeout),
                    };
                    return;
                }
            };

            if debug {
                tracing::debug!(
                    exit_code = ?output.exit_code,
                    success = output.success,
                    stdout_len = output.stdout.as_ref().map(|s| s.len()),
                    stderr_len = output.stderr.as_ref().map(|s| s.len()),
                    "Command completed"
                );
            }

            // Yield final result
            yield ToolEvent::Result(output);
        }))
    }

    fn is_read_only(&self) -> bool {
        false // Bash can modify system state
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Each execution is independent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_bash_simple_command() {
        let tool = BashTool;
        let params = BashParams {
            command: "echo 'Hello from Rust'".to_string(),
            timeout: 5000,
            description: None,
            run_in_background: false,
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();

        // Collect all events
        let events: Vec<_> = stream.collect().await;

        // Should have at least progress and result
        assert!(events.len() >= 2);

        // Last event should be Result
        if let ToolEvent::Result(output) = &events[events.len() - 1] {
            assert!(output.success);
            assert_eq!(output.exit_code, Some(0));
            assert!(output.stdout.as_ref().unwrap().contains("Hello from Rust"));
        } else {
            panic!("Expected ToolEvent::Result");
        }
    }

    #[tokio::test]
    async fn test_bash_stderr() {
        let tool = BashTool;
        let params = BashParams {
            command: "echo 'error' >&2".to_string(),
            timeout: 5000,
            description: None,
            run_in_background: false,
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        if let ToolEvent::Result(output) = &events[events.len() - 1] {
            assert!(output.success);
            assert!(output.stderr.as_ref().unwrap().contains("error"));
        }
    }

    #[tokio::test]
    async fn test_bash_exit_code() {
        let tool = BashTool;
        let params = BashParams {
            command: "exit 42".to_string(),
            timeout: 5000,
            description: None,
            run_in_background: false,
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        if let ToolEvent::Result(output) = &events[events.len() - 1] {
            assert!(!output.success);
            assert_eq!(output.exit_code, Some(42));
        }
    }

    #[tokio::test]
    async fn test_bash_empty_command() {
        let tool = BashTool;
        let params = BashParams {
            command: String::new(),
            timeout: 5000,
            description: None,
            run_in_background: false,
        };
        let ctx = ToolContext::default();
        let stream = tool.execute(params, &ctx).await.unwrap();
        let output: Vec<_> = stream.collect().await;
        // Empty command should succeed (bash -c "" exits 0)
        assert!(output
            .iter()
            .any(|e| matches!(e, ToolEvent::Result(o) if o.success)));
    }

    #[tokio::test]
    async fn test_bash_exit_code_propagated() {
        let tool = BashTool;
        let params = BashParams {
            command: "exit 42".to_string(),
            timeout: 5000,
            description: None,
            run_in_background: false,
        };
        let ctx = ToolContext::default();
        let stream = tool.execute(params, &ctx).await.unwrap();
        let output: Vec<_> = stream.collect().await;
        let finished = output
            .iter()
            .find_map(|e| {
                if let ToolEvent::Result(o) = e {
                    Some(o)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(finished.exit_code, Some(42));
    }

    #[tokio::test]
    async fn test_bash_stderr_captured() {
        let tool = BashTool;
        let params = BashParams {
            command: "echo stdout_msg && echo stderr_msg >&2".to_string(),
            timeout: 5000,
            description: None,
            run_in_background: false,
        };
        let ctx = ToolContext::default();
        let stream = tool.execute(params, &ctx).await.unwrap();
        let output: Vec<_> = stream.collect().await;
        let finished = output
            .iter()
            .find_map(|e| {
                if let ToolEvent::Result(o) = e {
                    Some(o)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(finished.stdout.as_ref().unwrap().contains("stdout_msg"));
        assert!(finished.stderr.as_ref().unwrap().contains("stderr_msg"));
    }
}
