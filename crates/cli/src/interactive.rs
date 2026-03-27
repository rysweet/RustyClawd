//! Interactive chat mode (REPL) for RustyClawd
//!
//! This module is the coordinator for the interactive session. It owns the
//! `InteractiveSession` struct and the main event loop, delegating to:
//!
//! - [`crate::streaming`] -- streaming message handling and SSE background tasks
//! - [`crate::conversation`] -- message processing, command dispatch, tool loop
//! - [`crate::tool_orchestrator`] -- tool execution spawning and result collection

use crate::commands::SlashCommands;
use crate::conversation::{self, SessionServices, StreamingState, ToolLoopState};
use crate::hooks;
use crate::hooks::NotificationType;
use crate::notification::NotificationManager;
use crate::plugins::mcp_proxy::McpProxy;
use crate::session::SessionStats;
use crate::session_persistence::{SessionInfo, SessionPersistence};
use crate::streaming;
use crate::terminal_guard;
use crate::tool_orchestrator;
use crate::tui::{ChatMessage, TuiState};
use anyhow::Result;
use rustyclawd_core::client::{Backend, Client, Config, MessageResponse};
use rustyclawd_core::{Context, MessageRole};
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Default model for interactive sessions (Anthropic backend)
pub(crate) const DEFAULT_MODEL: &str = "claude-opus-4-6";

/// Default model for Copilot backend sessions
pub(crate) const DEFAULT_COPILOT_MODEL: &str = "claude-sonnet-4.6";

/// Maximum tokens for responses
pub(crate) const MAX_TOKENS: u32 = 4096;

/// Type alias for the slash-command completion callback.
type CompletionCallback = Box<dyn Fn(&str) -> Vec<(String, Option<String>)> + Send>;

/// Interactive chat session with TUI
pub struct InteractiveSession {
    /// Anthropic API client (shared via Arc for SessionServices)
    client: Arc<Client>,
    /// Conversation context
    context: Context,
    /// TUI state
    tui: TuiState,
    /// Model to use
    model: String,
    /// Slash command system
    slash_commands: Arc<SlashCommands>,
    /// Session persistence manager
    persistence: Option<SessionPersistence>,
    /// MCP proxy for managing MCP servers
    mcp_proxy: Arc<Mutex<McpProxy>>,
    /// Session statistics tracking
    stats: SessionStats,
    /// Hooks system (optional)
    hooks: Option<Arc<hooks::HooksSystem>>,
    /// Session ID for hooks
    session_id: String,
    /// Notification manager (optional)
    notification_manager: Option<NotificationManager>,
    /// Streaming-related state (channel receivers, message index)
    streaming: StreamingState,
    /// Tool-use loop state (active tools, results, pending response)
    tool_state: ToolLoopState,
    /// Pending response for tool use loop processing
    pending_response: Option<MessageResponse>,
    /// List of tools that are explicitly allowed (empty means all tools allowed)
    allowed_tools: Vec<String>,
    /// List of tools that are explicitly disallowed
    disallowed_tools: Vec<String>,
}

impl InteractiveSession {
    /// Create a new interactive session
    pub async fn new() -> Result<Self> {
        Self::with_hooks(None).await
    }

    /// Create a new interactive session with optional hooks system
    pub async fn with_hooks(hooks: Option<Arc<hooks::HooksSystem>>) -> Result<Self> {
        Self::with_hooks_and_backend(hooks, Backend::Anthropic, None).await
    }

