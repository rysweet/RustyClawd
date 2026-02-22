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
#[path = "kill_shell_tests.rs"]
mod tests;
