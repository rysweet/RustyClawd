//! AgentOutput tool - Retrieve output from background agents
//!
//! Demonstrates:
//! - Background agent management
//! - Output retrieval
//! - Agent state tracking
//! - Token usage reporting

use crate::agent_registry::global_agent_registry;
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for AgentOutput tool
#[derive(Debug, Clone, Deserialize)]
pub struct AgentOutputParams {
    /// ID of the background agent
    pub agent_id: String,
}

/// Output from AgentOutput tool
#[derive(Debug, Serialize)]
pub struct AgentOutputOutput {
    /// Response text from the agent
    pub response: String,

    /// Agent status (running, completed, failed:reason)
    pub status: String,

    /// Agent ID
    pub agent_id: String,

    /// Input tokens used
    pub input_tokens: u32,

    /// Output tokens used
    pub output_tokens: u32,

    /// Total tokens used
    pub total_tokens: u32,
}

/// The AgentOutput tool
pub struct AgentOutputTool;

#[async_trait]
impl crate::Tool for AgentOutputTool {
    type Params = AgentOutputParams;
    type Output = AgentOutputOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "AgentOutput",
            description: "Retrieves output from running background agents",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let agent_id = params.agent_id.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Retrieving output from agent: {}", agent_id),
                percentage: None,
            };

            // Get the global agent registry
            let registry = global_agent_registry();

            // Check if the agent exists
            if !registry.exists(&agent_id).await {
                yield ToolEvent::Error {
                    message: format!("Agent not found: {}", agent_id),
                };
                return;
            }

            // Get output from the registry
            let (response, status, token_usage) = match registry.get_output(&agent_id).await {
                Ok(output) => output,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to get output: {}", e),
                    };
                    return;
                }
            };

            let total_tokens = token_usage.input_tokens + token_usage.output_tokens;

            if debug {
                tracing::debug!(
                    agent_id = %agent_id,
                    response_len = response.len(),
                    status = %status,
                    input_tokens = token_usage.input_tokens,
                    output_tokens = token_usage.output_tokens,
                    "Retrieved agent output"
                );
            }

            yield ToolEvent::Result(AgentOutputOutput {
                response,
                status,
                agent_id: params.agent_id.clone(),
                input_tokens: token_usage.input_tokens,
                output_tokens: token_usage.output_tokens,
                total_tokens,
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
    async fn test_agent_output_basic() {
        let registry = global_agent_registry();

        let test_agent_id = "test_agent_output_basic".to_string();
        registry
            .register(
                test_agent_id.clone(),
                "builder".to_string(),
                "sonnet".to_string(),
            )
            .await
            .ok();

        // Add some response text
        registry
            .append_response(&test_agent_id, "Hello from agent!".to_string())
            .await
            .ok();

        let tool = AgentOutputTool;
        let params = AgentOutputParams {
            agent_id: test_agent_id.clone(),
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

        assert_eq!(result.agent_id, test_agent_id);
        assert_eq!(result.status, "running");
        assert!(result.response.contains("Hello from agent!"));
    }

    #[tokio::test]
    async fn test_agent_output_with_tokens() {
        let registry = global_agent_registry();

        let test_agent_id = "test_agent_output_tokens".to_string();
        registry
            .register(
                test_agent_id.clone(),
                "builder".to_string(),
                "sonnet".to_string(),
            )
            .await
            .ok();

        // Update token usage
        registry
            .update_token_usage(&test_agent_id, 500, 250)
            .await
            .ok();

        let tool = AgentOutputTool;
        let params = AgentOutputParams {
            agent_id: test_agent_id.clone(),
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

        assert_eq!(result.input_tokens, 500);
        assert_eq!(result.output_tokens, 250);
        assert_eq!(result.total_tokens, 750);
    }

    #[tokio::test]
    async fn test_agent_output_completed_status() {
        let registry = global_agent_registry();

        let test_agent_id = "test_agent_output_completed".to_string();
        registry
            .register(
                test_agent_id.clone(),
                "builder".to_string(),
                "sonnet".to_string(),
            )
            .await
            .ok();

        // Mark as completed
        registry.mark_completed(&test_agent_id).await.ok();

        let tool = AgentOutputTool;
        let params = AgentOutputParams {
            agent_id: test_agent_id.clone(),
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

        assert_eq!(result.status, "completed");
    }

    #[tokio::test]
    async fn test_agent_output_failed_status() {
        let registry = global_agent_registry();

        let test_agent_id = "test_agent_output_failed".to_string();
        registry
            .register(
                test_agent_id.clone(),
                "builder".to_string(),
                "sonnet".to_string(),
            )
            .await
            .ok();

        // Mark as failed
        registry
            .mark_failed(&test_agent_id, "API timeout".to_string())
            .await
            .ok();

        let tool = AgentOutputTool;
        let params = AgentOutputParams {
            agent_id: test_agent_id.clone(),
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
        assert!(result.status.contains("API timeout"));
    }

    #[tokio::test]
    async fn test_agent_output_nonexistent() {
        let tool = AgentOutputTool;
        let params = AgentOutputParams {
            agent_id: "nonexistent_agent_id".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should get an error event
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error, "Expected error event for nonexistent agent");
    }

    #[tokio::test]
    async fn test_agent_output_preserves_buffer() {
        // Unlike BashOutput which clears buffer, AgentOutput preserves it
        let registry = global_agent_registry();

        let test_agent_id = "test_agent_output_preserves".to_string();
        registry
            .register(
                test_agent_id.clone(),
                "builder".to_string(),
                "sonnet".to_string(),
            )
            .await
            .ok();

        registry
            .append_response(&test_agent_id, "Response text".to_string())
            .await
            .ok();

        let tool = AgentOutputTool;
        let params = AgentOutputParams {
            agent_id: test_agent_id.clone(),
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
        assert!(result1.response.contains("Response text"));

        // Second read should still have the response
        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;
        let result2 = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();
        assert!(result2.response.contains("Response text"));
    }
}
