//! Claude Code CLI - Official Spec-Compliant Implementation
//!
//! This is a Rust implementation that matches Claude Code's exact CLI interface
//! as documented at https://code.claude.com/docs/en/cli-reference

// Each module below forms part of the CLI's internal library. Items within are
// public APIs consumed by sibling modules, not directly by main(). Targeted
// allow(dead_code) suppresses false positives from the binary-crate lint scope.
#[allow(dead_code)]
mod checkpoint;
mod cli_args;
#[allow(dead_code)]
mod command_handlers;
#[allow(dead_code)]
mod commands;
#[allow(dead_code)]
mod conversation;
#[allow(dead_code)]
mod hooks;
#[allow(dead_code)]
mod interactive;
mod mcp_commands;
#[allow(dead_code)]
mod mcp_dispatch;
#[allow(dead_code)]
mod mcp_serve;
mod notification;
#[allow(dead_code)]
mod permission_mode;
#[allow(dead_code)]
mod plugins;
#[allow(dead_code)]
mod schema_validator;
#[allow(dead_code)]
mod session;
#[allow(dead_code)]
mod session_graph;
#[allow(dead_code)]
mod session_graph_storage;
#[allow(dead_code)]
mod session_index;
#[allow(dead_code)]
mod session_persistence;
#[allow(dead_code)]
mod settings;
#[allow(dead_code)]
mod streaming;
#[allow(dead_code)]
mod terminal_guard;
mod tool_definitions;
#[allow(dead_code)]
mod tool_orchestrator;
// TODO: Migrate from ClientError::Api to specific error types (BadRequest, Unknown, etc.)
#[allow(deprecated)]
#[allow(dead_code)]
mod tool_executor;
#[allow(dead_code)]
mod tool_formatter;
#[allow(deprecated)]
#[allow(dead_code)]
mod tool_schema_errors;
#[allow(dead_code)]
mod tui;

// Split impl-block modules for App
mod app_runtime;
mod print_mode;

use anyhow::{Context as AnyhowContext, Result};
use clap::Parser;

use cli_args::Cli;

/// Unified CLI application state
struct App {
    /// CLI arguments
    pub(crate) cli: Cli,
    /// Settings hierarchy (retained for runtime access by subcommands and plugins)
    #[allow(dead_code)]
    pub(crate) settings: settings::Settings,
    /// Hooks system
    pub(crate) hooks: hooks::HooksSystem,
    /// Plugin system loader (retained for runtime plugin management)
    #[allow(dead_code)]
    pub(crate) plugin_loader: plugins::PluginLoader,
    /// Plugin executor (retained for runtime plugin execution)
    #[allow(dead_code)]
    pub(crate) plugin_executor: plugins::PluginExecutor,
    /// Slash command system (retained for interactive slash command dispatch)
    #[allow(dead_code)]
    pub(crate) slash_commands: Option<commands::SlashCommands>,
    /// Session for checkpointing
    pub(crate) session: checkpoint::Session,
    /// Session saver
    pub(crate) session_saver: checkpoint::SessionSaver,
    /// MCP proxy for managing MCP servers
    pub(crate) mcp_proxy: std::sync::Arc<tokio::sync::Mutex<plugins::mcp_proxy::McpProxy>>,
    /// Runtime agents defined via --agents flag (retained for agent dispatch)
    #[allow(dead_code)]
    pub(crate) runtime_agents: std::collections::HashMap<String, plugins::RuntimeAgentDefinition>,
}

/// Result of plugin discovery and loading
struct PluginState {
    loader: plugins::PluginLoader,
    executor: plugins::PluginExecutor,
    mcp_proxy: std::sync::Arc<tokio::sync::Mutex<plugins::mcp_proxy::McpProxy>>,
    runtime_agents: std::collections::HashMap<String, plugins::RuntimeAgentDefinition>,
}

impl App {
    /// Initialize the application with all systems.
    ///
    /// Delegates to focused helpers: init_logging, load_settings, init_hooks,
    /// load_plugins, and resolve_session.
    async fn new(cli: Cli) -> Result<Self> {
        Self::init_logging(&cli);
        let settings = Self::load_settings(&cli)?;
        let hooks = Self::init_hooks(&cli).await?;
        let plugin_state = Self::load_plugins(&cli)?;

        // Initialize slash command system
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

        let (session, session_saver) = Self::resolve_session(&cli)?;

        Ok(Self {
            cli,
            settings,
            hooks,
            plugin_loader: plugin_state.loader,
            plugin_executor: plugin_state.executor,
            slash_commands,
            session,
            session_saver,
            mcp_proxy: plugin_state.mcp_proxy,
            runtime_agents: plugin_state.runtime_agents,
        })
    }

    /// Configure the tracing subscriber based on CLI verbosity.
    fn init_logging(cli: &Cli) {
        let log_level = if cli.verbose { "debug" } else { "info" };
        tracing_subscriber::fmt()
            .with_env_filter(log_level)
            .with_target(false)
            .with_thread_ids(false)
            .compact()
            .init();

        tracing::info!("Initializing RustyClawd CLI...");
    }

    /// Load and validate the 5-tier settings hierarchy, applying CLI overrides.
    fn load_settings(cli: &Cli) -> Result<settings::Settings> {
        tracing::debug!("Loading settings hierarchy...");
        let settings_loader = if let Some(ref settings_path) = cli.settings {
            tracing::info!("Using custom settings file: {}", settings_path);
            settings::SettingsLoader::with_custom_path(settings_path)?
        } else {
            settings::SettingsLoader::new()
        };
        let settings = settings_loader
            .load_hierarchy()
            .map_err(|e| anyhow::anyhow!("Failed to load settings hierarchy: {}", e))?
            .merge();

        settings
            .validate()
            .map_err(|e| anyhow::anyhow!("Settings validation failed: {:?}", e))?;

        tracing::info!("Settings loaded and validated");
        Ok(settings)
    }

