//! Integration test for SlashCommand with real command files

use rustyclawd_tools::{SlashCommandTool, Tool, ToolContext, ToolEvent};
use futures::StreamExt;
use std::path::Path;

#[tokio::test]
async fn test_real_analyze_command() {
    // Check if command file exists before testing
    if !Path::new(".claude/commands/analyze.md").exists() {
        eprintln!("Skipping test - .claude/commands/analyze.md not found");
        return;
    }

    let tool = SlashCommandTool;
    let params = rustyclawd_tools::slash_command::SlashCommandParams {
        command: "/analyze".to_string(),
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output),
        _ => None,
    });

    assert!(result.is_some(), "Expected result event but got: {:?}", events);
    let output = result.unwrap();
    assert_eq!(output.command_name, "analyze");
    assert!(output.expanded_prompt.contains("Analyze") || output.expanded_prompt.contains("analysis"));
}

#[tokio::test]
async fn test_real_debug_command() {
    // Check if command file exists before testing
    if !Path::new(".claude/commands/debug.md").exists() {
        eprintln!("Skipping test - .claude/commands/debug.md not found");
        return;
    }

    let tool = SlashCommandTool;
    let params = rustyclawd_tools::slash_command::SlashCommandParams {
        command: "/debug".to_string(),
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output),
        _ => None,
    });

    assert!(result.is_some(), "Expected result event but got: {:?}", events);
    let output = result.unwrap();
    assert_eq!(output.command_name, "debug");
    assert!(output.expanded_prompt.contains("debug"));
}

#[tokio::test]
async fn test_real_ultrathink_command() {
    // Check if command file exists before testing
    if !Path::new(".claude/commands/ultrathink.md").exists() {
        eprintln!("Skipping test - .claude/commands/ultrathink.md not found");
        return;
    }

    let tool = SlashCommandTool;
    let params = rustyclawd_tools::slash_command::SlashCommandParams {
        command: "/ultrathink".to_string(),
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output),
        _ => None,
    });

    assert!(result.is_some(), "Expected result event but got: {:?}", events);
    let output = result.unwrap();
    assert_eq!(output.command_name, "ultrathink");
    assert!(output.expanded_prompt.contains("deep") || output.expanded_prompt.contains("Deep"));
}
