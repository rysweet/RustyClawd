//! Claude Code CLI - Official Spec-Compliant Implementation
//!
//! This is a Rust implementation that matches Claude Code's exact CLI interface
//! as documented at https://code.claude.com/docs/en/cli-reference

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(deprecated)] // TODO: Migrate from ClientError::Api to specific error types

mod checkpoint;
mod commands;
mod hooks;
mod interactive;
mod mcp_commands;
mod notification;
mod permission_mode;
mod plugins;
mod schema_validator;
mod session;
mod session_persistence;
mod settings;
mod terminal_guard;
mod tool_definitions;
mod tool_executor;
mod tool_formatter;
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
    /// (alias: --prompt)
    #[arg(short = 'p', long = "print", alias = "prompt")]
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
    /// MCP proxy for managing MCP servers
    mcp_proxy: std::sync::Arc<tokio::sync::Mutex<plugins::mcp_proxy::McpProxy>>,
    /// Runtime agents defined via --agents flag
    runtime_agents: std::collections::HashMap<String, plugins::RuntimeAgentDefinition>,
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

        // 4. Load plugins and initialize MCP proxy
        tracing::debug!("Discovering and loading plugins...");
        let mut plugin_loader = plugins::PluginLoader::new();
        let mut plugin_executor = plugins::PluginExecutor::new();
        let mut mcp_proxy = plugins::mcp_proxy::McpProxy::new();

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
                    plugin_loader.register(plugin.clone());

                    // Register MCP servers from plugin manifest
                    for mcp_server in &plugin.manifest.mcp_servers {
                        tracing::info!("Registering MCP server: {}", mcp_server.id);
                        mcp_proxy.register_server(mcp_server.clone());
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to discover plugins: {}", e);
            }
        }

        let mcp_proxy = std::sync::Arc::new(tokio::sync::Mutex::new(mcp_proxy));

        // 4.5 Parse and validate runtime agents from --agents flag
        let runtime_agents = if let Some(ref agents_json) = cli.agents {
            tracing::info!("Parsing runtime agents from --agents flag");
            match plugins::parse_runtime_agents(agents_json) {
                Ok(parsed_agents) => {
                    // Validate the agents
                    if let Err(errors) = plugins::validate_runtime_agents(&parsed_agents) {
                        return Err(anyhow::anyhow!(
                            "Invalid runtime agents: {}",
                            errors.join("; ")
                        ));
                    }

                    tracing::info!("Loaded {} runtime agents", parsed_agents.len());
                    for (id, agent) in &parsed_agents {
                        tracing::debug!(
                            "Runtime agent '{}': description='{}', tools={:?}, model={:?}",
                            id,
                            agent.description,
                            agent.tools,
                            agent.model
                        );
                    }

                    parsed_agents
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to parse --agents JSON: {}", e));
                }
            }
        } else {
            std::collections::HashMap::new()
        };

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
            mcp_proxy,
            runtime_agents,
        })
    }

    /// Run the application
    async fn run(mut self) -> Result<()> {
        // Handle subcommands first
        if let Some(command) = &self.cli.command {
            return self.run_subcommand(command).await;
        }

        // Perform scheduled update check (if applicable)
        self.check_for_updates_on_startup().await;

        // Call SessionStart hook
        self.execute_session_start_hook().await?;

        // Determine mode: print mode (one-shot) or interactive
        let result = self.determine_and_run_mode().await;

        // Execute Stop hook before session end (checks if work is complete)
        self.execute_stop_hook().await?;

        // Call SessionEnd hook (even on error)
        self.execute_session_end_hook().await?;

        // Save session before exit
        self.save_session()?;

        result
    }

    /// Check for updates on startup (background, non-blocking)
    async fn check_for_updates_on_startup(&self) {
        use rustyclawd::update::GitHubClient;
        use rustyclawd::update::UpdateScheduler;
        use rustyclawd::update::Version;

        tracing::debug!("Checking if scheduled update check is needed");

        // Try to create scheduler and check if update check is needed
        match UpdateScheduler::new() {
            Ok(scheduler) => {
                if !scheduler.should_check_on_startup() {
                    tracing::debug!("Update check not needed at this time");
                    return;
                }

                tracing::info!("Performing scheduled background update check");

                // Spawn background task to perform check
                let current_version = Version::current();
                let client = GitHubClient::new("rysweet", "RustyClawd");

                // We'll do a simple non-blocking check here
                tokio::spawn(async move {
                    match client.get_update_info(&current_version).await {
                        Ok(Some(update_info)) => {
                            tracing::info!(
                                "Update available: {} -> {}",
                                current_version,
                                update_info.latest_version
                            );
                            // Note: In a full implementation, we might show a notification
                            // For now, we just log it
                        }
                        Ok(None) => {
                            tracing::debug!("Already at latest version");
                        }
                        Err(e) => {
                            // Don't warn if there are simply no releases available yet
                            // This is expected for repos that haven't published releases
                            use rustyclawd::update::error::UpdateError;
                            if !matches!(e, UpdateError::NoReleasesAvailable) {
                                tracing::warn!("Background update check failed: {}", e);
                            } else {
                                tracing::debug!("No releases available for update check");
                            }
                        }
                    }
                });
            }
            Err(e) => {
                tracing::warn!("Failed to initialize update scheduler: {}", e);
            }
        }
    }

    /// Run subcommands (update, mcp, agent)
    async fn run_subcommand(&self, command: &Commands) -> Result<()> {
        match command {
            Commands::Update {
                check,
                force,
                rollback,
            } => self.handle_update_command(*check, *force, *rollback).await,
            Commands::Mcp { args } => {
                // Handle MCP commands
                match mcp_commands::handle_cli_command(self.mcp_proxy.clone(), args).await {
                    Ok(output) => {
                        println!("{}", output);
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Commands::Agent {
                agent_type,
                prompt,
                model,
            } => {
                self.handle_agent_command(agent_type, prompt, model.as_deref())
                    .await
            }
        }
    }

    /// Handle update command with all subcommands
    async fn handle_update_command(&self, check: bool, force: bool, rollback: bool) -> Result<()> {
        use rustyclawd::update::{
            format_update_message, handle_check_updates, handle_install_update, handle_rollback,
        };

        tracing::info!(
            "Processing update command: check={}, force={}, rollback={}",
            check,
            force,
            rollback
        );

        // Determine which operation to perform
        if rollback {
            // Rollback to previous version
            match handle_rollback().await {
                Ok(result) => {
                    println!("{}", format_update_message(&result));
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Update error: {}", e);
                    Err(e.into())
                }
            }
        } else if check {
            // Check for updates
            match handle_check_updates(force).await {
                Ok(result) => {
                    println!("{}", format_update_message(&result));
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Update error: {}", e);
                    Err(e.into())
                }
            }
        } else {
            // Install update
            match handle_install_update().await {
                Ok(result) => {
                    println!("{}", format_update_message(&result));
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Update error: {}", e);
                    Err(e.into())
                }
            }
        }
    }

    /// Handle agent command - invoke specialized agent with prompt from file
    async fn handle_agent_command(
        &self,
        agent_type: &str,
        prompt_file: &str,
        model: Option<&str>,
    ) -> Result<()> {
        use rustyclawd_tools::{AgentTool, Tool, ToolContext, ToolEvent};

        tracing::info!(
            "Invoking agent: type={}, prompt_file={}, model={:?}",
            agent_type,
            prompt_file,
            model
        );

        // Read prompt from file
        let prompt_content = std::fs::read_to_string(prompt_file)
            .with_context(|| format!("Failed to read prompt file: {}", prompt_file))?;

        // Create tool context
        let ctx = ToolContext {
            cwd: std::env::current_dir().unwrap_or_default(),
            debug: self.cli.verbose,
            metadata: serde_json::Value::Null,
            execution_context: rustyclawd_tools::ExecutionContext::NonInteractive,
            allowed_tools: self.cli.allowed_tools.clone(),
            disallowed_tools: self.cli.disallowed_tools.clone(),
        };

        // Create agent parameters
        let params = rustyclawd_tools::agent::AgentParams {
            description: format!("Agent invocation: {}", agent_type),
            prompt: prompt_content,
            subagent_type: agent_type.to_string(),
            model: model.map(|m| m.to_string()),
            resume: None,
            run_in_background: false,
        };

        // Execute agent tool
        let tool = AgentTool;
        let mut stream = tool
            .execute(params, &ctx)
            .await
            .with_context(|| format!("Failed to execute agent: {}", agent_type))?;

        // Process stream events
        use futures::StreamExt;
        while let Some(event) = stream.next().await {
            match event {
                ToolEvent::Result(output) => {
                    // Execute SubagentStop hook when agent completes
                    let context = hooks::HookContext::for_session(
                        self.session.id.clone(),
                        format!(".claude/sessions/{}/transcript.json", self.session.id),
                        std::env::current_dir()
                            .ok()
                            .and_then(|p| p.to_str().map(|s| s.to_string()))
                            .unwrap_or_default(),
                        "ask".to_string(),
                        hooks::HookEvent::SubagentStop,
                    );

                    match self
                        .hooks
                        .execute_hooks(hooks::HookEvent::SubagentStop, &context)
                        .await
                    {
                        Ok(results) => {
                            for result in results {
                                if let Some(hook_output) = result.parse_output() {
                                    // Check if hook is blocking subagent completion
                                    if let Some(decision) = hook_output.decision {
                                        if decision == hooks::types::StopDecision::Block {
                                            let reason = hook_output.reason.unwrap_or_else(|| {
                                                "Subagent stop blocked by hook".to_string()
                                            });
                                            return Err(anyhow::anyhow!(
                                                "Subagent completion blocked: {}",
                                                reason
                                            ));
                                        }
                                    }
                                }
                                if !result.is_success() {
                                    tracing::warn!("SubagentStop hook failed: {}", result.stderr);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to execute SubagentStop hooks: {}", e);
                            // Non-blocking - continue with output
                        }
                    }

                    // Output the agent response
                    println!("\n=== Agent Response ===\n");
                    println!("{}", output.response);
                    println!("\n=== Metadata ===");
                    println!("Agent ID: {}", output.agent_id);
                    println!("Agent Name: {}", output.agent_name);
                    println!("Model: {}", output.model);
                    println!(
                        "Tokens: {} input, {} output, {} total",
                        output.tokens_used.input_tokens,
                        output.tokens_used.output_tokens,
                        output.tokens_used.total_tokens
                    );
                    return Ok(());
                }
                ToolEvent::Error { message } => {
                    eprintln!("Agent error: {}", message);
                    return Err(anyhow::anyhow!("Agent execution failed: {}", message));
                }
                ToolEvent::Progress { step, percentage } => {
                    if self.cli.verbose {
                        if let Some(pct) = percentage {
                            eprintln!("[{:.0}%] {}", pct, step);
                        } else {
                            eprintln!("{}", step);
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Agent execution completed without result"))
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

    /// Execute Stop hook before session end
    async fn execute_stop_hook(&self) -> Result<()> {
        let context = hooks::HookContext::for_session(
            self.session.id.clone(),
            format!(".claude/sessions/{}/transcript.json", self.session.id),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            "ask".to_string(),
            hooks::HookEvent::Stop,
        );

        match self
            .hooks
            .execute_hooks(hooks::HookEvent::Stop, &context)
            .await
        {
            Ok(results) => {
                for result in results {
                    if let Some(output) = result.parse_output() {
                        // Check if hook is blocking session end
                        if let Some(decision) = output.decision {
                            if decision == hooks::types::StopDecision::Block {
                                let reason = output
                                    .reason
                                    .unwrap_or_else(|| "Session end blocked by hook".to_string());
                                tracing::warn!("Stop hook blocked session end: {}", reason);
                                // For non-interactive mode, we still exit but log the warning
                            }
                        }
                    }
                    if !result.is_success() {
                        tracing::warn!("Stop hook failed: {}", result.stderr);
                    }
                }
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to execute Stop hooks: {}", e);
                Ok(()) // Non-blocking - continue with shutdown
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
        // Pass hooks and tool restrictions to interactive session
        let hooks = std::sync::Arc::new(self.hooks.clone());
        let allowed_tools = self.cli.allowed_tools.clone();
        let disallowed_tools = self.cli.disallowed_tools.clone();
        interactive::run_interactive_with_config(Some(hooks), allowed_tools, disallowed_tools).await
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

        // Execute UserPromptSubmit hook BEFORE processing prompt
        let context = hooks::HookContext::for_user_prompt(
            self.session.id.clone(),
            format!(".claude/sessions/{}/transcript.json", self.session.id),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            "ask".to_string(),
            prompt.to_string(),
        );

        match self
            .hooks
            .execute_hooks(hooks::HookEvent::UserPromptSubmit, &context)
            .await
        {
            Ok(results) => {
                for result in results {
                    if result.is_blocking() {
                        return Err(anyhow::anyhow!("Prompt blocked by hook: {}", result.stderr));
                    }
                    if !result.is_success() {
                        eprintln!(
                            "⚠️  Warning: UserPromptSubmit hook failed: {}",
                            result.stderr
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "⚠️  Warning: Failed to execute UserPromptSubmit hooks: {}",
                    e
                );
                // Non-blocking - continue even if hook fails
            }
        }

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
        // Pass hooks, session ID, and permission mode to tool executor
        let hooks_for_tools = std::sync::Arc::new(self.hooks.clone());
        let session_id_for_tools = self.session.id.clone();

        // Parse permission mode from CLI
        let permission_mode = self
            .cli
            .permission_mode
            .as_ref()
            .map(|s| match s.as_str() {
                "plan" => permission_mode::PermissionMode::Plan,
                "auto-accept" | "auto" => permission_mode::PermissionMode::AutoAccept,
                _ => permission_mode::PermissionMode::Ask,
            })
            .unwrap_or(permission_mode::PermissionMode::Ask);

        // Clone allowed/disallowed tools from CLI for tool executor
        let allowed_tools_for_executor = self.cli.allowed_tools.clone();
        let disallowed_tools_for_executor = self.cli.disallowed_tools.clone();

        let response = match client
            .execute_with_tools(request.clone(), |tool_name, tool_input| {
                let hooks = hooks_for_tools.clone();
                let session_id = session_id_for_tools.clone();
                let allowed_tools = allowed_tools_for_executor.clone();
                let disallowed_tools = disallowed_tools_for_executor.clone();
                async move {
                    tool_executor::execute_tool_with_permission(
                        tool_name,
                        tool_input,
                        permission_mode,
                        Some(hooks),
                        Some(session_id),
                        None, // No notification manager in non-interactive mode
                        None, // No tool_use_id in non-interactive mode
                        allowed_tools,
                        disallowed_tools,
                    )
                    .await
                }
            })
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                if let Some(fallback) = fallback_model {
                    // Try fallback model if primary fails
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

                    let hooks_fallback = hooks_for_tools.clone();
                    let session_id_fallback = session_id_for_tools.clone();
                    let allowed_tools_fallback = allowed_tools_for_executor.clone();
                    let disallowed_tools_fallback = disallowed_tools_for_executor.clone();

                    client
                        .execute_with_tools(fallback_request, |tool_name, tool_input| {
                            let hooks = hooks_fallback.clone();
                            let session_id = session_id_fallback.clone();
                            let allowed_tools = allowed_tools_fallback.clone();
                            let disallowed_tools = disallowed_tools_fallback.clone();
                            async move {
                                tool_executor::execute_tool_with_permission(
                                    tool_name,
                                    tool_input,
                                    permission_mode,
                                    Some(hooks),
                                    Some(session_id),
                                    None, // No notification manager in non-interactive mode
                                    None, // No tool_use_id in non-interactive mode
                                    allowed_tools,
                                    disallowed_tools,
                                )
                                .await
                            }
                        })
                        .await?
                } else {
                    return Err(e.into());
                }
            }
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
    // CRITICAL: Set COLORTERM for Windows Terminal + WSL RGB color support
    // Windows Terminal doesn't set COLORTERM automatically in WSL, causing RGB
    // colors to be rendered incorrectly. This is a known issue with crossterm
    // and ratatui on Windows Terminal + WSL.
    // See: https://github.com/microsoft/terminal/issues/11057
    if std::env::var("COLORTERM").is_err() {
        std::env::set_var("COLORTERM", "truecolor");
    }

    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize and run the application
    let app = App::new(cli).await?;
    app.run().await?;

    Ok(())
}
