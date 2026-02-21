//! Built-in commands - /help, /exit, /clear, etc.
//!
//! Only commands that perform real work live here. Per Zero-BS philosophy,
//! commands that merely return "not yet implemented" or hardcoded static
//! text have been removed. When real implementations are ready, add them back.

use crate::commands::parser::Command;

/// Built-in command handler
pub struct BuiltinCommands;

impl BuiltinCommands {
    /// Check if command is a built-in
    pub fn is_builtin(name: &str) -> bool {
        matches!(
            name,
            "clear"
                | "exit"
                | "quit"
                | "help"
                | "version"
                | "permissions"
                | "login"
                | "bug"
                | "add-dir"
                | "fast"
                | "model"
        )
    }

    /// Execute a built-in command
    pub fn execute(cmd: &Command) -> Option<String> {
        match cmd.name.as_str() {
            "clear" => Some(Self::clear_command()),
            "exit" | "quit" => Some(Self::exit_command()),
            "help" => Some(Self::help(&cmd.args_str)),
            "version" => Some(Self::version_command()),
            "permissions" => Some(Self::permissions_command()),
            "login" => Some(Self::login_command()),
            "bug" => Some(Self::bug_command()),
            "add-dir" => Some(Self::add_dir_command(&cmd.args_str)),
            "fast" => Some(Self::fast_command()),
            "model" => Some(Self::model_command(&cmd.args_str)),
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
                 - /version             - Show version\n\
                 - /permissions         - Open permissions modal\n\
                 - /login               - Check authentication status\n\
                 - /bug                 - Report a bug\n\
                 - /add-dir <path>      - Add working directory\n\
                 - /fast                - Toggle fast mode\n\
                 - /model [alias|name]  - Show or switch model",
                term
            )
        } else {
            "Available Commands:\n\n\
             Slash Commands:\n\
               /help [search]         - Show this help message\n\
               /exit, /quit           - Exit the chat session\n\
               /clear                 - Clear conversation history\n\
               /version               - Show version information\n\
               /permissions           - Open permissions modal\n\
               /login                 - Check authentication status\n\
               /bug                   - Report a bug via GitHub\n\
               /add-dir <path>        - Add working directory\n\
               /fast                  - Toggle fast mode\n\
               /model                 - Show current model\n\
               /model <alias|name>    - Switch model (sonnet/opus/haiku or full ID)\n\n\
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
        "Exiting session...\nGoodbye!".to_string()
    }

    /// /clear - Clear history
    fn clear_command() -> String {
        "Conversation history cleared".to_string()
    }

    /// /version - Show real version from Cargo.toml
    fn version_command() -> String {
        format!("RustyClawd version {}", env!("CARGO_PKG_VERSION"))
    }

    /// /permissions - Return IPC marker for TUI to open permissions modal
    fn permissions_command() -> String {
        "[[OPEN_PERMISSIONS_MODAL]]".to_string()
    }

    /// /login - Check authentication status via ANTHROPIC_API_KEY env var
    fn login_command() -> String {
        let has_api_key = std::env::var("ANTHROPIC_API_KEY").is_ok();

        if has_api_key {
            "Authentication status: ANTHROPIC_API_KEY environment variable is set.\n\n\
             Full account login flow (OAuth browser authentication) is not yet implemented.\n\
             Currently, authentication is handled via the ANTHROPIC_API_KEY environment variable."
                .to_string()
        } else {
            "Authentication status: ANTHROPIC_API_KEY environment variable is NOT set.\n\n\
             To authenticate, set your API key:\n\
             export ANTHROPIC_API_KEY=your-key-here\n\n\
             You can find your API key at: https://console.anthropic.com\n\n\
             Full account login flow (OAuth browser authentication) is not yet implemented."
                .to_string()
        }
    }

    /// /bug - Report bugs via GitHub issues URL
    fn bug_command() -> String {
        const GITHUB_ISSUES_URL: &str = "https://github.com/rysweet/RustyClawd/issues";
        const VERSION: &str = env!("CARGO_PKG_VERSION");

        format!(
            "Bug Report:\n\n\
             To report a bug, open a GitHub issue:\n\
             {GITHUB_ISSUES_URL}\n\n\
             Include:\n\
             - RustyClawd version: {VERSION}\n\
             - OS: {os}\n\
             - Steps to reproduce\n\
             - Expected vs actual behavior\n\n\
             For security vulnerabilities, do NOT open a public issue.\n\
             Email: security@anthropic.com",
            os = std::env::consts::OS,
        )
    }

    /// /add-dir <directory> - Add additional working directory
    fn add_dir_command(args: &Option<String>) -> String {
        match args {
            Some(dir) => {
                let path = std::path::Path::new(dir);
                if path.exists() && path.is_dir() {
                    format!(
                        "Added directory to working set:\n  {}\n\n\
                         Note: Directory will be available in this session context.",
                        dir
                    )
                } else {
                    format!(
                        "Error: Directory does not exist or is not a directory: {}",
                        dir
                    )
                }
            }
            None => {
                "Usage: /add-dir <directory>\n\nExample:\n  /add-dir /path/to/project".to_string()
            }
        }
    }

