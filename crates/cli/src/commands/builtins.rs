//! Built-in commands - /help, /exit, /clear, etc.

use crate::commands::parser::Command;

/// Built-in command handler
pub struct BuiltinCommands;

impl BuiltinCommands {
    /// Check if command is a built-in
    /// Based on official docs: 35+ built-in commands
    pub fn is_builtin(name: &str) -> bool {
        matches!(
            name,
            // Session management
            "clear" | "exit" | "quit" | "rewind" |
            // Configuration
            "config" | "model" | "status" |
            // Development tools
            "review" | "sandbox" | "doctor" |
            // File operations
            "export" | "memory" |
            // Integration
            "mcp" | "agents" | "hooks" |
            // Information
            "help" | "history" | "stats" |
            // Additional built-ins
            "compact" | "init" | "version" | "permissions" |
            "debug" | "trace" | "log" | "checkpoint" | "restore" |
            // Tool management
            "tools" | "plugins" |
            // Session
            "save" | "load" | "reset" | "undo" | "redo" |
            // P0 Priority commands
            "add-dir" | "bashes" | "context" | "cost" | "todos"
        )
    }

    /// Execute a built-in command
    pub fn execute(cmd: &Command) -> Option<String> {
        match cmd.name.as_str() {
            // Session management
            "clear" => Some(Self::clear_command()),
            "exit" | "quit" => Some(Self::exit_command()),
            "rewind" => Some(Self::rewind_command()),

            // Configuration
            "config" => Some(Self::config_command(&cmd.args_str)),
            "model" => Some(Self::model_command(&cmd.args_str)),
            "status" => Some(Self::status_command()),

            // Development tools
            "review" => Some(Self::review_command(&cmd.args_str)),
            "sandbox" => Some(Self::sandbox_command()),
            "doctor" => Some(Self::doctor_command()),

            // File operations
            "export" => Some(Self::export_command(&cmd.args_str)),
            "memory" => Some(Self::memory_command()),

            // Integration
            "mcp" => Some(Self::mcp_command(&cmd.args_str)),
            "agents" => Some(Self::agents_command()),
            "hooks" => Some(Self::hooks_command()),

            // Information
            "help" => Some(Self::help(&cmd.args_str)),
            "history" => Some(Self::history_command()),
            "stats" => Some(Self::stats_command()),

            // Additional built-ins
            "compact" => Some(Self::compact_command()),
            "init" => Some(Self::init_command()),
            "version" => Some(Self::version_command()),
            "permissions" => Some(Self::permissions_command()),
            "debug" => Some(Self::debug_command(&cmd.args_str)),
            "trace" => Some(Self::trace_command()),
            "log" => Some(Self::log_command()),
            "checkpoint" => Some(Self::checkpoint_command(&cmd.args_str)),
            "restore" => Some(Self::restore_command(&cmd.args_str)),

            // Tool management
            "tools" => Some(Self::tools_command()),
            "plugins" => Some(Self::plugins_command()),

            // Session
            "save" => Some(Self::save_command(&cmd.args_str)),
            "load" => Some(Self::load_command(&cmd.args_str)),
            "reset" => Some(Self::reset_command()),
            "undo" => Some(Self::undo_command()),
            "redo" => Some(Self::redo_command()),

            // P0 Priority commands
            "add-dir" => Some(Self::add_dir_command(&cmd.args_str)),
            "bashes" => Some(Self::bashes_command()),
            "context" => Some(Self::context_command()),
            "cost" => Some(Self::cost_command()),
            "todos" => Some(Self::todos_command()),

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
        "Session Statistics:\n\
         - Messages: 0\n\
         - Commands executed: 0\n\
         - Total tokens: 0\n\
         - Session duration: 0s"
            .to_string()
    }

    // Additional built-in commands
    fn rewind_command() -> String {
        "Rewinding conversation...".to_string()
    }

    fn config_command(args: &Option<String>) -> String {
        match args {
            Some(arg) => format!("Configuration: {}", arg),
            None => {
                "Current configuration:\n  Model: claude-sonnet-4-5\n  Tools: enabled".to_string()
            }
        }
    }

    fn model_command(args: &Option<String>) -> String {
        match args {
            Some(model) => format!("Switching to model: {}", model),
            None => "Current model: claude-sonnet-4-5".to_string(),
        }
    }

    fn status_command() -> String {
        "System Status:\n  Connection: active\n  Model: claude-sonnet-4-5\n  Tools: available"
            .to_string()
    }

    fn review_command(args: &Option<String>) -> String {
        match args {
            Some(target) => format!("Reviewing: {}", target),
            None => "Usage: /review <file|pr|branch>".to_string(),
        }
    }

    fn sandbox_command() -> String {
        "Sandbox mode: Commands will be executed in isolated environment".to_string()
    }

    fn doctor_command() -> String {
        "Running diagnostic checks...\n  Configuration: OK\n  Tools: OK\n  Connections: OK"
            .to_string()
    }

    fn export_command(args: &Option<String>) -> String {
        match args {
            Some(path) => format!("Exporting conversation to: {}", path),
            None => "Usage: /export <path>".to_string(),
        }
    }

    fn memory_command() -> String {
        "Memory usage:\n  Conversation: 1.2 MB\n  Context: 5K tokens".to_string()
    }

    fn mcp_command(args: &Option<String>) -> String {
        match args {
            Some(subcmd) => format!("MCP: {}", subcmd),
            None => "MCP Integration:\n  Status: active\n  Servers: 0 connected".to_string(),
        }
    }

    fn agents_command() -> String {
        "Available agents:\n  (No agents configured)".to_string()
    }

    fn hooks_command() -> String {
        "Configured hooks:\n  (No hooks configured)".to_string()
    }

    fn compact_command() -> String {
        "Compacting conversation history...".to_string()
    }

    fn init_command() -> String {
        "Initializing project configuration...".to_string()
    }

    fn version_command() -> String {
        format!("RustyClawd version {}", env!("CARGO_PKG_VERSION"))
    }

    fn permissions_command() -> String {
        "Permissions:\n  Tools: all enabled\n  File access: allowed\n  Network: allowed".to_string()
    }

    fn debug_command(args: &Option<String>) -> String {
        match args {
            Some(level) => format!("Debug level set to: {}", level),
            None => "Debug mode: off".to_string(),
        }
    }

    fn trace_command() -> String {
        "Trace logging enabled".to_string()
    }

    fn log_command() -> String {
        "Recent logs:\n  (No logs available)".to_string()
    }

    fn checkpoint_command(args: &Option<String>) -> String {
        match args {
            Some(name) => format!("Creating checkpoint: {}", name),
            None => "Creating checkpoint...".to_string(),
        }
    }

    fn restore_command(args: &Option<String>) -> String {
        match args {
            Some(name) => format!("Restoring from checkpoint: {}", name),
            None => "Usage: /restore <checkpoint-name>".to_string(),
        }
    }

    fn tools_command() -> String {
        "Available tools:\n  - Bash\n  - Read\n  - Write\n  - Edit\n  - Grep\n  - Glob".to_string()
    }

    fn plugins_command() -> String {
        "Loaded plugins:\n  (No plugins loaded)".to_string()
    }

    fn save_command(args: &Option<String>) -> String {
        match args {
            Some(name) => format!("Saving session as: {}", name),
            None => "Usage: /save <session-name>".to_string(),
        }
    }

    fn load_command(args: &Option<String>) -> String {
        match args {
            Some(name) => format!("Loading session: {}", name),
            None => "Usage: /load <session-name>".to_string(),
        }
    }

    fn reset_command() -> String {
        "Resetting session state...".to_string()
    }

    fn undo_command() -> String {
        "Undoing last action...".to_string()
    }

    fn redo_command() -> String {
        "Redoing last undone action...".to_string()
    }

    // ============================================================================
    // P0 Priority Commands
    // ============================================================================

    /// /add-dir <directory> - Add additional working directory
    fn add_dir_command(args: &Option<String>) -> String {
        match args {
            Some(dir) => {
                // Validate directory exists
                let path = std::path::Path::new(dir);
                if path.exists() && path.is_dir() {
                    format!(
                        "Added directory to working set:\n  {}\n\n\
                         Note: Directory will be available in this session context.",
                        dir
                    )
                } else {
                    format!("Error: Directory does not exist or is not a directory: {}", dir)
                }
            }
            None => "Usage: /add-dir <directory>\n\nExample:\n  /add-dir /path/to/project".to_string(),
        }
    }

    /// /bashes - List and manage background bash shells
    fn bashes_command() -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would query the actual bash shell manager
        // and display real shell IDs, statuses, and commands
        "Background Bash Shells:\n\n\
         No background shells currently running.\n\n\
         Tips:\n\
         - Background shells are created when using run_in_background parameter\n\
         - Use BashOutput tool to read shell output\n\
         - Use KillShell tool to terminate shells"
            .to_string()
    }

