//! Claude Code CLI - Rust Translation
//!
//! An educational Rust translation of Claude Code's tool system.
//! Demonstrates Rust patterns for async CLI tools with streaming.

use anyhow::Result;
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

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .compact()
        .init();

    // Create tool context
    let ctx = ToolContext {
        debug: cli.debug,
        ..Default::default()
    };

    // Execute command
    match cli.command {
        Commands::Bash {
            command,
            timeout,
            description,
        } => {
            use claude_code_tools::bash::*;
            execute_tool(
                BashTool,
                BashParams {
                    command,
                    timeout,
                    description,
                    run_in_background: false,
                },
                &ctx,
            )
            .await?;
        }

        Commands::Read {
            file_path,
            offset,
            limit,
        } => {
            use claude_code_tools::read::*;
            execute_tool(
                ReadTool,
                ReadParams {
                    file_path,
                    offset,
                    limit,
                },
                &ctx,
            )
            .await?;
        }

        Commands::Write {
            file_path,
            content,
        } => {
            use claude_code_tools::write::*;
            execute_tool(WriteTool, WriteParams { file_path, content }, &ctx).await?;
        }

        Commands::Edit {
            file_path,
            old_string,
            new_string,
            replace_all,
        } => {
            use claude_code_tools::edit::*;
            execute_tool(
                EditTool,
                EditParams {
                    file_path,
                    old_string,
                    new_string,
                    replace_all,
                },
                &ctx,
            )
            .await?;
        }

        Commands::Glob { pattern, path } => {
            use claude_code_tools::glob_tool::*;
            execute_tool(GlobTool, GlobParams { pattern, path }, &ctx).await?;
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
            execute_tool(
                GrepTool,
                GrepParams {
                    pattern,
                    path,
                    output_mode: OutputMode::Content,
                    case_insensitive,
                    glob,
                    before_context: before,
                    after_context: after,
                    head_limit,
                },
                &ctx,
            )
            .await?;
        }
    }

    Ok(())
}

/// Generic tool execution with streaming
async fn execute_tool<T>(tool: T, params: T::Params, ctx: &ToolContext) -> Result<()>
where
    T: Tool,
{
    let mut stream = tool.execute(params, ctx).await?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Progress { step, percentage } => {
                if let Some(pct) = percentage {
                    println!("⏳ {} ({}%)", step, pct);
                } else {
                    println!("⏳ {}", step);
                }
            }

            ToolEvent::Result(output) => {
                // Serialize result as JSON for consistent output
                let json = serde_json::to_string_pretty(&output)?;
                println!("\n{}", json);
            }

            ToolEvent::Error { message } => {
                eprintln!("❌ Error: {}", message);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
