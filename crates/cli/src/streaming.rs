//! Streaming message handling for interactive sessions
//!
//! Handles streaming API responses: spawning background streaming tasks,
//! processing SSE events, and building complete MessageResponse objects.

use crate::tui::TuiState;
use anyhow::Result;
use futures::StreamExt;
use rustyclawd_core::client::{
    types::ContentBlock, Client, CreateMessageRequest, MessageResponse, StreamEvent, Usage,
};

/// Events sent from background streaming task to main event loop
#[derive(Debug, Clone)]
pub(crate) enum StreamingChannelEvent {
    /// Text content delta to append
    TextDelta { text: String },
    /// Token count update (input_tokens, output_tokens)
    TokenUpdate { input: u32, output: u32 },
    /// Streaming completed successfully with final response
    Complete { response: MessageResponse },
    /// Streaming failed with error
    Error { message: String },
    /// Thinking mode update (true = thinking, false = receiving tokens)
    ThinkingUpdate { thinking: bool },
    /// Extended thinking started (ContentBlockStart::Thinking received)
    ExtendedThinkingStarted,
    /// Extended thinking content delta received (signals phase transition)
    ExtendedThinkingDelta,
    /// Extended thinking stopped (ContentBlockStop received)
    ExtendedThinkingStopped,
}

/// Begin a streaming message in the TUI and return the message index.
pub(crate) fn begin_streaming(tui: &mut TuiState) -> usize {
    tui.begin_streaming_message()
}

/// Append streaming text to an existing message in the TUI.
pub(crate) fn append_streaming(tui: &mut TuiState, message_index: usize, text: &str) {
    tui.append_to_message(message_index, text);
}

/// Finalize a streaming message in the TUI.
pub(crate) fn finalize_streaming(tui: &mut TuiState, message_index: usize) {
    tui.finalize_streaming_message(message_index);
}

/// Process a single streaming channel event, updating TUI state accordingly.
///
/// Returns `Some(response)` if the stream completed, `None` otherwise.
pub(crate) fn handle_streaming_event(
    event: StreamingChannelEvent,
    tui: &mut TuiState,
    streaming_message_index: Option<usize>,
) -> Option<MessageResponse> {
    match event {
        StreamingChannelEvent::TextDelta { text } => {
            if let Some(idx) = streaming_message_index {
                append_streaming(tui, idx, &text);
            }
            None
        }
        StreamingChannelEvent::TokenUpdate { input, output } => {
            tui.update_token_count(input, output);
            None
        }
        StreamingChannelEvent::ThinkingUpdate { thinking } => {
            if !thinking {
                tui.push_debug("[STREAMING] First token received - thinking complete".to_string());
            }
            None
        }
        StreamingChannelEvent::ExtendedThinkingStarted => {
            tui.start_extended_thinking();
            tui.push_debug("[EXTENDED_THINKING] Started".to_string());
            None
        }
        StreamingChannelEvent::ExtendedThinkingDelta => {
            tui.append_thinking_content();
            None
        }
        StreamingChannelEvent::ExtendedThinkingStopped => {
            tui.stop_extended_thinking();
            tui.push_debug("[EXTENDED_THINKING] Stopped".to_string());
            None
        }
        StreamingChannelEvent::Complete { response } => {
            if let Some(idx) = streaming_message_index {
                finalize_streaming(tui, idx);
            }
            tui.set_status("Ready".to_string());
            Some(response)
        }
        StreamingChannelEvent::Error { message } => {
            tui.add_message(crate::tui::ChatMessage::system(format!(
                "Error: {}",
                message
            )));
            tui.set_status(format!("Error: {}", message));
            None
        }
    }
}

