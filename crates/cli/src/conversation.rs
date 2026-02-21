//! Conversation loop logic for interactive sessions
//!
//! Handles message sending, response processing, tool use loop continuation,
//! slash command dispatch, and the main REPL command handlers.

use crate::commands::SlashCommands;
use crate::hooks;
use crate::hooks::NotificationType;
use crate::mcp_commands;
use crate::notification::NotificationManager;
use crate::plugins::mcp_proxy::McpProxy;
use crate::session::SessionStats;
use crate::session_persistence::SessionPersistence;
use crate::streaming;
use crate::tool_orchestrator;
use crate::tui::{ChatMessage, TuiState};
use anyhow::Result;
use futures::StreamExt;
use rustyclawd_core::client::{Client, ClientError, Message as ApiMessage, MessageResponse};
use rustyclawd_core::{Context, Message, MessageRole};
use rustyclawd_tools::{
    bash::BashParams, BashTool, ExecutionContext, Tool, ToolContext, ToolEvent,
};
use std::sync::Arc;
use tokio::sync::Mutex;

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
    hooks: &Option<Arc<hooks::HooksSystem>>,
    session_id: &str,
    _notification_manager: &Option<NotificationManager>,
    client: &Client,
    model: &str,
    api_messages: &mut Vec<ApiMessage>,
    streaming_rx: &mut Option<
        tokio::sync::mpsc::UnboundedReceiver<streaming::StreamingChannelEvent>,
    >,
    streaming_message_index: &mut Option<usize>,
    response_rx: &mut Option<tokio::sync::oneshot::Receiver<MessageResponse>>,
) -> Result<()> {
    tui.push_debug("[PROCESS] Starting process_user_message".to_string());

    // Execute UserPromptSubmit hook BEFORE adding prompt to context
    if let Some(ref hooks_sys) = hooks {
        tui.push_debug("[PROCESS] Executing UserPromptSubmit hook".to_string());

        let hook_context = hooks::HookContext::for_user_prompt(
            session_id.to_string(),
            format!(".claude/sessions/{}/transcript.json", session_id),
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
    *api_messages = convert_messages_to_api_format(context);

    // Update status
    tui.set_status("Streaming...".to_string());

    // Stream the first turn (returns immediately, response processed via polling)
    start_streaming_turn(
        client,
        model,
        api_messages,
        tui,
        streaming_rx,
        streaming_message_index,
        response_rx,
    )?;

    tui.push_debug("[PROCESS] Completed process_user_message".to_string());

    Ok(())
}

/// Start a single streaming turn (non-blocking).
///
/// Spawns a background streaming task and stores the receivers for polling.
pub(crate) fn start_streaming_turn(
    client: &Client,
    model: &str,
    api_messages: &[ApiMessage],
    tui: &mut TuiState,
    streaming_rx: &mut Option<
        tokio::sync::mpsc::UnboundedReceiver<streaming::StreamingChannelEvent>,
    >,
    streaming_message_index: &mut Option<usize>,
    response_rx: &mut Option<tokio::sync::oneshot::Receiver<MessageResponse>>,
) -> Result<()> {
    let (event_rx, resp_rx, msg_idx) =
        streaming::spawn_streaming_task(client, model, api_messages, tui)?;

    *streaming_rx = Some(event_rx);
    *streaming_message_index = Some(msg_idx);

    tui.push_debug("[STREAM] Storing response receiver for polling".to_string());
    *response_rx = Some(resp_rx);

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
    active_tools: &mut std::collections::HashMap<String, String>,
    tool_results: &mut std::collections::HashMap<
        String,
        rustyclawd_core::client::types::ContentBlock,
    >,
    expected_tool_ids: &mut Vec<String>,
    pending_tool_response: &mut Option<MessageResponse>,
    tool_rx: &mut Option<
        tokio::sync::mpsc::UnboundedReceiver<tool_orchestrator::ToolExecutionEvent>,
    >,
    hooks: &Option<Arc<hooks::HooksSystem>>,
    session_id: &str,
    notification_manager: &Option<NotificationManager>,
    permission_mode: crate::permission_mode::PermissionMode,
    allowed_tools: &[String],
    disallowed_tools: &[String],
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
                if let Some(ref notification_mgr) = notification_manager {
                    notification_mgr
                        .notify(
                            session_id,
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
    *pending_tool_response = Some(response);

    // Spawn tools
    if let Some((rx, ids)) = tool_orchestrator::spawn_tools(
        tool_use_blocks,
        tui,
        stats,
        active_tools,
        tool_results,
        hooks,
        session_id,
        notification_manager,
        permission_mode,
        allowed_tools,
        disallowed_tools,
    ) {
        *tool_rx = Some(rx);
        *expected_tool_ids = ids;
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

/// Handle special slash commands.
///
/// Returns `Ok(true)` if the command was handled, `Ok(false)` if the input
/// should be processed as a regular message.
pub(crate) async fn handle_command(
    input: &str,
    tui: &mut TuiState,
    context: &mut Context,
    hooks: &Option<Arc<hooks::HooksSystem>>,
    session_id: &str,
    stats: &mut SessionStats,
    model: &str,
    persistence: &mut Option<SessionPersistence>,
    slash_commands: &Arc<SlashCommands>,
    mcp_proxy: &Arc<Mutex<McpProxy>>,
    notification_manager: &Option<NotificationManager>,
    client: &Client,
    api_messages: &mut Vec<ApiMessage>,
    streaming_rx: &mut Option<
        tokio::sync::mpsc::UnboundedReceiver<streaming::StreamingChannelEvent>,
    >,
    streaming_message_index: &mut Option<usize>,
    response_rx: &mut Option<tokio::sync::oneshot::Receiver<MessageResponse>>,
    allowed_tools: &[String],
    disallowed_tools: &[String],
) -> Result<bool> {
    // Handle "!" prefix for direct shell execution
    if let Some(stripped) = input.strip_prefix('!') {
        let command = stripped.trim();
        if command.is_empty() {
            tui.add_message(ChatMessage::system(
                "Error: No command specified after '!'".to_string(),
            ));
            return Ok(true);
        }

        execute_shell_command(command, tui, context, allowed_tools, disallowed_tools).await?;
        return Ok(true);
    }

    match input {
        "/exit" | "/quit" => {
            // Execute Stop hook to check if exit should be allowed
            if let Some(ref hooks_sys) = hooks {
                let hook_context = hooks::HookContext::for_session(
                    session_id.to_string(),
                    format!(".claude/sessions/{}/transcript.json", session_id),
                    get_cwd_string(),
                    "ask".to_string(),
                    hooks::HookEvent::Stop,
                );

                match hooks_sys
                    .execute_hooks(hooks::HookEvent::Stop, &hook_context)
                    .await
                {
                    Ok(results) => {
                        for result in results {
                            if let Some(output) = result.parse_output() {
                                if let Some(decision) = output.decision {
                                    if decision == hooks::types::StopDecision::Block {
                                        let reason = output
                                            .reason
                                            .unwrap_or_else(|| "Stop blocked by hook".to_string());
                                        tui.add_message(ChatMessage::system(format!(
                                            "Exit blocked: {}",
                                            reason
                                        )));
                                        return Ok(true);
                                    }
                                }
                            }
                            if !result.is_success() {
                                tracing::warn!("Stop hook failed: {}", result.stderr);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to execute Stop hooks: {}", e);
                    }
                }
            }

            // Auto-save before exit
            auto_save_session(persistence, context);

            tui.cleanup()?;
            println!("\nGoodbye, matey! Fair winds and following seas! \u{26F5}");
            std::process::exit(0);
        }
        "/clear" => {
            *context = Context::new();
            tui.add_message(ChatMessage::system(
                "Conversation history cleared".to_string(),
            ));
            tui.set_status("Conversation cleared".to_string());
            return Ok(true);
        }
        "/compact" => {
            if let Some(ref hooks_sys) = hooks {
                let hook_context = hooks::HookContext::for_session(
                    session_id.to_string(),
                    format!(".claude/sessions/{}/transcript.json", session_id),
                    get_cwd_string(),
                    "ask".to_string(),
                    hooks::HookEvent::PreCompact,
                );

                match hooks_sys
                    .execute_hooks(hooks::HookEvent::PreCompact, &hook_context)
                    .await
                {
                    Ok(results) => {
                        for result in results {
                            if !result.is_success() {
                                tui.add_message(ChatMessage::system(format!(
                                    "\u{26A0}\u{FE0F}  PreCompact hook failed: {}",
                                    result.stderr
                                )));
                                return Ok(true);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("PreCompact hook execution failed: {:?}", e);
                        tui.add_message(ChatMessage::system(format!(
                            "\u{26A0}\u{FE0F}  Failed to execute PreCompact hooks: {}",
                            e
                        )));
                        return Ok(true);
                    }
                }
            }

            tui.add_message(ChatMessage::system(
                "\u{2713} PreCompact hook fired.\n\nCompacting conversation history...\n(Full compaction logic awaits implementation)".to_string(),
            ));
            return Ok(true);
        }
        "/help" => {
            let custom_commands = slash_commands.list_commands();
            let mut help_text = "Built-in Commands:\n  /exit, /quit - Exit the session\n  /clear - Clear conversation history\n  /compact - Compact conversation history (fires PreCompact hook)\n  /help - Show this help\n  /stats - Show session statistics\n  /save [description] - Save checkpoint\n  /load <checkpoint_id> - Load checkpoint\n  /sessions - List available sessions\n  !<command> - Execute shell command directly\n\nMCP Commands:\n  /mcp-list - List all MCP servers\n  /mcp-start <server-id> - Start an MCP server\n  /mcp-stop <server-id> - Stop an MCP server\n  /mcp-tools <server-id> - List tools from server\n  /mcp-status <server-id> - Show server status\n".to_string();

            if !custom_commands.is_empty() {
                help_text.push_str("\nCustom Commands:\n");
                for cmd in custom_commands {
                    help_text.push_str(&format!("  /{}\n", cmd));
                }
            }

            help_text.push_str("\nPress Ctrl+C or Ctrl+D to exit.");

            tui.add_message(ChatMessage::system(help_text));
            return Ok(true);
        }
        "/stats" => {
            stats.update_duration();

            let stats_text = format!(
                "Session Statistics:\n\
                 Messages: {} ({} user, {} assistant)\n\
                 Input tokens: {}\n\
                 Output tokens: {}\n\
                 Total tokens: {}\n\
                 Tool calls: {}\n\
                 Model: {}\n\
                 Duration: {}s",
                stats.message_count,
                stats.user_message_count,
                stats.assistant_message_count,
                stats.input_tokens,
                stats.output_tokens,
                stats.total_tokens,
                stats.tool_calls,
                model,
                stats.duration_seconds
            );
            tui.add_message(ChatMessage::system(stats_text));
            return Ok(true);
        }
        "/cost" => {
            handle_cost_command(tui, stats);
            return Ok(true);
        }
        "/context" => {
            handle_context_command(tui, stats, model);
            return Ok(true);
        }
        "/usage" => {
            handle_usage_command(tui, stats);
            return Ok(true);
        }
        "/bashes" => {
            handle_bashes_command(tui).await?;
            return Ok(true);
        }
        _ if input.starts_with("/save") => {
            handle_save_command(input, tui, context, persistence)?;
            return Ok(true);
        }
        _ if input.starts_with("/load") => {
            handle_load_command(input, tui, context, persistence)?;
            return Ok(true);
        }
        "/sessions" => {
            handle_sessions_command(tui, persistence)?;
            return Ok(true);
        }
        _ if input.starts_with("/mcp-") => {
            if let Some((command, args)) = mcp_commands::parse_slash_command(input) {
                tui.set_status(format!("Executing MCP command: {}", input));

                match mcp_commands::handle_tui_command(mcp_proxy.clone(), &command, args).await {
                    Ok(output) => {
                        tui.add_message(ChatMessage::system(output));
                        tui.set_status("Ready".to_string());
                    }
                    Err(e) => {
                        tui.add_message(ChatMessage::system(format!("Error: {}", e)));
                        tui.set_status(format!("Error: {}", e));
                    }
                }
                return Ok(true);
            }
        }
        _ if input.starts_with('/') => {
            let command_name = input[1..].split_whitespace().next().unwrap_or("");

            if slash_commands.has_command(command_name) {
                if !slash_commands.should_intercept_locally(command_name) {
                    return Ok(false);
                }

                tui.set_status(format!("Executing command: {}", input));

                match slash_commands.execute(input).await {
                    Ok(result) => {
                        tui.add_message(ChatMessage::user(input.to_string()));
                        tui.add_message(ChatMessage::system(result.expanded_prompt.clone()));
                        context.add_message(Message::user(result.expanded_prompt.clone()));

                        // Process the expanded prompt
                        if let Err(e) = process_user_message(
                            &result.expanded_prompt,
                            true,
                            tui,
                            context,
                            hooks,
                            session_id,
                            notification_manager,
                            client,
                            model,
                            api_messages,
                            streaming_rx,
                            streaming_message_index,
                            response_rx,
                        )
                        .await
                        {
                            tui.add_message(ChatMessage::system(format!(
                                "Error processing command: {}",
                                e
                            )));
                        }
                    }
                    Err(e) => {
                        tui.add_message(ChatMessage::system(format!(
                            "Error executing command: {}",
                            e
                        )));
                    }
                }
                return Ok(true);
            }

            // Unknown command - pass through to Claude
            return Ok(false);
        }
        _ => {}
    }

    Ok(false)
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

/// Handle /cost command
pub(crate) fn handle_cost_command(tui: &mut TuiState, stats: &SessionStats) {
    const INPUT_COST_PER_MILLION: f64 = 3.0;
    const OUTPUT_COST_PER_MILLION: f64 = 15.0;

    let input_tokens = stats.input_tokens;
    let output_tokens = stats.output_tokens;
    let total_tokens = stats.total_tokens;

    let input_cost = (input_tokens as f64 / 1_000_000.0) * INPUT_COST_PER_MILLION;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * OUTPUT_COST_PER_MILLION;
    let total_cost = input_cost + output_cost;

    let cost_display = format!(
        "Token Usage & Cost Estimate:\n\n\
         Session Statistics:\n\
         - Input tokens:  {:>8}\n\
         - Output tokens: {:>8}\n\
         - Total tokens:  {:>8}\n\n\
         Estimated Cost (Claude Sonnet 4.5):\n\
         - Input:  ${:>7.4} ({} tokens @ ${}/M)\n\
         - Output: ${:>7.4} ({} tokens @ ${}/M)\n\
         - Total:  ${:>7.4}\n\n\
         Note: Costs are estimates based on current Anthropic pricing.",
        input_tokens,
        output_tokens,
        total_tokens,
        input_cost,
        input_tokens,
        INPUT_COST_PER_MILLION,
        output_cost,
        output_tokens,
        OUTPUT_COST_PER_MILLION,
        total_cost
    );

    tui.add_message(ChatMessage::system(cost_display));
}

/// Handle /context command
pub(crate) fn handle_context_command(tui: &mut TuiState, stats: &SessionStats, model: &str) {
    const MAX_CONTEXT_TOKENS: u64 = 200_000;

    let used_tokens = stats.total_tokens;
    let percentage = ((used_tokens as f64 / MAX_CONTEXT_TOKENS as f64) * 100.0) as u64;
    let percentage = percentage.min(100);

    let filled = (percentage / 2) as usize;
    let empty = 50 - filled;

    let context_display = format!(
        "Context Window Usage:\n\n\
         Used:      {:>7} tokens ({}%)\n\
         Available: {:>7} tokens\n\
         Maximum:   {:>7} tokens\n\n\
         Visual: [{}{}] {}%\n\n\
         Messages: {} ({} user, {} assistant)\n\
         Model: {}",
        used_tokens,
        percentage,
        MAX_CONTEXT_TOKENS - used_tokens,
        MAX_CONTEXT_TOKENS,
        "=".repeat(filled),
        " ".repeat(empty),
        percentage,
        stats.message_count,
        stats.user_message_count,
        stats.assistant_message_count,
        model
    );

    tui.add_message(ChatMessage::system(context_display));
}

/// Handle /usage command - Display real rate limit data
pub(crate) fn handle_usage_command(tui: &mut TuiState, stats: &SessionStats) {
    let rl = &stats.rate_limits;

    let mut output = String::from("API Usage & Rate Limits:\n\n");

    if rl.last_updated.is_none() {
        output.push_str(
            "No rate limit data available yet.\n\
             Rate limits are captured from API responses during conversation.\n\n\
             Tip: Send a message to populate rate limit information.",
        );
    } else {
        output.push_str("Rate Limits (Per Minute):\n");
        match (rl.requests_limit, rl.requests_remaining) {
            (Some(limit), Some(remaining)) => {
                let used = limit.saturating_sub(remaining);
                let percent = rl.requests_percentage().unwrap_or(0);
                output.push_str(&format!(
                    "- Requests:  {:>6} / {:<6} used ({}%)\n",
                    used, limit, percent
                ));
                output.push_str(&format!("- Remaining: {:>6} requests\n", remaining));
            }
            _ => {
                output.push_str("- Requests:  No data\n");
            }
        }

        output.push_str("\nToken Limits (Per Day):\n");
        match (rl.tokens_limit, rl.tokens_remaining) {
            (Some(limit), Some(remaining)) => {
                let used = limit.saturating_sub(remaining);
                let percent = rl.tokens_percentage().unwrap_or(0);
                output.push_str(&format!(
                    "- Tokens:    {:>10} / {:<10} used ({}%)\n",
                    used, limit, percent
                ));
                output.push_str(&format!("- Remaining: {:>10} tokens\n", remaining));
            }
            _ => {
                output.push_str("- Tokens:    No data\n");
            }
        }

        output.push_str("\nVisual Progress:\n");
        if let Some(req_pct) = rl.requests_percentage() {
            let filled = (req_pct / 2) as usize;
            let empty = 50usize.saturating_sub(filled);
            output.push_str(&format!(
                "Requests: [{}{}] {}%\n",
                "=".repeat(filled),
                " ".repeat(empty),
                req_pct
            ));
        }
        if let Some(tok_pct) = rl.tokens_percentage() {
            let filled = (tok_pct / 2) as usize;
            let empty = 50usize.saturating_sub(filled);
            output.push_str(&format!(
                "Tokens:   [{}{}] {}%\n",
                "=".repeat(filled),
                " ".repeat(empty),
                tok_pct
            ));
        }

        if let Some(updated) = rl.last_updated {
            output.push_str(&format!(
                "\nLast updated: {}\n",
                updated.format("%Y-%m-%d %H:%M:%S UTC")
            ));
        }
    }

    tui.add_message(ChatMessage::system(output));
}

/// Handle /bashes command - Display background shell information
pub(crate) async fn handle_bashes_command(tui: &mut TuiState) -> Result<()> {
    use rustyclawd_tools::process_registry::global_registry;

    let registry = global_registry();
    let shell_ids = registry.list_ids().await;

    if shell_ids.is_empty() {
        tui.add_message(ChatMessage::system(
            "Background Bash Shells:\n\n\
             No background shells currently running.\n\n\
             Tips:\n\
             - Background shells are created using Bash tool with run_in_background: true\n\
             - Use BashOutput tool to read shell output\n\
             - Use KillShell tool to terminate shells"
                .to_string(),
        ));
        return Ok(());
    }

    let mut output = format!("Background Bash Shells ({}):\n\n", shell_ids.len());

    for shell_id in &shell_ids {
        match registry.get_status(shell_id).await {
            Ok(status) => {
                let status_str = match status {
                    rustyclawd_tools::process_registry::ProcessStatus::Running => "Running",
                    rustyclawd_tools::process_registry::ProcessStatus::Completed(code) => {
                        if code == 0 {
                            "Completed (success)"
                        } else {
                            "Completed (error)"
                        }
                    }
                    rustyclawd_tools::process_registry::ProcessStatus::Failed(_) => "Failed",
                };

                output.push_str(&format!("  {} - {}\n", shell_id, status_str));
            }
            Err(_) => {
                output.push_str(&format!("  {} - Status unknown\n", shell_id));
            }
        }
    }

    output.push_str(
        "\nCommands:\n\
         - Use BashOutput tool with bash_id to read output\n\
         - Use KillShell tool with shell_id to terminate\n\n\
         Example: Ask the assistant to check output from a specific shell ID",
    );

    tui.add_message(ChatMessage::system(output));

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

/// Handle /save command
pub(crate) fn handle_save_command(
    input: &str,
    tui: &mut TuiState,
    context: &Context,
    persistence: &mut Option<SessionPersistence>,
) -> Result<()> {
    if let Some(ref mut persistence) = persistence {
        let description = input.strip_prefix("/save").unwrap_or("").trim().to_string();

        let description = if description.is_empty() {
            "Manual save".to_string()
        } else {
            description
        };

        let messages: Vec<Message> = context.messages().to_vec();

        match persistence.save_checkpoint(&messages, description.clone()) {
            Ok(checkpoint_id) => {
                tui.add_message(ChatMessage::system(format!(
                    "Checkpoint saved: {} ({})",
                    checkpoint_id, description
                )));
            }
            Err(e) => {
                tui.add_message(ChatMessage::system(format!(
                    "Failed to save checkpoint: {}",
                    e
                )));
            }
        }
    } else {
        tui.add_message(ChatMessage::system(
            "Session persistence not available".to_string(),
        ));
    }

    Ok(())
}

/// Handle /load command
pub(crate) fn handle_load_command(
    input: &str,
    tui: &mut TuiState,
    context: &mut Context,
    persistence: &mut Option<SessionPersistence>,
) -> Result<()> {
    if let Some(ref mut persistence) = persistence {
        let checkpoint_id = input.strip_prefix("/load").unwrap_or("").trim();

        if checkpoint_id.is_empty() {
            tui.add_message(ChatMessage::system(
                "Usage: /load <checkpoint_id>\nUse /sessions to list available checkpoints"
                    .to_string(),
            ));
            return Ok(());
        }

        match persistence.load_checkpoint(checkpoint_id) {
            Ok(messages) => {
                *context = Context::new();

                for msg in &messages {
                    context.add_message(msg.clone());

                    let chat_msg = match msg.role {
                        MessageRole::User => ChatMessage::user(msg.content.clone()),
                        MessageRole::Assistant => ChatMessage::assistant(msg.content.clone()),
                        MessageRole::System => ChatMessage::system(msg.content.clone()),
                    };

                    tui.add_message(chat_msg);
                }

                tui.add_message(ChatMessage::system(format!(
                    "Checkpoint loaded: {} ({} messages)",
                    checkpoint_id,
                    messages.len()
                )));
            }
            Err(e) => {
                tui.add_message(ChatMessage::system(format!(
                    "Failed to load checkpoint: {}",
                    e
                )));
            }
        }
    } else {
        tui.add_message(ChatMessage::system(
            "Session persistence not available".to_string(),
        ));
    }

    Ok(())
}

/// Handle /sessions command
pub(crate) fn handle_sessions_command(
    tui: &mut TuiState,
    persistence: &Option<SessionPersistence>,
) -> Result<()> {
    if let Some(ref persistence) = persistence {
        match persistence.list_checkpoints() {
            Ok(checkpoints) => {
                if checkpoints.is_empty() {
                    tui.add_message(ChatMessage::system(
                        "No checkpoints found for current session".to_string(),
                    ));
                } else {
                    let mut output = format!("Available checkpoints ({}):\n", checkpoints.len());
                    for (idx, (description, info)) in checkpoints.iter().enumerate() {
                        output.push_str(&format!(
                            "  {}. {} - {} messages, {}\n",
                            idx + 1,
                            description,
                            info.message_count,
                            info.format_age()
                        ));
                    }
                    output.push_str("\nUse /load <checkpoint_id> to restore a checkpoint");

                    tui.add_message(ChatMessage::system(output));
                }
            }
            Err(e) => {
                tui.add_message(ChatMessage::system(format!(
                    "Failed to list checkpoints: {}",
                    e
                )));
            }
        }
    } else {
        tui.add_message(ChatMessage::system(
            "Session persistence not available".to_string(),
        ));
    }

    Ok(())
}

/// Format network errors with user-friendly messages and troubleshooting hints
#[allow(dead_code)]
pub(crate) fn format_network_error(error: &ClientError) -> String {
    match error {
        ClientError::Timeout(msg) => {
            format!(
                "\u{23F1}\u{FE0F}  Request timed out\n\
                Details: {}\n\
                Tip: Check your internet connection or try again later.",
                msg
            )
        }
        ClientError::ConnectionError(msg) => {
            format!(
                "\u{1F50C} Connection failed\n\
                Details: {}\n\
                Tip: Verify you can reach api.anthropic.com",
                msg
            )
        }
        ClientError::DnsError(msg) => {
            format!(
                "\u{1F310} DNS resolution failed\n\
                Details: {}\n\
                Tip: Check your DNS settings or try a different network.",
                msg
            )
        }
        ClientError::NetworkError(msg) => {
            format!(
                "\u{1F4E1} Network error\n\
                Details: {}\n\
                Tip: Check your internet connection.",
                msg
            )
        }
        _ => error.to_string(),
    }
}
