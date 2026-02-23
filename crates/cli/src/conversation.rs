//! Conversation loop logic for interactive sessions
//!
//! Handles message sending, response processing, tool use loop continuation,
//! and streaming turn orchestration. Slash command dispatch lives in
//! [`crate::command_handlers`].

use crate::hooks;
use crate::hooks::NotificationType;
use crate::notification::NotificationManager;
use crate::permission_mode::PermissionMode;
use crate::plugins::mcp_proxy::McpProxy;
use crate::session::SessionStats;
use crate::session_persistence::SessionPersistence;
use crate::streaming;
use crate::tool_orchestrator;
use crate::tui::{ChatMessage, TuiState};
use anyhow::Result;
use futures::StreamExt;
use rustyclawd_core::client::{Client, Message as ApiMessage, MessageResponse};
use rustyclawd_core::{Context, Message, MessageRole};
use rustyclawd_tools::{
    bash::BashParams, BashTool, ExecutionContext, Tool, ToolContext, ToolEvent,
};
use std::sync::Arc;
use tokio::sync::Mutex;

// Re-export handle_command so that callers (interactive.rs) that import from
// `crate::conversation` continue to compile without changes.
pub(crate) use crate::command_handlers::handle_command;

use crate::commands::SlashCommands;

/// Mutable streaming-related state passed between the event loop and conversation functions.
pub(crate) struct StreamingState {
    pub rx: Option<tokio::sync::mpsc::UnboundedReceiver<streaming::StreamingChannelEvent>>,
    pub message_index: Option<usize>,
    pub response_rx: Option<tokio::sync::oneshot::Receiver<MessageResponse>>,
    pub api_messages: Vec<ApiMessage>,
}

/// Mutable state for the tool-use loop.
pub(crate) struct ToolLoopState {
    pub active_tools: std::collections::HashMap<String, String>,
    pub tool_results:
        std::collections::HashMap<String, rustyclawd_core::client::types::ContentBlock>,
    pub expected_tool_ids: Vec<String>,
    pub pending_tool_response: Option<MessageResponse>,
    pub tool_rx:
        Option<tokio::sync::mpsc::UnboundedReceiver<tool_orchestrator::ToolExecutionEvent>>,
}

/// Session-wide services and configuration that rarely change during a session.
///
/// Fields are owned/cloned to avoid borrow-checker conflicts when the caller
/// also needs mutable access to sibling fields of the same struct.
pub(crate) struct SessionServices {
    pub client: Arc<Client>,
    pub model: String,
    pub hooks: Option<Arc<hooks::HooksSystem>>,
    pub session_id: String,
    pub notification_manager: Option<NotificationManager>,
    pub permission_mode: PermissionMode,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub slash_commands: Arc<SlashCommands>,
    pub mcp_proxy: Arc<Mutex<McpProxy>>,
}