    /// Initialize the hooks system, loading configuration unless dangerous mode is active.
    async fn init_hooks(cli: &Cli) -> Result<hooks::HooksSystem> {
        tracing::debug!("Initializing hooks system...");
        let mut hooks = hooks::HooksSystem::new();

        if cli.dangerous_mode {
            tracing::warn!("DANGEROUS MODE: Skipping hooks initialization");
        } else {
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

        Ok(hooks)
    }

    /// Discover plugins, initialize MCP proxy, and parse runtime agents from CLI.
    fn load_plugins(cli: &Cli) -> Result<PluginState> {
        tracing::debug!("Discovering and loading plugins...");
        let mut plugin_loader = plugins::PluginLoader::new();
        let mut plugin_executor = plugins::PluginExecutor::new();
        let mut mcp_proxy = plugins::mcp_proxy::McpProxy::new();

        if let Some(ref mcp_config_path) = cli.mcp_config {
            tracing::info!("Using custom MCP config: {}", mcp_config_path);
        }

        let plugin_discovery = plugins::PluginDiscovery::new(".claude/plugins");
        match plugin_discovery.discover_all() {
            Ok(plugins) => {
                tracing::info!("Discovered {} plugins", plugins.len());
                for plugin in plugins {
                    tracing::debug!("Registering plugin: {}", plugin.id);
                    plugin_executor.register(plugin.clone());
                    plugin_loader.register(plugin.clone());

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

        // Parse and validate runtime agents from --agents flag
        let runtime_agents = if let Some(ref agents_json) = cli.agents {
            tracing::info!("Parsing runtime agents from --agents flag");
            match plugins::parse_runtime_agents(agents_json) {
                Ok(parsed_agents) => {
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

        Ok(PluginState {
            loader: plugin_loader,
            executor: plugin_executor,
            mcp_proxy,
            runtime_agents,
        })
    }

    /// Resolve the session: fork, continue, resume, or create new.
    /// Also handles --resume-from-checkpoint restoration.
    fn resolve_session(cli: &Cli) -> Result<(checkpoint::Session, checkpoint::SessionSaver)> {
        let session_saver = checkpoint::SessionSaver::with_default_storage()
            .context("Failed to initialize session saver")?;

        let checkpoint_limit = 50;

        let session = if let Some(ref fork_session_id) = cli.fork_session {
            tracing::info!("Forking from session: {}", fork_session_id);
            let loader = checkpoint::SessionLoader::with_default_storage()
                .context("Failed to initialize session loader")?;

            let original_session = loader
                .resume_session(fork_session_id, checkpoint_limit)
                .context(format!(
                    "Failed to load session to fork: {}",
                    fork_session_id
                ))?;

            let forked_session_id = format!("session-{}-fork", chrono::Utc::now().timestamp());
            let mut forked_session = checkpoint::Session::new(&forked_session_id, checkpoint_limit);
            forked_session.current_state = original_session.current_state.clone();

            tracing::info!("Created forked session: {}", forked_session_id);
            forked_session
        } else if cli.continue_session {
            tracing::info!("Continuing from last session");
            let loader = checkpoint::SessionLoader::with_default_storage()
                .context("Failed to initialize session loader")?;

            match loader.list_sessions() {
                Ok(mut sessions) => {
                    if let Some(last_session) = sessions.pop() {
                        loader
                            .resume_session(&last_session, checkpoint_limit)
                            .context("Failed to resume last session")?
                    } else {
                        let session_id = format!("session-{}", chrono::Utc::now().timestamp());
                        tracing::info!("No previous session found, starting new: {}", session_id);
                        checkpoint::Session::new(session_id, checkpoint_limit)
                    }
                }
                Err(_) => {
                    let session_id = format!("session-{}", chrono::Utc::now().timestamp());
                    tracing::info!("Starting new session: {}", session_id);
                    checkpoint::Session::new(session_id, checkpoint_limit)
                }
            }
        } else if let Some(ref session_id_opt) = cli.resume {
            if let Some(session_id) = session_id_opt {
                tracing::info!("Resuming session: {}", session_id);
                let loader = checkpoint::SessionLoader::with_default_storage()
                    .context("Failed to initialize session loader")?;

                loader
                    .resume_session(session_id, checkpoint_limit)
                    .context("Failed to resume session")?
            } else {
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
        } else if let Some(pr_number) = cli.from_pr {
            tracing::info!("Looking up session for PR #{}", pr_number);
            let index =
                session_index::SessionIndex::new().context("Failed to load session index")?;

            if let Some(session_id) = index.get_latest_session_for_pr(pr_number) {
                tracing::info!("Found session {} for PR #{}", session_id, pr_number);
                let loader = checkpoint::SessionLoader::with_default_storage()
                    .context("Failed to initialize session loader")?;

                loader
                    .resume_session(session_id, checkpoint_limit)
                    .with_context(|| {
                        format!(
                            "Failed to resume session {} for PR #{}",
                            session_id, pr_number
                        )
                    })?
            } else {
                return Err(anyhow::anyhow!(
                    "No session found linked to PR #{}. Use `--resume` to resume by session ID.",
                    pr_number
                ));
            }
        } else {
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

        Ok((session, session_saver))
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
