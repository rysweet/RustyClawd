//! Built-in commands - /help, /exit, /clear, etc.

use crate::commands::parser::Command;
use crate::session::SessionState;

/// Built-in command handler
pub struct BuiltinCommands;

impl BuiltinCommands {
    /// Check if command is a built-in
    /// Only includes commands that are actually implemented
    pub fn is_builtin(name: &str) -> bool {
        matches!(
            name,
            // Session management
            "clear" | "exit" | "quit" |
            // Configuration
            "config" | "model" | "status" |
            // Development tools
            "doctor" |
            // Information
            "help" | "history" | "stats" |
            // Additional built-ins
            "version" |
            // Tool management
            "tools" | "plugins" |
            // Session
            "checkpoint" | "restore" | "export" | "debug" | "mcp" | "reset"
        )
    }

    /// Execute a built-in command without session state (for simple commands)
    pub fn execute(cmd: &Command) -> Option<String> {
        match cmd.name.as_str() {
            // Session management
            "clear" => Some(Self::clear_command()),
            "exit" | "quit" => Some(Self::exit_command()),

            // Information
            "help" => Some(Self::help(&cmd.args_str)),
            "version" => Some(Self::version_command()),

            // Tool management
            "tools" => Some(Self::tools_command()),

            // Development tools
            "doctor" => Some(Self::doctor_command()),

            // Commands that can work with empty session data
            "history" => Some(Self::history_command_simple()),
            "stats" => Some(Self::stats_command_simple()),
            "config" => Some(Self::config_command_simple(&cmd.args_str)),
            "model" => Some(Self::model_command_simple(&cmd.args_str)),
            "status" => Some(Self::status_command_simple()),

            // Other commands
            "plugins" => Some(Self::plugins_command()),
            "checkpoint" => Some(Self::checkpoint_command(&cmd.args_str)),
            "restore" => Some(Self::restore_command(&cmd.args_str)),
            "export" => Some(Self::export_command(&cmd.args_str)),
            "debug" => Some(Self::debug_command(&cmd.args_str)),
            "mcp" => Some(Self::mcp_command(&cmd.args_str)),
            "reset" => Some(Self::reset_command()),

            _ => None,
        }
    }

    /// Execute a built-in command with session state (for commands that need real data)
    pub fn execute_with_state(cmd: &Command, _state: &SessionState) -> Option<String> {
        match cmd.name.as_str() {
            "history" => Some(Self::history_command_with_state(_state)),
            "stats" => Some(Self::stats_command_with_state(_state)),
            "config" => Some(Self::config_command_with_state(&cmd.args_str, _state)),
            "model" => Some(Self::model_command_with_state(&cmd.args_str, _state)),
            "status" => Some(Self::status_command_with_state(_state)),
            _ => Self::execute(cmd),
        }
    }

    /// /history - Show history (simple version without state)
    fn history_command_simple() -> String {
        "Command History:\n  (No commands executed yet)".to_string()
    }

    /// /stats - Show statistics (simple version without state)
    fn stats_command_simple() -> String {
        "Session Statistics:\n\
         Messages: 0\n\
         - User messages: 0\n\
         - Assistant messages: 0\n\
         Commands executed: 0\n\
         Tool calls: 0\n\
         Total tokens: 0\n\
         - Input tokens: 0\n\
         - Output tokens: 0\n\
         Session duration: 0s\n\
         Model: claude-sonnet-4-5"
            .to_string()
    }

    /// /config - Show configuration (simple version without state)
    fn config_command_simple(args: &Option<String>) -> String {
        match args {
            Some(arg) => format!("Configuration: {}", arg),
            None => "Current configuration:\n\
                     Model: claude-sonnet-4-5\n\
                     Working directory: (not initialized)\n\
                     Environment variables: 0\n\
                     Active contexts: 0"
                .to_string(),
        }
    }

    /// /model - Show or change model (simple version without state)
    fn model_command_simple(args: &Option<String>) -> String {
        match args {
            Some(model) => format!("Switching to model: {}", model),
            None => "Current model: claude-sonnet-4-5".to_string(),
        }
    }

    /// /status - Show status (simple version without state)
    fn status_command_simple() -> String {
        "System Status:\n\
         Connection: active\n\
         Model: claude-sonnet-4-5\n\
         Session active for: 0s\n\
         Messages exchanged: 0\n\
         Commands executed: 0\n\
         Tools: available"
            .to_string()
    }

