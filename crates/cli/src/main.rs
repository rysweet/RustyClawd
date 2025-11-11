//! Claude Code CLI - Unified System Integration
//!
//! Orchestrates all systems: settings, hooks, checkpoints, plugins, and commands.
//! Provides a complete CLI experience with proper lifecycle management.

mod checkpoint;
mod commands;
mod hooks;
mod interactive;
mod plugins;
mod settings;

use anyhow::{Context as AnyhowContext, Result};
use clap::{Parser, Subcommand};
use claude_code_tools::{
    BashTool, EditTool, GlobTool, GrepTool, ReadTool, Tool, ToolContext, ToolEvent, WriteTool,
};
use futures::StreamExt;

/// Claude Code - Rust Translation (Educational)
#[derive(Parser)]
#[command(name = "claude-code")]
#[command(author = "Educational Project")]
#[command(version = "0.1.0")]
#[command(about = "Rust translation of Claude Code for learning purposes", long_about = None)]
struct Cli {
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Resume a previous session
    #[arg(long)]
    resume: Option<String>,

    /// Checkpoint limit for session history
    #[arg(long, default_value = "50")]
    checkpoint_limit: usize,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// Start interactive chat session with Claude
    Chat,

    /// Execute a bash command
    Bash {
        /// The command to execute
        command: String,

        /// Timeout in milliseconds
        #[arg(short, long, default_value = "120000")]
        timeout: u64,

        /// Description of what the command does
        #[arg(short = 'D', long)]
        description: Option<String>,
    },

    /// Read a file
    Read {
        /// Path to the file to read
        file_path: String,

        /// Line offset to start reading from
        #[arg(long)]
        offset: Option<usize>,

        /// Number of lines to read
        #[arg(long)]
        limit: Option<usize>,
    },

    /// Write content to a file
    Write {
        /// Path to write to
        file_path: String,

        /// Content to write
        #[arg(long)]
        content: String,
    },

    /// Edit a file by replacing text
    Edit {
        /// Path to the file to edit
        file_path: String,

        /// Text to replace
        #[arg(long)]
        old_string: String,

        /// Replacement text
        #[arg(long)]
        new_string: String,

        /// Replace all occurrences
        #[arg(long)]
        replace_all: bool,
    },

    /// Find files by glob pattern
    Glob {
        /// Glob pattern (e.g., "**/*.rs")
        pattern: String,

        /// Directory to search in
        #[arg(long)]
        path: Option<String>,
    },

