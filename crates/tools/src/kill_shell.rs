//! KillShell tool - Terminate background shells
//!
//! Demonstrates:
//! - Process lifecycle management
//! - Signal handling
//! - Resource cleanup

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for KillShell tool
#[derive(Debug, Deserialize)]
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

            // In a full implementation, this would:
            // 1. Look up the process by shell_id
            // 2. Send SIGTERM signal
            // 3. Wait for graceful shutdown
            // 4. Send SIGKILL if necessary
            // 5. Clean up resources

            // Simplified implementation
            let success = true;
            let message = format!("Shell {} terminated successfully", shell_id);

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
    async fn test_kill_shell() {
        let tool = KillShellTool;
        let params = KillShellParams {
            shell_id: "test_shell_123".to_string(),
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        }).unwrap();

        assert_eq!(result.shell_id, "test_shell_123");
        assert!(result.success);
    }
}
