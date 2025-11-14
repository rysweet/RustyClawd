//! Interactive chat mode (REPL) for RustyClawd
//!
//! Provides a fully functional REPL with:
//! - Ratatui TUI with pirate ship banner
//! - Real-time streaming responses from Claude
//! - Multi-turn conversation context
//! - Graceful exit handling (Ctrl+D, /exit)
//! - Rust-colored theme

use crate::commands::SlashCommands;
use crate::terminal_guard;
use crate::tui::{ChatMessage, MessageRole as TuiMessageRole, TuiState};
use anyhow::Result;
use futures::StreamExt;
use rustyclawd_core::{
    client::{Client, Config, CreateMessageRequest, Message as ApiMessage, StreamEvent},
    Context, Message, MessageRole,
};
use rustyclawd_tools::{
    bash::BashParams, BashTool, ExecutionContext, Tool, ToolContext, ToolEvent,
};

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
    slash_commands: SlashCommands,
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
        let tui = TuiState::new()?;

        // Initialize slash command system
        let slash_commands = SlashCommands::new().await?;

        Ok(Self {
            client,
            context: Context::new(),
            tui,
            model: DEFAULT_MODEL.to_string(),
            slash_commands,
        })
    }

    /// Run the REPL loop
    pub async fn run(&mut self) -> Result<()> {
        // TUI shows welcome banner automatically

        loop {
            // Draw UI
            self.tui.draw()?;

            // Handle input
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
                let mut help_text = "Built-in Commands:\n  /exit, /quit - Exit the session\n  /clear - Clear conversation history\n  /help - Show this help\n  /stats - Show session statistics\n  !<command> - Execute shell command directly\n".to_string();

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
                let stats = format!(
                    "Messages: {}\nMemory usage: {} bytes\nModel: {}",
                    self.context.message_count(),
                    self.context.memory_usage(),
                    self.model
                );
                self.tui.add_message(ChatMessage {
                    role: TuiMessageRole::System,
                    content: stats,
                });
                return Ok(true);
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

    /// Process a user message and execute with tool support
    async fn process_user_message(&mut self, user_input: &str) -> Result<()> {
        // Add user message to TUI and context
        self.tui.add_message(ChatMessage {
            role: TuiMessageRole::User,
            content: user_input.to_string(),
        });
        self.context
            .add_message(Message::user(user_input.to_string()));

        // Update status
        self.tui.set_status("Claude is thinking...".to_string());

        // Convert context messages to API format
        let api_messages = self.convert_messages_to_api_format();

        // Get tool definitions
        let tools = crate::tool_definitions::get_all_tool_definitions();

        // Create API request with tools
        let request = CreateMessageRequest::new(self.model.clone(), api_messages, MAX_TOKENS)
            .with_tools(tools)
            .with_temperature(1.0); // Default temperature for Claude

        // Execute with tools - this handles the tool use loop automatically
        self.tui.set_status("Processing with tools...".to_string());

        let response = self
            .client
            .execute_with_tools(request, |tool_name, tool_input| async move {
                crate::tool_executor::execute_tool(tool_name, tool_input).await
            })
            .await?;

        // Extract text from response
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

        // Add complete assistant response to TUI and context
        if !response_text.is_empty() {
            self.tui.add_message(ChatMessage {
                role: TuiMessageRole::Assistant,
                content: response_text.clone(),
            });
            self.context.add_message(Message::assistant(response_text));
        }

        // Update status
        self.tui.set_status("Ready".to_string());

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
}

/// Entry point for interactive mode
pub async fn run_interactive() -> Result<()> {
    let mut session = InteractiveSession::new().await?;
    session.run().await
}