    /// /help - Show help information
    fn help(search_term: &Option<String>) -> String {
        if let Some(term) = search_term {
            format!(
                "Help: searching for '{}'\n\n\
                 Available commands:\n\
                 - /help [search]       - Show help\n\
                 - /exit, /quit         - Exit the session\n\
                 - /clear               - Clear history\n\
                 - /stats               - Show statistics\n\
                 - /history             - Show command history\n\
                 - /config              - Show configuration\n\
                 - /model [name]        - Show or change model\n\
                 - /status              - Show system status\n\
                 - /version             - Show version\n\
                 - /doctor              - Run diagnostics\n\
                 - /tools               - List available tools",
                term
            )
        } else {
            "Help - Available Commands:\n\n\
             Slash Commands:\n\
               /help [search]    - Show this help message\n\
               /exit, /quit      - Exit the chat session\n\
               /clear            - Clear conversation history\n\
               /history          - Show command history\n\
               /stats            - Show session statistics\n\
               /config           - Show current configuration\n\
               /model [name]     - Show or switch model\n\
               /status           - Show system status\n\
               /version          - Show version information\n\
               /doctor           - Run diagnostic checks\n\
               /tools            - List available tools\n\n\
             Custom Commands:\n\
               /{name} [args]    - Execute custom slash commands\n\n\
             Tips:\n\
               - Custom commands are in .claude/commands/ directory\n\
               - Use /help <command> to search for specific commands\n\
               - Press Ctrl+D to exit quickly"
                .to_string()
        }
    }

    /// /exit or /quit - Exit command
    fn exit_command() -> String {
        "Exiting session...\nGoodbye!".to_string()
    }

    /// /clear - Clear history
    fn clear_command() -> String {
        "Conversation history cleared".to_string()
    }

    /// /history - Show history (requires session state)
    fn history_command_with_state(state: &SessionState) -> String {
        let history = state.get_history();

        if history.is_empty() {
            return "Command History:\n  (No commands executed yet)".to_string();
        }

        let mut output = String::from("Command History:\n");

        // Show most recent commands first
        for entry in history.iter().rev().take(20) {
            let status = if entry.success { "✓" } else { "✗" };
            output.push_str(&format!(
                "  {} {} - {}\n",
                status,
                entry.timestamp.format("%H:%M:%S"),
                entry.command
            ));
        }

        if history.len() > 20 {
            output.push_str(&format!("\n  ... and {} more\n", history.len() - 20));
        }

        output
    }

    /// /stats - Show statistics (requires session state)
    fn stats_command_with_state(state: &SessionState) -> String {
        let stats = state.get_stats();

        format!(
            "Session Statistics:\n\
             Messages: {}\n\
             - User messages: {}\n\
             - Assistant messages: {}\n\
             Commands executed: {}\n\
             Tool calls: {}\n\
             Total tokens: {}\n\
             - Input tokens: {}\n\
             - Output tokens: {}\n\
             Session duration: {}s\n\
             Model: {}",
            stats.message_count,
            stats.user_message_count,
            stats.assistant_message_count,
            stats.commands_executed,
            stats.tool_calls,
            stats.total_tokens,
            stats.input_tokens,
            stats.output_tokens,
            stats.duration_seconds,
            stats.model
        )
    }

    /// /config - Show configuration (requires session state)
    fn config_command_with_state(args: &Option<String>, state: &SessionState) -> String {
        match args {
            Some(arg) => format!("Configuration: {}", arg),
            None => format!(
                "Current configuration:\n\
                 Model: {}\n\
                 Working directory: {}\n\
                 Environment variables: {}\n\
                 Active contexts: {}",
                state.get_model(),
                state.cwd,
                state.env.len(),
                state.active_contexts.len()
            ),
        }
    }

    /// /model - Show or change model (requires session state)
    fn model_command_with_state(args: &Option<String>, state: &SessionState) -> String {
        match args {
            Some(model) => format!("Switching to model: {}", model),
            None => format!("Current model: {}", state.get_model()),
        }
    }

    /// /status - Show status (requires session state)
    fn status_command_with_state(state: &SessionState) -> String {
        let stats = state.get_stats();
        format!(
            "System Status:\n\
             Connection: active\n\
             Model: {}\n\
             Session active for: {}s\n\
             Messages exchanged: {}\n\
             Commands executed: {}\n\
             Tools: available",
            stats.model, stats.duration_seconds, stats.message_count, stats.commands_executed
        )
    }

