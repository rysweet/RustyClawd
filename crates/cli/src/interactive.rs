//! Interactive chat mode (REPL) for RustyClawd
//!
//! Provides a fully functional REPL with:
//! - Ratatui TUI with pirate ship banner
//! - Real-time streaming responses from Claude
//! - Multi-turn conversation context
//! - Graceful exit handling (Ctrl+D, /exit)
//! - Session persistence with auto-save/resume
//! - Rust-colored theme

use crate::commands::SlashCommands;
use crate::hooks;
use crate::mcp_commands;
use crate::permission_mode::PermissionMode;
use crate::plugins::mcp_proxy::McpProxy;

// Import notification types
use crate::hooks::NotificationType;
use crate::notification::NotificationManager;
use crate::session::SessionStats;
use crate::session_persistence::{SessionInfo, SessionPersistence};
use crate::terminal_guard;
use crate::tool_executor;
use crate::tool_formatter;
use crate::tui::{ChatMessage, MessageRole as TuiMessageRole, TuiState};
use anyhow::Result;
use futures::StreamExt;
use rustyclawd_core::{
    client::{
        Client, ClientError, Config, CreateMessageRequest, Message as ApiMessage, StreamEvent,
    },
    Context, Message, MessageRole,
};
use rustyclawd_tools::{
    bash::BashParams, BashTool, ExecutionContext, Tool, ToolContext, ToolEvent,
};
use secrecy::ExposeSecret;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Default model for interactive sessions
const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250929";

/// Maximum tokens for responses
const MAX_TOKENS: u32 = 4096;

/// Events sent from background streaming task to main event loop
#[derive(Debug, Clone)]
enum StreamingChannelEvent {
    /// Text content delta to append
    TextDelta { text: String },
    /// Token count update (input_tokens, output_tokens)
    TokenUpdate { input: u32, output: u32 },
    /// Streaming completed successfully with final response
    Complete {
        response: rustyclawd_core::client::MessageResponse,
    },
    /// Streaming failed with error
    Error { message: String },
    /// Thinking mode update (true = thinking, false = receiving tokens)
    ThinkingUpdate { thinking: bool },
}

/// Events sent from background tool execution tasks to main event loop
#[derive(Debug, Clone)]
enum ToolExecutionEvent {
    /// Tool execution started
    Started {
        tool_id: String,
        tool_name: String,
        params: serde_json::Value,
    },
    /// Tool execution progress (optional)
    Progress { tool_id: String, message: String },
    /// Tool execution completed successfully
    Complete {
        tool_id: String,
        result: rustyclawd_core::client::types::ContentBlock,
    },
    /// Tool execution failed
    Error { tool_id: String, error: String },
}

/// Helper function to get current working directory as string
fn get_cwd_string() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// Interactive chat session with TUI
pub struct InteractiveSession {
    /// Anthropic API client
    client: Client,
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
    /// Channel receiver for streaming events from background task
    streaming_rx: Option<tokio::sync::mpsc::UnboundedReceiver<StreamingChannelEvent>>,
    /// Active streaming message index (if streaming)
    streaming_message_index: Option<usize>,
    /// Channel receiver for tool execution events from background tasks
    tool_rx: Option<tokio::sync::mpsc::UnboundedReceiver<ToolExecutionEvent>>,
    /// Active tool executions (tool_id -> tool_name)
    active_tools: std::collections::HashMap<String, String>,
    /// Completed tool results (tool_id -> result)
    tool_results: std::collections::HashMap<String, rustyclawd_core::client::types::ContentBlock>,
    /// Channel receiver for streaming response completion
    response_rx: Option<tokio::sync::oneshot::Receiver<rustyclawd_core::client::MessageResponse>>,
    /// API messages for current turn (needed for tool use loop continuation)
    api_messages: Vec<ApiMessage>,
    /// Pending response for tool use loop processing
    pending_response: Option<rustyclawd_core::client::MessageResponse>,
    /// Expected tool IDs for current batch (used to detect completion)
    expected_tool_ids: Vec<String>,
    /// Current response waiting for tool completion
    pending_tool_response: Option<rustyclawd_core::client::MessageResponse>,
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
        // Set execution context to TUI mode for process isolation
        terminal_guard::set_execution_context(terminal_guard::ExecutionContext::Tui);

        // Load API configuration from default location
        let config = Config::from_default_location().await?;
        let client = Client::new(config);

        // Initialize TUI
        let mut tui = TuiState::new()?;

        // Initialize slash command system
        let slash_commands = Arc::new(SlashCommands::new().await?);

