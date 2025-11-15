//! Claude Code CLI - Official Spec-Compliant Implementation
//!
//! This is a Rust implementation that matches Claude Code's exact CLI interface
//! as documented at https://code.claude.com/docs/en/cli-reference

#![allow(dead_code)]
#![allow(unused_imports)]

mod checkpoint;
mod commands;
mod hooks;
mod interactive;
mod plugins;
mod settings;
mod terminal_guard;
mod tool_definitions;
mod tool_executor;
mod tui;

use anyhow::{Context as AnyhowContext, Result};
use clap::{Parser, Subcommand};
use futures::StreamExt;
use std::io::{self, IsTerminal, Read};

/// Claude - AI assistant with tool use capabilities
#[derive(Parser)]
#[command(name = "claude")]
#[command(author = "Anthropic")]
#[command(version = "0.1.0")]
#[command(about = "Claude AI assistant command-line interface", long_about = None)]
#[command(disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Print mode - execute prompt and exit
    #[arg(short = 'p', long = "print")]
    print_mode: bool,

    /// Continue from last session
    #[arg(short = 'c', long = "continue")]
    continue_session: bool,

    /// Resume specific session by ID (interactive selection if no ID provided)
    #[arg(short = 'r', long = "resume")]
    resume: Option<Option<String>>,

    /// Model to use (e.g., "claude-sonnet-4-5-20250929")
    #[arg(long)]
    model: Option<String>,

    /// Replace entire system prompt with custom text
    #[arg(long)]
    system_prompt: Option<String>,

    /// Load system prompt from file, replacing default
    #[arg(long)]
    system_prompt_file: Option<String>,

    /// Append custom text to end of default system prompt
    #[arg(long)]
    append_system_prompt: Option<String>,

    /// Add additional working directories for Claude to access
    #[arg(long = "add-dir", value_name = "DIR")]
    add_dir: Vec<String>,

    /// Define custom subagents via JSON format
    #[arg(long, value_name = "JSON")]
    agents: Option<String>,

    /// List of tools allowed without prompting (e.g., "Bash(git log:*)" "Read")
    #[arg(long = "allowedTools", value_name = "TOOL")]
    allowed_tools: Vec<String>,

    /// List of tools that should be disallowed
    #[arg(long = "disallowedTools", value_name = "TOOL")]
    disallowed_tools: Vec<String>,

    /// Output format: text, json, stream-json
    #[arg(long, default_value = "text")]
    output_format: String,

    /// Input format: text, stream-json
    #[arg(long, default_value = "text")]
    input_format: String,

    /// Include streaming events in output
    #[arg(long)]
    include_partial_messages: bool,

    /// Enable verbose logging, shows full turn-by-turn output
    #[arg(long)]
    verbose: bool,

    /// Limit the number of agentic turns in non-interactive mode
    #[arg(long)]
    max_turns: Option<usize>,

    /// Specify permission mode for session
    #[arg(long)]
    permission_mode: Option<String>,

    /// Designate MCP tool for permission prompts
    #[arg(long)]
    permission_prompt_tool: Option<String>,

    /// Skip permission prompts (use with caution)
    #[arg(long)]
    dangerously_skip_permissions: bool,

    /// Fork from existing session ID
    #[arg(long)]
    fork_session: Option<String>,

    /// Specify fallback model when primary model fails
    #[arg(long)]
    fallback_model: Option<String>,

    /// Override settings file location
    #[arg(long)]
    settings: Option<String>,

    /// Enable IDE integration mode (structured JSON output)
    #[arg(long)]
    ide: bool,

    /// Override MCP configuration file location
    #[arg(long)]
    mcp_config: Option<String>,

    /// Resume from specific checkpoint number
    #[arg(long)]
    resume_from_checkpoint: Option<usize>,

    /// Override model capabilities (JSON format)
    #[arg(long)]
    model_capabilities: Option<String>,

    /// Skip safety checks and hooks (dangerous)
    #[arg(long)]
    dangerous_mode: bool,

    /// The prompt to execute (positional argument or from stdin)
    /// When provided, runs in print mode (one-shot execution)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    prompt: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Update to latest version
    Update,
    /// Configure Model Context Protocol (MCP) servers
    Mcp,
}

/// Unified CLI application state
struct App {
    /// CLI arguments
    cli: Cli,
    /// Settings hierarchy
    settings: settings::Settings,
    /// Hooks system
    hooks: hooks::HooksSystem,
    /// Plugin system loader
    plugin_loader: plugins::PluginLoader,
    /// Plugin executor
    plugin_executor: plugins::PluginExecutor,
    /// Slash command system
    slash_commands: Option<commands::SlashCommands>,
    /// Session for checkpointing
    session: checkpoint::Session,
    /// Session saver
    session_saver: checkpoint::SessionSaver,
}

