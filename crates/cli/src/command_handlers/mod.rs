//! Slash command dispatch and handler functions.
//!
//! This module routes built-in slash commands to focused handler submodules:
//! - `stats_handlers` — /stats, /cost, /context, /usage
//! - `session_handlers` — /save, /load, /sessions
//! - `utility_handlers` — /bashes, network error formatting
//!
//! Dispatch, exit, compact, help, MCP, and custom command/skill logic remain here.

mod session_handlers;
mod stats_handlers;
mod utility_handlers;

// Re-export handler functions so callers can use command_handlers::handle_cost_command etc.
pub(crate) use session_handlers::{
    handle_load_command, handle_save_command, handle_sessions_command,
};
pub(crate) use stats_handlers::{
    handle_context_command, handle_cost_command, handle_stats_command, handle_usage_command,
};
#[allow(unused_imports)]
pub(crate) use utility_handlers::format_network_error;
pub(crate) use utility_handlers::handle_bashes_command;

use crate::commands::SlashCommands;
use crate::conversation::{
    auto_save_session, execute_shell_command, process_user_message, SessionServices, StreamingState,
};
use crate::hooks;
use crate::mcp_commands;
use crate::session::SessionStats;
use crate::session_persistence::SessionPersistence;
use crate::tui::{ChatMessage, TuiState};
use anyhow::Result;
use rustyclawd_core::{Context, Message};
use rustyclawd_tools::{list_available_skills, load_skill_content};

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
    stats: &mut SessionStats,
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
        "\u{2713} PreCompact hook fired.\n\nCompacting conversation history...\n(Full compaction logic awaits implementation)".to_string(),
    ));

    Ok(())
}

/// Handle /help command.
async fn handle_help_command(tui: &mut TuiState, slash_commands: &SlashCommands) {
    let custom_commands = slash_commands.list_commands();
    let mut help_text = "Built-in Commands:\n  /exit, /quit - Exit the session\n  /clear - Clear conversation history\n  /compact - Compact conversation history (fires PreCompact hook)\n  /help - Show this help\n  /stats - Show session statistics\n  /save [description] - Save checkpoint\n  /load <checkpoint_id> - Load checkpoint\n  /sessions - List available sessions\n  !<command> - Execute shell command directly\n\nKeyboard Shortcuts:\n  F1 - Toggle debug panel\n\nTip: Text selection works natively - just click and drag in the messages window.\n\nMCP Commands:\n  /mcp-list - List all MCP servers\n  /mcp-start <server-id> - Start an MCP server\n  /mcp-stop <server-id> - Stop an MCP server\n  /mcp-tools <server-id> - List tools from server\n  /mcp-status <server-id> - Show server status\n".to_string();

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
