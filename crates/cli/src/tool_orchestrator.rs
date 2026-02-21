//! Tool execution orchestration for interactive sessions
//!
//! Manages spawning tool executions in background tasks, collecting results
//! via channels, and parsing tool output for the TUI display.

use crate::hooks;
use crate::notification::NotificationManager;
use crate::permission_mode::PermissionMode;
use crate::session::SessionStats;
use crate::tool_executor;
use crate::tui::TuiState;
use std::collections::HashMap;
use std::sync::Arc;

/// Events sent from background tool execution tasks to main event loop
#[derive(Debug, Clone)]
#[allow(dead_code)] // Variants are part of the event protocol; some not yet emitted
pub(crate) enum ToolExecutionEvent {
    /// Tool execution started
    Started {
        tool_id: String,
        tool_name: String,
        params: serde_json::Value,
    },
    /// Tool execution progress (optional)
    Progress { tool_id: String, message: String },
    /// Tool execution completed successfully
    Complete {
        tool_id: String,
        result: rustyclawd_core::client::types::ContentBlock,
    },
    /// Tool execution failed
    Error { tool_id: String, error: String },
}

/// Process a single tool execution event, updating TUI and tool tracking state.
///
/// Returns `true` if the event was a Complete or Error (tool finished).
pub(crate) fn handle_tool_event(
    event: ToolExecutionEvent,
    tui: &mut TuiState,
    active_tools: &mut HashMap<String, String>,
    tool_results: &mut HashMap<String, rustyclawd_core::client::types::ContentBlock>,
) -> bool {
    match event {
        ToolExecutionEvent::Started { .. } => {
            // Started events no longer used - messages created synchronously in spawn_tools()
            false
        }
        ToolExecutionEvent::Progress { tool_id, message } => {
            if let Some(tool_name) = active_tools.get(&tool_id) {
                tui.push_debug(format!("[TOOL:{}] {}", tool_name, message));
            }
            false
        }
        ToolExecutionEvent::Complete { tool_id, result } => {
            tui.push_debug(format!("[TOOL] Complete event received for: {}", tool_id));

            let tool_result = parse_tool_result(&result, tui);

            // Finalize tool message (updates UI with result)
            tui.finalize_tool_message(&tool_id, tool_result);
            tui.push_debug(format!("[TOOL] Message finalized for: {}", tool_id));

            // Store result for tool loop continuation
            if let Some(_tool_name) = active_tools.remove(&tool_id) {
                tool_results.insert(tool_id.clone(), result);
                tui.push_debug(format!("[TOOL] Result stored for tool loop: {}", tool_id));
            }
            true
        }
        ToolExecutionEvent::Error { tool_id, error } => {
            let tool_result = crate::tui::ToolResult {
                exit_code: Some(1),
                stdout: String::new(),
                stderr: error.clone(),
                is_error: true,
                raw_content: format!("Tool execution error: {}", error),
                structured_content: None,
            };

            tui.finalize_tool_message(&tool_id, tool_result);

            if let Some(_tool_name) = active_tools.remove(&tool_id) {
                let error_result = rustyclawd_core::client::types::ContentBlock::ToolResult {
                    tool_use_id: tool_id.clone(),
                    content: vec![rustyclawd_core::client::types::ContentBlock::Text {
                        text: format!("Tool execution error: {}", error),
                    }],
                    is_error: Some(true),
                };
                tool_results.insert(tool_id, error_result);
            }
            true
        }
    }
}