/// Helper function to get current working directory as string
fn get_cwd_string() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// Process a user message: execute hooks, add to context, and start streaming.
///
/// This is the main entry point for handling user input in the conversation.
pub(crate) async fn process_user_message(
    user_input: &str,
    skip_tui_display: bool,
    tui: &mut TuiState,
    context: &mut Context,
    services: &SessionServices,
    streaming: &mut StreamingState,
) -> Result<()> {
    tui.push_debug("[PROCESS] Starting process_user_message".to_string());

    // Check if session has been logged out
    if crate::command_handlers::is_logged_out() {
        tui.add_message(ChatMessage::system(
            "Session is logged out. No API calls will be made. Restart to log back in.".to_string(),
        ));
        return Ok(());
    }

    // Execute UserPromptSubmit hook BEFORE adding prompt to context
    if let Some(ref hooks_sys) = services.hooks {
        tui.push_debug("[PROCESS] Executing UserPromptSubmit hook".to_string());

        let hook_context = hooks::HookContext::for_user_prompt(
            services.session_id.to_string(),
            format!(".claude/sessions/{}/transcript.json", services.session_id),
            get_cwd_string(),
            "ask".to_string(),
            user_input.to_string(),
        );

        match hooks_sys
            .execute_hooks(hooks::HookEvent::UserPromptSubmit, &hook_context)
            .await
        {
            Ok(results) => {
                tui.push_debug("[PROCESS] UserPromptSubmit hook complete".to_string());
                for result in results {
                    if result.is_blocking() {
                        tui.add_message(ChatMessage::assistant(format!(
                            "Warning: Prompt blocked by hook: {}",
                            result.stderr
                        )));
                        return Ok(());
                    }
                    if !result.is_success() {
                        tracing::warn!("UserPromptSubmit hook failed: {}", result.stderr);
                    }
                }
            }
            Err(e) => {
                tui.push_debug(format!("[PROCESS] UserPromptSubmit hook error: {}", e));
                tracing::warn!("Failed to execute UserPromptSubmit hooks: {}", e);
            }
        }
    }

    tui.push_debug("[PROCESS] Adding user message to TUI".to_string());

    // Add user message to TUI (unless skipped for slash commands) and context
    if !skip_tui_display {
        tui.add_message(ChatMessage::user(user_input.to_string()));
    }
    context.add_message(Message::user(user_input.to_string()));

    tui.push_debug("[PROCESS] Starting stream_with_tools".to_string());

    // Initialize API messages for tool use loop
    streaming.api_messages = convert_messages_to_api_format(context);

    // Update status
    tui.set_status("Streaming...".to_string());

    // Stream the first turn (returns immediately, response processed via polling)
    start_streaming_turn(&services.client, &services.model, tui, streaming)?;

    tui.push_debug("[PROCESS] Completed process_user_message".to_string());

    Ok(())
}

/// Start a single streaming turn (non-blocking).
///
/// Spawns a background streaming task and stores the receivers for polling.
pub(crate) fn start_streaming_turn(
    client: &Client,
    model: &str,
    tui: &mut TuiState,
    streaming: &mut StreamingState,
) -> Result<()> {
    let (event_rx, resp_rx, msg_idx) =
        streaming::spawn_streaming_task(client, model, &streaming.api_messages, tui)?;

    streaming.rx = Some(event_rx);
    streaming.message_index = Some(msg_idx);

    tui.push_debug("[STREAM] Storing response receiver for polling".to_string());
    streaming.response_rx = Some(resp_rx);

    Ok(())
}