    /// Create a new interactive session with optional hooks system, backend, and model.
    pub async fn with_hooks_and_backend(
        hooks: Option<Arc<hooks::HooksSystem>>,
        backend: Backend,
        model_override: Option<String>,
    ) -> Result<Self> {
        // Set execution context to TUI mode for process isolation
        terminal_guard::set_execution_context(terminal_guard::ExecutionContext::Tui);

        // Load API configuration based on backend.
        // When no --provider was specified, fall back to Copilot if no Anthropic key.
        let (client, backend) = match backend {
            Backend::Copilot => {
                (
                    Arc::new(Client::new_copilot().await.map_err(|e| {
                        anyhow::anyhow!("Failed to initialize Copilot backend: {}", e)
                    })?),
                    Backend::Copilot,
                )
            }
            Backend::AzureFoundry => {
                return Err(anyhow::anyhow!(
                    "Azure AI Foundry backend is not yet supported in interactive mode. \
                     Use it via the skwaq CLI with [llm] reasoning = \"azure\" in skwaq.toml."
                ));
            }
            Backend::Anthropic => match Config::from_default_location().await {
                Ok(config) => (Arc::new(Client::new(config)?), Backend::Anthropic),
                Err(rustyclawd_core::client::ClientError::ApiKeyNotFound) => {
                    // No Anthropic key: try Copilot as fallback
                    match Client::new_copilot().await {
                        Ok(c) => {
                            eprintln!(
                                "No Anthropic API key found. \
                                 Using GitHub Copilot backend (detected via gh auth)."
                            );
                            (Arc::new(c), Backend::Copilot)
                        }
                        Err(_) => {
                            return Err(rustyclawd_core::client::ClientError::ApiKeyNotFound.into());
                        }
                    }
                }
                Err(e) => return Err(e.into()),
            },
        };

        // Initialize TUI
        let mut tui = TuiState::new()?;

        // Initialize slash command system
        let slash_commands = Arc::new(SlashCommands::new().await?);

        // Wire up autocomplete callback
        let commands_for_completion = Arc::clone(&slash_commands);
        tui.set_completion_callback(build_completion_callback(commands_for_completion));

        // Initialize session persistence
        let persistence = SessionPersistence::with_default_id().ok();

        // Initialize MCP proxy
        let mcp_proxy = Arc::new(Mutex::new(McpProxy::new()));

        // Generate session ID
        let session_id = format!("session-{}", chrono::Utc::now().timestamp());

        // Initialize notification manager if hooks are available
        let notification_manager = hooks
            .as_ref()
            .map(|hooks_sys| NotificationManager::new(Arc::clone(hooks_sys)));

        // Fire AuthSuccess notification AFTER Client::new()
        if let Some(ref notification_mgr) = notification_manager {
            notification_mgr
                .notify(
                    &session_id,
                    NotificationType::AuthSuccess,
                    "API authentication successful",
                )
                .await;
        }

        // Pick model: CLI override > backend default
        let model = model_override.unwrap_or_else(|| match backend {
            Backend::Copilot => DEFAULT_COPILOT_MODEL.to_string(),
            Backend::AzureFoundry => DEFAULT_COPILOT_MODEL.to_string(),
            Backend::Anthropic => DEFAULT_MODEL.to_string(),
        });

        Ok(Self {
            client,
            context: Context::new(),
            tui,
            stats: SessionStats::new(&model),
            model,
            slash_commands,
            persistence,
            mcp_proxy,
            hooks,
            session_id,
            notification_manager,
            streaming: StreamingState {
                rx: None,
                message_index: None,
                response_rx: None,
                api_messages: Vec::new(),
            },
            tool_state: ToolLoopState {
                active_tools: std::collections::HashMap::new(),
                tool_results: std::collections::HashMap::new(),
                expected_tool_ids: Vec::new(),
                pending_tool_response: None,
                tool_rx: None,
            },
            pending_response: None,
            allowed_tools: vec![],
            disallowed_tools: vec![],
        })
    }

    /// Build a `SessionServices` snapshot from current session state.
    ///
    /// Uses `Arc`/`Clone` so the result is independent of `&self`, allowing
    /// sibling fields to be mutably borrowed at the same time.
    fn session_services(&self) -> SessionServices {
        SessionServices {
            client: Arc::clone(&self.client),
            model: self.model.clone(),
            hooks: self.hooks.clone(),
            session_id: self.session_id.clone(),
            notification_manager: self.notification_manager.clone(),
            permission_mode: self.tui.permission_mode(),
            allowed_tools: self.allowed_tools.clone(),
            disallowed_tools: self.disallowed_tools.clone(),
            slash_commands: Arc::clone(&self.slash_commands),
            mcp_proxy: Arc::clone(&self.mcp_proxy),
        }
    }