    /// Search for text patterns using ripgrep
    Grep {
        /// Regex pattern to search for
        pattern: String,

        /// Path to search in
        #[arg(long)]
        path: Option<String>,

        /// Case insensitive
        #[arg(short = 'i')]
        case_insensitive: bool,

        /// Glob pattern to filter files
        #[arg(long)]
        glob: Option<String>,

        /// Lines before match
        #[arg(short = 'B')]
        before: Option<usize>,

        /// Lines after match
        #[arg(short = 'A')]
        after: Option<usize>,

        /// Limit results
        #[arg(long)]
        head_limit: Option<usize>,
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
}

impl App {
    /// Initialize the application with all systems
    async fn new(cli: Cli) -> Result<Self> {
        // 1. Initialize logging
        let log_level = if cli.debug { "debug" } else { "info" };
        tracing_subscriber::fmt()
            .with_env_filter(log_level)
            .with_target(false)
            .with_thread_ids(false)
            .compact()
            .init();

        tracing::info!("Initializing Claude Code CLI...");

        // 2. Load settings (5-tier hierarchy)
        tracing::debug!("Loading settings hierarchy...");
        let settings_loader = settings::SettingsLoader::new();
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

        // 4. Load plugins
        tracing::debug!("Discovering and loading plugins...");
        let mut plugin_loader = plugins::PluginLoader::new();
        let mut plugin_executor = plugins::PluginExecutor::new();

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
        let session_saver = checkpoint::SessionSaver::default()
            .context("Failed to initialize session saver")?;

        let session = if let Some(session_id) = &cli.resume {
            tracing::info!("Resuming session: {}", session_id);
            let loader = checkpoint::SessionLoader::default()
                .context("Failed to initialize session loader")?;

            loader
                .resume_session(session_id, cli.checkpoint_limit)
                .context("Failed to resume session")?
        } else {
            // Generate new session ID
            let session_id = format!("session-{}", chrono::Utc::now().timestamp());
            tracing::info!("Starting new session: {}", session_id);
            checkpoint::Session::new(session_id, cli.checkpoint_limit)
        };

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
        // Call SessionStart hook
        self.execute_session_start_hook().await?;

        // Dispatch to appropriate mode
        let is_chat = matches!(self.cli.command, None | Some(Commands::Chat));

        let result = if is_chat {
            // Interactive mode
            self.run_interactive().await
        } else {
            // Tool execution mode - clone the command first to avoid borrow issues
            let cmd = self.cli.command.clone().unwrap();
            self.run_tool_command(&cmd).await
        };

        // Call SessionEnd hook (even on error)
        self.execute_session_end_hook().await?;

        // Save session before exit
        self.save_session()?;

        result
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

        match self.hooks.execute_hooks(hooks::HookEvent::SessionStart, &context).await {
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

        match self.hooks.execute_hooks(hooks::HookEvent::SessionEnd, &context).await {
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
        interactive::run_interactive().await
    }

    /// Run a tool command with hooks and checkpoints
    async fn run_tool_command(&mut self, command: &Commands) -> Result<()> {
        // Create tool context
        let ctx = ToolContext {
            debug: self.cli.debug,
            ..Default::default()
        };

        // Execute pre-tool hook
        let tool_name = match command {
            Commands::Bash { .. } => "Bash",
            Commands::Read { .. } => "Read",
            Commands::Write { .. } => "Write",
            Commands::Edit { .. } => "Edit",
            Commands::Glob { .. } => "Glob",
            Commands::Grep { .. } => "Grep",
            Commands::Chat => unreachable!(),
        };

        self.execute_pre_tool_hook(tool_name).await?;

        // Create checkpoint before tool execution
        let checkpoint_id = self.session.create_checkpoint(Some(format!(
            "Before {} execution",
            tool_name
        )));
        tracing::debug!("Created checkpoint: {}", checkpoint_id);

        // Execute the tool
        let result = match command {
            Commands::Bash {
                command,
                timeout,
                description,
            } => {
                use claude_code_tools::bash::*;
                self.execute_tool(
                    BashTool,
                    BashParams {
                        command: command.clone(),
                        timeout: *timeout,
                        description: description.clone(),
                        run_in_background: false,
                    },
                    &ctx,
                )
                .await
            }

            Commands::Read {
                file_path,
                offset,
                limit,
            } => {
                use claude_code_tools::read::*;
                self.execute_tool(
                    ReadTool,
                    ReadParams {
                        file_path: file_path.clone(),
                        offset: *offset,
                        limit: *limit,
                    },
                    &ctx,
                )
                .await
            }

            Commands::Write {
                file_path,
                content,
            } => {
                use claude_code_tools::write::*;
                self.execute_tool(
                    WriteTool,
                    WriteParams {
                        file_path: file_path.clone(),
                        content: content.clone(),
                    },
                    &ctx,
                )
                .await
            }

            Commands::Edit {
                file_path,
                old_string,
                new_string,
                replace_all,
            } => {
                use claude_code_tools::edit::*;
                self.execute_tool(
                    EditTool,
                    EditParams {
                        file_path: file_path.clone(),
                        old_string: old_string.clone(),
                        new_string: new_string.clone(),
                        replace_all: *replace_all,
                    },
                    &ctx,
                )
                .await
            }

            Commands::Glob { pattern, path } => {
                use claude_code_tools::glob_tool::*;
                self.execute_tool(
                    GlobTool,
                    GlobParams {
                        pattern: pattern.clone(),
                        path: path.clone(),
                    },
                    &ctx,
                )
                .await
            }

            Commands::Grep {
                pattern,
                path,
                case_insensitive,
                glob,
                before,
                after,
                head_limit,
            } => {
                use claude_code_tools::grep::*;
                self.execute_tool(
                    GrepTool,
                    GrepParams {
                        pattern: pattern.clone(),
                        path: path.clone(),
                        output_mode: OutputMode::Content,
                        case_insensitive: *case_insensitive,
                        glob: glob.clone(),
                        before_context: *before,
                        after_context: *after,
                        head_limit: *head_limit,
                    },
                    &ctx,
                )
                .await
            }

            Commands::Chat => unreachable!(),
        };

        // Execute post-tool hook
        self.execute_post_tool_hook(tool_name).await?;

        // Save session after tool execution
        self.save_session()?;

        result
    }

    /// Execute PreToolUse hook
    async fn execute_pre_tool_hook(&self, tool_name: &str) -> Result<()> {
        let context = hooks::HookContext::for_tool(
            self.session.id.clone(),
            format!(".claude/sessions/{}/transcript.json", self.session.id),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            "ask".to_string(),
            hooks::HookEvent::PreToolUse,
            tool_name.to_string(),
        );

        match self.hooks.execute_hooks(hooks::HookEvent::PreToolUse, &context).await {
            Ok(results) => {
                for result in results {
                    if result.is_blocking() {
                        anyhow::bail!("PreToolUse hook blocked execution: {}", result.stderr);
                    }
                    if !result.is_success() {
                        tracing::warn!("PreToolUse hook warning: {}", result.stderr);
                    }
                }
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to execute PreToolUse hooks: {}", e);
                Ok(()) // Don't fail if hooks fail
            }
        }
    }

    /// Execute PostToolUse hook
    async fn execute_post_tool_hook(&self, tool_name: &str) -> Result<()> {
        let context = hooks::HookContext::for_tool(
            self.session.id.clone(),
            format!(".claude/sessions/{}/transcript.json", self.session.id),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            "ask".to_string(),
            hooks::HookEvent::PostToolUse,
            tool_name.to_string(),
        );

        match self.hooks.execute_hooks(hooks::HookEvent::PostToolUse, &context).await {
            Ok(results) => {
                for result in results {
                    if !result.is_success() {
                        tracing::warn!("PostToolUse hook failed: {}", result.stderr);
                    }
                }
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to execute PostToolUse hooks: {}", e);
                Ok(()) // Don't fail if hooks fail
            }
        }
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

    /// Generic tool execution with streaming
    async fn execute_tool<T>(&self, tool: T, params: T::Params, ctx: &ToolContext) -> Result<()>
    where
        T: Tool,
    {
        let mut stream = tool.execute(params, ctx).await?;

        while let Some(event) = stream.next().await {
            match event {
                ToolEvent::Progress { step, percentage } => {
                    if let Some(pct) = percentage {
                        println!("Progress: {} ({}%)", step, pct);
                    } else {
                        println!("Progress: {}", step);
                    }
                }

                ToolEvent::Result(output) => {
                    // Serialize result as JSON for consistent output
                    let json = serde_json::to_string_pretty(&output)?;
                    println!("\n{}", json);
                }

                ToolEvent::Error { message } => {
                    eprintln!("Error: {}", message);
                    std::process::exit(1);
                }
            }
        }

        Ok(())
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
