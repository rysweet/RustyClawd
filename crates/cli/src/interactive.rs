//! Interactive chat mode (REPL) for RustyClawd
//!
//! Provides a fully functional REPL with:
//! - Rustyline for input handling with history
//! - Real-time streaming responses from Claude
//! - Multi-turn conversation context
//! - Graceful exit handling (Ctrl+D, /exit)
//! - Slash command autocomplete with fuzzy matching

use anyhow::Result;
use rustyclawd_core::{
    client::{Client, Config, CreateMessageRequest, Message as ApiMessage, StreamEvent},
    Context, Message, MessageRole,
};
use rustyclawd_tools::{bash::BashParams, BashTool, Tool, ToolContext, ToolEvent};
use futures::StreamExt;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context as RustylineContext, Editor, Helper, Result as RustylineResult};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Default model for interactive sessions
const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250929";

/// Maximum tokens for responses
const MAX_TOKENS: u32 = 4096;

/// Slash command completer with fuzzy matching support
#[derive(Clone)]
struct SlashCommandCompleter {
    /// Available slash commands (sorted for consistency)
    commands: BTreeSet<String>,
}

impl SlashCommandCompleter {
    /// Create a new completer by scanning the .claude/commands directory
    fn new() -> Self {
        let mut commands = BTreeSet::new();

        // Add built-in commands
        commands.insert("/exit".to_string());
        commands.insert("/quit".to_string());
        commands.insert("/clear".to_string());
        commands.insert("/help".to_string());
        commands.insert("/stats".to_string());

        // Scan for custom commands in .claude/commands/
        if let Ok(commands_dir) = Self::find_commands_directory() {
            if let Ok(entries) = fs::read_dir(&commands_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "md") {
                        if let Some(stem) = path.file_stem() {
                            if let Some(name) = stem.to_str() {
                                commands.insert(format!("/{}", name));
                            }
                        }
                    }
                }
            }
        }

        Self { commands }
    }

    /// Find the .claude/commands directory
    fn find_commands_directory() -> Result<PathBuf> {
        // Start from current directory and walk up
        let mut current = std::env::current_dir()?;

        loop {
            let candidate = current.join(".claude").join("commands");
            if candidate.exists() && candidate.is_dir() {
                return Ok(candidate);
            }

            // Move up one directory
            if !current.pop() {
                break;
            }
        }

        // Fallback to .claude/commands relative to cwd
        Ok(PathBuf::from(".claude/commands"))
    }

    /// Perform fuzzy matching on a command
    /// Returns a score (higher is better), or None if no match
    fn fuzzy_match(&self, pattern: &str, candidate: &str) -> Option<i32> {
        let pattern = pattern.to_lowercase();
        let candidate = candidate.to_lowercase();

        // Exact match gets highest score
        if candidate == pattern {
            return Some(1000);
        }

        // Prefix match gets high score
        if candidate.starts_with(&pattern) {
            return Some(500);
        }

        // Fuzzy match: all pattern chars must appear in order in candidate
        let mut score = 0;
        let mut candidate_chars = candidate.chars();

        for pattern_char in pattern.chars() {
            let mut found = false;
            for candidate_char in candidate_chars.by_ref() {
                if candidate_char == pattern_char {
                    found = true;
                    score += 10;
                    break;
                }
                score -= 1; // Penalty for skipped chars
            }

            if !found {
                return None; // Pattern char not found
            }
        }

        Some(score)
    }

    /// Get completions for a given input
    fn get_completions(&self, line: &str) -> Vec<Pair> {
        // Only complete if line starts with /
        if !line.starts_with('/') {
            return vec![];
        }

        let query = line;

        // Score all commands and filter matches
        let mut matches: Vec<(i32, String)> = self.commands
            .iter()
            .filter_map(|cmd| {
                self.fuzzy_match(query, cmd)
                    .map(|score| (score, cmd.clone()))
            })
            .collect();

        // Sort by score (descending)
        matches.sort_by(|a, b| b.0.cmp(&a.0));

        // Convert to Pair format for rustyline
        matches
            .into_iter()
            .map(|(_, cmd)| Pair {
                display: cmd.clone(),
                replacement: cmd,
            })
            .collect()
    }
}

impl Completer for SlashCommandCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &RustylineContext<'_>,
    ) -> RustylineResult<(usize, Vec<Pair>)> {
        // Only complete up to cursor position
        let line = &line[..pos];

        // Find the start of the current word
        let start = line.rfind(char::is_whitespace)
            .map(|i| i + 1)
            .unwrap_or(0);

        let word = &line[start..];

        // Get completions
        let completions = self.get_completions(word);

        Ok((start, completions))
    }
}