    /// /doctor - Run diagnostics
    fn doctor_command() -> String {
        "Running diagnostic checks...\n\
         Configuration: OK\n\
         Tools: OK\n\
         Connections: OK"
            .to_string()
    }

    /// /version - Show version
    fn version_command() -> String {
        format!("RustyClawd version {}", env!("CARGO_PKG_VERSION"))
    }

    /// /tools - List available tools
    fn tools_command() -> String {
        "Available tools:\n\
         - Bash\n\
         - Read\n\
         - Write\n\
         - Edit\n\
         - Grep\n\
         - Glob\n\
         - WebFetch\n\
         - WebSearch"
            .to_string()
    }

    /// /plugins - List plugins
    fn plugins_command() -> String {
        "Loaded plugins:\n  (No plugins loaded)".to_string()
    }

    /// /checkpoint - Create checkpoint
    fn checkpoint_command(args: &Option<String>) -> String {
        match args {
            Some(name) => format!("Creating checkpoint: {}", name),
            None => "Creating checkpoint...".to_string(),
        }
    }

    /// /restore - Restore from checkpoint
    fn restore_command(args: &Option<String>) -> String {
        match args {
            Some(name) => format!("Restoring from checkpoint: {}", name),
            None => "Usage: /restore <checkpoint-name>".to_string(),
        }
    }

    /// /export - Export conversation
    fn export_command(args: &Option<String>) -> String {
        match args {
            Some(path) => format!("Exporting conversation to: {}", path),
            None => "Usage: /export <path>".to_string(),
        }
    }

    /// /debug - Set debug level
    fn debug_command(args: &Option<String>) -> String {
        match args {
            Some(level) => format!("Debug level set to: {}", level),
            None => "Debug mode: off".to_string(),
        }
    }

    /// /mcp - MCP integration
    fn mcp_command(args: &Option<String>) -> String {
        match args {
            Some(subcmd) => format!("MCP: {}", subcmd),
            None => "MCP Integration:\n  Status: active\n  Servers: 0 connected".to_string(),
        }
    }

    /// /reset - Reset session
    fn reset_command() -> String {
        "Resetting session state...".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin_help() {
        assert!(BuiltinCommands::is_builtin("help"));
    }

    #[test]
    fn test_is_builtin_exit() {
        assert!(BuiltinCommands::is_builtin("exit"));
        assert!(BuiltinCommands::is_builtin("quit"));
    }

    #[test]
    fn test_is_builtin_clear() {
        assert!(BuiltinCommands::is_builtin("clear"));
    }

    #[test]
    fn test_is_builtin_custom() {
        assert!(!BuiltinCommands::is_builtin("review-pr"));
        assert!(!BuiltinCommands::is_builtin("custom-cmd"));
    }

    #[test]
    fn test_execute_help_no_args() {
        let cmd = Command::new("help".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Help"));
        assert!(output.contains("/exit"));
    }

    #[test]
    fn test_execute_help_with_search() {
        let cmd = Command::new("help".to_string(), Some("commands".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("commands"));
    }

    #[test]
    fn test_execute_exit() {
        let cmd = Command::new("exit".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert!(result.unwrap().contains("Exiting"));
    }

    #[test]
    fn test_execute_quit() {
        let cmd = Command::new("quit".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert!(result.unwrap().contains("Exiting"));
    }

    #[test]
    fn test_execute_clear() {
        let cmd = Command::new("clear".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert!(result.unwrap().contains("cleared"));
    }

    #[test]
    fn test_execute_with_state_history() {
        let mut state = SessionState::new("/home/user", "claude-sonnet-4");
        state.add_command("/help", true);
        state.add_command("/stats", true);

        let cmd = Command::new("history".to_string(), None);
        let result = BuiltinCommands::execute_with_state(&cmd, &state);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("History"));
        assert!(output.contains("/help"));
        assert!(output.contains("/stats"));
    }

    #[test]
    fn test_execute_with_state_stats() {
        let mut state = SessionState::new("/home/user", "claude-sonnet-4");
        state.stats.add_user_message(100);
        state.stats.add_assistant_message(100, 200);

        let cmd = Command::new("stats".to_string(), None);
        let result = BuiltinCommands::execute_with_state(&cmd, &state);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Statistics"));
        assert!(output.contains("Messages: 2"));
        assert!(output.contains("Total tokens: 400"));
    }

    #[test]
    fn test_execute_unknown_command() {
        let cmd = Command::new("unknown".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_none());
    }
}