    /// Run the REPL loop
    pub async fn run(&mut self) -> Result<()> {
        // Check for resumable session and prompt user
        self.try_resume_session()?;

        loop {
            // Render TUI if dirty or animations active
            let has_animations = self.tui.is_streaming() || self.tui.has_active_tools();

            if self.tui.is_dirty() || has_animations {
                self.tui.draw()?;
                self.tui.clear_dirty();

                if has_animations {
                    self.tui.mark_dirty();
                }
            }

            // Poll for streaming events from background task (non-blocking)
            self.poll_streaming_events();

            // Poll for tool execution events from background tasks (non-blocking)
            self.poll_tool_events();

            // Check if all expected tools have completed
            self.check_tool_completion().await;

            // Poll for streaming response completion (non-blocking)
            self.poll_response_completion();

            // Process pending response if ready (continue tool use loop)
            if let Some(response) = self.pending_response.take() {
                self.tui.push_debug(
                    "[RESPONSE] Processing pending response for tool use loop".to_string(),
                );

                let services = self.session_services();
                if let Err(e) = conversation::process_response_in_tool_loop(
                    response,
                    &mut self.tui,
                    &mut self.context,
                    &mut self.stats,
                    &mut self.tool_state,
                    &services,
                )
                .await
                {
                    let error_msg = format!("Tool loop processing error: {}", e);
                    self.tui.set_status(format!("Error: {}", e));
                    self.tui.add_message(ChatMessage::system(error_msg));
                }
            }

            // Poll for terminal events
            self.poll_terminal_events(has_animations).await?;

            // Check exit condition
            if self.tui.should_exit() {
                self.tui.cleanup()?;
                break;
            }
        }

        Ok(())
    }

    /// Poll streaming channel events and dispatch to streaming handler.
    fn poll_streaming_events(&mut self) {
        if let Some(ref mut rx) = self.streaming.rx {
            match rx.try_recv() {
                Ok(event) => {
                    let response = streaming::handle_streaming_event(
                        event,
                        &mut self.tui,
                        self.streaming.message_index,
                    );

                    if let Some(response) = response {
                        // Track token usage
                        self.stats.add_assistant_message(
                            response.usage.input_tokens as u64,
                            response.usage.output_tokens as u64,
                        );
                        // Streaming completed - clean up
                        self.streaming.rx = None;
                        self.streaming.message_index = None;
                    }
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    // No events available
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    self.tui
                        .push_debug("[STREAMING] Channel disconnected unexpectedly".to_string());
                    self.streaming.rx = None;
                    self.streaming.message_index = None;
                }
            }
        }
    }

    /// Poll tool execution events and dispatch to tool orchestrator handler.
    fn poll_tool_events(&mut self) {
        if let Some(ref mut rx) = self.tool_state.tool_rx {
            match rx.try_recv() {
                Ok(event) => {
                    tool_orchestrator::handle_tool_event(
                        event,
                        &mut self.tui,
                        &mut self.tool_state.active_tools,
                        &mut self.tool_state.tool_results,
                    );
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {}
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    self.tool_state.tool_rx = None;
                }
            }
        }
    }

    /// Check if all expected tools have completed and continue the tool loop.
    async fn check_tool_completion(&mut self) {
        if self.tool_state.expected_tool_ids.is_empty() {
            return;
        }

        let all_tools_complete = self
            .tool_state
            .expected_tool_ids
            .iter()
            .all(|id| self.tool_state.tool_results.contains_key(id));

        if !all_tools_complete {
            return;
        }

        self.tui
            .push_debug("[TOOLS] All tools complete, continuing tool loop".to_string());

        // Collect results in order
        let mut tool_result_blocks = Vec::new();
        for id in &self.tool_state.expected_tool_ids {
            if let Some(result) = self.tool_state.tool_results.remove(id) {
                tool_result_blocks.push(result);
            }
        }

        self.tool_state.expected_tool_ids.clear();

        // If we have a pending response waiting for tools, continue the loop
        if let Some(response) = self.tool_state.pending_tool_response.take() {
            self.streaming
                .api_messages
                .push(rustyclawd_core::client::Message::with_blocks(
                    rustyclawd_core::client::Role::Assistant,
                    response.content,
                ));

            self.streaming
                .api_messages
                .push(rustyclawd_core::client::Message::with_blocks(
                    rustyclawd_core::client::Role::User,
                    tool_result_blocks,
                ));

            self.tui
                .push_debug("[TOOL_LOOP] Starting next turn after tools".to_string());

            if let Err(e) = conversation::start_streaming_turn(
                &self.client,
                &self.model,
                &mut self.tui,
                &mut self.streaming,
            ) {
                self.tui
                    .add_message(crate::tui::ChatMessage::system(format!(
                        "Failed to start streaming turn: {}",
                        e
                    )));
                self.tui
                    .push_debug(format!("[TOOL_LOOP] Streaming turn failed: {}", e));
            }
        }
    }