/// Process a completed response and continue the tool use loop if needed.
///
/// Checks for tool_use blocks in the response. If none, finalizes the turn.
/// If tools are present, spawns tool execution and stores state for continuation.
pub(crate) async fn process_response_in_tool_loop(
    response: MessageResponse,
    tui: &mut TuiState,
    context: &mut Context,
    stats: &mut SessionStats,
    tool_state: &mut ToolLoopState,
    services: &SessionServices,
) -> Result<()> {
    // Check if response contains tool use
    let mut tool_use_blocks = Vec::new();
    for block in &response.content {
        if let rustyclawd_core::client::types::ContentBlock::ToolUse { id, name, input } = block {
            tool_use_blocks.push((id.clone(), name.clone(), input.clone()));
        }
    }

    // If no tool use, we're done with this turn
    if tool_use_blocks.is_empty() {
        tui.set_status("Ready".to_string());

        let response_text = response
            .content
            .iter()
            .filter_map(|block| {
                if let rustyclawd_core::client::types::ContentBlock::Text { text } = block {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        if !response_text.is_empty() {
            if response_text.contains('?') {
                if let Some(ref notification_mgr) = services.notification_manager {
                    notification_mgr
                        .notify(
                            &services.session_id,
                            NotificationType::ElicitationDialog,
                            "AI is asking clarifying questions",
                        )
                        .await;
                }
            }

            context.add_message(Message::assistant(response_text));
        }

        return Ok(());
    }

    // Tool use present - spawn tool execution (non-blocking)
    tui.push_debug("[TOOL_LOOP] Spawning tool execution".to_string());

    // Store response for continuation after tools complete
    tool_state.pending_tool_response = Some(response);

    // Spawn tools
    let tool_services = tool_orchestrator::ToolServices {
        hooks: services.hooks.clone(),
        session_id: services.session_id.to_string(),
        notification_manager: services.notification_manager.clone(),
        permission_mode: services.permission_mode,
        allowed_tools: services.allowed_tools.to_vec(),
        disallowed_tools: services.disallowed_tools.to_vec(),
    };
    if let Some((rx, ids)) = tool_orchestrator::spawn_tools(
        tool_use_blocks,
        tui,
        stats,
        &mut tool_state.active_tools,
        &mut tool_state.tool_results,
        &tool_services,
    ) {
        tool_state.tool_rx = Some(rx);
        tool_state.expected_tool_ids = ids;
    }

    Ok(())
}

/// Convert context messages to API message format.
pub(crate) fn convert_messages_to_api_format(context: &Context) -> Vec<ApiMessage> {
    context
        .messages()
        .iter()
        .filter_map(|msg| match msg.role {
            MessageRole::User => Some(ApiMessage::user(msg.content.clone())),
            MessageRole::Assistant => Some(ApiMessage::assistant(msg.content.clone())),
            MessageRole::System => None,
        })
        .collect()
}

/// Execute a shell command directly and add to context.
pub(crate) async fn execute_shell_command(
    command: &str,
    tui: &mut TuiState,
    context: &mut Context,
    allowed_tools: &[String],
    disallowed_tools: &[String],
) -> Result<()> {
    tui.set_status(format!("Executing: {}", command));

    let ctx = ToolContext {
        cwd: std::env::current_dir().unwrap_or_default(),
        debug: false,
        metadata: serde_json::Value::Null,
        execution_context: ExecutionContext::Tui,
        allowed_tools: allowed_tools.to_vec(),
        disallowed_tools: disallowed_tools.to_vec(),
    };

    let params = BashParams {
        command: command.to_string(),
        timeout: 120_000,
        description: None,
        run_in_background: false,
    };

    let tool = BashTool;
    let mut stream = tool.execute(params, &ctx).await?;

    let mut stdout_output = String::new();
    let mut stderr_output = String::new();
    let mut exit_code = None;
    let mut success = false;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Progress { .. } => {}
            ToolEvent::Result(output) => {
                if let Some(ref stdout) = output.stdout {
                    if !stdout.is_empty() {
                        stdout_output = stdout.clone();
                    }
                }
                if let Some(ref stderr) = output.stderr {
                    if !stderr.is_empty() {
                        stderr_output = stderr.clone();
                    }
                }
                exit_code = output.exit_code;
                success = output.success;
            }
            ToolEvent::Error { message } => {
                tui.add_message(ChatMessage::system(format!("Error: {}", message)));
                return Err(anyhow::anyhow!("Command execution failed: {}", message));
            }
        }
    }

    let mut result_msg = format!("$ {}\n", command);

    if !stdout_output.is_empty() {
        result_msg.push_str(&format!("\n{}", stdout_output.trim()));
    }

    if !stderr_output.is_empty() {
        result_msg.push_str(&format!("\nStderr:\n{}", stderr_output.trim()));
    }

    if let Some(code) = exit_code {
        result_msg.push_str(&format!("\nExit code: {}", code));
    }

    tui.add_message(ChatMessage::system(result_msg.clone()));
    context.add_message(Message::user(result_msg));

    if success {
        tui.set_status("Command completed successfully".to_string());
    } else {
        tui.set_status(format!(
            "Command failed with exit code: {}",
            exit_code.unwrap_or(-1)
        ));
    }

    Ok(())
}

/// Auto-save session on exit.
pub(crate) fn auto_save_session(persistence: &mut Option<SessionPersistence>, context: &Context) {
    if let Some(ref mut persistence) = persistence {
        let messages: Vec<Message> = context.messages().to_vec();
        if let Err(e) = persistence.auto_save(&messages) {
            eprintln!("Warning: Failed to auto-save session: {}", e);
        }
    }
}
