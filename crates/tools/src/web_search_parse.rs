//! Search result parsing extracted from web_search streaming logic.
//!
//! Parses Claude API streaming events to extract web search results.

use crate::web_search::{SearchHit, SearchResultBlock};
use futures::{Stream, StreamExt};
use rustyclawd_core::client::types::{ContentBlockStart, ContentDelta, StreamEvent};
use std::fmt::Display;

/// Accumulated state while parsing a stream of search events.
#[derive(Debug, Default)]
struct ParseState {
    results: Vec<SearchResultBlock>,
    current_tool_id: Option<String>,
    current_tool_name: Option<String>,
    accumulated_json: String,
}

/// Parse search results from a Claude API event stream.
///
/// Consumes the stream of `StreamEvent` items and returns a list of
/// `SearchResultBlock` values found inside `web_search` tool-use blocks.
///
/// The error type `E` only needs `Display` so it can be formatted into
/// the returned `String` error.
pub async fn parse_search_results<S, E>(
    stream: S,
    debug: bool,
) -> Result<Vec<SearchResultBlock>, String>
where
    S: Stream<Item = Result<StreamEvent, E>> + Unpin,
    E: Display,
{
    let mut state = ParseState::default();

    tokio::pin!(stream);

    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(StreamEvent::ContentBlockStart { content_block, .. }) => {
                handle_block_start(&mut state, content_block, debug);
            }
            Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                handle_block_delta(&mut state, delta, debug);
            }
            Ok(StreamEvent::ContentBlockStop { .. }) => {
                handle_block_stop(&mut state, debug);
            }
            Ok(StreamEvent::MessageStop) => {
                if debug {
                    tracing::debug!("MessageStop - search complete");
                }
                break;
            }
            Ok(StreamEvent::Error { error }) => {
                return Err(format!("API error: {}", error.message));
            }
            Err(e) => {
                return Err(format!("Stream error: {}", e));
            }
            _ => {
                // Ignore other event types (Ping, MessageDelta, etc.)
            }
        }
    }

    Ok(state.results)
}

fn handle_block_start(state: &mut ParseState, content_block: ContentBlockStart, debug: bool) {
    if debug {
        tracing::debug!("ContentBlockStart: {:?}", content_block);
    }
    if let ContentBlockStart::ToolUse { id, name } = content_block {
        if name == "web_search" {
            state.current_tool_id = Some(id);
            state.current_tool_name = Some(name);
            state.accumulated_json.clear();
        }
    }
}

fn handle_block_delta(state: &mut ParseState, delta: ContentDelta, debug: bool) {
    if debug {
        tracing::trace!("ContentBlockDelta: {:?}", delta);
    }
    if let ContentDelta::InputJsonDelta { partial_json } = delta {
        if state.current_tool_name.as_deref() == Some("web_search") {
            state.accumulated_json.push_str(&partial_json);
        }
    }
}

fn handle_block_stop(state: &mut ParseState, debug: bool) {
    if debug {
        tracing::debug!(
            "ContentBlockStop - accumulated JSON length: {}",
            state.accumulated_json.len()
        );
    }

    if let (Some(tool_id), Some(tool_name)) = (&state.current_tool_id, &state.current_tool_name) {
        if tool_name == "web_search" && !state.accumulated_json.is_empty() {
            match serde_json::from_str::<serde_json::Value>(&state.accumulated_json) {
                Ok(json) => {
                    if debug {
                        tracing::debug!("Parsed search JSON successfully");
                    }

                    let search_hits = extract_hits_from_json(&json);

                    if !search_hits.is_empty() {
                        state.results.push(SearchResultBlock {
                            tool_use_id: tool_id.clone(),
                            content: search_hits,
                        });
                    }
                }
                Err(e) => {
                    if debug {
                        tracing::warn!(
                            "Failed to parse search JSON: {} - JSON preview: {}...",
                            e,
                            state.accumulated_json.chars().take(100).collect::<String>()
                        );
                    }
                }
            }

            state.accumulated_json.clear();
        }
    }

    state.current_tool_id = None;
    state.current_tool_name = None;
}

/// Extract `SearchHit` values from parsed JSON containing a `"results"` array.
fn extract_hits_from_json(json: &serde_json::Value) -> Vec<SearchHit> {
    json.get("results")
        .and_then(|v| v.as_array())
        .map(|results_array| {
            results_array
                .iter()
                .filter_map(|item| {
                    let title = item.get("title")?.as_str()?.to_string();
                    let url = item.get("url")?.as_str()?.to_string();
                    let snippet = item
                        .get("snippet")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());
                    Some(SearchHit {
                        title,
                        url,
                        snippet,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