        // Wire up autocomplete callback
        let commands_for_completion = Arc::clone(&slash_commands);
        tui.set_completion_callback(Box::new(move |prefix| {
            // Built-in commands that should appear in autocomplete
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
            ];

            // Filter built-in commands by prefix
            let mut results: Vec<(String, Option<String>)> = built_in_commands
                .into_iter()
                .filter(|(cmd, _)| cmd.starts_with(prefix))
                .map(|(cmd, desc)| (cmd.to_string(), desc))
                .collect();

            // Add custom commands from registry
            let mut custom = commands_for_completion.get_completions(prefix);
            results.append(&mut custom);

            // Sort by command name
            results.sort_by(|a, b| a.0.cmp(&b.0));

            results
        }));

        // Initialize session persistence
        let persistence = SessionPersistence::with_default_id().ok();

        // Initialize MCP proxy (empty for now, will be populated by App)
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

        Ok(Self {
            client,
            context: Context::new(),
            tui,
            model: DEFAULT_MODEL.to_string(),
            slash_commands,
            persistence,
            mcp_proxy,
            stats: SessionStats::new(DEFAULT_MODEL),
            hooks,
            session_id,
            notification_manager,
            streaming_rx: None,
            streaming_message_index: None,
            tool_rx: None,
            active_tools: std::collections::HashMap::new(),
            tool_results: std::collections::HashMap::new(),
            response_rx: None,
            api_messages: Vec::new(),
            pending_response: None,
            expected_tool_ids: Vec::new(),
            pending_tool_response: None,
            allowed_tools: vec![],
            disallowed_tools: vec![],
        })
    }

    /// Run the REPL loop
    pub async fn run(&mut self) -> Result<()> {
        // TUI shows welcome banner automatically

        // Check for resumable session and prompt user
        let session_info = if let Some(ref persistence) = self.persistence {
            persistence.check_resumable_session().ok().flatten()
        } else {
            None
        };

        if let Some(session_info) = session_info {
            // Temporarily cleanup TUI to show prompt
            self.tui.cleanup()?;

            if self.prompt_resume_session(&session_info)? {
                // Resume session
                if let Some(ref mut persistence) = self.persistence {
                    match persistence.resume_session() {
                        Ok(messages) => {
                            // Restore messages to context
                            for msg in messages {
                                self.context.add_message(msg.clone());

                                // Add to TUI display (will be added after TUI reinit)
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
            self.tui.set_completion_callback(Box::new(move |prefix| {
                // Built-in commands that should appear in autocomplete
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
                ];

                // Filter built-in commands by prefix
                let mut results: Vec<(String, Option<String>)> = built_in_commands
                    .into_iter()
                    .filter(|(cmd, _)| cmd.starts_with(prefix))
                    .map(|(cmd, desc)| (cmd.to_string(), desc))
                    .collect();

                // Add custom commands from registry
                let mut custom = commands_for_completion.get_completions(prefix);
                results.append(&mut custom);

                // Sort by command name
                results.sort_by(|a, b| a.0.cmp(&b.0));

                results
            }));
        }

        loop {
            // ALWAYS render if animations are active (tools executing or streaming)
            // This ensures continuous updates for throbbers and timers
            let has_animations = self.tui.is_streaming() || self.tui.has_active_tools();

            if self.tui.is_dirty() || has_animations {
                self.tui.draw()?;
                self.tui.clear_dirty();

                // If animations are active, immediately mark dirty for next frame
                if has_animations {
                    self.tui.mark_dirty();
                }
            }

            // Poll for streaming events from background task (non-blocking)
            if let Some(ref mut rx) = self.streaming_rx {
                match rx.try_recv() {
                    Ok(event) => match event {
                        StreamingChannelEvent::TextDelta { text } => {
                            if let Some(idx) = self.streaming_message_index {
                                self.tui.append_to_message(idx, &text);
                            }
                        }
                        StreamingChannelEvent::TokenUpdate { input, output } => {
                            self.tui.update_token_count(input, output);
                        }
                        StreamingChannelEvent::ThinkingUpdate { thinking } => {
                            // Thinking mode updated - token counter will reflect this
                            if !thinking {
                                // First token received - no longer thinking
                                self.tui.push_debug(
                                    "[STREAMING] First token received - thinking complete"
                                        .to_string(),
                                );
                            }
                        }
                        StreamingChannelEvent::Complete { response } => {
                            // Finalize streaming
                            if let Some(idx) = self.streaming_message_index {
                                self.tui.finalize_streaming_message(idx);
                            }
                            self.streaming_rx = None;
                            self.streaming_message_index = None;
                            self.tui.set_status("Ready".to_string());

                            // Track token usage
                            self.stats.add_assistant_message(
                                response.usage.input_tokens as u64,
                                response.usage.output_tokens as u64,
                            );

                            // Store response for tool use loop continuation
                            // (handled in stream_with_tools)
                        }
                        StreamingChannelEvent::Error { message } => {
                            // Streaming failed
                            self.tui
                                .add_message(ChatMessage::system(format!("Error: {}", message)));
                            self.tui.set_status(format!("Error: {}", message));
                            self.streaming_rx = None;
                            self.streaming_message_index = None;
                        }
                    },
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        // No events available - this is normal
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        // Channel closed unexpectedly
                        self.tui.push_debug(
                            "[STREAMING] Channel disconnected unexpectedly".to_string(),
                        );
                        self.streaming_rx = None;
                        self.streaming_message_index = None;
                    }
                }
            }

            // Poll for tool execution events from background tasks (non-blocking)
            if let Some(ref mut rx) = self.tool_rx {
                match rx.try_recv() {
                    Ok(event) => {
                        match event {
                            ToolExecutionEvent::Started { .. } => {
                                // Started events no longer used - messages created synchronously in spawn_tools()
                                // Keeping this variant for backward compatibility
                            }
                            ToolExecutionEvent::Progress { tool_id, message } => {
                                // Optional: show progress updates
                                if let Some(tool_name) = self.active_tools.get(&tool_id) {
                                    self.tui
                                        .push_debug(format!("[TOOL:{}] {}", tool_name, message));
                                }
                            }
                            ToolExecutionEvent::Complete { tool_id, result } => {
                                self.tui.push_debug(format!(
                                    "[TOOL] Complete event received for: {}",
                                    tool_id
                                ));

                                // Parse tool result
                                let tool_result = if let rustyclawd_core::client::types::ContentBlock::ToolResult { content, is_error, .. } = &result {
                                // Extract text from ContentBlocks
                                let content_text = content.iter()
                                    .filter_map(|block| {
                                        if let rustyclawd_core::client::types::ContentBlock::Text { text } = block {
                                            Some(text.as_str())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<&str>>()
                                    .join("");

                                // Debug: log the raw content
                                self.tui.push_debug(format!("[TOOL] Result content: {}", content_text));

                                // Try to parse as JSON (for bash tools)
                                let (exit_code, stdout, stderr) = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content_text) {
                                    let exit_code = json.get("exit_code").and_then(|v| v.as_i64()).map(|v| v as i32);
                                    let stdout = json.get("stdout").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let stderr = json.get("stderr").and_then(|v| v.as_str()).unwrap_or("").to_string();

                                    // Check for background process (has shell_id but no output)
                                    if let Some(shell_id) = json.get("shell_id").and_then(|v| v.as_str()) {
                                        self.tui.push_debug(format!("[TOOL] Background process registered: shell_id={}", shell_id));
                                    }

                                    (exit_code, stdout, stderr)
                                } else {
                                    // Plain text result
                                    (None, content_text.clone(), String::new())
                                };

                                crate::tui::ToolResult {
                                    exit_code,
                                    stdout,
                                    stderr,
                                    is_error: is_error.unwrap_or(false),
                                    raw_content: content_text,
                                    structured_content: None,
                                }
                            } else {
                                // Non-ToolResult content block (shouldn't happen)
                                crate::tui::ToolResult {
                                    exit_code: None,
                                    stdout: String::new(),
                                    stderr: "Unexpected result format".to_string(),
                                    is_error: true,
                                    raw_content: "Unexpected result format".to_string(),
                                    structured_content: None,
                                }
                            };

                                // Finalize tool message (updates UI with result)
                                self.tui.finalize_tool_message(&tool_id, tool_result);
                                self.tui.push_debug(format!(
                                    "[TOOL] Message finalized for: {}",
                                    tool_id
                                ));

                                // Store result for tool loop continuation
                                if let Some(_tool_name) = self.active_tools.remove(&tool_id) {
                                    self.tool_results.insert(tool_id.clone(), result);
                                    self.tui.push_debug(format!(
                                        "[TOOL] Result stored for tool loop: {}",
                                        tool_id
                                    ));
                                }
                            }
                            ToolExecutionEvent::Error { tool_id, error } => {
                                // Tool failed - create error result
                                let tool_result = crate::tui::ToolResult {
                                    exit_code: Some(1), // Generic error exit code
                                    stdout: String::new(),
                                    stderr: error.clone(),
                                    is_error: true,
                                    raw_content: format!("Tool execution error: {}", error),
                                    structured_content: None,
                                };

                                // Finalize tool message with error
                                self.tui.finalize_tool_message(&tool_id, tool_result);

                                // Store error as tool result for tool loop continuation
                                if let Some(_tool_name) = self.active_tools.remove(&tool_id) {
                                    let error_result =
                                        rustyclawd_core::client::types::ContentBlock::ToolResult {
                                            tool_use_id: tool_id.clone(),
                                            content: vec![rustyclawd_core::client::types::ContentBlock::Text {
                                                text: format!("Tool execution error: {}", error),
                                            }],
                                            is_error: Some(true),
                                        };
                                    self.tool_results.insert(tool_id, error_result);
                                }
                            }
                        }
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                        // No events available - this is normal
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        // Channel closed - tools done
                        self.tool_rx = None;
                    }
                }
            }

            // Check if all expected tools have completed
            if !self.expected_tool_ids.is_empty() {
                let all_tools_complete = self
                    .expected_tool_ids
                    .iter()
                    .all(|id| self.tool_results.contains_key(id));

                if all_tools_complete {
                    self.tui
                        .push_debug("[TOOLS] All tools complete, continuing tool loop".to_string());

                    // Collect results in order
                    let mut tool_result_blocks = Vec::new();
                    for id in &self.expected_tool_ids {
                        if let Some(result) = self.tool_results.remove(id) {
                            tool_result_blocks.push(result);
                        }
                    }

                    // Clear expected tool IDs
                    self.expected_tool_ids.clear();

                    // If we have a pending response waiting for tools, continue the loop
                    if let Some(response) = self.pending_tool_response.take() {
                        // Add assistant's response with tool_use blocks to API messages
                        self.api_messages.push(ApiMessage::with_blocks(
                            rustyclawd_core::client::Role::Assistant,
                            response.content,
                        ));

                        // Add tool results as user message to API messages
                        self.api_messages.push(ApiMessage::with_blocks(
                            rustyclawd_core::client::Role::User,
                            tool_result_blocks,
                        ));

                        // Continue to next turn (spawn new streaming task)
                        self.tui
                            .push_debug("[TOOL_LOOP] Starting next turn after tools".to_string());
                        let _ = self
                            .stream_single_turn_with_messages(&self.api_messages.clone())
                            .await;
                    }
                }
            }

            // Poll for streaming response completion (non-blocking)
            if let Some(ref mut rx) = self.response_rx {
                match rx.try_recv() {
                    Ok(response) => {
                        // Response complete - store for processing
                        self.tui
                            .push_debug("[RESPONSE] Streaming response complete".to_string());
                        self.pending_response = Some(response);
                        self.response_rx = None;
                    }
                    Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                        // Still waiting - this is normal
                    }
                    Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                        // Channel closed unexpectedly
                        self.tui
                            .push_debug("[RESPONSE] Channel closed unexpectedly".to_string());
                        self.response_rx = None;
                    }
                }
            }

            // Process pending response if ready (continue tool use loop)
            if let Some(response) = self.pending_response.take() {
                self.tui.push_debug(
                    "[RESPONSE] Processing pending response for tool use loop".to_string(),
                );

                // Continue tool use loop with this response
                if let Err(e) = self.process_response_in_tool_loop(response).await {
                    let error_msg = format!("Tool loop processing error: {}", e);
                    self.tui.set_status(format!("Error: {}", e));
                    self.tui.add_message(ChatMessage::system(error_msg));
                }
            }

            // Poll for terminal events
            // Use shorter timeout when animations are active to ensure continuous updates
            use crossterm::event;
            use std::time::Duration;

            let poll_timeout = if has_animations {
                // Short timeout when animating (for smooth throbber/timer updates)
                Duration::from_millis(100)
            } else {
                // Normal timeout when idle (for responsiveness without burning CPU)
                Duration::from_millis(16)
            };

            if event::poll(poll_timeout)? {
                let terminal_event = event::read()?;

                // Handle terminal event
                if let Some(input) = self.handle_terminal_event(terminal_event)? {
                    let input = input.trim();

                    // Handle empty input
                    if input.is_empty() {
                        continue;
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
                        continue;
                    }

                    // Handle special commands
                    if self.handle_command(input).await? {
                        continue;
                    }

                    // Process user message and get Claude's response
                    if let Err(e) = self.process_user_message(input, false).await {
                        self.tui
                            .add_message(ChatMessage::system(format!("Error: {}", e)));
                        self.tui.set_status(format!("Error: {}", e));
                    }
                }
            }

            // Check exit condition
            if self.tui.should_exit() {
                // CRITICAL: Explicit cleanup on Ctrl+C / Ctrl+D exit
                self.tui.cleanup()?;
                break;
            }
        }

        Ok(())
    }

    /// Handle terminal event (keyboard, resize, etc.)
    /// Returns Some(input) if user submitted input, None otherwise
    fn handle_terminal_event(&mut self, event: crossterm::event::Event) -> Result<Option<String>> {
        // Just return the input - don't add message here (process_user_message will do it)
        self.tui.handle_event(event)
    }

    /// Handle special commands
    /// Returns true if command was handled, false if input should be processed as message
    async fn handle_command(&mut self, input: &str) -> Result<bool> {
        // Handle "!" prefix for direct shell execution
        if let Some(stripped) = input.strip_prefix('!') {
            let command = stripped.trim();
            if command.is_empty() {
                self.tui.add_message(ChatMessage::system(
                    "Error: No command specified after '!'".to_string(),
                ));
                return Ok(true);
            }

            self.execute_shell_command(command).await?;
            return Ok(true);
        }

        match input {
            "/exit" | "/quit" => {
                // Execute Stop hook to check if exit should be allowed
                if let Some(ref hooks) = self.hooks {
                    let context = hooks::HookContext::for_session(
                        self.session_id.clone(),
                        format!(".claude/sessions/{}/transcript.json", self.session_id),
                        get_cwd_string(),
                        "ask".to_string(),
                        hooks::HookEvent::Stop,
                    );

                    match hooks.execute_hooks(hooks::HookEvent::Stop, &context).await {
                        Ok(results) => {
                            for result in results {
                                if let Some(output) = result.parse_output() {
                                    // Check if hook is blocking exit
                                    if let Some(decision) = output.decision {
                                        if decision == hooks::types::StopDecision::Block {
                                            let reason = output.reason.unwrap_or_else(|| {
                                                "Stop blocked by hook".to_string()
                                            });
                                            self.tui.add_message(ChatMessage::system(format!(
                                                "Exit blocked: {}",
                                                reason
                                            )));
                                            return Ok(true); // Continue session
                                        }
                                    }
                                }
                                if !result.is_success() {
                                    tracing::warn!("Stop hook failed: {}", result.stderr);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to execute Stop hooks: {}", e);
                            // Non-blocking - continue with exit even if hook fails
                        }
                    }
                }

                // Auto-save before exit
                self.auto_save_session();

                self.tui.cleanup()?;
                println!("\nGoodbye, matey! Fair winds and following seas! ⛵");
                std::process::exit(0);
            }
            "/clear" => {
                self.context = Context::new();
                self.tui.add_message(ChatMessage::system(
                    "Conversation history cleared".to_string(),
                ));
                self.tui.set_status("Conversation cleared".to_string());
                return Ok(true);
            }
            "/compact" => {
                // Fire PreCompact hook BEFORE compaction
                if let Some(ref hooks) = self.hooks {
                    let context = hooks::HookContext::for_session(
                        self.session_id.clone(),
                        format!(".claude/sessions/{}/transcript.json", self.session_id),
                        get_cwd_string(),
                        "ask".to_string(),
                        hooks::HookEvent::PreCompact,
                    );

                    match hooks
                        .execute_hooks(hooks::HookEvent::PreCompact, &context)
                        .await
                    {
                        Ok(results) => {
                            for result in results {
                                if !result.is_success() {
                                    self.tui.add_message(ChatMessage::system(format!(
                                        "⚠️  PreCompact hook failed: {}",
                                        result.stderr
                                    )));
                                    return Ok(true);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("PreCompact hook execution failed: {:?}", e);
                            self.tui.add_message(ChatMessage::system(format!(
                                "⚠️  Failed to execute PreCompact hooks: {}",
                                e
                            )));
                            return Ok(true);
                        }
                    }
                }

                // PreCompact hook fired successfully
                self.tui.add_message(ChatMessage::system("✓ PreCompact hook fired.\n\nCompacting conversation history...\n(Full compaction logic awaits implementation)".to_string(),));
                return Ok(true);
            }
            "/help" => {
                // Show all commands - built-in and custom
                let custom_commands = self.slash_commands.list_commands();
                let mut help_text = "Built-in Commands:\n  /exit, /quit - Exit the session\n  /clear - Clear conversation history\n  /compact - Compact conversation history (fires PreCompact hook)\n  /help - Show this help\n  /stats - Show session statistics\n  /save [description] - Save checkpoint\n  /load <checkpoint_id> - Load checkpoint\n  /sessions - List available sessions\n  !<command> - Execute shell command directly\n\nMCP Commands:\n  /mcp-list - List all MCP servers\n  /mcp-start <server-id> - Start an MCP server\n  /mcp-stop <server-id> - Stop an MCP server\n  /mcp-tools <server-id> - List tools from server\n  /mcp-status <server-id> - Show server status\n".to_string();

                if !custom_commands.is_empty() {
                    help_text.push_str("\nCustom Commands:\n");
                    for cmd in custom_commands {
                        help_text.push_str(&format!("  /{}\n", cmd));
                    }
                }

                help_text.push_str("\nPress Ctrl+C or Ctrl+D to exit.");

                self.tui.add_message(ChatMessage::system(help_text));
                return Ok(true);
            }
            "/stats" => {
                // Update duration before displaying
                self.stats.update_duration();

                let stats = format!(
                    "Session Statistics:\n\
                     Messages: {} ({} user, {} assistant)\n\
                     Input tokens: {}\n\
                     Output tokens: {}\n\
                     Total tokens: {}\n\
                     Tool calls: {}\n\
                     Model: {}\n\
                     Duration: {}s",
                    self.stats.message_count,
                    self.stats.user_message_count,
                    self.stats.assistant_message_count,
                    self.stats.input_tokens,
                    self.stats.output_tokens,
                    self.stats.total_tokens,
                    self.stats.tool_calls,
                    self.model,
                    self.stats.duration_seconds
                );
                self.tui.add_message(ChatMessage::system(stats));
                return Ok(true);
            }
            "/cost" => {
                self.handle_cost_command();
                return Ok(true);
            }
            "/context" => {
                self.handle_context_command();
                return Ok(true);
            }
            "/usage" => {
                self.handle_usage_command();
                return Ok(true);
            }
            "/bashes" => {
                self.handle_bashes_command().await?;
                return Ok(true);
            }
            _ if input.starts_with("/save") => {
                self.handle_save_command(input)?;
                return Ok(true);
            }
            _ if input.starts_with("/load") => {
                self.handle_load_command(input)?;
                return Ok(true);
            }
            "/sessions" => {
                self.handle_sessions_command()?;
                return Ok(true);
            }
            _ if input.starts_with("/mcp-") => {
                // Handle MCP commands
                if let Some((command, args)) = mcp_commands::parse_slash_command(input) {
                    self.tui
                        .set_status(format!("Executing MCP command: {}", input));

                    match mcp_commands::handle_tui_command(self.mcp_proxy.clone(), &command, args)
                        .await
                    {
                        Ok(output) => {
                            self.tui.add_message(ChatMessage::system(output));
                            self.tui.set_status("Ready".to_string());
                        }
                        Err(e) => {
                            self.tui
                                .add_message(ChatMessage::system(format!("Error: {}", e)));
                            self.tui.set_status(format!("Error: {}", e));
                        }
                    }
                    return Ok(true);
                }
            }
            _ if input.starts_with('/') => {
                // Try custom slash command
                let command_name = input[1..].split_whitespace().next().unwrap_or("");

                if self.slash_commands.has_command(command_name) {
                    // Check if command should be intercepted locally
                    if !self.slash_commands.should_intercept_locally(command_name) {
                        // Pass through to Claude for model invocation
                        return Ok(false);
                    }

                    // Execute custom command locally
                    self.tui.set_status(format!("Executing command: {}", input));

                    match self.slash_commands.execute(input).await {
                        Ok(result) => {
                            // Add slash command invocation as user message
                            self.tui.add_message(ChatMessage::user(input.to_string()));

                            self.tui
                                .add_message(ChatMessage::system(result.expanded_prompt.clone()));

                            // Add to conversation context
                            self.context
                                .add_message(Message::user(result.expanded_prompt.clone()));

                            // Process the expanded prompt as if user typed it
                            // Skip TUI display since we already showed it as a collapsed system message
                            if let Err(e) = self
                                .process_user_message(&result.expanded_prompt, true)
                                .await
                            {
                                self.tui.add_message(ChatMessage::system(format!(
                                    "Error processing command: {}",
                                    e
                                )));
                            }
                        }
                        Err(e) => {
                            self.tui.add_message(ChatMessage::system(format!(
                                "Error executing command: {}",
                                e
                            )));
                        }
                    }
                    return Ok(true);
                }

                // Unknown command - pass through to Claude
                // Claude may handle it via SlashCommand tool or provide appropriate error
                return Ok(false);
            }
            _ => {}
        }

        Ok(false)
    }

    /// Execute a shell command directly and add to context
    async fn execute_shell_command(&mut self, command: &str) -> Result<()> {
        self.tui.set_status(format!("Executing: {}", command));

        // Create tool context with TUI execution context
        let ctx = ToolContext {
            cwd: std::env::current_dir().unwrap_or_default(),
            debug: false,
            metadata: serde_json::Value::Null,
            execution_context: ExecutionContext::Tui,
            allowed_tools: self.allowed_tools.clone(),
            disallowed_tools: self.disallowed_tools.clone(),
        };

        // Create bash tool parameters
        let params = BashParams {
            command: command.to_string(),
            timeout: 120_000, // 2 minutes default
            description: None,
            run_in_background: false,
        };

        // Execute the command
        let tool = BashTool;
        let mut stream = tool.execute(params, &ctx).await?;

        let mut stdout_output = String::new();
        let mut stderr_output = String::new();
        let mut exit_code = None;
        let mut success = false;

        // Process the stream
        while let Some(event) = stream.next().await {
            match event {
                ToolEvent::Progress { .. } => {
                    // Optionally show progress (currently silent)
                }
                ToolEvent::Result(output) => {
                    if let Some(ref stdout) = output.stdout {
                        if !stdout.is_empty() {
                            stdout_output = stdout.clone();
                        }
                    }

                    if let Some(ref stderr) = output.stderr {
                        if !stderr.is_empty() {
                            stderr_output = stderr.clone();
                        }
                    }

                    exit_code = output.exit_code;
                    success = output.success;
                }
                ToolEvent::Error { message } => {
                    self.tui
                        .add_message(ChatMessage::system(format!("Error: {}", message)));
                    return Err(anyhow::anyhow!("Command execution failed: {}", message));
                }
            }
        }

        // Format output for display and context
        let mut result_msg = format!("$ {}\n", command);

        if !stdout_output.is_empty() {
            result_msg.push_str(&format!("\n{}", stdout_output.trim()));
        }

        if !stderr_output.is_empty() {
            result_msg.push_str(&format!("\nStderr:\n{}", stderr_output.trim()));
        }

        if let Some(code) = exit_code {
            result_msg.push_str(&format!("\nExit code: {}", code));
        }

        // Add to TUI
        self.tui
            .add_message(ChatMessage::system(result_msg.clone()));

        // Add to context as a user message (tool use result)
        self.context.add_message(Message::user(result_msg));

        // Update status
        if success {
            self.tui
                .set_status("Command completed successfully".to_string());
        } else {
            self.tui.set_status(format!(
                "Command failed with exit code: {}",
                exit_code.unwrap_or(-1)
            ));
        }

        Ok(())
    }

    /// Process a user message with streaming and tool support
    async fn process_user_message(
        &mut self,
        user_input: &str,
        skip_tui_display: bool,
    ) -> Result<()> {
        self.tui
            .push_debug("[PROCESS] Starting process_user_message".to_string());

        // Execute UserPromptSubmit hook BEFORE adding prompt to context
        if let Some(ref hooks) = self.hooks {
            self.tui
                .push_debug("[PROCESS] Executing UserPromptSubmit hook".to_string());

            let context = hooks::HookContext::for_user_prompt(
                self.session_id.clone(),
                format!(".claude/sessions/{}/transcript.json", self.session_id),
                get_cwd_string(),
                "ask".to_string(),
                user_input.to_string(),
            );

            match hooks
                .execute_hooks(hooks::HookEvent::UserPromptSubmit, &context)
                .await
            {
                Ok(results) => {
                    self.tui
                        .push_debug("[PROCESS] UserPromptSubmit hook complete".to_string());
                    for result in results {
                        if result.is_blocking() {
                            self.tui.add_message(ChatMessage::assistant(format!(
                                "⚠️  Prompt blocked by hook: {}",
                                result.stderr
                            )));
                            return Ok(());
                        }
                        if !result.is_success() {
                            tracing::warn!("UserPromptSubmit hook failed: {}", result.stderr);
                        }
                    }
                }
                Err(e) => {
                    self.tui
                        .push_debug(format!("[PROCESS] UserPromptSubmit hook error: {}", e));
                    tracing::warn!("Failed to execute UserPromptSubmit hooks: {}", e);
                    // Non-blocking - continue even if hook fails
                }
            }
        }

        self.tui
            .push_debug("[PROCESS] Adding user message to TUI".to_string());

        // Add user message to TUI (unless skipped for slash commands) and context
        if !skip_tui_display {
            self.tui
                .add_message(ChatMessage::user(user_input.to_string()));
        }
        self.context
            .add_message(Message::user(user_input.to_string()));

        self.tui
            .push_debug("[PROCESS] Starting stream_with_tools".to_string());

        // Stream response with tool use loop
        self.stream_with_tools().await?;

        self.tui
            .push_debug("[PROCESS] Completed process_user_message".to_string());

        Ok(())
    }

    /// Manages the tool use loop with streaming
    ///
    /// This method only INITIATES the first turn - the actual loop happens
    /// in the main event loop via response polling and process_response_in_tool_loop()
    async fn stream_with_tools(&mut self) -> Result<()> {
        // Initialize API messages for tool use loop
        self.api_messages = self.convert_messages_to_api_format();

        // Update status
        self.tui.set_status("Streaming...".to_string());

        // Stream the first turn (returns immediately, response processed via polling)
        let _ = self
            .stream_single_turn_with_messages(&self.api_messages.clone())
            .await;

        // The rest of the tool loop happens in the main event loop via response polling
        // See process_response_in_tool_loop() for continuation logic
        Ok(())
    }

    /// Process a completed response and continue tool use loop if needed
    ///
    /// This method is called from the main event loop when a streaming response completes.
    /// It checks for tool use and either:
    /// - Completes the turn (no tool use)
    /// - Executes tools and continues to next turn (tool use present)
    async fn process_response_in_tool_loop(
        &mut self,
        response: rustyclawd_core::client::MessageResponse,
    ) -> Result<()> {
        // Check if response contains tool use
        let mut tool_use_blocks = Vec::new();
        for block in &response.content {
            if let rustyclawd_core::client::types::ContentBlock::ToolUse { id, name, input } = block
            {
                tool_use_blocks.push((id.clone(), name.clone(), input.clone()));
            }
        }

        // If no tool use, we're done with this turn
        if tool_use_blocks.is_empty() {
            self.tui.set_status("Ready".to_string());

            // Add final text response to context
            let response_text = response
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

            if !response_text.is_empty() {
                // Check if response contains questions (ElicitationDialog trigger)
                if response_text.contains('?') {
                    if let Some(ref notification_mgr) = self.notification_manager {
                        notification_mgr
                            .notify(
                                &self.session_id,
                                NotificationType::ElicitationDialog,
                                "Claude is asking clarifying questions",
                            )
                            .await;
                    }
                }

                self.context.add_message(Message::assistant(response_text));
            }

            return Ok(());
        }

        // Tool use present - spawn tool execution (non-blocking)
        self.tui
            .push_debug("[TOOL_LOOP] Spawning tool execution".to_string());

        // Store response for continuation after tools complete
        self.pending_tool_response = Some(response);

        // Spawn tools (returns immediately, results via polling)
        self.spawn_tools(tool_use_blocks)?;

        // The main event loop will detect tool completion and continue
        // See lines 406-444 for completion detection and continuation
        Ok(())
    }

    /// Streams a single turn and returns the complete response
    /// Spawns streaming in background task for non-blocking operation
    async fn stream_single_turn_with_messages(
        &mut self,
        api_messages: &[ApiMessage],
    ) -> Result<rustyclawd_core::client::MessageResponse> {
        self.tui
            .push_debug("[STREAM] Starting stream_single_turn_with_messages".to_string());

        // Create channels for communication with background task
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        self.tui
            .push_debug("[STREAM] Channels created, preparing background task".to_string());

        // Get tool definitions
        let tools = crate::tool_definitions::get_all_tool_definitions();

        // Create API request with tools and streaming enabled
        let request =
            CreateMessageRequest::new(self.model.clone(), api_messages.to_vec(), MAX_TOKENS)
                .with_tools(tools)
                .with_temperature(1.0)
                .with_stream(true);

        // Clone client data needed for background task
        let api_url = self.client.api_url().to_string();
        let api_key = self
            .client
            .config()
            .api_key
            .expose_secret()
            .expose()
            .to_string();
        let api_version = self.client.api_version().to_string();
        let http_client = self.client.http_client().clone();
        let model = self.model.clone();

        // Spawn background task for streaming (completely independent of self)
        tokio::spawn(async move {
            // Make HTTP request
            let url = format!("{}/v1/messages", api_url);
            let http_response = match http_client
                .post(&url)
                .header("x-api-key", api_key)
                .header("anthropic-version", api_version)
                .header("content-type", "application/json")
                .header("accept", "text/event-stream")
                .json(&request)
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    let _ = event_tx.send(StreamingChannelEvent::Error {
                        message: format!("HTTP request failed: {}", e),
                    });
                    return;
                }
            };

            // Check for HTTP errors
            if !http_response.status().is_success() {
                let status = http_response.status();
                let error_text = http_response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                let _ = event_tx.send(StreamingChannelEvent::Error {
                    message: format!("HTTP {}: {}", status, error_text),
                });
                return;
            }

            // Convert response body into event stream
            use rustyclawd_core::client::EventStream;
            let byte_stream = http_response.bytes_stream();
            let mut stream = EventStream::new(byte_stream);

            // Track response data
            let mut message_id = String::new();
            let mut response_content = Vec::new();
            let mut current_text = String::new();
            let mut current_tool_use: Option<(String, String, String)> = None; // (id, name, json)
            let mut usage = rustyclawd_core::client::Usage {
                input_tokens: 0,
                output_tokens: 0,
            };
            let mut stop_reason = None;
            let mut thinking = true; // Start in thinking mode

            // Process stream events and send to main loop via channel
            while let Some(result) = stream.next().await {
                match result {
                    Ok(event) => match event {
                        StreamEvent::MessageStart { message } => {
                            message_id = message.id.clone();
                            usage = message.usage.clone();

                            // Send initial token count
                            let _ = event_tx.send(StreamingChannelEvent::TokenUpdate {
                                input: message.usage.input_tokens,
                                output: message.usage.output_tokens,
                            });
                        }
                        StreamEvent::ContentBlockStart {
                            content_block:
                                rustyclawd_core::client::types::ContentBlockStart::Text { .. },
                            ..
                        } => {
                            // Starting a text block
                        }
                        StreamEvent::ContentBlockStart {
                            content_block:
                                rustyclawd_core::client::types::ContentBlockStart::Thinking,
                            ..
                        } => {
                            // Starting a thinking block - notify TUI
                            let _ = event_tx
                                .send(StreamingChannelEvent::ThinkingUpdate { thinking: true });
                        }
                        StreamEvent::ContentBlockStart {
                            content_block:
                                rustyclawd_core::client::types::ContentBlockStart::ToolUse { id, name },
                            ..
                        } => {
                            // Starting a tool use block
                            current_tool_use = Some((id, name, String::new()));
                        }
                        StreamEvent::ContentBlockDelta {
                            delta: rustyclawd_core::client::types::ContentDelta::TextDelta { text },
                            ..
                        } => {
                            // Send text delta to main loop for display
                            let _ = event_tx
                                .send(StreamingChannelEvent::TextDelta { text: text.clone() });

                            // First text received - no longer thinking
                            if thinking {
                                thinking = false;
                                let _ = event_tx.send(StreamingChannelEvent::ThinkingUpdate {
                                    thinking: false,
                                });
                            }

                            current_text.push_str(&text);
                        }
                        StreamEvent::ContentBlockDelta {
                            delta:
                                rustyclawd_core::client::types::ContentDelta::ThinkingDelta { thinking },
                            ..
                        } => {
                            // Thinking content - display but don't include in final response
                            let _ = event_tx.send(StreamingChannelEvent::TextDelta {
                                text: thinking.clone(),
                            });
                            // Accumulate thinking text separately if needed
                            current_text.push_str(&thinking);
                        }
                        StreamEvent::ContentBlockDelta {
                            delta:
                                rustyclawd_core::client::types::ContentDelta::SignatureDelta { .. },
                            ..
                        } => {
                            // Signature delta - we don't display this to users
                            // Just accumulate for the content block
                        }
                        StreamEvent::ContentBlockDelta {
                            delta:
                                rustyclawd_core::client::types::ContentDelta::InputJsonDelta {
                                    partial_json,
                                },
                            ..
                        } => {
                            // Accumulate tool input JSON
                            if let Some((_, _, ref mut json)) = current_tool_use {
                                json.push_str(&partial_json);
                            }
                        }
                        StreamEvent::ContentBlockStop { .. } => {
                            // Finalize current block
                            if !current_text.is_empty() {
                                response_content.push(
                                    rustyclawd_core::client::types::ContentBlock::Text {
                                        text: current_text.clone(),
                                    },
                                );
                                current_text.clear();
                            }

                            if let Some((id, name, json)) = current_tool_use.take() {
                                // Parse tool input
                                match serde_json::from_str(&json) {
                                    Ok(input) => {
                                        response_content.push(
                                            rustyclawd_core::client::types::ContentBlock::ToolUse {
                                                id,
                                                name,
                                                input,
                                            },
                                        );
                                    }
                                    Err(e) => {
                                        let _ = event_tx.send(StreamingChannelEvent::Error {
                                            message: format!(
                                                "Failed to parse tool input JSON: {}",
                                                e
                                            ),
                                        });
                                        return;
                                    }
                                }
                            }
                        }
                        StreamEvent::MessageDelta {
                            delta,
                            usage: usage_delta,
                        } => {
                            stop_reason = delta.stop_reason.clone();
                            usage = usage_delta.clone();

                            // Send updated token count
                            let _ = event_tx.send(StreamingChannelEvent::TokenUpdate {
                                input: usage.input_tokens,
                                output: usage.output_tokens,
                            });
                        }
                        StreamEvent::MessageStop => {
                            // Stream complete
                            break;
                        }
                        StreamEvent::Ping => {
                            // Keep-alive, ignore
                        }
                        StreamEvent::Error { error } => {
                            let _ = event_tx.send(StreamingChannelEvent::Error {
                                message: format!("API error: {}", error.message),
                            });
                            return;
                        }
                    },
                    Err(e) => {
                        let _ = event_tx.send(StreamingChannelEvent::Error {
                            message: format!("Stream error: {}", e),
                        });
                        return;
                    }
                }
            }

            // Build complete response
            let response = rustyclawd_core::client::MessageResponse {
                id: message_id,
                type_field: "message".to_string(),
                role: rustyclawd_core::client::Role::Assistant,
                content: response_content,
                model,
                stop_reason,
                stop_sequence: None,
                usage,
            };

            // Send complete response via oneshot channel
            let _ = response_tx.send(response.clone());

            // Send completion event via unbounded channel
            let _ = event_tx.send(StreamingChannelEvent::Complete { response });
        });

        self.tui
            .push_debug("[STREAM] Background task spawned, setting up TUI".to_string());

        // Begin streaming message in TUI
        let message_index = self.tui.begin_streaming_message();

        // Store channel receiver and message index for main event loop to poll
        self.streaming_rx = Some(event_rx);
        self.streaming_message_index = Some(message_index);

        self.tui
            .push_debug("[STREAM] Storing response receiver for polling".to_string());

        // Store response receiver for non-blocking polling in main event loop
        // DO NOT AWAIT HERE - this would block the main thread!
        self.response_rx = Some(response_rx);

        // Return immediately - response will be processed via polling
        // The main event loop will detect completion and continue tool use loop
        self.tui
            .push_debug("[STREAM] Background task spawned, returning immediately".to_string());

        // Return a placeholder - actual response processed via polling
        // This is a temporary hack until we refactor the return type
        Err(anyhow::anyhow!(
            "Response pending - will be processed via polling"
        ))
    }

    /// Spawn tools in background tasks (non-blocking)
    ///
    /// Tools execute in background tasks to keep UI responsive.
    /// Results are collected via channel events in the main event loop.
    /// This method returns IMMEDIATELY - tool completion detected by polling.
    fn spawn_tools(
        &mut self,
        tool_use_blocks: Vec<(String, String, serde_json::Value)>,
    ) -> Result<()> {
        if tool_use_blocks.is_empty() {
            return Ok(());
        }

        // Create channel for tool execution events
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

        // Store receiver for main loop polling
        self.tool_rx = Some(event_rx);

        // Clear any previous tool results and store expected IDs
        self.tool_results.clear();
        self.active_tools.clear();
        self.expected_tool_ids = tool_use_blocks
            .iter()
            .map(|(id, _, _)| id.clone())
            .collect();

        // Create tool messages FIRST (synchronously) to avoid race conditions
        for (id, name, input) in &tool_use_blocks {
            self.tui
                .begin_tool_message(id.clone(), name.clone(), input.clone());
            self.active_tools.insert(id.clone(), name.clone());
            self.stats.add_tool_call();
        }

        // Spawn background task for each tool
        for (id, name, input) in tool_use_blocks {
            // Clone data for background task
            let hooks = self.hooks.as_ref().map(Arc::clone);
            let session_id = Some(self.session_id.clone());
            let notification_manager = self.notification_manager.clone();
            let permission_mode = self.tui.permission_mode();
            let allowed_tools = self.allowed_tools.clone();
            let disallowed_tools = self.disallowed_tools.clone();
            let tx = event_tx.clone();

            // Spawn tool execution in background
            tokio::spawn(async move {
                // Execute the tool
                let result = tool_executor::execute_tool_with_permission(
                    name.clone(),
                    input,
                    permission_mode,
                    hooks,
                    session_id,
                    notification_manager.as_ref(),
                    Some(id.clone()),
                    allowed_tools,
                    disallowed_tools,
                )
                .await;

                // Send Complete or Error event
                match result {
                    Ok(output) => {
                        let _ = tx.send(ToolExecutionEvent::Complete {
                            tool_id: id.clone(),
                            result: rustyclawd_core::client::types::ContentBlock::ToolResult {
                                tool_use_id: id,
                                content: vec![rustyclawd_core::client::types::ContentBlock::Text {
                                    text: output.to_string(),
                                }],
                                is_error: None,
                            },
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(ToolExecutionEvent::Error {
                            tool_id: id.clone(),
                            error: e.to_string(),
                        });
                    }
                }
            });
        }

        // Drop sender so channel closes when all tools complete
        drop(event_tx);

        // Return immediately - tool completion detected via polling in main event loop
        // See lines 406-444 for completion detection and continuation
        Ok(())
    }

    /// Convert context messages to API message format
    fn convert_messages_to_api_format(&self) -> Vec<ApiMessage> {
        self.context
            .messages()
            .iter()
            .filter_map(|msg| {
                // API only accepts user and assistant roles, not system
                match msg.role {
                    MessageRole::User => Some(ApiMessage::user(msg.content.clone())),
                    MessageRole::Assistant => Some(ApiMessage::assistant(msg.content.clone())),
                    MessageRole::System => None, // Skip system messages
                }
            })
            .collect()
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

    /// Auto-save session on exit
    fn auto_save_session(&mut self) {
        if let Some(ref mut persistence) = self.persistence {
            let messages: Vec<Message> = self.context.messages().to_vec();
            if let Err(e) = persistence.auto_save(&messages) {
                eprintln!("Warning: Failed to auto-save session: {}", e);
            }
        }
    }

    /// Handle /save command
    fn handle_save_command(&mut self, input: &str) -> Result<()> {
        if let Some(ref mut persistence) = self.persistence {
            // Extract description from command
            let description = input.strip_prefix("/save").unwrap_or("").trim().to_string();

            let description = if description.is_empty() {
                "Manual save".to_string()
            } else {
                description
            };

            let messages: Vec<Message> = self.context.messages().to_vec();

            match persistence.save_checkpoint(&messages, description.clone()) {
                Ok(checkpoint_id) => {
                    self.tui.add_message(ChatMessage::system(format!(
                        "Checkpoint saved: {} ({})",
                        checkpoint_id, description
                    )));
                }
                Err(e) => {
                    self.tui.add_message(ChatMessage::system(format!(
                        "Failed to save checkpoint: {}",
                        e
                    )));
                }
            }
        } else {
            self.tui.add_message(ChatMessage::system(
                "Session persistence not available".to_string(),
            ));
        }

        Ok(())
    }

    /// Handle /load command
    fn handle_load_command(&mut self, input: &str) -> Result<()> {
        if let Some(ref mut persistence) = self.persistence {
            // Extract checkpoint ID from command
            let checkpoint_id = input.strip_prefix("/load").unwrap_or("").trim();

            if checkpoint_id.is_empty() {
                self.tui.add_message(ChatMessage::system(
                    "Usage: /load <checkpoint_id>\nUse /sessions to list available checkpoints"
                        .to_string(),
                ));
                return Ok(());
            }

            match persistence.load_checkpoint(checkpoint_id) {
                Ok(messages) => {
                    // Clear current context and TUI
                    self.context = Context::new();

                    // Restore messages
                    for msg in &messages {
                        self.context.add_message(msg.clone());

                        let chat_msg = match msg.role {
                            MessageRole::User => ChatMessage::user(msg.content.clone()),
                            MessageRole::Assistant => ChatMessage::assistant(msg.content.clone()),
                            MessageRole::System => ChatMessage::system(msg.content.clone()),
                        };

                        self.tui.add_message(chat_msg);
                    }

                    self.tui.add_message(ChatMessage::system(format!(
                        "Checkpoint loaded: {} ({} messages)",
                        checkpoint_id,
                        messages.len()
                    )));
                }
                Err(e) => {
                    self.tui.add_message(ChatMessage::system(format!(
                        "Failed to load checkpoint: {}",
                        e
                    )));
                }
            }
        } else {
            self.tui.add_message(ChatMessage::system(
                "Session persistence not available".to_string(),
            ));
        }

        Ok(())
    }

    /// Handle /cost command
    fn handle_cost_command(&mut self) {
        // Pricing as of 2025 (Claude Sonnet 4.5)
        const INPUT_COST_PER_MILLION: f64 = 3.0;
        const OUTPUT_COST_PER_MILLION: f64 = 15.0;

        let input_tokens = self.stats.input_tokens;
        let output_tokens = self.stats.output_tokens;
        let total_tokens = self.stats.total_tokens;

        let input_cost = (input_tokens as f64 / 1_000_000.0) * INPUT_COST_PER_MILLION;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * OUTPUT_COST_PER_MILLION;
        let total_cost = input_cost + output_cost;

        let cost_display = format!(
            "Token Usage & Cost Estimate:\n\n\
             Session Statistics:\n\
             - Input tokens:  {:>8}\n\
             - Output tokens: {:>8}\n\
             - Total tokens:  {:>8}\n\n\
             Estimated Cost (Claude Sonnet 4.5):\n\
             - Input:  ${:>7.4} ({} tokens @ ${}/M)\n\
             - Output: ${:>7.4} ({} tokens @ ${}/M)\n\
             - Total:  ${:>7.4}\n\n\
             Note: Costs are estimates based on current Anthropic pricing.",
            input_tokens,
            output_tokens,
            total_tokens,
            input_cost,
            input_tokens,
            INPUT_COST_PER_MILLION,
            output_cost,
            output_tokens,
            OUTPUT_COST_PER_MILLION,
            total_cost
        );

        self.tui.add_message(ChatMessage::system(cost_display));
    }

    /// Handle /context command
    fn handle_context_command(&mut self) {
        const MAX_TOKENS: u64 = 200_000; // Claude Sonnet 4.5 context window

        let used_tokens = self.stats.total_tokens;
        let percentage = ((used_tokens as f64 / MAX_TOKENS as f64) * 100.0) as u64;
        let percentage = percentage.min(100); // Cap at 100%

        // Visual bar (50 chars wide)
        let filled = (percentage / 2) as usize;
        let empty = 50 - filled;

        let context_display = format!(
            "Context Window Usage:\n\n\
             Used:      {:>7} tokens ({}%)\n\
             Available: {:>7} tokens\n\
             Maximum:   {:>7} tokens\n\n\
             Visual: [{}{}] {}%\n\n\
             Messages: {} ({} user, {} assistant)\n\
             Model: {}",
            used_tokens,
            percentage,
            MAX_TOKENS - used_tokens,
            MAX_TOKENS,
            "=".repeat(filled),
            " ".repeat(empty),
            percentage,
            self.stats.message_count,
            self.stats.user_message_count,
            self.stats.assistant_message_count,
            self.model
        );

        self.tui.add_message(ChatMessage::system(context_display));
    }

    /// Handle /sessions command
    fn handle_sessions_command(&mut self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            match persistence.list_checkpoints() {
                Ok(checkpoints) => {
                    if checkpoints.is_empty() {
                        self.tui.add_message(ChatMessage::system(
                            "No checkpoints found for current session".to_string(),
                        ));
                    } else {
                        let mut output =
                            format!("Available checkpoints ({}):\n", checkpoints.len());
                        for (idx, (description, info)) in checkpoints.iter().enumerate() {
                            output.push_str(&format!(
                                "  {}. {} - {} messages, {}\n",
                                idx + 1,
                                description,
                                info.message_count,
                                info.format_age()
                            ));
                        }
                        output.push_str("\nUse /load <checkpoint_id> to restore a checkpoint");

                        self.tui.add_message(ChatMessage::system(output));
                    }
                }
                Err(e) => {
                    self.tui.add_message(ChatMessage::system(format!(
                        "Failed to list checkpoints: {}",
                        e
                    )));
                }
            }
        } else {
            self.tui.add_message(ChatMessage::system(
                "Session persistence not available".to_string(),
            ));
        }

        Ok(())
    }

    /// Handle /usage command - Display real rate limit data
    fn handle_usage_command(&mut self) {
        let rl = &self.stats.rate_limits;

        let mut output = String::from("API Usage & Rate Limits:\n\n");

        // Check if we have any rate limit data
        if rl.last_updated.is_none() {
            output.push_str(
                "No rate limit data available yet.\n\
                 Rate limits are captured from API responses during conversation.\n\n\
                 Tip: Send a message to Claude to populate rate limit information.",
            );
        } else {
            // Requests per minute
            output.push_str("Rate Limits (Per Minute):\n");
            match (rl.requests_limit, rl.requests_remaining) {
                (Some(limit), Some(remaining)) => {
                    let used = limit.saturating_sub(remaining);
                    let percent = rl.requests_percentage().unwrap_or(0);
                    output.push_str(&format!(
                        "- Requests:  {:>6} / {:<6} used ({}%)\n",
                        used, limit, percent
                    ));
                    output.push_str(&format!("- Remaining: {:>6} requests\n", remaining));
                }
                _ => {
                    output.push_str("- Requests:  No data\n");
                }
            }

            // Tokens per day
            output.push_str("\nToken Limits (Per Day):\n");
            match (rl.tokens_limit, rl.tokens_remaining) {
                (Some(limit), Some(remaining)) => {
                    let used = limit.saturating_sub(remaining);
                    let percent = rl.tokens_percentage().unwrap_or(0);
                    output.push_str(&format!(
                        "- Tokens:    {:>10} / {:<10} used ({}%)\n",
                        used, limit, percent
                    ));
                    output.push_str(&format!("- Remaining: {:>10} tokens\n", remaining));
                }
                _ => {
                    output.push_str("- Tokens:    No data\n");
                }
            }

            // Visual progress bars
            output.push_str("\nVisual Progress:\n");
            if let Some(req_pct) = rl.requests_percentage() {
                let filled = (req_pct / 2) as usize;
                let empty = 50usize.saturating_sub(filled);
                output.push_str(&format!(
                    "Requests: [{}{}] {}%\n",
                    "=".repeat(filled),
                    " ".repeat(empty),
                    req_pct
                ));
            }
            if let Some(tok_pct) = rl.tokens_percentage() {
                let filled = (tok_pct / 2) as usize;
                let empty = 50usize.saturating_sub(filled);
                output.push_str(&format!(
                    "Tokens:   [{}{}] {}%\n",
                    "=".repeat(filled),
                    " ".repeat(empty),
                    tok_pct
                ));
            }

            // Last updated timestamp
            if let Some(updated) = rl.last_updated {
                output.push_str(&format!(
                    "\nLast updated: {}\n",
                    updated.format("%Y-%m-%d %H:%M:%S UTC")
                ));
            }
        }

        self.tui.add_message(ChatMessage::system(output));
    }

    /// Handle /bashes command - Display background shell information
    async fn handle_bashes_command(&mut self) -> Result<()> {
        use rustyclawd_tools::process_registry::global_registry;

        let registry = global_registry();
        let shell_ids = registry.list_ids().await;

        if shell_ids.is_empty() {
            self.tui.add_message(ChatMessage::system("Background Bash Shells:\n\n\
                          No background shells currently running.\n\n\
                          Tips:\n\
                          - Background shells are created using Bash tool with run_in_background: true\n\
                          - Use BashOutput tool to read shell output\n\
                          - Use KillShell tool to terminate shells"
                    .to_string(),));
            return Ok(());
        }

        let mut output = format!("Background Bash Shells ({}):\n\n", shell_ids.len());

        for shell_id in &shell_ids {
            // Get status for each shell
            match registry.get_status(shell_id).await {
                Ok(status) => {
                    let status_str = match status {
                        rustyclawd_tools::process_registry::ProcessStatus::Running => "Running",
                        rustyclawd_tools::process_registry::ProcessStatus::Completed(code) => {
                            if code == 0 {
                                "Completed (success)"
                            } else {
                                "Completed (error)"
                            }
                        }
                        rustyclawd_tools::process_registry::ProcessStatus::Failed(_) => "Failed",
                    };

                    output.push_str(&format!("  {} - {}\n", shell_id, status_str));
                }
                Err(_) => {
                    output.push_str(&format!("  {} - Status unknown\n", shell_id));
                }
            }
        }

        output.push_str(
            "\nCommands:\n\
             - Use BashOutput tool with bash_id to read output\n\
             - Use KillShell tool with shell_id to terminate\n\n\
             Example: Ask Claude to check output from a specific shell ID",
        );

        self.tui.add_message(ChatMessage::system(output));

        Ok(())
    }

    /// Format network errors with user-friendly messages and troubleshooting hints
    fn format_network_error(&self, error: &ClientError) -> String {
        match error {
            ClientError::Timeout(msg) => {
                format!(
                    "⏱️  Request timed out\n\
                    Details: {}\n\
                    Tip: Check your internet connection or try again later.",
                    msg
                )
            }
            ClientError::ConnectionError(msg) => {
                format!(
                    "🔌 Connection failed\n\
                    Details: {}\n\
                    Tip: Verify you can reach api.anthropic.com",
                    msg
                )
            }
            ClientError::DnsError(msg) => {
                format!(
                    "🌐 DNS resolution failed\n\
                    Details: {}\n\
                    Tip: Check your DNS settings or try a different network.",
                    msg
                )
            }
            ClientError::NetworkError(msg) => {
                format!(
                    "📡 Network error\n\
                    Details: {}\n\
                    Tip: Check your internet connection.",
                    msg
                )
            }
            _ => error.to_string(),
        }
    }
}

/// Entry point for interactive mode
pub async fn run_interactive() -> Result<()> {
    run_interactive_with_hooks(None).await
}

/// Entry point for interactive mode with optional hooks system
pub async fn run_interactive_with_hooks(hooks: Option<Arc<hooks::HooksSystem>>) -> Result<()> {
    run_interactive_with_config(hooks, vec![], vec![]).await
}

/// Entry point for interactive mode with full configuration
pub async fn run_interactive_with_config(
    hooks: Option<Arc<hooks::HooksSystem>>,
    allowed_tools: Vec<String>,
    disallowed_tools: Vec<String>,
) -> Result<()> {
    let mut session = InteractiveSession::with_hooks(hooks).await?;
    session.allowed_tools = allowed_tools;
    session.disallowed_tools = disallowed_tools;
    session.run().await
}