/// Spawn a background streaming task that sends SSE events over channels.
///
/// Returns the event receiver, response oneshot receiver, and TUI message index.
/// The caller stores these for polling in the main event loop.
pub(crate) fn spawn_streaming_task(
    client: &rustyclawd_core::client::Client,
    model: &str,
    api_messages: &[rustyclawd_core::client::Message],
    tui: &mut TuiState,
) -> Result<(
    tokio::sync::mpsc::UnboundedReceiver<StreamingChannelEvent>,
    tokio::sync::oneshot::Receiver<MessageResponse>,
    usize,
)> {
    tui.push_debug("[STREAM] Starting stream_single_turn_with_messages".to_string());

    // Create channels for communication with background task
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    let (response_tx, response_rx) = tokio::sync::oneshot::channel();

    tui.push_debug("[STREAM] Channels created, preparing background task".to_string());

    // Get tool definitions
    let tools = crate::tool_definitions::get_all_tool_definitions();

    // Create API request with tools and streaming enabled
    let request = CreateMessageRequest::new(
        model.to_string(),
        api_messages.to_vec(),
        super::interactive::MAX_TOKENS,
    )
    .with_tools(tools)
    .with_temperature(1.0)
    .with_stream(true);

    let model_owned = model.to_string();

    // Clone the client for the background task — this carries the backend
    // config and Copilot auth so streaming dispatches correctly.
    let client_clone = client.clone();

    // Spawn background task for streaming
    tokio::spawn(async move {
        run_streaming_loop(event_tx, response_tx, client_clone, model_owned, request).await;
    });

    tui.push_debug("[STREAM] Background task spawned, setting up TUI".to_string());

    // Begin streaming message in TUI
    let message_index = begin_streaming(tui);

    tui.push_debug("[STREAM] Background task spawned, returning immediately".to_string());

    Ok((event_rx, response_rx, message_index))
}