    /// Poll for streaming response completion.
    fn poll_response_completion(&mut self) {
        if let Some(ref mut rx) = self.streaming.response_rx {
            match rx.try_recv() {
                Ok(response) => {
                    self.tui
                        .push_debug("[RESPONSE] Streaming response complete".to_string());
                    self.pending_response = Some(response);
                    self.streaming.response_rx = None;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    self.tui
                        .push_debug("[RESPONSE] Channel closed unexpectedly".to_string());
                    self.streaming.response_rx = None;
                }
            }
        }
    }

    /// Poll terminal events and dispatch user input.
    async fn poll_terminal_events(&mut self, has_animations: bool) -> Result<()> {
        use crossterm::event;
        use std::time::Duration;

        let poll_timeout = if has_animations {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(16)
        };

        if event::poll(poll_timeout)? {
            let terminal_event = event::read()?;

            if let Some(input) = self.tui.handle_event(terminal_event)? {
                let input = input.trim();

                if input.is_empty() {
                    return Ok(());
                }

                self.tui.push_debug(
                    "[SUBMIT] Input received, firing IdlePrompt notification".to_string(),
                );

                // Fire IdlePrompt notification
                if let Some(ref notification_mgr) = self.notification_manager {
                    notification_mgr
                        .notify(
                            &self.session_id,
                            NotificationType::IdlePrompt,
                            "Awaiting user input",
                        )
                        .await;
                }

                self.tui
                    .push_debug("[SUBMIT] IdlePrompt notification complete".to_string());

                // Handle permission mode change event (from Shift+Tab)
                if let Some(mode_name) = input.strip_prefix("__permission_mode_changed:") {
                    self.tui.add_message(ChatMessage::system(format!(
                        "Permission mode changed to: {}",
                        mode_name
                    )));
                    return Ok(());
                }

                // Handle /model command - show or switch current model
                if input == "/model" || input.starts_with("/model ") {
                    let args = input.strip_prefix("/model").unwrap_or("").trim();
                    if args.is_empty() {
                        // Show current model
                        self.tui.add_message(ChatMessage::system(format!(
                            "Current model: {}",
                            self.model
                        )));
                    } else {
                        // Resolve alias and switch model
                        let resolved = resolve_model_alias(args);
                        self.model = resolved.to_string();
                        self.stats.set_model(&self.model);
                        self.tui.add_message(ChatMessage::system(format!(
                            "Switched to model: {}",
                            self.model
                        )));
                        self.tui.set_status(format!("Model: {}", self.model));
                    }
                    return Ok(());
                }

                // Handle special commands
                let services = self.session_services();
                let handled = conversation::handle_command(
                    input,
                    &mut self.tui,
                    &mut self.context,
                    &services,
                    &mut self.stats,
                    &mut self.persistence,
                    &mut self.streaming,
                )
                .await?;

                if handled {
                    return Ok(());
                }

                // Process user message and get Claude's response
                if let Err(e) = conversation::process_user_message(
                    input,
                    false,
                    &mut self.tui,
                    &mut self.context,
                    &services,
                    &mut self.streaming,
                )
                .await
                {
                    self.tui
                        .add_message(ChatMessage::system(format!("Error: {}", e)));
                    self.tui.set_status(format!("Error: {}", e));
                }
            }
        }

        Ok(())
    }

    /// Try to resume a previous session on startup.
    fn try_resume_session(&mut self) -> Result<()> {
        let session_info = if let Some(ref persistence) = self.persistence {
            persistence.check_resumable_session().ok().flatten()
        } else {
            None
        };

        if let Some(session_info) = session_info {
            self.tui.cleanup()?;

            if self.prompt_resume_session(&session_info)? {
                if let Some(ref mut persistence) = self.persistence {
                    match persistence.resume_session() {
                        Ok(messages) => {
                            for msg in messages {
                                self.context.add_message(msg.clone());

                                let chat_msg = match msg.role {
                                    MessageRole::User => ChatMessage::user(msg.content),
                                    MessageRole::Assistant => ChatMessage::assistant(msg.content),
                                    MessageRole::System => ChatMessage::system(msg.content),
                                };
                                self.tui.add_message(chat_msg);
                            }

                            self.tui.add_message(ChatMessage::system(format!(
                                "Session resumed ({} messages, {})",
                                session_info.message_count,
                                session_info.format_age()
                            )));
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to resume session: {}", e);
                        }
                    }
                }
            }

            // Reinitialize TUI after prompt
            self.tui = TuiState::new()?;
            let commands_for_completion = Arc::clone(&self.slash_commands);
            self.tui
                .set_completion_callback(build_completion_callback(commands_for_completion));
        }

        Ok(())
    }

