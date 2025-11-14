use rustyclawd_tools::{SlashCommandTool, Tool, ToolContext, ToolEvent};
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let tool = SlashCommandTool;

    // Test with analyze command
    let params = rustyclawd_tools::slash_command::SlashCommandParams {
        command: "/analyze".to_string(),
    };
    let ctx = ToolContext::default();

    println!("Testing /analyze command...");
    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    for event in events {
        match event {
            ToolEvent::Result(output) => {
                println!("Command: {}", output.command);
                println!("Command Name: {}", output.command_name);
                println!("Expanded Prompt:\n{}", output.expanded_prompt);
            }
            ToolEvent::Error { message } => {
                println!("Error: {}", message);
            }
            _ => {}
        }
    }

    println!("\n\nTesting /debug command...");
    let params = rustyclawd_tools::slash_command::SlashCommandParams {
        command: "/debug".to_string(),
    };
    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    for event in events {
        match event {
            ToolEvent::Result(output) => {
                println!("Command: {}", output.command);
                println!("Command Name: {}", output.command_name);
                println!("Expanded Prompt:\n{}", output.expanded_prompt);
            }
            ToolEvent::Error { message } => {
                println!("Error: {}", message);
            }
            _ => {}
        }
    }

    println!("\n\nTesting /ultrathink command...");
    let params = rustyclawd_tools::slash_command::SlashCommandParams {
        command: "/ultrathink".to_string(),
    };
    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    for event in events {
        match event {
            ToolEvent::Result(output) => {
                println!("Command: {}", output.command);
                println!("Command Name: {}", output.command_name);
                println!("Expanded Prompt:\n{}", output.expanded_prompt);
            }
            ToolEvent::Error { message } => {
                println!("Error: {}", message);
            }
            _ => {}
        }
    }
}
