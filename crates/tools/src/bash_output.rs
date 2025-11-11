//! BashOutput tool - Retrieve output from background shells
//!
//! Demonstrates:
//! - Background process management
//! - Output buffering and retrieval
//! - Regex filtering of output
//! - Shell state tracking

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
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

            // In a full implementation, this would:
            // 1. Look up the background shell process by ID
            // 2. Read buffered output from it
            // 3. Apply regex filter if provided
            // 4. Return status and output

            // Simplified implementation: Simulates background shell
            let stdout = format!("Output from shell {}\n", bash_id);
            let stderr = String::new();
            let status = "running".to_string();

            // Apply filter if provided
            let filtered_stdout = if let Some(pattern) = &filter {
                if let Ok(re) = regex::Regex::new(pattern) {
                    stdout.lines()
                        .filter(|line| re.is_match(line))
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    if debug {
                        tracing::warn!("Invalid regex pattern: {}", pattern);
                    }
                    stdout
                }
            } else {
                stdout
            };

            if debug {
                tracing::debug!(
                    bash_id = %bash_id,
                    filter = ?filter,
                    output_len = filtered_stdout.len(),
                    "Retrieved shell output"
                );
            }

            yield ToolEvent::Result(BashOutputOutput {
                stdout: filtered_stdout,
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
        let tool = BashOutputTool;
        let params = BashOutputParams {
            bash_id: "test_shell_1".to_string(),
            filter: None,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        }).unwrap();

        assert_eq!(result.bash_id, "test_shell_1");
        assert_eq!(result.status, "running");
    }
}
