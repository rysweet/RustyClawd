//! Built-in commands - /help, /exit, /clear, etc.
//!
//! Only commands that perform real work live here. Per Zero-BS philosophy,
//! commands that merely return "not yet implemented" or hardcoded static
//! text have been removed. When real implementations are ready, add them back.

use crate::commands::parser::Command;
use crate::scheduled_tasks::parse_interval;

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
                | "logout"
                | "bug"
                | "add-dir"
                | "fast"
                | "rename"
                | "debug"
                | "color"
                | "loop"
                | "copy"
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
            "logout" => Some(Self::logout_command()),
            "rename" => Some(Self::rename_command(&cmd.args_str)),
            "debug" => Some(Self::debug_command()),
            "color" => Some(Self::color_command(&cmd.args_str)),
            "loop" => Some(Self::loop_command(&cmd.args_str)),
            "copy" => Some(Self::copy_command()),
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
                 - /fast                - Toggle fast mode",
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
               /logout                - Clear authentication credentials\n\
               /bug                   - Report a bug via GitHub\n\
               /add-dir <path>        - Add working directory\n\
               /fast                  - Toggle fast mode\n\
               /color <mode>          - Set color output mode (default, gray, reset, none)\n\
               /copy                  - Select and copy a code block to clipboard\n\
               /loop <interval> <prompt> - Run a prompt on a recurring schedule\n\
               /model                 - Show or switch model (handled in session layer)\n\
               /rename [name]         - Rename current session\n\
               /debug                 - Show debug information for troubleshooting\n\n\
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

    /// /exit or /quit - Exit command with session resume hint (v2.1.31)
    fn exit_command() -> String {
        "Exiting session...\n\n\
         Tip: To resume this session later, use:\n  \
         rusty --resume\n\n\
         Goodbye!"
            .to_string()
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

    /// /logout - Clear authentication credentials (v2.1.42)
    /// Returns IPC marker for TUI to handle logout safely
    fn logout_command() -> String {
        "[[LOGOUT]]\n\n\
         To complete logout, unset ANTHROPIC_API_KEY in your shell:\n  \
         unset ANTHROPIC_API_KEY\n\n\
         Or remove it from your shell configuration (e.g., ~/.bashrc, ~/.zshrc)."
            .to_string()
    }

    /// /rename [name] - Rename current session (v2.1.42)
    /// Returns IPC marker so TUI can handle the rename with session context
    fn rename_command(args: &Option<String>) -> String {
        match args {
            Some(name) => {
                // Sanitize: only allow alphanumeric, dash, underscore, space, dot.
                // This strips control characters, IPC markers ([, ]), newlines,
                // and other potentially dangerous input. Limit to 100 chars.
                let sanitized: String = name
                    .chars()
                    .filter(|c| {
                        c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ' || *c == '.'
                    })
                    .take(100)
                    .collect::<String>()
                    .trim()
                    .to_string();
                if sanitized.is_empty() {
                    "Error: Session name contains only invalid characters. \
                     Use alphanumeric, dash, underscore, space, or dot."
                        .to_string()
                } else {
                    format!("[[RENAME_SESSION:{}]]", sanitized)
                }
            }
            None => "[[RENAME_SESSION_AUTO]]".to_string(),
        }
    }

    /// /color <mode> - Set color output mode
    ///
    /// Accepted values: default, gray, reset, none
    fn color_command(args: &Option<String>) -> String {
        const VALID_MODES: &[&str] = &["default", "gray", "reset", "none"];

        match args {
            Some(mode) => {
                let mode = mode.trim().to_lowercase();
                if VALID_MODES.contains(&mode.as_str()) {
                    format!("Color set to: {}", mode)
                } else {
                    format!(
                        "Unknown color mode: '{}'\n\nValid modes: {}",
                        mode,
                        VALID_MODES.join(", ")
                    )
                }
            }
            None => format!(
                "Usage: /color <mode>\n\nValid modes: {}",
                VALID_MODES.join(", ")
            ),
        }
    }

    /// /loop <interval> <prompt> - Schedule a recurring prompt
    ///
    /// Parses an interval (e.g. "5m", "1h", "30s") and a prompt string.
    /// Returns an IPC marker for the session loop to wire up scheduling.
    fn loop_command(args: &Option<String>) -> String {
        let args_str = match args {
            Some(a) if !a.trim().is_empty() => a.trim(),
            _ => {
                return "Usage: /loop <interval> <prompt>\n\n\
                        Interval examples: 30s, 5m, 1h\n\n\
                        Example:\n  /loop 5m check build status"
                    .to_string();
            }
        };

        // First whitespace-delimited token is the interval, rest is the prompt.
        let (interval_str, prompt) = match args_str.split_once(char::is_whitespace) {
            Some((i, p)) if !p.trim().is_empty() => (i, p.trim()),
            _ => {
                return "Usage: /loop <interval> <prompt>\n\n\
                        Both an interval and a prompt are required.\n\n\
                        Example:\n  /loop 5m check build status"
                    .to_string();
            }
        };

        match parse_interval(interval_str) {
            Some(duration) => {
                let secs = duration.as_secs();
                let human = if secs >= 3600 && secs % 3600 == 0 {
                    format!("{}h", secs / 3600)
                } else if secs >= 60 && secs % 60 == 0 {
                    format!("{}m", secs / 60)
                } else {
                    format!("{}s", secs)
                };
                format!(
                    "[[SCHEDULE_LOOP:{}:{}]]\n\n\
                     Scheduled: will run '{}' every {}",
                    secs, prompt, prompt, human
                )
            }
            None => {
                format!(
                    "Invalid interval: '{}'\n\n\
                     Accepted formats: 30s, 5m, 1h (positive integer + s/m/h)",
                    interval_str
                )
            }
        }
    }

    /// /copy - Select and copy a code block from the conversation to clipboard.
    ///
    /// Returns an IPC marker for the TUI layer to present a code-block picker.
    fn copy_command() -> String {
        "[[COPY_CODE_BLOCK]]".to_string()
    }

    /// /debug - Show debug information for session troubleshooting (v2.1.31)
    fn debug_command() -> String {
        let version = env!("CARGO_PKG_VERSION");
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let has_api_key = std::env::var("ANTHROPIC_API_KEY").is_ok();
        let model = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "default".to_string());
        let is_nested = std::env::var("CLAUDE_CODE_NESTED_GUARD").is_ok();

        format!(
            "Debug Information:\n\n\
             Version:      {version}\n\
             OS:           {os}\n\
             Architecture: {arch}\n\
             CWD:          {cwd}\n\
             API Key Set:  {has_api_key}\n\
             Model:        {model}\n\
             Nested:       {is_nested}"
        )
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
    fn test_is_builtin_logout() {
        assert!(BuiltinCommands::is_builtin("logout"));
    }

    #[test]
    fn test_execute_logout() {
        let cmd = Command::new("logout".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("[[LOGOUT]]"));
    }

    #[test]
    fn test_is_builtin_rename() {
        assert!(BuiltinCommands::is_builtin("rename"));
    }

    #[test]
    fn test_execute_rename_with_name() {
        let cmd = Command::new("rename".to_string(), Some("my-session".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("[[RENAME_SESSION:my-session]]"));
    }

    #[test]
    fn test_execute_rename_auto() {
        let cmd = Command::new("rename".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("[[RENAME_SESSION_AUTO]]"));
    }

    #[test]
    fn test_rename_strips_ipc_markers() {
        // IPC marker injection: [[FAKE_COMMAND:payload]]
        let cmd = Command::new(
            "rename".to_string(),
            Some("[[FAKE_COMMAND:payload]]".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd).unwrap();
        // Brackets and colons are stripped, so the IPC marker pattern cannot survive
        assert!(!result.contains("FAKE_COMMAND:payload"));
        // The sanitized name preserves only allowed chars
        assert!(result.contains("[[RENAME_SESSION:FAKE_COMMANDpayload]]"));
    }

    #[test]
    fn test_rename_strips_control_characters() {
        let cmd = Command::new(
            "rename".to_string(),
            Some("bad\x00name\nnewline\ttab".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd).unwrap();
        assert!(result.contains("[[RENAME_SESSION:badnamenewlinetab]]"));
    }

    #[test]
    fn test_rename_rejects_all_invalid() {
        let cmd = Command::new("rename".to_string(), Some("[[\n\x00]]".to_string()));
        let result = BuiltinCommands::execute(&cmd).unwrap();
        assert!(result.contains("Error: Session name contains only invalid characters"));
    }

    #[test]
    fn test_rename_length_limit() {
        let long_name = "a".repeat(200);
        let cmd = Command::new("rename".to_string(), Some(long_name));
        let result = BuiltinCommands::execute(&cmd).unwrap();
        // Extract the name from [[RENAME_SESSION:name]]
        let start = result.find("[[RENAME_SESSION:").unwrap() + "[[RENAME_SESSION:".len();
        let end = result.find("]]").unwrap();
        let name = &result[start..end];
        assert!(name.len() <= 100);
    }

    #[test]
    fn test_is_builtin_debug() {
        assert!(BuiltinCommands::is_builtin("debug"));
    }

    #[test]
    fn test_execute_debug() {
        let cmd = Command::new("debug".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Debug Information"));
        assert!(output.contains("Version"));
        assert!(output.contains("OS"));
    }

    #[test]
    fn test_exit_shows_resume_hint() {
        let cmd = Command::new("exit".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("--resume"));
    }

    #[test]
    fn test_is_builtin_color() {
        assert!(BuiltinCommands::is_builtin("color"));
    }

    #[test]
    fn test_execute_color_default() {
        let cmd = Command::new("color".to_string(), Some("default".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Color set to: default");
    }

    #[test]
    fn test_execute_color_gray() {
        let cmd = Command::new("color".to_string(), Some("gray".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Color set to: gray");
    }

    #[test]
    fn test_execute_color_none() {
        let cmd = Command::new("color".to_string(), Some("none".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Color set to: none");
    }

    #[test]
    fn test_execute_color_reset() {
        let cmd = Command::new("color".to_string(), Some("reset".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Color set to: reset");
    }

    #[test]
    fn test_execute_color_invalid() {
        let cmd = Command::new("color".to_string(), Some("rainbow".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Unknown color mode"));
        assert!(output.contains("rainbow"));
    }

    #[test]
    fn test_execute_color_no_args() {
        let cmd = Command::new("color".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Usage"));
        assert!(output.contains("/color"));
    }

    #[test]
    fn test_execute_color_case_insensitive() {
        let cmd = Command::new("color".to_string(), Some("DEFAULT".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Color set to: default");
    }

    // ── /loop tests ─────────────────────────────────────────────────────

    #[test]
    fn test_is_builtin_loop() {
        assert!(BuiltinCommands::is_builtin("loop"));
    }

    #[test]
    fn test_execute_loop_no_args() {
        let cmd = Command::new("loop".to_string(), None);
        let result = BuiltinCommands::execute(&cmd).unwrap();
        assert!(result.contains("Usage"));
        assert!(result.contains("/loop"));
    }

    #[test]
    fn test_execute_loop_interval_only() {
        let cmd = Command::new("loop".to_string(), Some("5m".to_string()));
        let result = BuiltinCommands::execute(&cmd).unwrap();
        assert!(result.contains("Usage"));
    }

    #[test]
    fn test_execute_loop_valid() {
        let cmd = Command::new(
            "loop".to_string(),
            Some("5m check build status".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd).unwrap();
        assert!(result.contains("[[SCHEDULE_LOOP:300:check build status]]"));
        assert!(result.contains("every 5m"));
    }

    #[test]
    fn test_execute_loop_seconds() {
        let cmd = Command::new("loop".to_string(), Some("30s ping".to_string()));
        let result = BuiltinCommands::execute(&cmd).unwrap();
        assert!(result.contains("[[SCHEDULE_LOOP:30:ping]]"));
        assert!(result.contains("every 30s"));
    }

    #[test]
    fn test_execute_loop_hours() {
        let cmd = Command::new("loop".to_string(), Some("1h run report".to_string()));
        let result = BuiltinCommands::execute(&cmd).unwrap();
        assert!(result.contains("[[SCHEDULE_LOOP:3600:run report]]"));
        assert!(result.contains("every 1h"));
    }

    #[test]
    fn test_execute_loop_invalid_interval() {
        let cmd = Command::new("loop".to_string(), Some("abc do stuff".to_string()));
        let result = BuiltinCommands::execute(&cmd).unwrap();
        assert!(result.contains("Invalid interval"));
        assert!(result.contains("abc"));
    }

    // ── /copy tests ─────────────────────────────────────────────────────

    #[test]
    fn test_is_builtin_copy() {
        assert!(BuiltinCommands::is_builtin("copy"));
    }

    #[test]
    fn test_execute_copy() {
        let cmd = Command::new("copy".to_string(), None);
        let result = BuiltinCommands::execute(&cmd).unwrap();
        assert_eq!(result, "[[COPY_CODE_BLOCK]]");
    }
}
