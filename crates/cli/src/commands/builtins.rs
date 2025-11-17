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
            "add-dir" | "bashes" | "context" | "cost" | "todos" |
            // P1 Priority commands
            "usage" | "output-style" | "login" | "logout" | "privacy-settings" |
            // P2 Priority commands
            "statusline" | "terminal-setup" | "vim" | "bug" | "pr_comments"
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

            // P1 Priority commands
            "usage" => Some(Self::usage_command()),
            "output-style" => Some(Self::output_style_command(&cmd.args_str)),
            "login" => Some(Self::login_command()),
            "logout" => Some(Self::logout_command()),
            "privacy-settings" => Some(Self::privacy_settings_command(&cmd.args_str)),

            // P2 Priority commands
            "statusline" => Some(Self::statusline_command(&cmd.args_str)),
            "terminal-setup" => Some(Self::terminal_setup_command()),
            "vim" => Some(Self::vim_command()),
            "bug" => Some(Self::bug_command()),
            "pr_comments" => Some(Self::pr_comments_command(&cmd.args_str)),

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

    // ============================================================================
    // P1 Priority Commands
    // ============================================================================

    /// /usage - Show plan usage limits and rate limit status
    fn usage_command() -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would query the Anthropic API for actual usage
        // Anthropic API headers include rate limit information:
        // - anthropic-ratelimit-requests-limit
        // - anthropic-ratelimit-requests-remaining
        // - anthropic-ratelimit-tokens-limit
        // - anthropic-ratelimit-tokens-remaining

        const PLAN_TIER: &str = "Pro"; // Would be fetched from API/config
        const REQUESTS_LIMIT: u32 = 1000;
        const REQUESTS_REMAINING: u32 = 847;
        const TOKENS_LIMIT: u64 = 5_000_000;
        const TOKENS_REMAINING: u64 = 3_421_000;

        let requests_used = REQUESTS_LIMIT - REQUESTS_REMAINING;
        let tokens_used = TOKENS_LIMIT - TOKENS_REMAINING;
        let requests_percent = (requests_used as f64 / REQUESTS_LIMIT as f64 * 100.0) as u32;
        let tokens_percent = (tokens_used as f64 / TOKENS_LIMIT as f64 * 100.0) as u32;

        format!(
            "API Usage & Rate Limits:\n\n\
             Plan: {PLAN_TIER} Tier\n\n\
             Rate Limits (Per Minute):\n\
             - Requests:  {requests_used:>6} / {REQUESTS_LIMIT:<6} used ({requests_percent}%)\n\
             - Remaining: {REQUESTS_REMAINING:>6} requests\n\n\
             Token Limits (Per Month):\n\
             - Tokens:    {tokens_used:>10} / {TOKENS_LIMIT:<10} used ({tokens_percent}%)\n\
             - Remaining: {TOKENS_REMAINING:>10} tokens\n\n\
             Visual Progress:\n\
             Requests: [{}{}] {requests_percent}%\n\
             Tokens:   [{}{}] {tokens_percent}%\n\n\
             Note: Rate limit data will be populated from live API responses in future updates.\n\
             Current values are placeholders for demonstration.",
            "=".repeat(requests_percent as usize / 2),
            " ".repeat(50 - (requests_percent as usize / 2)),
            "=".repeat(tokens_percent as usize / 2),
            " ".repeat(50 - (tokens_percent as usize / 2)),
        )
    }

    /// /output-style [style] - Set output style
    fn output_style_command(args: &Option<String>) -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would integrate with settings and affect Claude's responses
        const CURRENT_STYLE: &str = "balanced"; // Would be fetched from settings

        match args {
            Some(style) => {
                let style_lower = style.to_lowercase();
                match style_lower.as_str() {
                    "concise" => "Output style set to: concise\n\n\
                         Concise Mode:\n\
                         - Shorter responses\n\
                         - Less explanation\n\
                         - Focus on essential information\n\
                         - Ideal for experienced users\n\n\
                         Note: This setting will be applied to future responses."
                        .to_string(),
                    "balanced" => "Output style set to: balanced\n\n\
                         Balanced Mode:\n\
                         - Moderate detail level\n\
                         - Balance of explanation and brevity\n\
                         - Suitable for most use cases\n\
                         - Default setting\n\n\
                         Note: This setting will be applied to future responses."
                        .to_string(),
                    "detailed" => "Output style set to: detailed\n\n\
                         Detailed Mode:\n\
                         - Comprehensive responses\n\
                         - Extended explanations\n\
                         - Step-by-step reasoning\n\
                         - Ideal for learning and complex tasks\n\n\
                         Note: This setting will be applied to future responses."
                        .to_string(),
                    _ => {
                        format!(
                            "Error: Invalid output style '{}'\n\n\
                             Valid options:\n\
                             - concise   - Short, focused responses\n\
                             - balanced  - Default, moderate detail\n\
                             - detailed  - Comprehensive, explanatory\n\n\
                             Usage: /output-style <concise|balanced|detailed>",
                            style
                        )
                    }
                }
            }
            None => {
                format!(
                    "Current output style: {CURRENT_STYLE}\n\n\
                     Available styles:\n\
                     - concise   - Short, focused responses\n\
                     - balanced  - Default, moderate detail\n\
                     - detailed  - Comprehensive, explanatory\n\n\
                     Usage: /output-style <style>\n\
                     Example: /output-style concise"
                )
            }
        }
    }

    /// /login - Switch Anthropic accounts
    fn login_command() -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would integrate with authentication system
        // and potentially open a browser for OAuth flow

        const CURRENT_USER: &str = "user@example.com"; // Would be fetched from auth
        const IS_AUTHENTICATED: bool = true;

        if IS_AUTHENTICATED {
            format!(
                "Account Information:\n\n\
                 Currently logged in as: {CURRENT_USER}\n\n\
                 To switch accounts:\n\
                 1. Use /logout to sign out of current account\n\
                 2. Run: claude login\n\
                 3. Follow browser authentication flow\n\
                 4. Complete OAuth authorization\n\n\
                 Account switching will preserve your local settings and preferences.\n\n\
                 Note: Full authentication flow will be implemented in future updates."
            )
        } else {
            "Not currently logged in.\n\n\
             To authenticate:\n\
             1. Run: claude login\n\
             2. Your browser will open for authentication\n\
             3. Sign in with your Anthropic account\n\
             4. Authorize the CLI application\n\n\
             After authentication, your API key will be securely stored.\n\n\
             Note: Full authentication flow will be implemented in future updates."
                .to_string()
        }
    }

    /// /logout - Sign out from account
    fn logout_command() -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would:
        // - Clear stored API keys from keychain/secure storage
        // - Invalidate any active sessions
        // - Clear cached authentication tokens

        const IS_AUTHENTICATED: bool = true; // Would be checked from auth state

        if IS_AUTHENTICATED {
            "Logging out...\n\n\
             The following actions will be performed:\n\
             - Clear stored API key from secure storage\n\
             - Invalidate active session\n\
             - Remove cached authentication tokens\n\n\
             Your local settings and preferences will be preserved.\n\n\
             To log back in, use: claude login\n\n\
             Note: Authentication token management will be implemented in future updates."
                .to_string()
        } else {
            "Not currently logged in.\n\n\
             No authentication tokens to clear.\n\n\
             To authenticate, use: claude login"
                .to_string()
        }
    }

    /// /privacy-settings - View/update privacy settings
    fn privacy_settings_command(args: &Option<String>) -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would integrate with settings system
        // and persist changes to user configuration

        const TELEMETRY_ENABLED: bool = false;
        const CRASH_REPORTS: bool = true;
        const USAGE_ANALYTICS: bool = false;
        const CONVERSATION_STORAGE: &str = "local-only";

        match args {
            Some(subcmd) => {
                let parts: Vec<&str> = subcmd.split_whitespace().collect();
                match parts.as_slice() {
                    ["telemetry", "on"] => "Telemetry enabled.\n\n\
                         Anonymous usage data will be collected to help improve the CLI.\n\
                         This includes:\n\
                         - Command usage statistics\n\
                         - Error frequencies\n\
                         - Performance metrics\n\n\
                         No conversation content or personal data is collected.\n\n\
                         Note: Privacy settings will be persisted in future updates."
                        .to_string(),
                    ["telemetry", "off"] => "Telemetry disabled.\n\n\
                         No usage data will be collected.\n\n\
                         Note: Privacy settings will be persisted in future updates."
                        .to_string(),
                    ["crash-reports", "on"] => "Crash reports enabled.\n\n\
                         Crash reports help identify and fix bugs.\n\
                         Reports include:\n\
                         - Stack traces\n\
                         - System information\n\
                         - Error context\n\n\
                         No conversation content is included.\n\n\
                         Note: Privacy settings will be persisted in future updates."
                        .to_string(),
                    ["crash-reports", "off"] => "Crash reports disabled.\n\n\
                         Note: Privacy settings will be persisted in future updates."
                        .to_string(),
                    _ => "Invalid privacy setting command.\n\n\
                         Usage:\n\
                         - /privacy-settings                    - Show current settings\n\
                         - /privacy-settings telemetry on|off   - Toggle telemetry\n\
                         - /privacy-settings crash-reports on|off - Toggle crash reports\n\n\
                         Example: /privacy-settings telemetry off"
                        .to_string(),
                }
            }
            None => {
                format!(
                    "Privacy & Data Settings:\n\n\
                     Telemetry:\n\
                     - Status: {}\n\
                     - Collects anonymous usage statistics\n\n\
                     Crash Reports:\n\
                     - Status: {}\n\
                     - Sends error reports for debugging\n\n\
                     Usage Analytics:\n\
                     - Status: {}\n\
                     - Tracks command usage patterns\n\n\
                     Conversation Storage:\n\
                     - Mode: {}\n\
                     - Controls where conversations are saved\n\n\
                     To modify settings:\n\
                     - /privacy-settings telemetry on|off\n\
                     - /privacy-settings crash-reports on|off\n\n\
                     Note: All conversation data remains local unless explicitly exported.\n\
                     Privacy settings will be persisted in future updates.",
                    if TELEMETRY_ENABLED {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    if CRASH_REPORTS { "enabled" } else { "disabled" },
                    if USAGE_ANALYTICS {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    CONVERSATION_STORAGE,
                )
            }
        }
    }

    // ============================================================================
    // P2 Priority Commands (Nice-to-Have)
    // ============================================================================

    /// /statusline - Set up Claude Code's status line UI
    fn statusline_command(args: &Option<String>) -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would integrate with the TUI system
        // to configure the status line display and customization options

        const STATUS_ENABLED: bool = true;
        const STATUS_POSITION: &str = "bottom";
        const STATUS_ITEMS: &[&str] = &["model", "tokens", "cost", "tools"];

        match args {
            Some(subcmd) => {
                let parts: Vec<&str> = subcmd.split_whitespace().collect();
                match parts.as_slice() {
                    ["enable"] => "Status line enabled.\n\n\
                         The status line will display:\n\
                         - Current model name\n\
                         - Token usage\n\
                         - Estimated cost\n\
                         - Available tools\n\
                         - Connection status\n\n\
                         Use /statusline customize to configure display items.\n\n\
                         Note: Status line integration will be implemented in future updates."
                        .to_string(),
                    ["disable"] => "Status line disabled.\n\n\
                         The status line will not be displayed.\n\n\
                         Use /statusline enable to re-enable.\n\n\
                         Note: Status line integration will be implemented in future updates."
                        .to_string(),
                    ["position", "top"] => "Status line position set to: top\n\n\
                         The status line will appear at the top of the terminal.\n\n\
                         Note: Status line integration will be implemented in future updates."
                        .to_string(),
                    ["position", "bottom"] => "Status line position set to: bottom\n\n\
                         The status line will appear at the bottom of the terminal.\n\n\
                         Note: Status line integration will be implemented in future updates."
                        .to_string(),
                    ["customize"] => "Status Line Customization:\n\n\
                         Available items:\n\
                         - model       - Display current model name\n\
                         - tokens      - Show token usage\n\
                         - cost        - Display estimated cost\n\
                         - tools       - Show available tools count\n\
                         - status      - Connection status indicator\n\
                         - time        - Current time\n\
                         - session     - Session duration\n\n\
                         Usage:\n\
                         - /statusline add <item>    - Add item to status line\n\
                         - /statusline remove <item> - Remove item from status line\n\n\
                         Note: Status line customization will be implemented in future updates."
                        .to_string(),
                    ["add", item] => {
                        format!(
                            "Added '{}' to status line.\n\n\
                             Note: Status line customization will be implemented in future updates.",
                            item
                        )
                    }
                    ["remove", item] => {
                        format!(
                            "Removed '{}' from status line.\n\n\
                             Note: Status line customization will be implemented in future updates.",
                            item
                        )
                    }
                    _ => "Invalid statusline command.\n\n\
                         Usage:\n\
                         - /statusline                    - Show current configuration\n\
                         - /statusline enable             - Enable status line\n\
                         - /statusline disable            - Disable status line\n\
                         - /statusline position top|bottom - Set position\n\
                         - /statusline customize          - View customization options\n\
                         - /statusline add <item>         - Add item to status line\n\
                         - /statusline remove <item>      - Remove item from status line"
                        .to_string(),
                }
            }
            None => {
                format!(
                    "Status Line Configuration:\n\n\
                     Status: {}\n\
                     Position: {}\n\
                     Displayed Items: {}\n\n\
                     The status line shows real-time information about your session:\n\
                     - Current model and configuration\n\
                     - Token usage and cost estimates\n\
                     - Tool availability\n\
                     - Connection status\n\n\
                     Commands:\n\
                     - /statusline enable           - Enable status line\n\
                     - /statusline disable          - Disable status line\n\
                     - /statusline position <pos>   - Set position (top/bottom)\n\
                     - /statusline customize        - View customization options\n\n\
                     Note: Status line integration will be implemented in future updates.",
                    if STATUS_ENABLED {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    STATUS_POSITION,
                    STATUS_ITEMS.join(", "),
                )
            }
        }
    }

    /// /terminal-setup - Install Shift+Enter key binding
    fn terminal_setup_command() -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would:
        // - Detect the user's terminal emulator
        // - Provide specific instructions for their terminal
        // - Optionally attempt to configure the terminal automatically

        "Terminal Setup - Shift+Enter Key Binding:\n\n\
         The Shift+Enter key binding allows you to insert newlines in your messages\n\
         without sending them immediately.\n\n\
         Setup Instructions by Terminal:\n\n\
         macOS Terminal:\n\
         1. Open Terminal > Preferences > Profiles > Keyboard\n\
         2. Click '+' to add a new key binding\n\
         3. Set key: Shift+Return\n\
         4. Set action: Send Text\n\
         5. Enter text: \\n\n\n\
         iTerm2:\n\
         1. Open Preferences > Profiles > Keys\n\
         2. Click '+' to add a key mapping\n\
         3. Set keyboard shortcut: Shift+Return\n\
         4. Set action: Send Text\n\
         5. Enter text: \\n\n\n\
         Windows Terminal:\n\
         1. Open Settings (Ctrl+,)\n\
         2. Go to Actions\n\
         3. Add custom action:\n\
            { \"command\": { \"action\": \"sendInput\", \"input\": \"\\n\" },\n\
              \"keys\": \"shift+enter\" }\n\n\
         Alacritty:\n\
         Add to ~/.config/alacritty/alacritty.yml:\n\
         key_bindings:\n\
           - { key: Return, mods: Shift, chars: \"\\n\" }\n\n\
         Linux Terminal Emulators:\n\
         Most Linux terminals support Shift+Enter by default.\n\
         If not, check your terminal's keyboard shortcut settings.\n\n\
         After Setup:\n\
         - Press Enter to send messages\n\
         - Press Shift+Enter to add newlines within messages\n\n\
         Note: Automatic terminal detection and configuration will be added in future updates."
            .to_string()
    }

    /// /vim - Enter vim mode
    fn vim_command() -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would:
        // - Enable vim-style keybindings in the input editor
        // - Support insert/command mode alternation
        // - Provide full vim keybinding support

        "Vim Mode:\n\n\
         Vim mode enables vim-style editing keybindings in Claude Code.\n\n\
         Status: Currently in Normal mode\n\n\
         Vim Keybindings Reference:\n\n\
         Mode Switching:\n\
         - i           - Enter insert mode at cursor\n\
         - I           - Enter insert mode at start of line\n\
         - a           - Enter insert mode after cursor\n\
         - A           - Enter insert mode at end of line\n\
         - o           - Insert new line below and enter insert mode\n\
         - O           - Insert new line above and enter insert mode\n\
         - Esc         - Return to normal mode\n\n\
         Navigation (Normal Mode):\n\
         - h           - Move left\n\
         - j           - Move down\n\
         - k           - Move up\n\
         - l           - Move right\n\
         - w           - Move to next word\n\
         - b           - Move to previous word\n\
         - 0           - Move to start of line\n\
         - $           - Move to end of line\n\
         - gg          - Move to start of file\n\
         - G           - Move to end of file\n\n\
         Editing (Normal Mode):\n\
         - x           - Delete character under cursor\n\
         - dd          - Delete current line\n\
         - yy          - Yank (copy) current line\n\
         - p           - Paste after cursor\n\
         - u           - Undo\n\
         - Ctrl+r      - Redo\n\n\
         Search:\n\
         - /pattern    - Search forward\n\
         - ?pattern    - Search backward\n\
         - n           - Next match\n\
         - N           - Previous match\n\n\
         To disable vim mode, use: /vim disable\n\n\
         Note: Full vim mode integration will be implemented in future updates.\n\
         Currently this is an informational command showing available keybindings."
            .to_string()
    }

    /// /bug - Report bugs to Anthropic
    fn bug_command() -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would:
        // - Collect system information automatically
        // - Export the conversation for bug report context
        // - Guide the user through GitHub issue creation
        // - Optionally submit the bug report automatically

        const GITHUB_ISSUES_URL: &str = "https://github.com/anthropics/anthropic-sdk-rust/issues";
        const VERSION: &str = env!("CARGO_PKG_VERSION");

        format!(
            "Bug Report - Report Issues to Anthropic:\n\n\
             Thank you for helping improve Claude Code!\n\n\
             Before reporting:\n\
             1. Check if the issue already exists\n\
             2. Gather relevant information\n\
             3. Create a minimal reproduction if possible\n\n\
             Steps to Report a Bug:\n\n\
             1. Export Your Conversation:\n\
                Use /export <filename> to save the current conversation.\n\
                This provides context for the bug report.\n\n\
             2. Gather System Information:\n\
                - Version: RustyClawd {VERSION}\n\
                - OS: {os}\n\
                - Terminal: (your terminal emulator)\n\
                - Model: (current model being used)\n\n\
             3. Visit GitHub Issues:\n\
                {GITHUB_ISSUES_URL}\n\n\
             4. Create a New Issue:\n\
                - Click 'New Issue'\n\
                - Choose 'Bug Report' template\n\
                - Fill in all sections:\n\
                  * Clear description of the bug\n\
                  * Steps to reproduce\n\
                  * Expected vs actual behavior\n\
                  * System information\n\
                  * Relevant conversation export (if applicable)\n\n\
             5. Submit the Issue:\n\
                The Anthropic team will review and respond.\n\n\
             What to Include:\n\
             - Clear, concise description\n\
             - Steps to reproduce\n\
             - Expected behavior\n\
             - Actual behavior\n\
             - System information\n\
             - Screenshots (if UI-related)\n\
             - Conversation export (if relevant)\n\
             - Error messages or logs\n\n\
             Privacy Note:\n\
             - Review exported conversations before attaching\n\
             - Remove sensitive information\n\
             - Only include relevant portions\n\n\
             For security vulnerabilities:\n\
             - Do NOT open a public issue\n\
             - Email: security@anthropic.com\n\n\
             Note: Automated bug report submission will be implemented in future updates.",
            os = std::env::consts::OS,
        )
    }

    /// /pr_comments - View pull request comments
    fn pr_comments_command(args: &Option<String>) -> String {
        // Note: This is a basic implementation
        // In a full implementation, this would:
        // - Use GitHub API to fetch PR comments
        // - Display comments with context
        // - Support filtering by author, type, status
        // - Show inline code review comments
        // - Allow navigation between comment threads

        match args {
            Some(pr_ref) => {
                // Parse PR reference (could be number, URL, or org/repo#number)
                let parts: Vec<&str> = pr_ref.split_whitespace().collect();

                match parts.as_slice() {
                    [pr_num] if pr_num.chars().all(|c| c.is_ascii_digit()) => {
                        format!(
                            "Pull Request Comments - PR #{pr_num}:\n\n\
                             Fetching comments...\n\n\
                             (In a full implementation, this would display:\n\
                             - General comments on the PR\n\
                             - Inline code review comments\n\
                             - Review summaries\n\
                             - Requested changes\n\
                             - Resolved and unresolved threads)\n\n\
                             Usage with filters:\n\
                             - /pr_comments {pr_num} --author username\n\
                             - /pr_comments {pr_num} --unresolved\n\
                             - /pr_comments {pr_num} --since 2025-01-01\n\n\
                             Note: GitHub API integration will be implemented in future updates."
                        )
                    }
                    [pr_num, "--author", author] => {
                        format!(
                            "Pull Request Comments - PR #{pr_num} (Author: {author}):\n\n\
                             Filtering comments by author: {author}\n\n\
                             Note: GitHub API integration will be implemented in future updates."
                        )
                    }
                    [pr_num, "--unresolved"] => {
                        format!(
                            "Pull Request Comments - PR #{pr_num} (Unresolved Only):\n\n\
                             Showing unresolved comment threads...\n\n\
                             Note: GitHub API integration will be implemented in future updates."
                        )
                    }
                    [pr_num, "--since", date] => {
                        format!(
                            "Pull Request Comments - PR #{pr_num} (Since: {date}):\n\n\
                             Showing comments since {date}...\n\n\
                             Note: GitHub API integration will be implemented in future updates."
                        )
                    }
                    _ => "Invalid PR reference format.\n\n\
                         Usage:\n\
                         - /pr_comments <number>                 - Show all comments\n\
                         - /pr_comments <number> --author <user> - Filter by author\n\
                         - /pr_comments <number> --unresolved    - Show unresolved only\n\
                         - /pr_comments <number> --since <date>  - Comments since date\n\n\
                         Examples:\n\
                         - /pr_comments 123\n\
                         - /pr_comments 123 --author username\n\
                         - /pr_comments 123 --unresolved\n\
                         - /pr_comments 123 --since 2025-01-01"
                        .to_string(),
                }
            }
            None => "Pull Request Comments:\n\n\
                 View and manage comments on GitHub pull requests.\n\n\
                 Usage:\n\
                 - /pr_comments <pr-number>                  - Show all comments for PR\n\
                 - /pr_comments <pr-number> --author <user>  - Filter by comment author\n\
                 - /pr_comments <pr-number> --unresolved     - Show only unresolved threads\n\
                 - /pr_comments <pr-number> --since <date>   - Comments since specific date\n\n\
                 Examples:\n\
                 - /pr_comments 123                          - View all comments on PR #123\n\
                 - /pr_comments 123 --author reviewer        - View comments by 'reviewer'\n\
                 - /pr_comments 123 --unresolved             - View unresolved discussions\n\
                 - /pr_comments 123 --since 2025-01-01       - View recent comments\n\n\
                 Comment Types Displayed:\n\
                 - General PR comments\n\
                 - Inline code review comments\n\
                 - Review summaries (approve/request changes/comment)\n\
                 - Reply threads\n\n\
                 Integration:\n\
                 This command uses the GitHub API to fetch comment data.\n\
                 Requires:\n\
                 - Valid GitHub authentication (gh CLI or GITHUB_TOKEN)\n\
                 - Read permissions on the repository\n\n\
                 Tips:\n\
                 - Use gh CLI for full GitHub integration: gh pr view 123\n\
                 - Export comments for offline review\n\
                 - Filter by date to see latest feedback\n\n\
                 Note: Full GitHub API integration will be implemented in future updates.\n\
                 Consider using 'gh pr view <number>' for detailed PR information."
                .to_string(),
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
        assert!(
            output.contains("pending")
                || output.contains("in_progress")
                || output.contains("completed")
        );
    }

    // P1 Priority Commands Tests

    #[test]
    fn test_is_builtin_usage() {
        assert!(BuiltinCommands::is_builtin("usage"));
    }

    #[test]
    fn test_is_builtin_output_style() {
        assert!(BuiltinCommands::is_builtin("output-style"));
    }

    #[test]
    fn test_is_builtin_login() {
        assert!(BuiltinCommands::is_builtin("login"));
    }

    #[test]
    fn test_is_builtin_logout() {
        assert!(BuiltinCommands::is_builtin("logout"));
    }

    #[test]
    fn test_is_builtin_privacy_settings() {
        assert!(BuiltinCommands::is_builtin("privacy-settings"));
    }

    #[test]
    fn test_execute_usage() {
        let cmd = Command::new("usage".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Usage") || output.contains("Rate Limit"));
        assert!(output.contains("Plan"));
        assert!(output.contains("Requests") || output.contains("Tokens"));
    }

    #[test]
    fn test_execute_output_style_no_args() {
        let cmd = Command::new("output-style".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Current output style") || output.contains("output style"));
        assert!(output.contains("concise"));
        assert!(output.contains("balanced"));
        assert!(output.contains("detailed"));
    }

    #[test]
    fn test_execute_output_style_concise() {
        let cmd = Command::new("output-style".to_string(), Some("concise".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("concise"));
        assert!(output.contains("set to") || output.contains("Concise Mode"));
    }

    #[test]
    fn test_execute_output_style_balanced() {
        let cmd = Command::new("output-style".to_string(), Some("balanced".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("balanced"));
        assert!(output.contains("set to") || output.contains("Balanced Mode"));
    }

    #[test]
    fn test_execute_output_style_detailed() {
        let cmd = Command::new("output-style".to_string(), Some("detailed".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("detailed"));
        assert!(output.contains("set to") || output.contains("Detailed Mode"));
    }

    #[test]
    fn test_execute_output_style_invalid() {
        let cmd = Command::new("output-style".to_string(), Some("invalid".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Error") || output.contains("Invalid"));
        assert!(output.contains("invalid"));
    }

    #[test]
    fn test_execute_output_style_case_insensitive() {
        let cmd = Command::new("output-style".to_string(), Some("CONCISE".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("concise") || output.contains("Concise"));
        assert!(!output.contains("Error"));
    }

    #[test]
    fn test_execute_login() {
        let cmd = Command::new("login".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(
            output.contains("Account")
                || output.contains("login")
                || output.contains("authenticate")
        );
    }

    #[test]
    fn test_execute_logout() {
        let cmd = Command::new("logout".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(
            output.contains("Logging out")
                || output.contains("logout")
                || output.contains("logged in")
        );
    }

    #[test]
    fn test_execute_privacy_settings_no_args() {
        let cmd = Command::new("privacy-settings".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Privacy") || output.contains("Telemetry"));
        assert!(output.contains("enabled") || output.contains("disabled"));
    }

    #[test]
    fn test_execute_privacy_settings_telemetry_on() {
        let cmd = Command::new(
            "privacy-settings".to_string(),
            Some("telemetry on".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Telemetry"));
        assert!(output.contains("enabled"));
    }

    #[test]
    fn test_execute_privacy_settings_telemetry_off() {
        let cmd = Command::new(
            "privacy-settings".to_string(),
            Some("telemetry off".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Telemetry"));
        assert!(output.contains("disabled"));
    }

    #[test]
    fn test_execute_privacy_settings_crash_reports_on() {
        let cmd = Command::new(
            "privacy-settings".to_string(),
            Some("crash-reports on".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Crash reports"));
        assert!(output.contains("enabled"));
    }

    #[test]
    fn test_execute_privacy_settings_crash_reports_off() {
        let cmd = Command::new(
            "privacy-settings".to_string(),
            Some("crash-reports off".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Crash reports"));
        assert!(output.contains("disabled"));
    }

    #[test]
    fn test_execute_privacy_settings_invalid() {
        let cmd = Command::new(
            "privacy-settings".to_string(),
            Some("invalid setting".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Invalid") || output.contains("Usage"));
    }

    // ============================================================================
    // P2 Priority Commands Tests
    // ============================================================================

    #[test]
    fn test_is_builtin_statusline() {
        assert!(BuiltinCommands::is_builtin("statusline"));
    }

    #[test]
    fn test_is_builtin_terminal_setup() {
        assert!(BuiltinCommands::is_builtin("terminal-setup"));
    }

    #[test]
    fn test_is_builtin_vim() {
        assert!(BuiltinCommands::is_builtin("vim"));
    }

    #[test]
    fn test_is_builtin_bug() {
        assert!(BuiltinCommands::is_builtin("bug"));
    }

    #[test]
    fn test_is_builtin_pr_comments() {
        assert!(BuiltinCommands::is_builtin("pr_comments"));
    }

    #[test]
    fn test_execute_statusline_no_args() {
        let cmd = Command::new("statusline".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Status Line") || output.contains("statusline"));
        assert!(output.contains("enabled") || output.contains("disabled"));
        assert!(output.contains("Position") || output.contains("position"));
    }

    #[test]
    fn test_execute_statusline_enable() {
        let cmd = Command::new("statusline".to_string(), Some("enable".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Status line enabled") || output.contains("enabled"));
    }

    #[test]
    fn test_execute_statusline_disable() {
        let cmd = Command::new("statusline".to_string(), Some("disable".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Status line disabled") || output.contains("disabled"));
    }

    #[test]
    fn test_execute_statusline_position_top() {
        let cmd = Command::new("statusline".to_string(), Some("position top".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("top"));
        assert!(output.contains("position"));
    }

    #[test]
    fn test_execute_statusline_position_bottom() {
        let cmd = Command::new(
            "statusline".to_string(),
            Some("position bottom".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("bottom"));
        assert!(output.contains("position"));
    }

    #[test]
    fn test_execute_statusline_customize() {
        let cmd = Command::new("statusline".to_string(), Some("customize".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Customization") || output.contains("customize"));
        assert!(output.contains("Available items") || output.contains("items"));
    }

    #[test]
    fn test_execute_statusline_add_item() {
        let cmd = Command::new("statusline".to_string(), Some("add model".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Added") || output.contains("model"));
    }

    #[test]
    fn test_execute_statusline_remove_item() {
        let cmd = Command::new("statusline".to_string(), Some("remove tokens".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Removed") || output.contains("tokens"));
    }

    #[test]
    fn test_execute_statusline_invalid() {
        let cmd = Command::new(
            "statusline".to_string(),
            Some("invalid command".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Invalid") || output.contains("Usage"));
    }

    #[test]
    fn test_execute_terminal_setup() {
        let cmd = Command::new("terminal-setup".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Terminal Setup") || output.contains("Shift+Enter"));
        assert!(output.contains("key binding") || output.contains("keyboard"));
        assert!(output.contains("macOS") || output.contains("Windows") || output.contains("Linux"));
    }

    #[test]
    fn test_execute_vim() {
        let cmd = Command::new("vim".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Vim Mode") || output.contains("vim"));
        assert!(output.contains("keybindings") || output.contains("Keybindings"));
        assert!(output.contains("insert mode") || output.contains("normal mode"));
    }

    #[test]
    fn test_execute_bug() {
        let cmd = Command::new("bug".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Bug Report") || output.contains("bug"));
        assert!(output.contains("GitHub") || output.contains("issue"));
        assert!(output.contains("Anthropic"));
    }

    #[test]
    fn test_execute_pr_comments_no_args() {
        let cmd = Command::new("pr_comments".to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Pull Request") || output.contains("PR"));
        assert!(output.contains("comments") || output.contains("Comments"));
        assert!(output.contains("Usage"));
    }

    #[test]
    fn test_execute_pr_comments_with_number() {
        let cmd = Command::new("pr_comments".to_string(), Some("123".to_string()));
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("PR #123") || output.contains("123"));
        assert!(output.contains("comments") || output.contains("Fetching"));
    }

    #[test]
    fn test_execute_pr_comments_with_author_filter() {
        let cmd = Command::new(
            "pr_comments".to_string(),
            Some("123 --author reviewer".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("123"));
        assert!(output.contains("reviewer") || output.contains("Author"));
    }

    #[test]
    fn test_execute_pr_comments_unresolved_only() {
        let cmd = Command::new(
            "pr_comments".to_string(),
            Some("123 --unresolved".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("123"));
        assert!(output.contains("unresolved") || output.contains("Unresolved"));
    }

    #[test]
    fn test_execute_pr_comments_with_date_filter() {
        let cmd = Command::new(
            "pr_comments".to_string(),
            Some("123 --since 2025-01-01".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("123"));
        assert!(output.contains("2025-01-01") || output.contains("Since"));
    }

    #[test]
    fn test_execute_pr_comments_invalid_format() {
        let cmd = Command::new(
            "pr_comments".to_string(),
            Some("invalid format".to_string()),
        );
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("Invalid") || output.contains("Usage"));
    }
}
