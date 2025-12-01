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
use crate::mcp_commands;
use crate::plugins::mcp_proxy::McpProxy;
use crate::session::SessionStats;
use crate::session_persistence::{SessionInfo, SessionPersistence};
use crate::terminal_guard;
use crate::tool_formatter;
use crate::tui::{ChatMessage, MessageRole as TuiMessageRole, TuiState};
use anyhow::Result;
use futures::StreamExt;
use rustyclawd_core::{
    client::{Client, ClientError, Config, CreateMessageRequest, Message as ApiMessage, StreamEvent},
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
}

impl InteractiveSession {
    /// Create a new interactive session
    pub async fn new() -> Result<Self> {
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
            commands_for_completion.get_completions(prefix)
        }));

        // Initialize session persistence
        let persistence = SessionPersistence::with_default_id().ok();

        // Initialize MCP proxy (empty for now, will be populated by App)
        let mcp_proxy = Arc::new(Mutex::new(McpProxy::new()));

        Ok(Self {
            client,
            context: Context::new(),
            tui,
            model: DEFAULT_MODEL.to_string(),
            slash_commands,
            persistence,
            mcp_proxy,
            stats: SessionStats::new(DEFAULT_MODEL),
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
                                let role = match msg.role {
                                    MessageRole::User => TuiMessageRole::User,
                                    MessageRole::Assistant => TuiMessageRole::Assistant,
                                    MessageRole::System => TuiMessageRole::System,
                                };
                                self.tui.add_message(ChatMessage {
                                    role,
                                    content: msg.content,
                                });
                            }

                            self.tui.add_message(ChatMessage {
                                role: TuiMessageRole::System,
                                content: format!(
                                    "Session resumed ({} messages, {})",
                                    session_info.message_count,
                                    session_info.format_age()
                                ),
                            });
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
                commands_for_completion.get_completions(prefix)
            }));
        }

        loop {
            // Draw UI
            self.tui.draw()?;

            // Handle input (Ctrl+C is handled by TuiState::handle_key_event)
            if let Some(input) = self.tui.handle_input()? {
                let input = input.trim();

                // Handle empty input
                if input.is_empty() {
                    continue;
                }

                // Handle special commands
                if self.handle_command(input).await? {
                    continue;
                }

                // Process user message and get Claude's response
                if let Err(e) = self.process_user_message(input).await {
                    self.tui.add_message(ChatMessage {
                        role: TuiMessageRole::System,
                        content: format!("Error: {}", e),
                    });
                    self.tui.set_status(format!("Error: {}", e));
                }
            }
        }
    }

    /// Handle special commands
    /// Returns true if command was handled, false if input should be processed as message
    async fn handle_command(&mut self, input: &str) -> Result<bool> {
        // Handle "!" prefix for direct shell execution
        if let Some(stripped) = input.strip_prefix('!') {
            let command = stripped.trim();
            if command.is_empty() {
                self.tui.add_message(ChatMessage {
                    role: TuiMessageRole::System,
                    content: "Error: No command specified after '!'".to_string(),
                });
                return Ok(true);
            }

            self.execute_shell_command(command).await?;
            return Ok(true);
        }

        match input {
            "/exit" | "/quit" => {
                // Auto-save before exit
                self.auto_save_session();

                self.tui.cleanup()?;
                println!("\nGoodbye, matey! Fair winds and following seas! ⛵");
                std::process::exit(0);
            }
            "/clear" => {
                self.context = Context::new();
                self.tui.add_message(ChatMessage {
                    role: TuiMessageRole::System,
                    content: "Conversation history cleared".to_string(),
                });
                self.tui.set_status("Conversation cleared".to_string());
                return Ok(true);
            }
            "/help" => {
                // Show all commands - built-in and custom
                let custom_commands = self.slash_commands.list_commands();
                let mut help_text = "Built-in Commands:\n  /exit, /quit - Exit the session\n  /clear - Clear conversation history\n  /help - Show this help\n  /stats - Show session statistics\n  /save [description] - Save checkpoint\n  /load <checkpoint_id> - Load checkpoint\n  /sessions - List available sessions\n  !<command> - Execute shell command directly\n\nMCP Commands:\n  /mcp-list - List all MCP servers\n  /mcp-start <server-id> - Start an MCP server\n  /mcp-stop <server-id> - Stop an MCP server\n  /mcp-tools <server-id> - List tools from server\n  /mcp-status <server-id> - Show server status\n".to_string();

                if !custom_commands.is_empty() {
                    help_text.push_str("\nCustom Commands:\n");
                    for cmd in custom_commands {
                        help_text.push_str(&format!("  /{}\n", cmd));
                    }
                }

                help_text.push_str("\nPress Ctrl+C or Ctrl+D to exit.");

                self.tui.add_message(ChatMessage {
                    role: TuiMessageRole::System,
                    content: help_text,
                });
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
                self.tui.add_message(ChatMessage {
                    role: TuiMessageRole::System,
                    content: stats,
                });
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
                            self.tui.add_message(ChatMessage {
                                role: TuiMessageRole::System,
                                content: output,
                            });
                            self.tui.set_status("Ready".to_string());
                        }
                        Err(e) => {
                            self.tui.add_message(ChatMessage {
                                role: TuiMessageRole::System,
                                content: format!("Error: {}", e),
                            });
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
                    // Execute custom command
                    self.tui.set_status(format!("Executing command: {}", input));

                    match self.slash_commands.execute(input).await {
                        Ok(result) => {
                            // Add expanded prompt as user message
                            self.tui.add_message(ChatMessage {
                                role: TuiMessageRole::User,
                                content: format!("{}\n\n[Command expanded to:]", input),
                            });

                            self.tui.add_message(ChatMessage {
                                role: TuiMessageRole::System,
                                content: result.expanded_prompt.clone(),
                            });

                            // Add to conversation context
                            self.context
                                .add_message(Message::user(result.expanded_prompt.clone()));

                            // Process the expanded prompt as if user typed it
                            if let Err(e) = self.process_user_message(&result.expanded_prompt).await
                            {
                                self.tui.add_message(ChatMessage {
                                    role: TuiMessageRole::System,
                                    content: format!("Error processing command: {}", e),
                                });
                            }
                        }
                        Err(e) => {
                            self.tui.add_message(ChatMessage {
                                role: TuiMessageRole::System,
                                content: format!("Error executing command: {}", e),
                            });
                        }
                    }
                    return Ok(true);
                }

                // Unknown command
                self.tui.add_message(ChatMessage {
                    role: TuiMessageRole::System,
                    content: format!(
                        "Unknown command: {}\nType /help for available commands",
                        input
                    ),
                });
                return Ok(true);
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
                    self.tui.add_message(ChatMessage {
                        role: TuiMessageRole::System,
                        content: format!("Error: {}", message),
                    });
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
        self.tui.add_message(ChatMessage {
            role: TuiMessageRole::System,
            content: result_msg.clone(),
        });

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
    async fn process_user_message(&mut self, user_input: &str) -> Result<()> {
        // Add user message to TUI and context
        self.tui.add_message(ChatMessage {
            role: TuiMessageRole::User,
            content: user_input.to_string(),
        });
        self.context
            .add_message(Message::user(user_input.to_string()));

        // Stream response with tool use loop
        self.stream_with_tools().await?;

        Ok(())
    }

    /// Manages the tool use loop with streaming
    async fn stream_with_tools(&mut self) -> Result<()> {
        // High limit for complex agentic workflows
        const MAX_ITERATIONS: usize = 10_000;
        let mut iteration = 0;

        // Track API-level messages for tool use loop (separate from context)
        let mut api_messages = self.convert_messages_to_api_format();

        loop {
            iteration += 1;
            if iteration > MAX_ITERATIONS {
                return Err(anyhow::anyhow!(
                    "Tool execution exceeded maximum iterations"
                ));
            }

            // Update status
            self.tui.set_status("Streaming...".to_string());

            // Stream a single turn
            let response = self.stream_single_turn_with_messages(&api_messages).await?;

            // Check if response contains tool use
            let mut tool_use_blocks = Vec::new();
            for block in &response.content {
                if let rustyclawd_core::client::types::ContentBlock::ToolUse { id, name, input } =
                    block
                {
                    tool_use_blocks.push((id.clone(), name.clone(), input.clone()));
                }
            }

            // If no tool use, we're done
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
                    self.context.add_message(Message::assistant(response_text));
                }

                return Ok(());
            }

            // Execute tools and get results
            let tool_result_blocks = self.execute_tools(tool_use_blocks).await?;

            // Add assistant's response with tool_use blocks to API messages
            api_messages.push(ApiMessage::with_blocks(
                rustyclawd_core::client::Role::Assistant,
                response.content,
            ));

            // Add tool results as user message to API messages
            api_messages.push(ApiMessage::with_blocks(
                rustyclawd_core::client::Role::User,
                tool_result_blocks,
            ));
        }
    }

    /// Streams a single turn and returns the complete response
    async fn stream_single_turn_with_messages(
        &mut self,
        api_messages: &[ApiMessage],
    ) -> Result<rustyclawd_core::client::MessageResponse> {
        // Get tool definitions
        let tools = crate::tool_definitions::get_all_tool_definitions();

        // Create API request with tools and streaming enabled
        let request =
            CreateMessageRequest::new(self.model.clone(), api_messages.to_vec(), MAX_TOKENS)
                .with_tools(tools)
                .with_temperature(1.0)
                .with_stream(true);

        // Make HTTP request directly to capture rate limit headers
        let url = format!("{}/v1/messages", self.client.api_url());
        let http_response = match self
            .client
            .http_client()
            .post(&url)
            .header(
                "x-api-key",
                self.client.config().api_key.expose_secret().expose(),
            )
            .header("anthropic-version", self.client.api_version())
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&request)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                // Convert reqwest error to ClientError for user-friendly messages
                let client_error = ClientError::from(e);
                return Err(anyhow::anyhow!("{}", self.format_network_error(&client_error)));
            }
        };

        // Extract rate limit headers before consuming response
        let headers = http_response.headers();
        self.stats.rate_limits.update_from_headers(headers);

        // Check for HTTP errors
        if !http_response.status().is_success() {
            let status = http_response.status();
            let error_text = http_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!("HTTP {}: {}", status, error_text));
        }

        // Convert response body into event stream
        use rustyclawd_core::client::EventStream;
        let byte_stream = http_response.bytes_stream();
        let mut stream = EventStream::new(byte_stream);

        // Begin streaming message in TUI
        let message_index = self.tui.begin_streaming_message();

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

        // Process stream events
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => match event {
                    StreamEvent::MessageStart { message } => {
                        message_id = message.id;
                        usage = message.usage;
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
                        // Append text to TUI in real-time
                        self.tui.append_to_message(message_index, &text);
                        current_text.push_str(&text);

                        // Draw UI to show updates
                        self.tui.draw()?;
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
                            let input: serde_json::Value = serde_json::from_str(&json)?;
                            response_content.push(
                                rustyclawd_core::client::types::ContentBlock::ToolUse {
                                    id,
                                    name,
                                    input,
                                },
                            );
                        }
                    }
                    StreamEvent::MessageDelta {
                        delta,
                        usage: usage_delta,
                    } => {
                        stop_reason = delta.stop_reason;
                        usage = usage_delta;
                    }
                    StreamEvent::MessageStop => {
                        // Stream complete
                        break;
                    }
                    StreamEvent::Ping => {
                        // Keep-alive, ignore
                    }
                    StreamEvent::Error { error } => {
                        return Err(anyhow::anyhow!("Stream error: {}", error.message));
                    }
                },
                Err(e) => {
                    return Err(anyhow::anyhow!("Stream error: {}", e));
                }
            }
        }

        // Finalize streaming message in TUI
        self.tui.finalize_streaming_message(message_index);

        // Build complete response
        let response = rustyclawd_core::client::MessageResponse {
            id: message_id,
            type_field: "message".to_string(),
            role: rustyclawd_core::client::Role::Assistant,
            content: response_content,
            model: self.model.clone(),
            stop_reason,
            stop_sequence: None,
            usage,
        };

        // Track token usage in session stats
        self.stats.add_assistant_message(
            response.usage.input_tokens as u64,
            response.usage.output_tokens as u64,
        );

        Ok(response)
    }

    /// Execute tools and return result blocks
    async fn execute_tools(
        &mut self,
        tool_use_blocks: Vec<(String, String, serde_json::Value)>,
    ) -> Result<Vec<rustyclawd_core::client::types::ContentBlock>> {
        let mut tool_result_blocks = Vec::new();

        for (id, name, input) in tool_use_blocks {
            // Show formatted tool call with icon
            let tool_call_msg = tool_formatter::format_tool_call(&name, &input);
            self.tui.set_status(format!("Executing: {}", name));
            self.tui.add_message(ChatMessage {
                role: TuiMessageRole::System,
                content: tool_call_msg,
            });

            // Show formatted parameters if interesting
            let params_msg = tool_formatter::format_tool_params(&name, &input);
            if !params_msg.is_empty() && params_msg != "Processing..." {
                self.tui.add_message(ChatMessage {
                    role: TuiMessageRole::System,
                    content: format!("  {}", params_msg),
                });
            }

            // Track tool call
            self.stats.add_tool_call();

            // Execute the tool
            match crate::tool_executor::execute_tool(name.clone(), input.clone()).await {
                Ok(result) => {
                    // Show formatted success message
                    let success_msg = tool_formatter::format_tool_success(&name, &result);
                    self.tui.add_message(ChatMessage {
                        role: TuiMessageRole::System,
                        content: format!("  {}", success_msg),
                    });

                    tool_result_blocks.push(
                        rustyclawd_core::client::types::ContentBlock::ToolResult {
                            tool_use_id: id,
                            content: result.to_string(),
                            is_error: None,
                        },
                    );
                }
                Err(e) => {
                    // Show formatted error message
                    let error_msg = tool_formatter::format_tool_error(&name, &e.to_string());
                    self.tui.add_message(ChatMessage {
                        role: TuiMessageRole::System,
                        content: format!("  {}", error_msg),
                    });

                    tool_result_blocks.push(
                        rustyclawd_core::client::types::ContentBlock::ToolResult {
                            tool_use_id: id,
                            content: format!("Tool execution error: {}", e),
                            is_error: Some(true),
                        },
                    );
                }
            }
        }

        Ok(tool_result_blocks)
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
                    self.tui.add_message(ChatMessage {
                        role: TuiMessageRole::System,
                        content: format!("Checkpoint saved: {} ({})", checkpoint_id, description),
                    });
                }
                Err(e) => {
                    self.tui.add_message(ChatMessage {
                        role: TuiMessageRole::System,
                        content: format!("Failed to save checkpoint: {}", e),
                    });
                }
            }
        } else {
            self.tui.add_message(ChatMessage {
                role: TuiMessageRole::System,
                content: "Session persistence not available".to_string(),
            });
        }

        Ok(())
    }

    /// Handle /load command
    fn handle_load_command(&mut self, input: &str) -> Result<()> {
        if let Some(ref mut persistence) = self.persistence {
            // Extract checkpoint ID from command
            let checkpoint_id = input.strip_prefix("/load").unwrap_or("").trim();

            if checkpoint_id.is_empty() {
                self.tui.add_message(ChatMessage {
                    role: TuiMessageRole::System,
                    content:
                        "Usage: /load <checkpoint_id>\nUse /sessions to list available checkpoints"
                            .to_string(),
                });
                return Ok(());
            }

            match persistence.load_checkpoint(checkpoint_id) {
                Ok(messages) => {
                    // Clear current context and TUI
                    self.context = Context::new();

                    // Restore messages
                    for msg in &messages {
                        self.context.add_message(msg.clone());

                        let role = match msg.role {
                            MessageRole::User => TuiMessageRole::User,
                            MessageRole::Assistant => TuiMessageRole::Assistant,
                            MessageRole::System => TuiMessageRole::System,
                        };

                        self.tui.add_message(ChatMessage {
                            role,
                            content: msg.content.clone(),
                        });
                    }

                    self.tui.add_message(ChatMessage {
                        role: TuiMessageRole::System,
                        content: format!(
                            "Checkpoint loaded: {} ({} messages)",
                            checkpoint_id,
                            messages.len()
                        ),
                    });
                }
                Err(e) => {
                    self.tui.add_message(ChatMessage {
                        role: TuiMessageRole::System,
                        content: format!("Failed to load checkpoint: {}", e),
                    });
                }
            }
        } else {
            self.tui.add_message(ChatMessage {
                role: TuiMessageRole::System,
                content: "Session persistence not available".to_string(),
            });
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

        self.tui.add_message(ChatMessage {
            role: TuiMessageRole::System,
            content: cost_display,
        });
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

        self.tui.add_message(ChatMessage {
            role: TuiMessageRole::System,
            content: context_display,
        });
    }

    /// Handle /sessions command
    fn handle_sessions_command(&mut self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            match persistence.list_checkpoints() {
                Ok(checkpoints) => {
                    if checkpoints.is_empty() {
                        self.tui.add_message(ChatMessage {
                            role: TuiMessageRole::System,
                            content: "No checkpoints found for current session".to_string(),
                        });
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

                        self.tui.add_message(ChatMessage {
                            role: TuiMessageRole::System,
                            content: output,
                        });
                    }
                }
                Err(e) => {
                    self.tui.add_message(ChatMessage {
                        role: TuiMessageRole::System,
                        content: format!("Failed to list checkpoints: {}", e),
                    });
                }
            }
        } else {
            self.tui.add_message(ChatMessage {
                role: TuiMessageRole::System,
                content: "Session persistence not available".to_string(),
            });
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

        self.tui.add_message(ChatMessage {
            role: TuiMessageRole::System,
            content: output,
        });
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

    /// Handle /bashes command - Display background shell information
    async fn handle_bashes_command(&mut self) -> Result<()> {
        use rustyclawd_tools::process_registry::global_registry;

        let registry = global_registry();
        let shell_ids = registry.list_ids().await;

        if shell_ids.is_empty() {
            self.tui.add_message(ChatMessage {
                role: TuiMessageRole::System,
                content: "Background Bash Shells:\n\n\
                          No background shells currently running.\n\n\
                          Tips:\n\
                          - Background shells are created using Bash tool with run_in_background: true\n\
                          - Use BashOutput tool to read shell output\n\
                          - Use KillShell tool to terminate shells"
                    .to_string(),
            });
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

        self.tui.add_message(ChatMessage {
            role: TuiMessageRole::System,
            content: output,
        });

        Ok(())
    }
}

/// Entry point for interactive mode
pub async fn run_interactive() -> Result<()> {
    let mut session = InteractiveSession::new().await?;
    session.run().await
}
