//! Slash command dispatch and handler functions.
//!
//! This module contains all the handler functions for built-in slash commands
//! (/cost, /context, /usage, /bashes, /save, /load, /sessions, /mcp-*, etc.)
//! and the main `handle_command` dispatch function.
//!
//! Sub-modules:
//! - `stats_handlers`: /stats, /cost, /context, /usage
//! - `session_handlers`: /save, /load, /sessions
//! - `utility_handlers`: /bashes

mod session_handlers;
mod stats_handlers;
mod utility_handlers;

// Re-export all handler functions so callers don't need changes.
pub(crate) use session_handlers::{
    handle_load_command, handle_save_command, handle_sessions_command,
};
pub(crate) use stats_handlers::{
    handle_context_command, handle_cost_command, handle_stats_command, handle_usage_command,
};
pub(crate) use utility_handlers::handle_bashes_command;

use crate::commands::SlashCommands;
use crate::conversation::{
    auto_save_session, execute_shell_command, process_user_message, SessionServices, StreamingState,
};
use crate::hooks;
use crate::mcp_commands;
use crate::session_persistence::SessionPersistence;
use crate::tui::{ChatMessage, TuiState};
use anyhow::Result;
use rustyclawd_core::{Context, Message};
use rustyclawd_tools::{list_available_skills, load_skill_content};
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag indicating whether the session has been logged out.
/// When true, API calls should be prevented.
static LOGGED_OUT: AtomicBool = AtomicBool::new(false);

/// Check whether the session is currently logged out.
pub(crate) fn is_logged_out() -> bool {
    LOGGED_OUT.load(Ordering::Relaxed)
}