/// The actual streaming loop that runs inside a spawned task.
///
/// Reads SSE events from the HTTP response and forwards them to the main loop
/// via the unbounded channel. Sends the final assembled MessageResponse via
/// the oneshot channel.
async fn run_streaming_loop(
    event_tx: tokio::sync::mpsc::UnboundedSender<StreamingChannelEvent>,
    response_tx: tokio::sync::oneshot::Sender<MessageResponse>,
    client: Client,
    model: String,
    request: CreateMessageRequest,
) {
    // Use the Client's create_message_stream which dispatches to the correct
    // backend (Anthropic SSE or Copilot OpenAI SSE).
    let mut stream = match client.create_message_stream(request).await {
        Ok(s) => s,
        Err(e) => {
            let _ = event_tx.send(StreamingChannelEvent::Error {
                message: format!("Streaming request failed: {}", e),
            });
            return;
        }
    };

    // Track response data
    let mut message_id = String::new();
    let mut response_content: Vec<ContentBlock> = Vec::new();
    let mut current_text = String::new();
    let mut current_tool_use: Option<(String, String, String)> = None; // (id, name, json)
    let mut usage = Usage {
        input_tokens: 0,
        output_tokens: 0,
        speed: None,
    };
    let mut stop_reason = None;
    let mut thinking = true;
    let mut in_thinking_block = false;

    // Process stream events and send to main loop via channel
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => match event {
                StreamEvent::MessageStart { message } => {
                    message_id = message.id.clone();
                    usage = message.usage.clone();

                    let _ = event_tx.send(StreamingChannelEvent::TokenUpdate {
                        input: message.usage.input_tokens,
                        output: message.usage.output_tokens,
                    });
                }
                StreamEvent::ContentBlockStart {
                    content_block: rustyclawd_core::client::types::ContentBlockStart::Text { .. },
                    ..
                } => {
                    // Starting a text block
                }
                StreamEvent::ContentBlockStart {
                    content_block: rustyclawd_core::client::types::ContentBlockStart::Thinking,
                    ..
                } => {
                    in_thinking_block = true;
                    let _ = event_tx.send(StreamingChannelEvent::ThinkingUpdate { thinking: true });
                    let _ = event_tx.send(StreamingChannelEvent::ExtendedThinkingStarted);
                }
                StreamEvent::ContentBlockStart {
                    content_block:
                        rustyclawd_core::client::types::ContentBlockStart::ToolUse { id, name },
                    ..
                } => {
                    current_tool_use = Some((id, name, String::new()));
                }
                StreamEvent::ContentBlockDelta {
                    delta: rustyclawd_core::client::types::ContentDelta::TextDelta { text },
                    ..
                } => {
                    let _ = event_tx.send(StreamingChannelEvent::TextDelta { text: text.clone() });

                    if thinking {
                        thinking = false;
                        let _ = event_tx
                            .send(StreamingChannelEvent::ThinkingUpdate { thinking: false });
                    }

                    current_text.push_str(&text);
                }
                StreamEvent::ContentBlockDelta {
                    delta:
                        rustyclawd_core::client::types::ContentDelta::ThinkingDelta {
                            thinking: thinking_text,
                        },
                    ..
                } => {
                    let _ = event_tx.send(StreamingChannelEvent::ExtendedThinkingDelta);
                    let _ = event_tx.send(StreamingChannelEvent::TextDelta {
                        text: thinking_text.clone(),
                    });
                    current_text.push_str(&thinking_text);
                }
                StreamEvent::ContentBlockDelta {
                    delta: rustyclawd_core::client::types::ContentDelta::SignatureDelta { .. },
                    ..
                } => {
                    // Signature delta - not displayed to users
                }
                StreamEvent::ContentBlockDelta {
                    delta:
                        rustyclawd_core::client::types::ContentDelta::InputJsonDelta { partial_json },
                    ..
                } => {
                    if let Some((_, _, ref mut json)) = current_tool_use {
                        json.push_str(&partial_json);
                    }
                }
                StreamEvent::ContentBlockStop { .. } => {
                    if in_thinking_block {
                        in_thinking_block = false;
                        let _ = event_tx.send(StreamingChannelEvent::ExtendedThinkingStopped);
                    }

                    if !current_text.is_empty() {
                        response_content.push(ContentBlock::Text {
                            text: current_text.clone(),
                        });
                        current_text.clear();
                    }

                    if let Some((id, name, json)) = current_tool_use.take() {
                        match serde_json::from_str(&json) {
                            Ok(input) => {
                                response_content.push(ContentBlock::ToolUse { id, name, input });
                            }
                            Err(e) => {
                                let _ = event_tx.send(StreamingChannelEvent::Error {
                                    message: format!("Failed to parse tool input JSON: {}", e),
                                });
                                return;
                            }
                        }
                    }
                }
                StreamEvent::MessageDelta {
                    delta,
                    usage: usage_delta,
                } => {
                    stop_reason = delta.stop_reason.clone();
                    usage = usage_delta.clone();

                    let _ = event_tx.send(StreamingChannelEvent::TokenUpdate {
                        input: usage.input_tokens,
                        output: usage.output_tokens,
                    });
                }
                StreamEvent::MessageStop => {
                    break;
                }
                StreamEvent::Ping => {
                    // Keep-alive, ignore
                }
                StreamEvent::Error { error } => {
                    let _ = event_tx.send(StreamingChannelEvent::Error {
                        message: format!("API error: {}", error.message),
                    });
                    return;
                }
            },
            Err(e) => {
                let _ = event_tx.send(StreamingChannelEvent::Error {
                    message: format!("Stream error: {}", e),
                });
                return;
            }
        }
    }

    // Build complete response
    let response = MessageResponse {
        id: message_id,
        type_field: "message".to_string(),
        role: rustyclawd_core::client::Role::Assistant,
        content: response_content,
        model,
        stop_reason,
        stop_sequence: None,
        usage,
    };

    // Send complete response via oneshot channel
    let _ = response_tx.send(response.clone());

    // Send completion event via unbounded channel
    let _ = event_tx.send(StreamingChannelEvent::Complete { response });
}
