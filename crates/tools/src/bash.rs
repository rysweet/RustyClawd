//! Bash tool - Execute shell commands
//!
//! This tool executes bash commands and streams their output.
//! It demonstrates:
//! - Async process spawning with Tokio
//! - Streaming stdout/stderr
//! - Timeout handling
//! - Error propagation

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
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
}

fn default_timeout() -> u64 {
    120_000 // 2 minutes default
}

/// Output from the Bash tool
#[derive(Debug, Serialize)]
pub struct BashOutput {
    /// Stdout from the command
    pub stdout: String,

    /// Stderr from the command
    pub stderr: String,

    /// Exit code
    pub exit_code: i32,

    /// Whether the command succeeded (exit code 0)
    pub success: bool,
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
                    "Executing bash command"
                );
            }

            // Spawn bash process
            let mut child = match Command::new("bash")
                .arg("-c")
                .arg(&command)
                .current_dir(cwd)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to spawn bash: {}", e),
                    };
                    return;
                }
            };

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
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))??;
                let stderr_output = stderr_handle.await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))??;

                Ok::<_, std::io::Error>(BashOutput {
                    stdout: stdout_output,
                    stderr: stderr_output,
                    exit_code: status.code().unwrap_or(-1),
                    success: status.success(),
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
                    exit_code = output.exit_code,
                    success = output.success,
                    stdout_len = output.stdout.len(),
                    stderr_len = output.stderr.len(),
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
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();

        // Collect all events
        let events: Vec<_> = stream.collect().await;

        // Should have at least progress and result
        assert!(events.len() >= 2);

        // Last event should be Result
        if let ToolEvent::Result(output) = &events[events.len() - 1] {
            assert!(output.success);
            assert_eq!(output.exit_code, 0);
            assert!(output.stdout.contains("Hello from Rust"));
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
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        if let ToolEvent::Result(output) = &events[events.len() - 1] {
            assert!(output.success);
            assert!(output.stderr.contains("error"));
        }
    }

    #[tokio::test]
    async fn test_bash_exit_code() {
        let tool = BashTool;
        let params = BashParams {
            command: "exit 42".to_string(),
            timeout: 5000,
            description: None,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        if let ToolEvent::Result(output) = &events[events.len() - 1] {
            assert!(!output.success);
            assert_eq!(output.exit_code, 42);
        }
    }
}