impl Hinter for SlashCommandCompleter {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &RustylineContext<'_>) -> Option<String> {
        // Only hint if cursor is at end
        if pos < line.len() {
            return None;
        }

        // Only hint for slash commands
        if !line.starts_with('/') {
            return None;
        }

        // Find best match
        let completions = self.get_completions(line);
        if let Some(first) = completions.first() {
            let hint = &first.replacement[line.len()..];
            if !hint.is_empty() {
                return Some(hint.to_string());
            }
        }

        None
    }
}

impl Highlighter for SlashCommandCompleter {}

impl Validator for SlashCommandCompleter {}

impl Helper for SlashCommandCompleter {}

/// Interactive chat session
pub struct InteractiveSession {
    /// Anthropic API client
    client: Client,
    /// Conversation context
    context: Context,
    /// Rustyline editor for input with slash command completion
    editor: Editor<SlashCommandCompleter, rustyline::history::DefaultHistory>,
    /// Model to use
    model: String,
}

impl InteractiveSession {
    /// Create a new interactive session
    pub async fn new() -> Result<Self> {
        // Load API configuration from default location
        let config = Config::from_default_location().await?;
        let client = Client::new(config);

        // Initialize rustyline editor with slash command completion
        let completer = SlashCommandCompleter::new();
        let mut editor = Editor::new()?;
        editor.set_helper(Some(completer));

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
        println!("Shell: !<command> (e.g., !ls, !git status)");
        println!("Press Ctrl+D or type /exit to quit");
        println!("Tip: Type / and press Tab for command autocomplete");
        println!();
    }

    /// Read input from user with rustyline
    fn read_input(&mut self) -> RustylineResult<String> {
        self.editor.readline("You> ")
    }

    /// Handle special commands
    /// Returns true if command was handled, false if input should be processed as message
    async fn handle_command(&mut self, input: &str) -> Result<bool> {
        // Handle "!" prefix for direct shell execution
        if input.starts_with('!') {
            let command = input[1..].trim();
            if command.is_empty() {
                eprintln!("Error: No command specified after '!'\n");
                return Ok(true);
            }

            self.execute_shell_command(command).await?;
            return Ok(true);
        }

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

    /// Execute a shell command directly and add to context
    async fn execute_shell_command(&mut self, command: &str) -> Result<()> {
        println!("\n$ {}\n", command);

        // Create tool context
        let ctx = ToolContext {
            cwd: std::env::current_dir().unwrap_or_default(),
            debug: false,
            metadata: serde_json::Value::Null,
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
                    // Display stdout
                    if let Some(ref stdout) = output.stdout {
                        if !stdout.is_empty() {
                            print!("{}", stdout);
                            stdout_output = stdout.clone();
                        }
                    }

                    // Display stderr
                    if let Some(ref stderr) = output.stderr {
                        if !stderr.is_empty() {
                            eprint!("{}", stderr);
                            stderr_output = stderr.clone();
                        }
                    }

                    exit_code = output.exit_code;
                    success = output.success;
                }
                ToolEvent::Error { message } => {
                    eprintln!("Error: {}", message);
                    return Err(anyhow::anyhow!("Command execution failed: {}", message));
                }
            }
        }

        println!();

        // Format output for context
        let mut result_msg = format!("Executed command: `{}`\n", command);

        if !stdout_output.is_empty() {
            result_msg.push_str(&format!("\nStdout:\n```\n{}\n```", stdout_output.trim()));
        }

        if !stderr_output.is_empty() {
            result_msg.push_str(&format!("\nStderr:\n```\n{}\n```", stderr_output.trim()));
        }

        if let Some(code) = exit_code {
            result_msg.push_str(&format!("\nExit code: {}", code));
        }

        // Add to context as a user message (tool use result)
        self.context.add_message(Message::user(result_msg));

        // Show status
        if success {
            println!("✓ Command completed successfully\n");
        } else {
            println!("✗ Command failed with exit code: {}\n", exit_code.unwrap_or(-1));
        }

        Ok(())
    }

    /// Print help message
    fn print_help(&self) {
        println!("\n📖 Available Commands:");
        println!("  /exit, /quit  - Exit the chat session");
        println!("  /clear        - Clear conversation history");
        println!("  /stats        - Show session statistics");
        println!("  /help         - Show this help message");
        println!("  !<command>    - Execute shell command directly");
        println!();
        println!("💡 Tips:");
        println!("  - Press Ctrl+D to exit");
        println!("  - Press Ctrl+C to cancel current input");
        println!("  - Use !ls, !git status, etc. for direct shell execution");
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
