//! Test the Anthropic API client with streaming
//!
//! This example demonstrates:
//! - Loading API key from ~/.claude-msec-k
//! - Making a simple request to Claude
//! - Streaming the response in real-time
//! - Proper error handling
//!
//! Usage:
//!   cargo run --example api_test

use futures::StreamExt;
use rustyclawd_core::client::{Client, Config, CreateMessageRequest, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Anthropic API Client Test ===\n");

    // Load API configuration from default location
    println!("Loading API key from ~/.claude-msec-k...");
    let config = Config::from_default_location().await?;
    println!("API key loaded successfully!\n");

    // Create the client
    let client = Client::new(config);

    // Test 1: Non-streaming request
    println!("--- Test 1: Non-streaming Request ---");
    test_non_streaming(&client).await?;

    println!("\n");

    // Test 2: Streaming request
    println!("--- Test 2: Streaming Request ---");
    test_streaming(&client).await?;

    println!("\n=== All tests completed successfully! ===");

    Ok(())
}

/// Test a simple non-streaming request
async fn test_non_streaming(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let request = CreateMessageRequest::new(
        "claude-3-5-sonnet-20241022",
        vec![Message::user("Say 'Hello, World!' and nothing else.")],
        100,
    );

    println!("Sending request...");
    let response = client.create_message(request).await?;

    println!("Response received!");
    println!("  Message ID: {}", response.id);
    println!("  Model: {}", response.model);
    println!("  Stop reason: {:?}", response.stop_reason);
    println!(
        "  Usage: {} input tokens, {} output tokens",
        response.usage.input_tokens, response.usage.output_tokens
    );

    // Extract text from content blocks
    for (i, block) in response.content.iter().enumerate() {
        match block {
            rustyclawd_core::client::ContentBlock::Text { text } => {
                println!("  Content[{}]: {}", i, text);
            }
            rustyclawd_core::client::ContentBlock::ToolUse { .. } => {
                println!("  Content[{}]: [tool_use]", i);
            }
            rustyclawd_core::client::ContentBlock::ToolResult { .. } => {
                println!("  Content[{}]: [tool_result]", i);
            }
        }
    }

    Ok(())
}

/// Test streaming request with real-time output
async fn test_streaming(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let request = CreateMessageRequest::new(
        "claude-3-5-sonnet-20241022",
        vec![Message::user(
            "Count from 1 to 5, with each number on a new line.",
        )],
        100,
    )
    .with_stream(true);

    println!("Starting stream...");
    let mut stream = client.create_message_stream(request).await?;

    println!("Streaming response:\n---");

    let mut message_id: Option<String> = None;
    let mut collected_text = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                use rustyclawd_core::client::StreamEvent;

                match event {
                    StreamEvent::MessageStart { message } => {
                        message_id = Some(message.id.clone());
                        println!("[Message started: {}]", message.id);
                    }
                    StreamEvent::ContentBlockStart { index, .. } => {
                        println!("[Content block {} started]", index);
                    }
                    StreamEvent::ContentBlockDelta { delta, .. } => {
                        if let rustyclawd_core::client::types::ContentDelta::TextDelta { text } =
                            delta {
                            // Print in real-time without newline
                            print!("{}", text);
                            use std::io::Write;
                            std::io::stdout().flush()?;
                            collected_text.push_str(&text);
                        }
                    }
                    StreamEvent::ContentBlockStop { .. } => {
                        println!("\n[Content block stopped]");
                    }
                    StreamEvent::MessageDelta { delta, usage } => {
                        println!(
                            "[Message delta - stop_reason: {:?}, output_tokens: {}]",
                            delta.stop_reason, usage.output_tokens
                        );
                    }
                    StreamEvent::MessageStop => {
                        println!("[Message stopped]");
                    }
                    StreamEvent::Ping => {
                        // Ignore ping events
                    }
                    StreamEvent::Error { error } => {
                        eprintln!("API Error: {}", error.message);
                        return Err(error.message.into());
                    }
                }
            }
            Err(e) => {
                eprintln!("Stream error: {}", e);
                return Err(e.into());
            }
        }
    }

    println!("---");
    println!("\nStream complete!");
    if let Some(id) = message_id {
        println!("  Message ID: {}", id);
    }
    println!("  Collected text length: {} chars", collected_text.len());

    Ok(())
}