/// Parse a tool result ContentBlock into a TUI-displayable ToolResult.
fn parse_tool_result(
    result: &rustyclawd_core::client::types::ContentBlock,
    tui: &mut TuiState,
) -> crate::tui::ToolResult {
    if let rustyclawd_core::client::types::ContentBlock::ToolResult {
        content, is_error, ..
    } = result
    {
        // Extract text from ContentBlocks
        let content_text = content
            .iter()
            .filter_map(|block| {
                if let rustyclawd_core::client::types::ContentBlock::Text { text } = block {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<&str>>()
            .join("");

        tui.push_debug(format!("[TOOL] Result content: {}", content_text));

        // Try to parse as JSON (for bash tools)
        let (exit_code, stdout, stderr) =
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content_text) {
                let exit_code = json
                    .get("exit_code")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);
                let stdout = json
                    .get("stdout")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let stderr = json
                    .get("stderr")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(shell_id) = json.get("shell_id").and_then(|v| v.as_str()) {
                    tui.push_debug(format!(
                        "[TOOL] Background process registered: shell_id={}",
                        shell_id
                    ));
                }

                (exit_code, stdout, stderr)
            } else {
                (None, content_text.clone(), String::new())
            };

        crate::tui::ToolResult {
            exit_code,
            stdout,
            stderr,
            is_error: is_error.unwrap_or(false),
            raw_content: content_text,
            structured_content: None,
        }
    } else {
        crate::tui::ToolResult {
            exit_code: None,
            stdout: String::new(),
            stderr: "Unexpected result format".to_string(),
            is_error: true,
            raw_content: "Unexpected result format".to_string(),
            structured_content: None,
        }
    }
}

/// Spawn tool executions in background tasks (non-blocking).
///
/// Creates tool messages in the TUI synchronously, then spawns background
/// tasks for each tool. Results are collected via the returned channel receiver.
///
/// Returns `(event_receiver, expected_tool_ids)`.
pub(crate) fn spawn_tools(
    tool_use_blocks: Vec<(String, String, serde_json::Value)>,
    tui: &mut TuiState,
    stats: &mut SessionStats,
    active_tools: &mut HashMap<String, String>,
    tool_results: &mut HashMap<String, rustyclawd_core::client::types::ContentBlock>,
    hooks: &Option<Arc<hooks::HooksSystem>>,
    session_id: &str,
    notification_manager: &Option<NotificationManager>,
    permission_mode: PermissionMode,
    allowed_tools: &[String],
    disallowed_tools: &[String],
) -> Option<(
    tokio::sync::mpsc::UnboundedReceiver<ToolExecutionEvent>,
    Vec<String>,
)> {
    if tool_use_blocks.is_empty() {
        return None;
    }

    // Create channel for tool execution events
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

    // Clear any previous tool results and store expected IDs
    tool_results.clear();
    active_tools.clear();
    let expected_tool_ids: Vec<String> = tool_use_blocks
        .iter()
        .map(|(id, _, _)| id.clone())
        .collect();

    // Create tool messages FIRST (synchronously) to avoid race conditions
    for (id, name, input) in &tool_use_blocks {
        tui.begin_tool_message(id.clone(), name.clone(), input.clone());
        active_tools.insert(id.clone(), name.clone());
        stats.add_tool_call();
    }

    // Spawn background task for each tool
    for (id, name, input) in tool_use_blocks {
        let hooks_clone = hooks.as_ref().map(Arc::clone);
        let session_id_clone = Some(session_id.to_string());
        let notification_manager_clone = notification_manager.clone();
        let allowed_tools_clone = allowed_tools.to_vec();
        let disallowed_tools_clone = disallowed_tools.to_vec();
        let tx = event_tx.clone();

        tokio::spawn(async move {
            let result = tool_executor::execute_tool_with_permission(
                name.clone(),
                input,
                permission_mode,
                tool_executor::ToolExecutionParams {
                    hooks: hooks_clone,
                    session_id: session_id_clone,
                    notification_manager: notification_manager_clone.as_ref(),
                    tool_use_id: Some(id.clone()),
                    allowed_tools: allowed_tools_clone,
                    disallowed_tools: disallowed_tools_clone,
                },
            )
            .await;

            match result {
                Ok(output) => {
                    let _ = tx.send(ToolExecutionEvent::Complete {
                        tool_id: id.clone(),
                        result: rustyclawd_core::client::types::ContentBlock::ToolResult {
                            tool_use_id: id,
                            content: vec![rustyclawd_core::client::types::ContentBlock::Text {
                                text: output.to_string(),
                            }],
                            is_error: None,
                        },
                    });
                }
                Err(e) => {
                    let _ = tx.send(ToolExecutionEvent::Error {
                        tool_id: id.clone(),
                        error: e.to_string(),
                    });
                }
            }
        });
    }

    // Drop sender so channel closes when all tools complete
    drop(event_tx);

    Some((event_rx, expected_tool_ids))
}
