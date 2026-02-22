//! CLI argument definitions for RustyClawd.
//!
//! Contains the top-level `Cli` struct (parsed by clap) and the `Commands` enum
//! for subcommands. Pure data -- no business logic lives here.

use clap::{Parser, Subcommand};

/// Claude - AI assistant with tool use capabilities
#[derive(Parser)]
#[command(name = "claude")]
#[command(author = "Anthropic")]
#[command(version = "0.1.0")]
#[command(about = "RustyClawd - AI assistant command-line interface", long_about = None)]
#[command(disable_help_subcommand = true)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,

    /// Print mode - execute prompt and exit
    /// (alias: --prompt)
    #[arg(short = 'p', long = "print", alias = "prompt")]
    pub(crate) print_mode: bool,

    /// Continue from last session
    #[arg(short = 'c', long = "continue")]
    pub(crate) continue_session: bool,

    /// Resume specific session by ID (interactive selection if no ID provided)
    #[arg(short = 'r', long = "resume")]
    pub(crate) resume: Option<Option<String>>,

    /// Resume session linked to GitHub PR number
    #[arg(long = "from-pr")]
    pub(crate) from_pr: Option<u64>,

    /// Start in an isolated git worktree (v2.1.49)
    #[arg(short = 'w', long = "worktree")]
    pub(crate) worktree: bool,

    /// Model to use (e.g., "claude-sonnet-4-5-20250929")
    #[arg(long)]
    pub(crate) model: Option<String>,

    /// Replace entire system prompt with custom text
    #[arg(long)]
    pub(crate) system_prompt: Option<String>,

    /// Load system prompt from file, replacing default
    #[arg(long)]
    pub(crate) system_prompt_file: Option<String>,

    /// Append custom text to end of default system prompt
    #[arg(long)]
    pub(crate) append_system_prompt: Option<String>,

    /// Add additional working directories for Claude to access
    #[arg(long = "add-dir", value_name = "DIR")]
    pub(crate) add_dir: Vec<String>,

    /// Define custom subagents via JSON format
    #[arg(long, value_name = "JSON")]
    pub(crate) agents: Option<String>,

    /// List of tools allowed without prompting (e.g., "Bash(git log:*)" "Read")
    #[arg(long = "allowedTools", value_name = "TOOL")]
    pub(crate) allowed_tools: Vec<String>,

    /// List of tools that should be disallowed
    #[arg(long = "disallowedTools", value_name = "TOOL")]
    pub(crate) disallowed_tools: Vec<String>,

    /// Output format: text, json, stream-json
    #[arg(long, default_value = "text")]
    pub(crate) output_format: String,

    /// Input format: text, stream-json
    #[arg(long, default_value = "text")]
    pub(crate) input_format: String,

    /// Include streaming events in output
    #[arg(long)]
    pub(crate) include_partial_messages: bool,

    /// Enable verbose logging, shows full turn-by-turn output
    #[arg(long)]
    pub(crate) verbose: bool,

    /// Limit the number of agentic turns in non-interactive mode
    #[arg(long)]
    pub(crate) max_turns: Option<usize>,

    /// Specify permission mode for session
    #[arg(long)]
    pub(crate) permission_mode: Option<String>,

    /// Designate MCP tool for permission prompts
    #[arg(long)]
    pub(crate) permission_prompt_tool: Option<String>,

    /// Skip permission prompts (use with caution)
    #[arg(long)]
    pub(crate) dangerously_skip_permissions: bool,

    /// Fork from existing session ID
    #[arg(long)]
    pub(crate) fork_session: Option<String>,

    /// Specify fallback model when primary model fails
    #[arg(long)]
    pub(crate) fallback_model: Option<String>,

    /// Override settings file location
    #[arg(long)]
    pub(crate) settings: Option<String>,

    /// Enable IDE integration mode (structured JSON output)
    #[arg(long)]
    pub(crate) ide: bool,

    /// Override MCP configuration file location
    #[arg(long)]
    pub(crate) mcp_config: Option<String>,

    /// Resume from specific checkpoint number
    #[arg(long)]
    pub(crate) resume_from_checkpoint: Option<usize>,

    /// Override model capabilities (JSON format)
    #[arg(long)]
    pub(crate) model_capabilities: Option<String>,

    /// Skip safety checks and hooks (dangerous)
    #[arg(long)]
    pub(crate) dangerous_mode: bool,

    /// The prompt to execute (positional argument or from stdin)
    /// When provided, runs in print mode (one-shot execution)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub(crate) prompt: Vec<String>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Manage application updates
    Update {
        /// Check for available updates without installing
        #[arg(long)]
        check: bool,

        /// Force update check even if interval hasn't elapsed
        #[arg(long)]
        force: bool,

        /// Rollback to the previous version
        #[arg(long)]
        rollback: bool,
    },
    /// Manage Model Context Protocol (MCP) servers
    Mcp {
        /// MCP subcommand and arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Invoke a specialized agent with a prompt
    Agent {
        /// Type of agent to invoke (loads from .claude/agents/<type>.md)
        agent_type: String,

        /// Path to file containing the prompt
        #[arg(long)]
        prompt: String,

        /// Optional model override (haiku, sonnet, opus)
        #[arg(long)]
        model: Option<String>,
    },
    /// List all configured agents (v2.1.50)
    Agents,
    /// Manage authentication
    Auth {
        /// Auth subcommand: login, status, logout
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}