    /// Prompt user to resume session (outside of TUI)
    fn prompt_resume_session(&self, info: &SessionInfo) -> Result<bool> {
        println!("\nPrevious session found:");
        println!("  Messages: {}", info.message_count);
        println!("  Last saved: {}", info.format_age());
        print!("\nResume session? [Y/n]: ");
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        let response = response.trim().to_lowercase();
        Ok(response.is_empty() || response == "y" || response == "yes")
    }
}

/// Resolve a model alias or name to the canonical model ID.
///
/// Supports shorthand aliases:
/// - `sonnet` → `claude-sonnet-4-6`
/// - `opus`   → `claude-opus-4-6`
/// - `haiku`  → `claude-haiku-4-5-20251001`
///
/// Any other value is returned as-is (treated as a literal model ID).
fn resolve_model_alias(name: &str) -> &str {
    match name {
        "sonnet" => "claude-sonnet-4-6",
        "opus" => "claude-opus-4-6",
        "haiku" => "claude-haiku-4-5-20251001",
        other => other,
    }
}

/// Build the autocomplete callback for slash commands.
fn build_completion_callback(commands: Arc<SlashCommands>) -> CompletionCallback {
    Box::new(move |prefix| {
        let built_in_commands = vec![
            ("help", Some("Show available commands".to_string())),
            ("exit", Some("Exit the session".to_string())),
            ("quit", Some("Exit the session".to_string())),
            ("clear", Some("Clear conversation history".to_string())),
            ("compact", Some("Compact conversation history".to_string())),
            ("stats", Some("Show session statistics".to_string())),
            (
                "cost",
                Some("Show token usage and cost estimate".to_string()),
            ),
            ("context", Some("Show context window usage".to_string())),
            ("usage", Some("Show API usage and rate limits".to_string())),
            (
                "bashes",
                Some("Show background shell processes".to_string()),
            ),
            ("save", Some("[description] - Save checkpoint".to_string())),
            (
                "load",
                Some("<checkpoint_id> - Load checkpoint".to_string()),
            ),
            ("sessions", Some("List available checkpoints".to_string())),
            ("mcp-list", Some("List all MCP servers".to_string())),
            (
                "mcp-start",
                Some("<server-id> - Start MCP server".to_string()),
            ),
            (
                "mcp-stop",
                Some("<server-id> - Stop MCP server".to_string()),
            ),
            (
                "mcp-tools",
                Some("<server-id> - List server tools".to_string()),
            ),
            (
                "mcp-status",
                Some("<server-id> - Show server status".to_string()),
            ),
            (
                "model",
                Some("[alias|name] - Show or switch model".to_string()),
            ),
        ];

        let mut results: Vec<(String, Option<String>)> = built_in_commands
            .into_iter()
            .filter(|(cmd, _)| cmd.starts_with(prefix))
            .map(|(cmd, desc)| (cmd.to_string(), desc))
            .collect();

        let mut custom = commands.get_completions(prefix);
        results.append(&mut custom);

        results.sort_by(|a, b| a.0.cmp(&b.0));

        results
    })
}

/// Entry point for interactive mode
pub async fn run_interactive() -> Result<()> {
    run_interactive_with_hooks(None).await
}

/// Entry point for interactive mode with optional hooks system
pub async fn run_interactive_with_hooks(hooks: Option<Arc<hooks::HooksSystem>>) -> Result<()> {
    run_interactive_with_config(hooks, vec![], vec![], Backend::Anthropic, None).await
}

/// Entry point for interactive mode with full configuration
pub async fn run_interactive_with_config(
    hooks: Option<Arc<hooks::HooksSystem>>,
    allowed_tools: Vec<String>,
    disallowed_tools: Vec<String>,
    backend: Backend,
    model_override: Option<String>,
) -> Result<()> {
    let mut session =
        InteractiveSession::with_hooks_and_backend(hooks, backend, model_override).await?;
    session.allowed_tools = allowed_tools;
    session.disallowed_tools = disallowed_tools;
    session.run().await
}
