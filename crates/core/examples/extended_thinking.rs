//! Example demonstrating Extended Thinking feature
//!
//! This example shows how to enable Claude's Extended Thinking capability,
//! which allows the model to show its reasoning process before answering.
//!
//! Usage:
//!   cargo run --example extended_thinking

use futures::StreamExt;
use rustyclawd_core::client::{Client, Config, CreateMessageRequest, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Extended Thinking Example ===\n");

    // Load API configuration
    let config = Config::from_default_location().await?;
    let client = Client::new(config);

    // Example 1: Non-streaming with Extended Thinking
    println!("--- Example 1: Non-streaming Request with Extended Thinking ---\n");
    example_non_streaming(&client).await?;

    println!("\n");

    // Example 2: Streaming with Extended Thinking (see reasoning in real-time)
    println!("--- Example 2: Streaming Request with Extended Thinking ---\n");
    example_streaming(&client).await?;

    println!("\n=== Examples completed successfully! ===");

    Ok(())
}

/// Example: Non-streaming request with Extended Thinking
async fn example_non_streaming(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let request = CreateMessageRequest::new(
        "claude-sonnet-4-5-20250929",
        vec![Message::user(
            "Solve this math problem step by step: What is 47 * 83 + 125?",
        )],
        4096,
    )
    .with_thinking(4000); // Enable Extended Thinking with 4000 token budget

    println!("Sending request with Extended Thinking enabled (budget: 4000 tokens)...\n");
    let response = client.create_message(request).await?;

    println!("Response received!\n");
    println!("Message ID: {}", response.id);
    println!(
        "Usage: {} input tokens, {} output tokens\n",
        response.usage.input_tokens, response.usage.output_tokens
    );

    // Display all content blocks
    for (i, block) in response.content.iter().enumerate() {
        match block {
            rustyclawd_core::client::ContentBlock::Thinking { thinking, signature } => {
                println!("--- [Block {}]: THINKING PROCESS ---", i);
                if let Some(sig) = signature {
                    println!("Signature: {}...\n", &sig[..sig.len().min(32)]);
                }
                println!("{}\n", thinking);
                println!("--- END THINKING ---\n");
            }
            rustyclawd_core::client::ContentBlock::Text { text } => {
                println!("--- [Block {}]: FINAL ANSWER ---", i);
                println!("{}\n", text);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Example: Streaming request with Extended Thinking (see reasoning in real-time)
async fn example_streaming(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let request = CreateMessageRequest::new(
        "claude-sonnet-4-5-20250929",
        vec![Message::user(
            "Explain the concept of recursion with a practical example.",
        )],
        4096,
    )
    .with_stream(true)
    .with_thinking(2048); // Enable Extended Thinking with 2048 token budget

    println!("Starting stream with Extended Thinking enabled (budget: 2048 tokens)...\n");
    let mut stream = client.create_message_stream(request).await?;

    let mut in_thinking_block = false;

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                use rustyclawd_core::client::StreamEvent;
                use rustyclawd_core::client::types::{ContentBlockStart, ContentDelta};

                match event {
                    StreamEvent::MessageStart { message } => {
                        println!("[Message started: {}]\n", message.id);
                    }
                    StreamEvent::ContentBlockStart {
                        index,
                        content_block,
                    } => {
                        match content_block {
                            ContentBlockStart::Thinking { .. } => {
                                in_thinking_block = true;
                                println!("--- THINKING PROCESS (Block {}) ---", index);
                            }
                            ContentBlockStart::Text { .. } => {
                                if in_thinking_block {
                                    println!("\n--- END THINKING ---\n");
                                }
                                println!("--- FINAL ANSWER (Block {}) ---", index);
                            }
                            _ => {}
                        }
                    }
                    StreamEvent::ContentBlockDelta { delta, .. } => {
                        match delta {
                            ContentDelta::ThinkingDelta { thinking } => {
                                // Print thinking process in real-time
                                print!("{}", thinking);
                                use std::io::Write;
                                std::io::stdout().flush()?;
                            }
                            ContentDelta::TextDelta { text } => {
                                // Print final answer in real-time
                                print!("{}", text);
                                use std::io::Write;
                                std::io::stdout().flush()?;
                            }
                            ContentDelta::SignatureDelta { .. } => {
                                // Signature is included but not displayed
                            }
                            _ => {}
                        }
                    }
                    StreamEvent::ContentBlockStop { .. } => {
                        // Block ended
                        println!();
                    }
                    StreamEvent::MessageStop => {
                        println!("\n[Message stopped]");
                    }
                    StreamEvent::Error { error } => {
                        eprintln!("API Error: {}", error.message);
                        return Err(error.message.into());
                    }
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("Stream error: {}", e);
                return Err(e.into());
            }
        }
    }

    Ok(())
}
