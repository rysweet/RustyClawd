//! Utility command handlers.
//!
//! Handles /bashes and network error formatting utilities.

use crate::tui::{ChatMessage, TuiState};
use anyhow::Result;

/// Handle /bashes command - Display background shell information.
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

/// Format network errors with user-friendly messages and troubleshooting hints.
#[allow(dead_code)]
pub(crate) fn format_network_error(error: &rustyclawd_core::client::ClientError) -> String {
    use rustyclawd_core::client::ClientError;

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