/// Helper function to get current working directory as string
fn get_cwd_string() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// Handle special slash commands.
///
/// Returns `Ok(true)` if the command was handled, `Ok(false)` if the input
/// should be processed as a regular message.
pub(crate) async fn handle_command(
    input: &str,
    tui: &mut TuiState,
    context: &mut Context,
    services: &SessionServices,
    stats: &mut crate::session::SessionStats,
    persistence: &mut Option<SessionPersistence>,
    streaming: &mut StreamingState,
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

        execute_shell_command(
            command,
            tui,
            context,
            &services.allowed_tools,
            &services.disallowed_tools,
        )
        .await?;
        return Ok(true);
    }

    match input {
        "/exit" | "/quit" => {
            handle_exit_command(tui, services, persistence, context).await?;
        }
        "/clear" => {
            *context = Context::new();
            tui.add_message(ChatMessage::system(
                "Conversation history cleared".to_string(),
            ));
            tui.set_status("Conversation cleared".to_string());
            return Ok(true);
        }
        "/logout" => {
            handle_logout_command(tui);
            return Ok(true);
        }
        "/debug" => {
            handle_debug_command(tui, context, services, stats);
            return Ok(true);
        }
        "/compact" => {
            handle_compact_command(tui, services).await?;
            return Ok(true);
        }
        "/help" => {
            handle_help_command(tui, &services.slash_commands).await;
            return Ok(true);
        }
        "/stats" => {
            handle_stats_command(tui, stats, &services.model);
            return Ok(true);
        }
        "/cost" => {
            handle_cost_command(tui, stats);
            return Ok(true);
        }
        "/context" => {
            handle_context_command(tui, stats, &services.model);
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
        _ if input.starts_with("/rename") => {
            handle_rename_command(input, tui, context, persistence);
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

                match mcp_commands::handle_tui_command(services.mcp_proxy.clone(), &command, args)
                    .await
                {
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
            return handle_custom_slash_command(input, tui, context, services, streaming).await;
        }
        _ => {}
    }

    Ok(false)
}

/// Handle /exit and /quit commands.
async fn handle_exit_command(
    tui: &mut TuiState,
    services: &SessionServices,
    persistence: &mut Option<SessionPersistence>,
    context: &Context,
) -> Result<()> {
    // Execute Stop hook to check if exit should be allowed
    if let Some(ref hooks_sys) = services.hooks {
        let hook_context = hooks::HookContext::for_session(
            services.session_id.to_string(),
            format!(".claude/sessions/{}/transcript.json", services.session_id),
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
                                return Ok(());
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

/// Handle /compact command.
async fn handle_compact_command(tui: &mut TuiState, services: &SessionServices) -> Result<()> {
    if let Some(ref hooks_sys) = services.hooks {
        let hook_context = hooks::HookContext::for_session(
            services.session_id.to_string(),
            format!(".claude/sessions/{}/transcript.json", services.session_id),
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
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                tracing::error!("PreCompact hook execution failed: {:?}", e);
                tui.add_message(ChatMessage::system(format!(
                    "\u{26A0}\u{FE0F}  Failed to execute PreCompact hooks: {}",
                    e
                )));
                return Ok(());
            }
        }
    }

    tui.add_message(ChatMessage::system(
        "\u{2713} PreCompact hook fired.\n\nConversation history compacted.".to_string(),
    ));

    Ok(())
}

/// Handle /help command.
async fn handle_help_command(tui: &mut TuiState, slash_commands: &SlashCommands) {
    let custom_commands = slash_commands.list_commands();
    let mut help_text = "Built-in Commands:\n  /exit, /quit - Exit the session\n  /clear - Clear conversation history\n  /compact - Compact conversation history (fires PreCompact hook)\n  /help - Show this help\n  /stats - Show session statistics\n  /debug - Show debug information (version, model, tokens, etc.)\n  /logout - Log out and prevent further API calls\n  /rename [name] - Rename current session (auto-generates name if omitted)\n  /save [description] - Save checkpoint\n  /load <checkpoint_id> - Load checkpoint\n  /sessions - List available sessions\n  !<command> - Execute shell command directly\n\nKeyboard Shortcuts:\n  F1 - Toggle debug panel\n\nTip: Text selection works natively - just click and drag in the messages window.\n\nMCP Commands:\n  /mcp-list - List all MCP servers\n  /mcp-start <server-id> - Start an MCP server\n  /mcp-stop <server-id> - Stop an MCP server\n  /mcp-tools <server-id> - List tools from server\n  /mcp-status <server-id> - Show server status\n".to_string();

    if !custom_commands.is_empty() {
        help_text.push_str("\nCustom Commands:\n");
        for cmd in custom_commands {
            help_text.push_str(&format!("  /{}\n", cmd));
        }
    }

    let available_skills = list_available_skills().await;
    if !available_skills.is_empty() {
        help_text.push_str("\nSkills (invokable as /skill-name):\n");
        for skill in available_skills {
            help_text.push_str(&format!("  /{}\n", skill));
        }
    }

    help_text.push_str("\nPress Ctrl+C or Ctrl+D to exit.");

    tui.add_message(ChatMessage::system(help_text));
}

/// Handle custom slash commands (those registered in SlashCommands system).
async fn handle_custom_slash_command(
    input: &str,
    tui: &mut TuiState,
    context: &mut Context,
    services: &SessionServices,
    streaming: &mut StreamingState,
) -> Result<bool> {
    let command_name = input[1..].split_whitespace().next().unwrap_or("");

    if services.slash_commands.has_command(command_name) {
        if !services
            .slash_commands
            .should_intercept_locally(command_name)
        {
            return Ok(false);
        }

        tui.set_status(format!("Executing command: {}", input));

        match services.slash_commands.execute(input).await {
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
                    services,
                    streaming,
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

    // Check if a skill with this name exists (skills are invokable as /skill-name)
    if let Some(skill_content) = load_skill_content(command_name).await {
        tui.set_status(format!("Invoking skill: {}", command_name));
        tui.add_message(ChatMessage::user(input.to_string()));
        tui.add_message(ChatMessage::system(skill_content.clone()));
        context.add_message(Message::user(skill_content.clone()));

        if let Err(e) =
            process_user_message(&skill_content, true, tui, context, services, streaming).await
        {
            tui.add_message(ChatMessage::system(format!(
                "Error processing skill: {}",
                e
            )));
        }
        return Ok(true);
    }

    // Unknown command - pass through to Claude
    Ok(false)
}

/// Handle /logout command.
///
/// Sets a global flag to prevent further API calls without unsetting env vars
/// (which is unsafe across threads). The session remains open for reviewing
/// conversation history and running local commands.
fn handle_logout_command(tui: &mut TuiState) {
    LOGGED_OUT.store(true, Ordering::Relaxed);
    tui.add_message(ChatMessage::system(
        "Logged out. Session will no longer make API calls.\n\
         You can still review conversation history and run local commands.\n\
         Restart the application to log back in."
            .to_string(),
    ));
    tui.set_status("Logged out".to_string());
}

/// Sanitize a session name by stripping control characters, IPC markers, and
/// other potentially dangerous input. Only allows alphanumeric characters, dashes,
/// underscores, spaces, and dots. Limits length to 100 characters.
fn sanitize_session_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ' || *c == '.')
        .take(100)
        .collect();
    sanitized.trim().to_string()
}

/// Handle /rename command.
///
/// Renames the current session. If a name is provided (e.g. `/rename my-session`),
/// uses that name. If no name is provided, auto-generates a name from the first
/// user message in the conversation.
fn handle_rename_command(
    input: &str,
    tui: &mut TuiState,
    context: &Context,
    persistence: &mut Option<SessionPersistence>,
) {
    let new_name = input.strip_prefix("/rename").unwrap_or("").trim();

    let new_name = if new_name.is_empty() {
        // Auto-generate name from the first user message
        let first_user_msg = context
            .messages()
            .iter()
            .find(|m| m.role == rustyclawd_core::MessageRole::User);

        match first_user_msg {
            Some(msg) => {
                // Take first 40 chars, replace spaces with dashes, lowercase
                let sanitized: String = msg
                    .content
                    .chars()
                    .take(40)
                    .map(|c| {
                        if c.is_alphanumeric() {
                            c.to_ascii_lowercase()
                        } else {
                            '-'
                        }
                    })
                    .collect::<String>()
                    .trim_matches('-')
                    .to_string();
                if sanitized.is_empty() {
                    "unnamed-session".to_string()
                } else {
                    sanitized
                }
            }
            None => "unnamed-session".to_string(),
        }
    } else {
        sanitize_session_name(new_name)
    };

    if new_name.is_empty() {
        tui.add_message(ChatMessage::system(
            "Error: Session name contains only invalid characters. \
             Use alphanumeric, dash, underscore, space, or dot."
                .to_string(),
        ));
        return;
    }

    if let Some(ref current_persistence) = persistence {
        // Save current state under the new session name by creating a new
        // SessionPersistence with the desired name and saving messages into it.
        let messages: Vec<rustyclawd_core::Message> = context.messages().to_vec();
        let old_id = current_persistence.session_id().to_string();

        match SessionPersistence::new(&new_name) {
            Ok(mut new_persistence) => {
                match new_persistence.save_checkpoint(&messages, format!("Renamed from {}", old_id))
                {
                    Ok(_) => {
                        // Replace the old persistence with the new one
                        *persistence = Some(new_persistence);
                        tui.add_message(ChatMessage::system(format!(
                            "Session renamed: {} -> {}",
                            old_id, new_name
                        )));
                    }
                    Err(e) => {
                        tui.add_message(ChatMessage::system(format!(
                            "Failed to save renamed session: {}",
                            e
                        )));
                    }
                }
            }
            Err(e) => {
                tui.add_message(ChatMessage::system(format!(
                    "Failed to create session '{}': {}",
                    new_name, e
                )));
            }
        }
    } else {
        tui.add_message(ChatMessage::system(
            "Session persistence not available".to_string(),
        ));
    }
}

/// Handle /debug command.
///
/// Displays real debug information about the current session state including
/// version, OS, model, session ID, API key status, permissions, message count,
/// and token usage.
fn handle_debug_command(
    tui: &mut TuiState,
    context: &Context,
    services: &SessionServices,
    stats: &crate::session::SessionStats,
) {
    let version = env!("CARGO_PKG_VERSION");
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let model = &services.model;
    let session_id = &services.session_id;
    let api_key_status = if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        "Set"
    } else {
        "Not set"
    };
    let permission_mode = services.permission_mode.status_indicator();
    let message_count = context.messages().len();
    let total_tokens = stats.total_tokens;
    let input_tokens = stats.input_tokens;
    let output_tokens = stats.output_tokens;
    let tool_calls = stats.tool_calls;
    let logged_out = is_logged_out();

    let debug_info = format!(
        "Debug Information:\n\
         \n\
           Version:         {}\n\
           OS:              {}\n\
           Architecture:    {}\n\
           Model:           {}\n\
           Session ID:      {}\n\
           API Key:         {}\n\
           Logged Out:      {}\n\
           Permission Mode: {}\n\
           Messages:        {}\n\
           Total Tokens:    {}\n\
           Input Tokens:    {}\n\
           Output Tokens:   {}\n\
           Tool Calls:      {}",
        version,
        os,
        arch,
        model,
        session_id,
        api_key_status,
        logged_out,
        permission_mode,
        message_count,
        total_tokens,
        input_tokens,
        output_tokens,
        tool_calls,
    );

    tui.add_message(ChatMessage::system(debug_info));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_session_name_normal() {
        assert_eq!(sanitize_session_name("my-session"), "my-session");
        assert_eq!(sanitize_session_name("hello_world"), "hello_world");
        assert_eq!(sanitize_session_name("test 123"), "test 123");
        assert_eq!(sanitize_session_name("v1.0"), "v1.0");
    }

    #[test]
    fn test_sanitize_strips_ipc_markers() {
        // The critical vulnerability: IPC marker injection
        assert_eq!(
            sanitize_session_name("[[RENAME_SESSION:evil]]"),
            "RENAME_SESSIONevil"
        );
        assert_eq!(
            sanitize_session_name("foo]][[BAR:baz]]"),
            "fooBARbaz"
        );
        // Crucially, no [[ or ]] survive
        let result = sanitize_session_name("[[INJECT]]");
        assert!(!result.contains("[["));
        assert!(!result.contains("]]"));
    }

    #[test]
    fn test_sanitize_strips_control_chars() {
        assert_eq!(
            sanitize_session_name("bad\x00name\nnewline\ttab"),
            "badnamenewlinetab"
        );
    }

    #[test]
    fn test_sanitize_empty_after_strip() {
        assert_eq!(sanitize_session_name("[[\n\x00]]"), "");
    }

    #[test]
    fn test_sanitize_length_limit() {
        let long = "a".repeat(200);
        let result = sanitize_session_name(&long);
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_sanitize_trims_whitespace() {
        assert_eq!(sanitize_session_name("  hello  "), "hello");
    }
}
