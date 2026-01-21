//! Example of using the Agent tool for sub-agent orchestration
//!
//! This example demonstrates how to:
//! - Invoke a specialized sub-agent
//! - Pass a task and context
//! - Stream responses from the agent
//! - Handle agent outputs
//!
//! To run this example:
//! ```bash
//! # Set your API key
//! export ANTHROPIC_API_KEY="your-key-here"
//!
//! # Run the example
//! cargo run --example agent_example
//! ```

use futures::StreamExt;
use rustyclawd_tools::{AgentTool, Tool, ToolContext, ToolEvent};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("agent_example=debug,rustyclawd_tools=debug")
        .init();

    // Check for API key
    if env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("Error: ANTHROPIC_API_KEY environment variable not set");
        eprintln!("Please set your API key:");
        eprintln!("  export ANTHROPIC_API_KEY='your-key-here'");
        std::process::exit(1);
    }

    println!("=== Agent Tool Example ===\n");

    // Create the agent tool
    let tool = AgentTool;

    // Define the task for the agent
    let params = serde_json::json!({
        "description": "Analyze code structure",
        "prompt": "Explain the benefits of the Rust ownership system in 2-3 sentences.",
        "subagent_type": "example",
        "model": "haiku",  // Use fast model for demo
    });

    let params: rustyclawd_tools::agent::AgentParams = serde_json::from_value(params)?;

    // Set up context
    let ctx = ToolContext {
        cwd: env::current_dir()?,
        debug: true,
        metadata: serde_json::Value::Null,
        execution_context: rustyclawd_tools::ExecutionContext::default(),
        allowed_tools: vec![],
        disallowed_tools: vec![],
    };

    println!("Invoking 'example' agent with haiku model...\n");

    // Execute the agent
    let mut stream = tool.execute(params, &ctx).await?;

    // Process the stream
    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Progress { step, percentage } => {
                if let Some(pct) = percentage {
                    println!("[Progress {:.0}%] {}", pct, step);
                } else {
                    println!("[Progress] {}", step);
                }
            }
            ToolEvent::Result(output) => {
                println!("\n=== Agent Response ===\n");
                println!("Agent ID: {}", output.agent_id);
                println!("Agent Name: {}", output.agent_name);
                println!("Model: {}", output.model);
                println!(
                    "Tokens Used: {} (input) + {} (output) = {} (total)\n",
                    output.tokens_used.input_tokens,
                    output.tokens_used.output_tokens,
                    output.tokens_used.total_tokens
                );
                println!("Response:\n{}\n", output.response);
                println!("=== End Response ===");
            }
            ToolEvent::Error { message } => {
                eprintln!("Error: {}", message);
                return Err(message.into());
            }
        }
    }

    println!("\nAgent execution completed successfully!");

    Ok(())
}
