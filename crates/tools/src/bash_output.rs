//! BashOutput tool - Retrieve output from background shells
//!
//! Demonstrates:
//! - Background process management
//! - Output buffering and retrieval
//! - Regex filtering of output
//! - Shell state tracking

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use crate::process_registry::global_registry;
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for BashOutput tool
#[derive(Debug, Deserialize)]
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

        let test_shell_id = "test_shell_1".to_string();
        registry.register(test_shell_id.clone(), child).await.ok();

        // Add some test output to the registry
        registry.append_output(&test_shell_id, "Line 1".to_string(), false).await.ok();
        registry.append_output(&test_shell_id, "Line 2".to_string(), false).await.ok();

        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: test_shell_id.clone(),
            filter: None,
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        }).unwrap();

        assert_eq!(result.bash_id, "test_shell_1");
        assert_eq!(result.status, "running");
        assert!(result.stdout.contains("Line 1"));
        assert!(result.stdout.contains("Line 2"));
    }
}
