//! Minimal API client test
//!
//! Usage: cargo run --example simple_test

use rustyclawd_core::client::{Client, Config, CreateMessageRequest, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Anthropic API Client\n");

    // Load config
    let config = Config::from_default_location().await?;
    let client = Client::new(config);

    // Try with Claude 3 Haiku (more widely available)
    let request = CreateMessageRequest::new(
        "claude-3-haiku-20240307",
        vec![Message::user("Hi! Just say 'Hello'")],
        50,
    );

    println!("Sending request to Claude 3 Haiku...");
    match client.create_message(request).await {
        Ok(response) => {
            println!("SUCCESS!");
            println!("Model: {}", response.model);
            for block in response.content {
                match block {
                    rustyclawd_core::client::ContentBlock::Text { text } => {
                        println!("Response: {}", text);
                    }
                    rustyclawd_core::client::ContentBlock::ToolUse { .. } => {
                        println!("[tool_use block]");
                    }
                    rustyclawd_core::client::ContentBlock::ToolResult { .. } => {
                        println!("[tool_result block]");
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("\nNote: Try different model IDs like:");
            eprintln!("  - claude-3-haiku-20240307");
            eprintln!("  - claude-3-opus-20240229");
            eprintln!("  - claude-3-sonnet-20240229");
        }
    }

    Ok(())
}