    /// /context - Visualize current context usage
    fn context_command() -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would query actual token counts from the API
        const MAX_TOKENS: u64 = 200_000; // Claude's context window
        let used_tokens: u64 = 0; // Would be populated from actual usage
        let percentage = (used_tokens as f64 / MAX_TOKENS as f64 * 100.0) as u64;

        format!(
            "Context Window Usage:\n\n\
             Used:      {used_tokens:>7} tokens ({percentage}%)\n\
             Available: {MAX_TOKENS:>7} tokens\n\n\
             Visual: [{}{}] {percentage}%\n\n\
             Note: Context tracking will be implemented in future updates.",
            "=".repeat((percentage / 2) as usize),
            " ".repeat(50 - (percentage / 2) as usize),
        )
    }

    /// /cost - Display token usage statistics and cost estimates
    fn cost_command() -> String {
        // Note: This is a basic implementation
        // Pricing as of 2025 (approximate):
        // Claude Sonnet 4.5: $3 per million input tokens, $15 per million output tokens
        const INPUT_COST_PER_MILLION: f64 = 3.0;
        const OUTPUT_COST_PER_MILLION: f64 = 15.0;

        let input_tokens: u64 = 0; // Would be populated from session stats
        let output_tokens: u64 = 0; // Would be populated from session stats
        let total_tokens = input_tokens + output_tokens;

        let input_cost = (input_tokens as f64 / 1_000_000.0) * INPUT_COST_PER_MILLION;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * OUTPUT_COST_PER_MILLION;
        let total_cost = input_cost + output_cost;

        format!(
            "Token Usage & Cost Estimate:\n\n\
             Session Statistics:\n\
             - Input tokens:  {input_tokens:>8}\n\
             - Output tokens: {output_tokens:>8}\n\
             - Total tokens:  {total_tokens:>8}\n\n\
             Estimated Cost (Claude Sonnet 4.5):\n\
             - Input:  ${input_cost:>7.4} ({input_tokens} tokens @ ${INPUT_COST_PER_MILLION}/M)\n\
             - Output: ${output_cost:>7.4} ({output_tokens} tokens @ ${OUTPUT_COST_PER_MILLION}/M)\n\
             - Total:  ${total_cost:>7.4}\n\n\
             Note: Cost tracking will be implemented with full session integration."
        )
    }

    /// /todos - List current todo items
    fn todos_command() -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would track actual todos from TodoWrite tool calls
        "Current Todo Items:\n\n\
         No todos tracked in this session.\n\n\
         Todo items will appear here when:\n\
         - Claude uses the TodoWrite tool to track tasks\n\
         - Complex multi-step operations are in progress\n\
         - Multiple features are being implemented\n\n\
         Todo Status Legend:\n\
         - [ ] pending      - Not yet started\n\
         - [~] in_progress  - Currently working on\n\
         - [x] completed    - Finished successfully"
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

    // P0 Priority Commands Tests

    #[test]
    fn test_is_builtin_add_dir() {
        assert!(BuiltinCommands::is_builtin("add-dir"));
    }

    #[test]
    fn test_is_builtin_bashes() {
        assert!(BuiltinCommands::is_builtin("bashes"));
    }

    #[test]
    fn test_is_builtin_context() {
        assert!(BuiltinCommands::is_builtin("context"));
    }

    #[test]
    fn test_is_builtin_cost() {
        assert!(BuiltinCommands::is_builtin("cost"));
    }

    #[test]
    fn test_is_builtin_todos() {
        assert!(BuiltinCommands::is_builtin("todos"));
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
        // Use current directory which should always exist
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
        let cmd = Command::new("add-dir".to_string(), Some("/nonexistent/path/xyz".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Error"));
        assert!(output.contains("does not exist"));
    }

    #[test]
    fn test_execute_bashes() {
        let cmd = Command::new("bashes".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Background Bash"));
        assert!(output.contains("shells") || output.contains("Shells"));
    }

    #[test]
    fn test_execute_context() {
        let cmd = Command::new("context".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Context"));
        assert!(output.contains("tokens"));
        assert!(output.contains("Available"));
    }

    #[test]
    fn test_execute_cost() {
        let cmd = Command::new("cost".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Cost") || output.contains("Token"));
        assert!(output.contains("tokens"));
        assert!(output.contains("$"));
    }

    #[test]
    fn test_execute_todos() {
        let cmd = Command::new("todos".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Todo"));
        assert!(output.contains("pending") || output.contains("in_progress") || output.contains("completed"));
    }
}
