//! Test streaming with the API client
//!
//! Usage: cargo run --example stream_test

use futures::StreamExt;
use rustyclawd_core::client::{Client, Config, CreateMessageRequest, Message, StreamEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Streaming API Test ===\n");

    // Load config
    let config = Config::from_default_location().await?;
    let client = Client::new(config);

    // Create streaming request
    let request = CreateMessageRequest::new(
        "claude-3-haiku-20240307",
        vec![Message::user("Count from 1 to 5, one number per line.")],
        100,
    )
    .with_stream(true);

    println!("Starting stream...\n");
    let mut stream = client.create_message_stream(request).await?;

    let mut text_buffer = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => match event {
                StreamEvent::MessageStart { message } => {
                    println!("[Message started: {}]", message.id);
                }
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    let rustyclawd_core::client::types::ContentDelta::TextDelta { text } = delta;
                    // Print in real-time
                    print!("{}", text);
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    text_buffer.push_str(&text);
                }
                StreamEvent::MessageDelta { usage, .. } => {
                    println!("\n\n[Finished - {} output tokens]", usage.output_tokens);
                }
                StreamEvent::MessageStop => {
                    println!("[Stream ended]");
                }
                _ => {}
            },
            Err(e) => {
                eprintln!("\nError: {}", e);
                return Err(e.into());
            }
        }
    }

    println!("\n\n=== Stream Complete ===");
    println!("Total text length: {} chars", text_buffer.len());

    Ok(())
}
