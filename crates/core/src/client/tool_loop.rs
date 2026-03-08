//! Tool execution loop for the Anthropic API client.
//!
//! Implements `execute_with_tools` which handles the full tool use protocol:
//! send request -> check for tool_use blocks -> execute tools -> send results -> repeat.
//!
//! Also provides `execute_with_tools_and_events` for SDK-compatible per-turn
//! streaming, emitting [`ToolLoopEvent`]s at each stage of the loop.

use super::error::{ClientError, ClientResult};
use super::types::{ContentBlock, Message, Role};
use super::{request::CreateMessageRequest, response::MessageResponse, Client};

/// Events emitted during tool execution loop.
///
/// Consumers (e.g. the `stream-json` output mode) receive these events to emit
/// SDK-compatible newline-delimited JSON messages during execution rather than
/// waiting for the final response.
#[derive(Debug, Clone)]
pub enum ToolLoopEvent {
    /// Assistant responded (may contain text and/or tool_use blocks).
    AssistantMessage(MessageResponse),
    /// A tool invocation is about to begin.
    ToolUse { id: String, name: String },
    /// A tool returned a result.
    ToolResult { tool_use_id: String, is_error: bool },
}

impl Client {
    /// Execute a message with automatic tool calling loop
    ///
    /// This method handles the full tool use protocol:
    /// 1. Sends initial request with tools
    /// 2. If Claude returns tool_use blocks, executes them
    /// 3. Sends tool results back to Claude
    /// 4. Repeats until Claude returns a text response
    ///
    /// # Arguments
    ///
    /// * `request` - Initial request with tools configured
    /// * `tool_executor` - Callback to execute tool calls
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rustyclawd_core::client::{Client, Config, CreateMessageRequest, Message};
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = Config::from_default_location().await?;
    ///     let client = Client::new(config)?;
    ///
    ///     let request = CreateMessageRequest::new(
    ///         "claude-sonnet-4-5-20250929",
    ///         vec![Message::user("Run ls command")],
    ///         4096,
    ///     );
    ///
    ///     let response = client.execute_with_tools(request, |tool_name, tool_input| async move {
    ///         // Execute tool and return result as JSON
    ///         Ok(serde_json::json!({"output": "file1.txt\nfile2.txt"}))
    ///     }).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn execute_with_tools<F, Fut>(
        &self,
        mut request: CreateMessageRequest,
        tool_executor: F,
    ) -> ClientResult<MessageResponse>
    where
        F: Fn(String, serde_json::Value) -> Fut,
        Fut: std::future::Future<Output = ClientResult<serde_json::Value>>,
    {
        // High limit for complex agentic workflows
        const MAX_ITERATIONS: usize = 10_000;
        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration > MAX_ITERATIONS {
                return Err(ClientError::Unknown(
                    "Tool execution exceeded maximum iterations".to_string(),
                ));
            }

            // Execute the request
            let response = self.create_message(request.clone()).await?;

            // Check if response contains tool use
            let mut has_tool_use = false;
            let mut tool_result_blocks = Vec::new();

            for block in &response.content {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    has_tool_use = true;

                    // Log tool invocation details
                    tracing::debug!(tool = %name, "Invoking tool");
                    if let Ok(pretty_input) = serde_json::to_string_pretty(input) {
                        tracing::debug!(tool = %name, input = %pretty_input, "Tool input");
                    }

                    // Execute the tool
                    match tool_executor(name.clone(), input.clone()).await {
                        Ok(result) => {
                            // Log tool result
                            if let Ok(pretty_result) = serde_json::to_string_pretty(&result) {
                                tracing::debug!(tool = %name, result = %pretty_result, "Tool result");
                            }

                            tool_result_blocks.push(ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: vec![ContentBlock::Text {
                                    text: result.to_string(),
                                }],
                                is_error: None,
                            });
                        }
                        Err(e) => {
                            // Log tool error
                            tracing::warn!(tool = %name, error = %e, "Tool execution failed");

                            tool_result_blocks.push(ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: vec![ContentBlock::Text {
                                    text: format!("Tool execution error: {}", e),
                                }],
                                is_error: Some(true),
                            });
                        }
                    }
                }
            }

            // If no tool use, we're done
            if !has_tool_use {
                return Ok(response);
            }

            // Build the next request with tool results
            // First, add the assistant's response with tool_use blocks to conversation
            request.messages.push(Message::with_blocks(
                Role::Assistant,
                response.content.clone(),
            ));

            // Then add tool results as user message with tool_result blocks
            request
                .messages
                .push(Message::with_blocks(Role::User, tool_result_blocks));
        }
    }

    /// Execute with tools, calling `on_event` for each turn.
    ///
    /// Behaves identically to [`execute_with_tools`](Self::execute_with_tools)
    /// but emits [`ToolLoopEvent`]s so callers can stream SDK-compatible
    /// messages during execution rather than waiting for the final response.
    ///
    /// Returns `(MessageResponse, num_turns)` where `num_turns` is the number
    /// of API round-trips performed.
    pub async fn execute_with_tools_and_events<F, Fut, E, EFut>(
        &self,
        mut request: CreateMessageRequest,
        tool_executor: F,
        on_event: E,
    ) -> ClientResult<(MessageResponse, u32)>
    where
        F: Fn(String, serde_json::Value) -> Fut,
        Fut: std::future::Future<Output = ClientResult<serde_json::Value>>,
        E: Fn(ToolLoopEvent) -> EFut,
        EFut: std::future::Future<Output = ()>,
    {
        const MAX_ITERATIONS: usize = 10_000;
        let mut iteration = 0;
        let mut num_turns: u32 = 0;

        loop {
            iteration += 1;
            if iteration > MAX_ITERATIONS {
                return Err(ClientError::Unknown(
                    "Tool execution exceeded maximum iterations".to_string(),
                ));
            }

            let response = self.create_message(request.clone()).await?;
            num_turns += 1;

            // Check if response contains tool use
            let mut has_tool_use = false;
            let mut tool_result_blocks = Vec::new();

            // Emit the assistant message for every turn
            on_event(ToolLoopEvent::AssistantMessage(response.clone())).await;

            for block in &response.content {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    has_tool_use = true;

                    tracing::debug!(tool = %name, "Invoking tool");
                    if let Ok(pretty_input) = serde_json::to_string_pretty(input) {
                        tracing::debug!(tool = %name, input = %pretty_input, "Tool input");
                    }

                    // Emit ToolUse event before execution
                    on_event(ToolLoopEvent::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                    })
                    .await;

                    match tool_executor(name.clone(), input.clone()).await {
                        Ok(result) => {
                            if let Ok(pretty_result) = serde_json::to_string_pretty(&result) {
                                tracing::debug!(tool = %name, result = %pretty_result, "Tool result");
                            }

                            // Emit ToolResult event
                            on_event(ToolLoopEvent::ToolResult {
                                tool_use_id: id.clone(),
                                is_error: false,
                            })
                            .await;

                            tool_result_blocks.push(ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: vec![ContentBlock::Text {
                                    text: result.to_string(),
                                }],
                                is_error: None,
                            });
                        }
                        Err(e) => {
                            tracing::warn!(tool = %name, error = %e, "Tool execution failed");

                            on_event(ToolLoopEvent::ToolResult {
                                tool_use_id: id.clone(),
                                is_error: true,
                            })
                            .await;

                            tool_result_blocks.push(ContentBlock::ToolResult {
                                tool_use_id: id.clone(),
                                content: vec![ContentBlock::Text {
                                    text: format!("Tool execution error: {}", e),
                                }],
                                is_error: Some(true),
                            });
                        }
                    }
                }
            }

            if !has_tool_use {
                return Ok((response, num_turns));
            }

            request.messages.push(Message::with_blocks(
                Role::Assistant,
                response.content.clone(),
            ));
            request
                .messages
                .push(Message::with_blocks(Role::User, tool_result_blocks));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::response::{MessageResponse, Usage};

    fn make_text_response(text: &str) -> MessageResponse {
        MessageResponse {
            id: "msg_test".to_string(),
            type_field: "message".to_string(),
            role: Role::Assistant,
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
            model: "claude-sonnet-4-6".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: Usage {
                input_tokens: 10,
                output_tokens: 20,
                speed: None,
            },
        }
    }

    #[test]
    fn tool_loop_event_debug_and_clone() {
        let resp = make_text_response("hello");
        let event = ToolLoopEvent::AssistantMessage(resp);
        let cloned = event.clone();
        // Verify Debug impl doesn't panic
        let _ = format!("{:?}", cloned);
    }

    #[test]
    fn tool_loop_event_variants() {
        let assistant = ToolLoopEvent::AssistantMessage(make_text_response("hi"));
        assert!(matches!(assistant, ToolLoopEvent::AssistantMessage(_)));

        let tool_use = ToolLoopEvent::ToolUse {
            id: "tu_1".to_string(),
            name: "Bash".to_string(),
        };
        assert!(matches!(tool_use, ToolLoopEvent::ToolUse { .. }));

        let tool_result = ToolLoopEvent::ToolResult {
            tool_use_id: "tu_1".to_string(),
            is_error: false,
        };
        assert!(matches!(
            tool_result,
            ToolLoopEvent::ToolResult { is_error: false, .. }
        ));
    }

    #[test]
    fn tool_loop_event_error_result() {
        let event = ToolLoopEvent::ToolResult {
            tool_use_id: "tu_err".to_string(),
            is_error: true,
        };
        match event {
            ToolLoopEvent::ToolResult {
                tool_use_id,
                is_error,
            } => {
                assert_eq!(tool_use_id, "tu_err");
                assert!(is_error);
            }
            _ => panic!("Expected ToolResult"),
        }
    }
}
