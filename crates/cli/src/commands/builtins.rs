//! Built-in commands - /help, /exit, /clear, etc.

use crate::commands::parser::Command;
use anyhow::Result;

/// Built-in command handler
pub struct BuiltinCommands;

impl BuiltinCommands {
    /// Check if command is a built-in
    pub fn is_builtin(name: &str) -> bool {
        matches!(
            name,
            "help" | "exit" | "quit" | "clear" | "history" | "stats"
        )
    }

    /// Execute a built-in command
    pub fn execute(cmd: &Command) -> Option<String> {
        match cmd.name.as_str() {
            "help" => Some(Self::help(&cmd.args_str)),
            "exit" | "quit" => Some(Self::exit_command()),
            "clear" => Some(Self::clear_command()),
            "history" => Some(Self::history_command()),
            "stats" => Some(Self::stats_command()),
            _ => None,
        }
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
                 - /stats               - Show statistics",
                term
            )
        } else {
            "📖 Help - Available Commands:\n\n\
             Slash Commands:\n\
               /help [search]    - Show this help message\n\
               /exit, /quit      - Exit the chat session\n\
               /clear            - Clear conversation history\n\
               /history          - Show command history\n\
               /stats            - Show session statistics\n\n\
             Custom Commands:\n\
               /amplihack:*      - Amplihack custom commands\n\
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
        "👋 Exiting session...\nGoodbye!".to_string()
    }

    /// /clear - Clear history
    fn clear_command() -> String {
        "✓ Conversation history cleared".to_string()
    }

    /// /history - Show history
    fn history_command() -> String {
        "📜 Command History:\n\
         [Use 'history' command to see previous commands]\n\
         Note: History display would be populated in interactive mode"
            .to_string()
    }

    /// /stats - Show statistics
    fn stats_command() -> String {
        "📊 Session Statistics:\n\
         - Messages: 0\n\
         - Commands executed: 0\n\
         - Total tokens: 0\n\
         - Session duration: 0s"
            .to_string()
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
        let cmd = Command::new("help".to_string(), Some("slash-commands".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("slash-commands"));
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
    fn test_execute_history() {
        let cmd = Command::new("history".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert!(result.unwrap().contains("History"));
    }

    #[test]
    fn test_execute_stats() {
        let cmd = Command::new("stats".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert!(result.unwrap().contains("Statistics"));
    }

    #[test]
    fn test_execute_unknown_command() {
        let cmd = Command::new("unknown".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_none());
    }
}
