//! Interactive chat mode (REPL) for RustyClawd
//!
//! Provides a fully functional REPL with:
//! - Rustyline for input handling with history
//! - Real-time streaming responses from Claude
//! - Multi-turn conversation context
//! - Graceful exit handling (Ctrl+D, /exit)

use anyhow::Result;
use rustyclawd_core::{
    client::{Client, Config, CreateMessageRequest, Message as ApiMessage, StreamEvent},
    Context, Message, MessageRole,
};
use futures::StreamExt;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result as RustylineResult};
use std::io::{self, Write};

/// Default model for interactive sessions
const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250929";

/// Maximum tokens for responses
const MAX_TOKENS: u32 = 4096;

/// Interactive chat session
pub struct InteractiveSession {
    /// Anthropic API client
    client: Client,
    /// Conversation context
    context: Context,
    /// Rustyline editor for input
    editor: DefaultEditor,
    /// Model to use
    model: String,
}

impl InteractiveSession {
    /// Create a new interactive session
    pub async fn new() -> Result<Self> {
        // Load API configuration from default location
        let config = Config::from_default_location().await?;
        let client = Client::new(config);

        // Initialize rustyline editor
        let editor = DefaultEditor::new()?;

        Ok(Self {
            client,
            context: Context::new(),
            editor,
            model: DEFAULT_MODEL.to_string(),
        })
    }

    /// Run the REPL loop
    pub async fn run(&mut self) -> Result<()> {
        self.print_welcome();

        loop {
            // Read user input
            match self.read_input() {
                Ok(line) => {
                    let line = line.trim();

                    // Handle empty input
                    if line.is_empty() {
                        continue;
                    }

                    // Handle special commands
                    if self.handle_command(line).await? {
                        continue;
                    }

                    // Process user message and get Claude's response
                    if let Err(e) = self.process_user_message(line).await {
                        eprintln!("\n❌ Error: {}", e);
                        eprintln!("Please try again or type /exit to quit.\n");
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl+C - cancel current input
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl+D - exit gracefully
                    println!("\nGoodbye!");
                    break;
                }
                Err(e) => {
                    eprintln!("Error reading input: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Print welcome message
    fn print_welcome(&self) {
        println!("╔═══════════════════════════════════════════════╗");
        println!("║         RustyClawd Interactive Mode           ║");
        println!("║       Chat with Claude - Rust Edition         ║");
        println!("╚═══════════════════════════════════════════════╝");
        println!();
        println!("Model: {}", self.model);
        println!("Commands: /exit, /clear, /help");
        println!("Press Ctrl+D or type /exit to quit");
        println!();
    }

    /// Read input from user with rustyline
    fn read_input(&mut self) -> RustylineResult<String> {
        self.editor.readline("You> ")
    }

    /// Handle special commands
    /// Returns true if command was handled, false if input should be processed as message
    async fn handle_command(&mut self, input: &str) -> Result<bool> {
        match input {
            "/exit" | "/quit" => {
                println!("Goodbye!");
                std::process::exit(0);
            }
            "/clear" => {
                self.context = Context::new();
                println!("✓ Conversation history cleared\n");
                return Ok(true);
            }
            "/help" => {
                self.print_help();
                return Ok(true);
            }
            "/stats" => {
                self.print_stats();
                return Ok(true);
            }
            _ if input.starts_with('/') => {
                eprintln!("Unknown command: {}", input);
                eprintln!("Type /help for available commands\n");
                return Ok(true);
            }
            _ => {}
        }

        Ok(false)
    }

    /// Print help message
    fn print_help(&self) {
        println!("\n📖 Available Commands:");
        println!("  /exit, /quit  - Exit the chat session");
        println!("  /clear        - Clear conversation history");
        println!("  /stats        - Show session statistics");
        println!("  /help         - Show this help message");
        println!();
        println!("💡 Tips:");
        println!("  - Press Ctrl+D to exit");
        println!("  - Press Ctrl+C to cancel current input");
        println!("  - Multi-line input is not supported yet");
        println!();
    }

    /// Print session statistics
    fn print_stats(&self) {
        println!("\n📊 Session Statistics:");
        println!("  Messages: {}", self.context.message_count());
        println!("  Memory usage: {} bytes", self.context.memory_usage());
        println!("  Model: {}", self.model);
        println!();
    }

    /// Process a user message and stream Claude's response
    async fn process_user_message(&mut self, user_input: &str) -> Result<()> {
        // Add user message to history
        self.editor.add_history_entry(user_input)?;
        self.context.add_message(Message::user(user_input.to_string()));

        // Convert context messages to API format
        let api_messages = self.convert_messages_to_api_format();

        // Create API request
        let request = CreateMessageRequest::new(self.model.clone(), api_messages, MAX_TOKENS)
            .with_stream(true);

        // Call API and stream response
        print!("\nClaude> ");
        io::stdout().flush()?;

        let mut response_text = String::new();

        let mut stream = self.client.create_message_stream(request).await?;

        while let Some(event) = stream.next().await {
            match event {
                Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                    // Extract text from delta
                    let text = match delta {
                        rustyclawd_core::client::types::ContentDelta::TextDelta { text } => text,
                    };

                    // Print to terminal in real-time
                    print!("{}", text);
                    io::stdout().flush()?;

                    // Accumulate for history
                    response_text.push_str(&text);
                }
                Ok(StreamEvent::MessageStop) => {
                    // End of response
                    println!("\n");
                    break;
                }
                Ok(StreamEvent::Error { error }) => {
                    return Err(anyhow::anyhow!("API error: {}", error.message));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Stream error: {}", e));
                }
                _ => {
                    // Ignore other event types (MessageStart, ContentBlockStart, etc.)
                }
            }
        }

        // Add assistant response to context
        if !response_text.is_empty() {
            self.context
                .add_message(Message::assistant(response_text));
        }

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
                    MessageRole::User => {
                        Some(ApiMessage::user(msg.content.clone()))
                    }
                    MessageRole::Assistant => {
                        Some(ApiMessage::assistant(msg.content.clone()))
                    }
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