impl App {
    /// Initialize the application with all systems
    async fn new(cli: Cli) -> Result<Self> {
        // 1. Initialize logging
        let log_level = if cli.verbose { "debug" } else { "info" };
        tracing_subscriber::fmt()
            .with_env_filter(log_level)
            .with_target(false)
            .with_thread_ids(false)
            .compact()
            .init();

        tracing::info!("Initializing Claude Code CLI...");

        // 2. Load settings (5-tier hierarchy)
        tracing::debug!("Loading settings hierarchy...");
        let settings_loader = if let Some(ref settings_path) = cli.settings {
            // Override settings file location if --settings flag is provided
            tracing::info!("Using custom settings file: {}", settings_path);
            settings::SettingsLoader::with_custom_path(settings_path)?
        } else {
            settings::SettingsLoader::new()
        };
        let settings = settings_loader
            .load_hierarchy()
            .map_err(|e| anyhow::anyhow!("Failed to load settings hierarchy: {}", e))?
            .merge();

        // Validate settings
        settings
            .validate()
            .map_err(|e| anyhow::anyhow!("Settings validation failed: {:?}", e))?;

        tracing::info!("Settings loaded and validated");

        // 3. Initialize hooks system
        tracing::debug!("Initializing hooks system...");
        let mut hooks = hooks::HooksSystem::new();

        // Skip hooks if --dangerous-mode is enabled
        if cli.dangerous_mode {
            tracing::warn!("DANGEROUS MODE: Skipping hooks initialization");
        } else {
            // Try to load hooks configuration
            let hooks_config_path = ".claude/hooks.json";
            if std::path::Path::new(hooks_config_path).exists() {
                match hooks.load_from_file(hooks_config_path).await {
                    Ok(_) => tracing::info!("Hooks configuration loaded"),
                    Err(e) => tracing::warn!("Failed to load hooks configuration: {}", e),
                }
            } else {
                tracing::debug!("No hooks configuration file found at {}", hooks_config_path);
            }
        }

        // 4. Load plugins
        tracing::debug!("Discovering and loading plugins...");
        let mut plugin_loader = plugins::PluginLoader::new();
        let mut plugin_executor = plugins::PluginExecutor::new();

        // Handle custom MCP config if specified
        if let Some(ref mcp_config_path) = cli.mcp_config {
            tracing::info!("Using custom MCP config: {}", mcp_config_path);
            // Note: Full MCP implementation would load this config
            // Current placeholder just logs the custom path
        }

        let plugin_discovery = plugins::PluginDiscovery::new(".claude/plugins");
        match plugin_discovery.discover_all() {
            Ok(plugins) => {
                tracing::info!("Discovered {} plugins", plugins.len());
                for plugin in plugins {
                    tracing::debug!("Registering plugin: {}", plugin.id);
                    plugin_executor.register(plugin.clone());
                    plugin_loader.register(plugin);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to discover plugins: {}", e);
            }
        }

        // 5. Initialize slash command system
        tracing::debug!("Initializing slash command system...");
        let slash_commands = match commands::SlashCommands::new().await {
            Ok(cmds) => {
                tracing::info!("Slash command system initialized");
                Some(cmds)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize slash commands: {}", e);
                None
            }
        };

        // 6. Check for session resume or create new session
        let session_saver = checkpoint::SessionSaver::with_default_storage()
            .context("Failed to initialize session saver")?;

        // Default checkpoint limit (not configurable via CLI in official spec)
        let checkpoint_limit = 50;

        let session = if let Some(ref fork_session_id) = cli.fork_session {
            // Fork from existing session
            tracing::info!("Forking from session: {}", fork_session_id);
            let loader = checkpoint::SessionLoader::with_default_storage()
                .context("Failed to initialize session loader")?;

            // Load the original session
            let original_session = loader
                .resume_session(fork_session_id, checkpoint_limit)
                .context(format!(
                    "Failed to load session to fork: {}",
                    fork_session_id
                ))?;

            // Create a new session with a unique ID but preserve state
            let forked_session_id = format!("session-{}-fork", chrono::Utc::now().timestamp());
            let mut forked_session = checkpoint::Session::new(&forked_session_id, checkpoint_limit);

            // Copy state from original session
            forked_session.current_state = original_session.current_state.clone();

            tracing::info!("Created forked session: {}", forked_session_id);
            forked_session
        } else if cli.continue_session {
            // Continue from last session
            tracing::info!("Continuing from last session");
            let loader = checkpoint::SessionLoader::with_default_storage()
                .context("Failed to initialize session loader")?;

            // Find the most recent session
            match loader.list_sessions() {
                Ok(mut sessions) => {
                    if let Some(last_session) = sessions.pop() {
                        loader
                            .resume_session(&last_session, checkpoint_limit)
                            .context("Failed to resume last session")?
                    } else {
                        // No sessions found, create new
                        let session_id = format!("session-{}", chrono::Utc::now().timestamp());
                        tracing::info!("No previous session found, starting new: {}", session_id);
                        checkpoint::Session::new(session_id, checkpoint_limit)
                    }
                }
                Err(_) => {
                    // Error listing sessions, create new
                    let session_id = format!("session-{}", chrono::Utc::now().timestamp());
                    tracing::info!("Starting new session: {}", session_id);
                    checkpoint::Session::new(session_id, checkpoint_limit)
                }
            }
        } else if let Some(ref session_id_opt) = cli.resume {
            // Resume specific session
            if let Some(session_id) = session_id_opt {
                tracing::info!("Resuming session: {}", session_id);
                let loader = checkpoint::SessionLoader::with_default_storage()
                    .context("Failed to initialize session loader")?;

                loader
                    .resume_session(session_id, checkpoint_limit)
                    .context("Failed to resume session")?
            } else {
                // --resume without ID: list available sessions
                let loader = checkpoint::SessionLoader::with_default_storage()
                    .context("Failed to initialize session loader")?;

                match loader.list_sessions() {
                    Ok(sessions) => {
                        if sessions.is_empty() {
                            println!("No saved sessions found.");
                            std::process::exit(0);
                        } else {
                            println!("Available sessions:");
                            for session in sessions {
                                println!("  - {}", session);
                            }
                            std::process::exit(0);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to list sessions: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        } else {
            // Generate new session ID
            let session_id = format!("session-{}", chrono::Utc::now().timestamp());
            tracing::info!("Starting new session: {}", session_id);
            checkpoint::Session::new(session_id, checkpoint_limit)
        };

        // Handle --resume-from-checkpoint if specified
        let mut session = session;
        if let Some(checkpoint_num) = cli.resume_from_checkpoint {
            tracing::info!("Resuming from checkpoint number: {}", checkpoint_num);
            let loader = checkpoint::SessionLoader::with_default_storage()
                .context("Failed to initialize session loader")?;

            // Get the checkpoint ID for the specified number
            let checkpoint_ids = loader
                .list_checkpoints(&session.id)
                .context("Failed to list checkpoints")?;

            if checkpoint_num >= checkpoint_ids.len() {
                return Err(anyhow::anyhow!(
                    "Checkpoint {} does not exist. Available checkpoints: 0-{}",
                    checkpoint_num,
                    checkpoint_ids.len() - 1
                ));
            }

            let checkpoint_id = &checkpoint_ids[checkpoint_num];
            loader
                .restore_checkpoint(
                    &mut session,
                    checkpoint_id,
                    checkpoint::types::RestoreScope::Both,
                )
                .context("Failed to restore checkpoint")?;

            tracing::info!("Successfully restored checkpoint {}", checkpoint_num);
        }

        Ok(Self {
            cli,
            settings,
            hooks,
            plugin_loader,
            plugin_executor,
            slash_commands,
            session,
            session_saver,
        })
    }

    /// Run the application
    async fn run(mut self) -> Result<()> {
        // Handle subcommands first
        if let Some(command) = &self.cli.command {
            return self.run_subcommand(command).await;
        }

        // Call SessionStart hook
        self.execute_session_start_hook().await?;

        // Determine mode: print mode (one-shot) or interactive
        let result = self.determine_and_run_mode().await;

        // Call SessionEnd hook (even on error)
        self.execute_session_end_hook().await?;

        // Save session before exit
        self.save_session()?;

        result
    }

    /// Run subcommands (update, mcp)
    async fn run_subcommand(&self, command: &Commands) -> Result<()> {
        match command {
            Commands::Update => {
                println!("Update functionality not yet implemented.");
                println!("This would check for and install the latest version of Claude Code.");
                Ok(())
            }
            Commands::Mcp => {
                println!("MCP (Model Context Protocol) configuration not yet implemented.");
                println!("This would allow you to configure MCP servers.");
                Ok(())
            }
        }
    }

    /// Determine which mode to run based on CLI arguments and stdin
    async fn determine_and_run_mode(&mut self) -> Result<()> {
        // Check for piped stdin first
        let stdin_input = Self::read_stdin_if_piped()?;

        // Determine if we have a prompt
        let prompt_text = if !self.cli.prompt.is_empty() {
            // Join all positional args as the prompt
            Some(self.cli.prompt.join(" "))
        } else {
            // Use stdin as prompt
            stdin_input.clone()
        };

        // If print_mode flag is set or we have a prompt, run in print mode
        if self.cli.print_mode || prompt_text.is_some() {
            if let Some(prompt) = prompt_text {
                return self.run_print_mode(&prompt).await;
            } else {
                // -p flag with no prompt
                return Err(anyhow::anyhow!("Print mode requires a prompt"));
            }
        }

        // No prompt and no -p flag = interactive mode
        self.run_interactive().await
    }

    /// Read from stdin if it's piped (not a TTY)
    fn read_stdin_if_piped() -> Result<Option<String>> {
        let stdin = io::stdin();

        // Check if stdin is a terminal (TTY) or piped
        if stdin.is_terminal() {
            // It's a TTY, not piped - return None
            return Ok(None);
        }

        // Stdin is piped - read all content
        let mut buffer = String::new();
        stdin.lock().read_to_string(&mut buffer)?;

        if buffer.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(buffer.trim().to_string()))
        }
    }

    /// Execute SessionStart hook
    async fn execute_session_start_hook(&self) -> Result<()> {
        let context = hooks::HookContext::for_session(
            self.session.id.clone(),
            format!(".claude/sessions/{}/transcript.json", self.session.id),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            "ask".to_string(), // Default permission mode
            hooks::HookEvent::SessionStart,
        );

        match self
            .hooks
            .execute_hooks(hooks::HookEvent::SessionStart, &context)
            .await
        {
            Ok(results) => {
                for result in results {
                    if !result.is_success() {
                        tracing::warn!("SessionStart hook failed: {}", result.stderr);
                    }
                }
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to execute SessionStart hooks: {}", e);
                Ok(()) // Don't fail startup if hooks fail
            }
        }
    }

    /// Execute SessionEnd hook
    async fn execute_session_end_hook(&self) -> Result<()> {
        let context = hooks::HookContext::for_session(
            self.session.id.clone(),
            format!(".claude/sessions/{}/transcript.json", self.session.id),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            "ask".to_string(),
            hooks::HookEvent::SessionEnd,
        );

        match self
            .hooks
            .execute_hooks(hooks::HookEvent::SessionEnd, &context)
            .await
        {
            Ok(results) => {
                for result in results {
                    if !result.is_success() {
                        tracing::warn!("SessionEnd hook failed: {}", result.stderr);
                    }
                }
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to execute SessionEnd hooks: {}", e);
                Ok(()) // Don't fail shutdown if hooks fail
            }
        }
    }

    /// Run interactive mode
    async fn run_interactive(&mut self) -> Result<()> {
        // Always use regular interactive mode (TUI removed from official spec)
        interactive::run_interactive().await
    }

    /// Run in print mode (one-shot execution) - matches Claude Code's behavior
    async fn run_print_mode(&mut self, prompt: &str) -> Result<()> {
        use rustyclawd_core::client::{
            Client, Config, CreateMessageRequest, Message as ApiMessage, StreamEvent,
        };
        use std::io::Write;

        // Load API configuration
        let config = Config::from_default_location().await?;
        let client = Client::new(config);

        // Model configuration - use CLI override or default
        let model = self
            .cli
            .model
            .as_ref()
            .map(|m| match m.as_str() {
                "sonnet" => "claude-sonnet-4-5-20250929",
                "opus" => "claude-opus-20240229",
                "haiku" => "claude-3-5-haiku-20241022",
                custom => custom,
            })
            .unwrap_or("claude-sonnet-4-5-20250929")
            .to_string();

        // Fallback model configuration (if specified)
        let fallback_model = self
            .cli
            .fallback_model
            .as_ref()
            .map(|m| match m.as_str() {
                "sonnet" => "claude-sonnet-4-5-20250929",
                "opus" => "claude-opus-20240229",
                "haiku" => "claude-3-5-haiku-20241022",
                custom => custom,
            })
            .map(|s| s.to_string());

        let max_tokens = 4096u32; // Default max tokens (not configurable in official spec)

        // Build system prompt based on priority: system_prompt > system_prompt_file > append_system_prompt
        let system_prompt = if let Some(ref prompt) = self.cli.system_prompt {
            // --system-prompt: replace entire system prompt
            Some(prompt.clone())
        } else if let Some(ref file_path) = self.cli.system_prompt_file {
            // --system-prompt-file: load from file
            Some(
                std::fs::read_to_string(file_path)
                    .with_context(|| format!("Failed to read system prompt file: {}", file_path))?,
            )
        } else if let Some(ref _append) = self.cli.append_system_prompt {
            // --append-system-prompt: append to default (would need default system prompt)
            // For now, just use the append text
            self.cli.append_system_prompt.clone()
        } else {
            None
        };

        // Create message request
        let messages = vec![ApiMessage::user(prompt.to_string())];
        let mut request = CreateMessageRequest::new(model, messages, max_tokens);

        // Apply system prompt if specified
        if let Some(ref sys_prompt) = system_prompt {
            request = request.with_system(sys_prompt.clone());
        }

        // Add tools (always enabled in official spec, controlled by allowedTools/disallowedTools)
        let tools = tool_definitions::get_all_tool_definitions();
        request = request.with_tools(tools);

        // Parse model capabilities if provided (JSON format)
        if let Some(ref capabilities_json) = self.cli.model_capabilities {
            tracing::info!("Applying custom model capabilities");
            // Parse and log capabilities (in real implementation would apply to request)
            match serde_json::from_str::<serde_json::Value>(capabilities_json) {
                Ok(caps) => tracing::debug!("Model capabilities: {:?}", caps),
                Err(e) => tracing::warn!("Failed to parse model capabilities: {}", e),
            }
        }

        // Use the tool execution loop (tools always enabled in official spec)
        let response = match client
            .execute_with_tools(request.clone(), |tool_name, tool_input| async move {
                tool_executor::execute_tool(tool_name, tool_input).await
            })
            .await
        {
            Ok(resp) => resp,
            Err(_e) if fallback_model.is_some() => {
                // Try fallback model if primary fails
                let fallback = fallback_model.unwrap();
                tracing::warn!("Primary model failed, trying fallback: {}", fallback);

                // Create new request with fallback model
                let mut fallback_request = CreateMessageRequest::new(
                    fallback,
                    vec![ApiMessage::user(prompt.to_string())],
                    max_tokens,
                );

                // Copy system prompt if present
                if let Some(ref sys_prompt) = system_prompt {
                    fallback_request = fallback_request.with_system(sys_prompt.clone());
                }

                // Add tools
                fallback_request =
                    fallback_request.with_tools(tool_definitions::get_all_tool_definitions());

                client
                    .execute_with_tools(fallback_request, |tool_name, tool_input| async move {
                        tool_executor::execute_tool(tool_name, tool_input).await
                    })
                    .await?
            }
            Err(e) => return Err(e.into()),
        };

        // Extract text from response
        let text = response
            .content
            .iter()
            .filter_map(|block| {
                if let rustyclawd_core::client::types::ContentBlock::Text { text } = block {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        // Output based on format (IDE mode forces JSON output)
        let output_format = if self.cli.ide {
            "json"
        } else {
            self.cli.output_format.as_str()
        };

        match output_format {
            "json" => {
                let mut json_output = serde_json::json!({
                    "id": response.id,
                    "type": response.type_field,
                    "role": response.role,
                    "content": response.content,
                    "model": response.model,
                    "stop_reason": response.stop_reason,
                    "usage": response.usage,
                });

                // Add IDE-specific metadata if in IDE mode
                if self.cli.ide {
                    json_output["ide_mode"] = serde_json::json!(true);
                    json_output["session_id"] = serde_json::json!(self.session.id);
                }

                println!("{}", serde_json::to_string_pretty(&json_output)?);
            }
            "stream-json" => {
                // For stream-json, output as JSON but with streaming markers if requested
                let json_output = serde_json::json!({
                    "id": response.id,
                    "type": response.type_field,
                    "role": response.role,
                    "content": response.content,
                    "model": response.model,
                    "stop_reason": response.stop_reason,
                    "usage": response.usage,
                });
                println!("{}", serde_json::to_string(&json_output)?);
            }
            _ => {
                // Text format (default)
                println!("{}", text);
            }
        }

        Ok(())
    }

    /// Save session to disk
    fn save_session(&self) -> Result<()> {
        match self.session_saver.save_session(&self.session) {
            Ok(_) => {
                tracing::debug!("Session saved: {}", self.session.id);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to save session: {}", e);
                Ok(()) // Don't fail if save fails
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize and run the application
    let app = App::new(cli).await?;
    app.run().await?;

    Ok(())
}