    /// /fast - Toggle fast mode (returns IPC marker for TUI)
    fn fast_command() -> String {
        "[[TOGGLE_FAST_MODE]]".to_string()
    }

    /// /model [alias|name] - Show or switch model (returns IPC marker for TUI)
    ///
    /// With no arguments, returns a marker to display the current model.
    /// With an alias or full model name, returns a marker to switch the model.
    ///
    /// Supported aliases: sonnet, opus, haiku
    fn model_command(args: &Option<String>) -> String {
        match args {
            None => "[[SHOW_MODEL]]".to_string(),
            Some(alias) => {
                let resolved = match alias.trim() {
                    "sonnet" => "claude-sonnet-4-6",
                    "opus" => "claude-opus-4-6",
                    "haiku" => "claude-haiku-4-5-20251001",
                    other => other,
                };
                format!("[[SET_MODEL:{}]]", resolved)
            }
        }
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
    fn test_removed_stubs_no_longer_builtin() {
        // These were removed because they only returned static/placeholder text
        let removed = vec![
            "bashes",
            "context",
            "cost",
            "todos",
            "usage",
            "logout",
            "compact",
            "rewind",
            "config",
            "status",
            "review",
            "sandbox",
            "doctor",
            "export",
            "memory",
            "mcp",
            "agents",
            "hooks",
            "init",
            "debug",
            "trace",
            "log",
            "checkpoint",
            "restore",
            "tools",
            "plugins",
            "save",
            "load",
            "reset",
            "undo",
            "redo",
            "history",
            "stats",
            "output-style",
            "privacy-settings",
            "statusline",
            "terminal-setup",
            "vim",
            "pr_comments",
        ];
        for cmd in removed {
            assert!(
                !BuiltinCommands::is_builtin(cmd),
                "Removed stub '{}' should no longer be a builtin",
                cmd
            );
        }
    }

    #[test]
    fn test_execute_help_no_args() {
        let cmd = Command::new("help".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Available Commands"));
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
    fn test_execute_unknown_command() {
        let cmd = Command::new("unknown".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_none());
    }

    #[test]
    fn test_execute_version() {
        let cmd = Command::new("version".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("RustyClawd version"));
    }

    #[test]
    fn test_execute_permissions() {
        let cmd = Command::new("permissions".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "[[OPEN_PERMISSIONS_MODAL]]");
    }

    #[test]
    fn test_execute_login() {
        let cmd = Command::new("login".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn test_execute_bug() {
        let cmd = Command::new("bug".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("rysweet/RustyClawd"));
    }

    #[test]
    fn test_is_builtin_add_dir() {
        assert!(BuiltinCommands::is_builtin("add-dir"));
    }

    #[test]
    fn test_execute_add_dir_no_args() {
        let cmd = Command::new("add-dir".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Usage"));
        assert!(output.contains("/add-dir"));
    }

    #[test]
    fn test_execute_add_dir_with_valid_dir() {
        let cwd = std::env::current_dir().unwrap();
        let cwd_str = cwd.to_string_lossy().to_string();

        let cmd = Command::new("add-dir".to_string(), Some(cwd_str.clone()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Added directory"));
        assert!(output.contains(&cwd_str));
    }

    #[test]
    fn test_execute_add_dir_with_invalid_dir() {
        let cmd = Command::new(
            "add-dir".to_string(),
            Some("/nonexistent/path/xyz".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Error"));
        assert!(output.contains("does not exist"));
    }

    #[test]
    fn test_is_builtin_fast() {
        assert!(BuiltinCommands::is_builtin("fast"));
    }

    #[test]
    fn test_execute_fast() {
        let cmd = Command::new("fast".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("[[TOGGLE_FAST_MODE]]"));
    }

    #[test]
    fn test_is_builtin_model() {
        assert!(BuiltinCommands::is_builtin("model"));
    }

    #[test]
    fn test_execute_model_no_args() {
        let cmd = Command::new("model".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "[[SHOW_MODEL]]");
    }

    #[test]
    fn test_execute_model_opus_alias() {
        let cmd = Command::new("model".to_string(), Some("opus".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "[[SET_MODEL:claude-opus-4-6]]");
    }

    #[test]
    fn test_execute_model_sonnet_alias() {
        let cmd = Command::new("model".to_string(), Some("sonnet".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "[[SET_MODEL:claude-sonnet-4-6]]");
    }

    #[test]
    fn test_execute_model_haiku_alias() {
        let cmd = Command::new("model".to_string(), Some("haiku".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "[[SET_MODEL:claude-haiku-4-5-20251001]]");
    }

    #[test]
    fn test_execute_model_full_name() {
        let cmd = Command::new("model".to_string(), Some("claude-opus-4-6".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "[[SET_MODEL:claude-opus-4-6]]");
    }
}
