//! BashOutput tool - Retrieve output from background shells
//!
//! Demonstrates:
//! - Background process management
//! - Output buffering and retrieval
//! - Regex filtering of output
//! - Shell state tracking

use crate::process_registry::global_registry;
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for BashOutput tool
#[derive(Debug, Clone, Deserialize)]
pub struct BashOutputParams {
    /// ID of the background shell
    pub bash_id: String,

    /// Optional regex filter for output lines
    #[serde(default)]
    pub filter: Option<String>,
}

/// Output from BashOutput tool
#[derive(Debug, Serialize)]
pub struct BashOutputOutput {
    /// Output from the shell (stdout)
    pub stdout: String,

    /// Error output (stderr)
    pub stderr: String,

    /// Shell status
    pub status: String,

    /// Shell ID
    pub bash_id: String,
}

/// The BashOutput tool
pub struct BashOutputTool;

#[async_trait]
impl crate::Tool for BashOutputTool {
    type Params = BashOutputParams;
    type Output = BashOutputOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "BashOutput",
            description: "Retrieves output from running background bash shells",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let bash_id = params.bash_id.clone();
        let filter = params.filter.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Retrieving output from shell: {}", bash_id),
                percentage: None,
            };

            // Get the global process registry
            let registry = global_registry();

            // Check if the process exists
            if !registry.exists(&bash_id).await {
                yield ToolEvent::Error {
                    message: format!("Shell not found: {}", bash_id),
                };
                return;
            }

            // Parse filter regex if provided
            let filter_regex = if let Some(pattern) = &filter {
                match regex::Regex::new(pattern) {
                    Ok(re) => Some(re),
                    Err(e) => {
                        if debug {
                            tracing::warn!("Invalid regex pattern: {} - {}", pattern, e);
                        }
                        yield ToolEvent::Error {
                            message: format!("Invalid regex pattern: {}", e),
                        };
                        return;
                    }
                }
            } else {
                None
            };

            // Get output from the registry
            let (stdout, stderr, status) = match registry.get_output(&bash_id, filter_regex.as_ref()).await {
                Ok(output) => output,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to get output: {}", e),
                    };
                    return;
                }
            };

            if debug {
                tracing::debug!(
                    bash_id = %bash_id,
                    filter = ?filter,
                    stdout_len = stdout.len(),
                    stderr_len = stderr.len(),
                    status = %status,
                    "Retrieved shell output"
                );
            }

            yield ToolEvent::Result(BashOutputOutput {
                stdout,
                stderr,
                status,
                bash_id: params.bash_id.clone(),
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true // Reading output doesn't modify state
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Multiple reads are safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_bash_output_basic() {
        // Register a test process first with some test output
        let registry = global_registry();

        // Create a mock process handle by spawning a simple command that exits immediately
        let child = tokio::process::Command::new("echo")
            .arg("test output")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_shell_basic".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Add some test output to the registry
        registry
            .append_output(&test_shell_id, "Line 1".to_string(), false)
            .await
            .ok();
        registry
            .append_output(&test_shell_id, "Line 2".to_string(), false)
            .await
            .ok();

        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: test_shell_id.clone(),
            filter: None,
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

        assert_eq!(result.bash_id, "test_shell_basic");
        assert_eq!(result.status, "running");
        assert!(result.stdout.contains("Line 1"));
        assert!(result.stdout.contains("Line 2"));
    }

    #[tokio::test]
    async fn test_bash_output_with_stderr() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_shell_stderr".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Add stdout and stderr
        registry
            .append_output(&test_shell_id, "Standard output".to_string(), false)
            .await
            .ok();
        registry
            .append_output(&test_shell_id, "Error message".to_string(), true)
            .await
            .ok();

        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: test_shell_id.clone(),
            filter: None,
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

        assert!(result.stdout.contains("Standard output"));
        assert!(result.stderr.contains("Error message"));

        // Clean up
        registry.kill(&test_shell_id).await.ok();
    }

    #[tokio::test]
    async fn test_bash_output_regex_filter() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_shell_filter".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Add multiple lines, only some match the filter
        registry
            .append_output(
                &test_shell_id,
                "ERROR: Something went wrong".to_string(),
                false,
            )
            .await
            .ok();
        registry
            .append_output(&test_shell_id, "INFO: Normal operation".to_string(), false)
            .await
            .ok();
        registry
            .append_output(&test_shell_id, "ERROR: Another problem".to_string(), false)
            .await
            .ok();
        registry
            .append_output(&test_shell_id, "DEBUG: Details here".to_string(), false)
            .await
            .ok();

        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: test_shell_id.clone(),
            filter: Some("ERROR:.*".to_string()),
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

        // Should only contain ERROR lines
        assert!(result.stdout.contains("ERROR: Something went wrong"));
        assert!(result.stdout.contains("ERROR: Another problem"));
        assert!(!result.stdout.contains("INFO:"));
        assert!(!result.stdout.contains("DEBUG:"));

        // Clean up
        registry.kill(&test_shell_id).await.ok();
    }

    #[tokio::test]
    async fn test_bash_output_invalid_regex() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_shell_invalid_regex".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: test_shell_id.clone(),
            filter: Some("[invalid[regex".to_string()),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should get an error event
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error, "Expected error event for invalid regex");

        // Clean up
        registry.kill(&test_shell_id).await.ok();
    }

    #[tokio::test]
    async fn test_bash_output_nonexistent_shell() {
        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: "nonexistent_shell_id".to_string(),
            filter: None,
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should get an error event
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error, "Expected error event for nonexistent shell");
    }

    #[tokio::test]
    async fn test_bash_output_buffer_cleared() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_shell_buffer".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Add output
        registry
            .append_output(&test_shell_id, "First batch".to_string(), false)
            .await
            .ok();

        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: test_shell_id.clone(),
            filter: None,
        };
        let ctx = ToolContext::default();

        // First read
        let stream = tool.execute(params.clone(), &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;
        let result1 = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert!(result1.stdout.contains("First batch"));

        // Second read should return empty (buffer was cleared)
        let stream = tool.execute(params.clone(), &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;
        let result2 = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert_eq!(result2.stdout, "");

        // Add new output
        registry
            .append_output(&test_shell_id, "Second batch".to_string(), false)
            .await
            .ok();

        // Third read should get only new output
        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;
        let result3 = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert!(result3.stdout.contains("Second batch"));
        assert!(!result3.stdout.contains("First batch"));

        // Clean up
        registry.kill(&test_shell_id).await.ok();
    }

    #[tokio::test]
    async fn test_bash_output_completed_status() {
        let registry = global_registry();
        let child = tokio::process::Command::new("echo")
            .arg("done")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_shell_completed".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Mark as completed
        registry.mark_completed(&test_shell_id, 0).await.ok();

        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: test_shell_id.clone(),
            filter: None,
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

        assert_eq!(result.status, "completed:0");
    }

    #[tokio::test]
    async fn test_bash_output_failed_status() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_shell_failed".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Mark as failed
        registry
            .mark_failed(&test_shell_id, "Test failure".to_string())
            .await
            .ok();

        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: test_shell_id.clone(),
            filter: None,
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

        assert!(result.status.starts_with("failed:"));
        assert!(result.status.contains("Test failure"));
    }

    #[tokio::test]
    async fn test_bash_output_empty_output() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_shell_empty".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Don't add any output

        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: test_shell_id.clone(),
            filter: None,
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

        assert_eq!(result.stdout, "");
        assert_eq!(result.stderr, "");

        // Clean up
        registry.kill(&test_shell_id).await.ok();
    }

    #[tokio::test]
    async fn test_bash_output_multiline() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_shell_multiline".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Add multiple lines
        for i in 1..=10 {
            registry
                .append_output(&test_shell_id, format!("Line {}", i), false)
                .await
                .ok();
        }

        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: test_shell_id.clone(),
            filter: None,
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

        // Check all lines are present
        for i in 1..=10 {
            assert!(result.stdout.contains(&format!("Line {}", i)));
        }

        // Clean up
        registry.kill(&test_shell_id).await.ok();
    }

    #[tokio::test]
    async fn test_bash_output_case_sensitive_filter() {
        let registry = global_registry();
        let child = tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn test process");

        let test_shell_id = "test_shell_case".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        registry
            .append_output(&test_shell_id, "error lowercase".to_string(), false)
            .await
            .ok();
        registry
            .append_output(&test_shell_id, "ERROR uppercase".to_string(), false)
            .await
            .ok();
        registry
            .append_output(&test_shell_id, "Error mixed".to_string(), false)
            .await
            .ok();

        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: test_shell_id.clone(),
            filter: Some("^ERROR".to_string()),
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

        // Should only match uppercase ERROR
        assert!(result.stdout.contains("ERROR uppercase"));
        assert!(!result.stdout.contains("error lowercase"));
        assert!(!result.stdout.contains("Error mixed"));

        // Clean up
        registry.kill(&test_shell_id).await.ok();
    }
}
