//! Session-related command handlers (/save, /load, /sessions).

use crate::session_persistence::SessionPersistence;
use crate::tui::{ChatMessage, TuiState};
use anyhow::Result;
use rustyclawd_core::{Context, Message, MessageRole};

/// Handle /save command.
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

/// Handle /load command.
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

/// Handle /sessions command.
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
