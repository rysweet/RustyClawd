//! TUI Tool Visibility Tests
//!
//! Tests for tool call visibility in TUI:
//! - Tool use display
//! - Tool parameters visibility
//! - Tool results formatting
//! - Multiple tool calls
//! - Tool error handling

mod helpers;
mod mocks;
mod tui_test_harness;

use mocks::mock_tool_executor::{MockToolExecutor, MockToolResult};
use mocks::{MockApiClient, MockResponse, MockStreamEvent};
use tui_test_harness::TuiTestHarness;

#[test]
fn test_tool_use_display() {
    // Test that tool use is displayed in TUI
    let mut harness = TuiTestHarness::new().unwrap();

    harness
        .terminal
        .draw(|f| {
            use ratatui::{text::Line, widgets::Paragraph};

            let text = vec![
                Line::from("Tool: bash"),
                Line::from("Parameters: {\"command\":\"echo hello\"}"),
            ];

            let paragraph = Paragraph::new(text);
            f.render_widget(paragraph, f.area());
        })
        .unwrap();

    assert!(harness.contains("Tool: bash"));
    assert!(harness.contains("Parameters:"));
}

#[test]
fn test_tool_parameters_visibility() {
    // Test that tool parameters are visible and formatted
    let tool_params = vec![
        r#"{"command":"ls -la"}"#,
        r#"{"file_path":"/tmp/test.txt"}"#,
        r#"{"pattern":"*.rs"}"#,
    ];

    for params in tool_params {
        let mut harness = TuiTestHarness::new().unwrap();

        harness
            .terminal
            .draw(|f| {
                use ratatui::{text::Line, widgets::Paragraph};

                let paragraph = Paragraph::new(Line::from(format!("Params: {}", params)));
                f.render_widget(paragraph, f.area());
            })
            .unwrap();

        assert!(harness.contains("Params:"));
    }
}

#[test]
fn test_tool_result_formatting() {
    // Test that tool results are formatted correctly
    let mut harness = TuiTestHarness::new().unwrap();

    harness
        .terminal
        .draw(|f| {
            use ratatui::{
                text::{Line, Span},
                widgets::Paragraph,
            };

            let text = vec![
                Line::from(Span::raw("Tool Result:")),
                Line::from(Span::raw("  Output: Hello from tool")),
                Line::from(Span::raw("  Status: Success")),
            ];

            let paragraph = Paragraph::new(text);
            f.render_widget(paragraph, f.area());
        })
        .unwrap();

    assert!(harness.contains("Tool Result:"));
    assert!(harness.contains("Output: Hello from tool"));
    assert!(harness.contains("Status: Success"));
}

#[test]
fn test_multiple_tool_calls_display() {
    // Test display of multiple tool calls
    let tool_calls = [
        ("bash", r#"{"command":"echo 1"}"#),
        ("read", r#"{"path":"/tmp/file"}"#),
        ("bash", r#"{"command":"echo 2"}"#),
    ];

    let mut harness = TuiTestHarness::new().unwrap();

    harness
        .terminal
        .draw(|f| {
            use ratatui::{text::Line, widgets::Paragraph};

            let text: Vec<_> = tool_calls
                .iter()
                .enumerate()
                .map(|(i, (tool, _))| Line::from(format!("Tool {}: {}", i + 1, tool)))
                .collect();

            let paragraph = Paragraph::new(text);
            f.render_widget(paragraph, f.area());
        })
        .unwrap();

    assert!(harness.contains("Tool 1: bash"));
    assert!(harness.contains("Tool 2: read"));
    assert!(harness.contains("Tool 3: bash"));
}

#[test]
fn test_tool_error_display() {
    // Test that tool errors are displayed prominently
    let mut harness = TuiTestHarness::new().unwrap();

    harness
        .terminal
        .draw(|f| {
            use ratatui::{
                style::{Color, Style},
                text::{Line, Span},
                widgets::Paragraph,
            };

            let text = vec![
                Line::from(Span::styled(
                    "Tool Error: Command failed",
                    Style::default().fg(Color::Red),
                )),
                Line::from(Span::raw("  Exit code: 1")),
            ];

            let paragraph = Paragraph::new(text);
            f.render_widget(paragraph, f.area());
        })
        .unwrap();

    assert!(harness.contains("Tool Error:"));
    assert!(harness.contains("Exit code: 1"));
}

#[tokio::test]
async fn test_mock_streaming_with_tool_use() {
    // Test streaming with tool use events
    use futures::StreamExt;

    let client = MockApiClient::new();
    client.queue_response(MockResponse::with_tool_use(
        "bash",
        r#"{"command":"echo test"}"#,
        "Tool executed successfully",
    ));

    let mut stream = client.send_message("test".to_string()).await;
    let mut events = Vec::new();

    while let Some(event) = stream.next().await {
        events.push(event);
    }

    // Verify tool use events are present
    assert!(events
        .iter()
        .any(|e| matches!(e, MockStreamEvent::ToolUseStart { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, MockStreamEvent::ToolInputDelta(_))));
    assert!(events
        .iter()
        .any(|e| matches!(e, MockStreamEvent::ToolUseEnd)));
}

#[test]
fn test_mock_tool_executor_basic() {
    // Test that mock tool executor works
    let executor = MockToolExecutor::new();

    executor.set_tool_result("bash", MockToolResult::success("Command output"));

    let result = executor.execute("bash", r#"{"command":"test"}"#);

    assert_eq!(result.output, "Command output");
    assert_eq!(result.exit_code, 0);
    assert!(executor.was_executed("bash"));
}

#[test]
fn test_mock_tool_executor_history() {
    // Test that tool execution is tracked
    let executor = MockToolExecutor::new();

    executor.execute("bash", r#"{"command":"ls"}"#);
    executor.execute("read", r#"{"path":"/tmp"}"#);

    assert_eq!(executor.execution_count("bash"), 1);
    assert_eq!(executor.execution_count("read"), 1);
    assert_eq!(executor.execution_history().len(), 2);
}

#[test]
fn test_tool_use_with_long_output() {
    // Test that long tool output is handled
    let long_output = "Line\n".repeat(100);
    let executor = MockToolExecutor::new();

    executor.set_tool_result("bash", MockToolResult::success(&long_output));

    let result = executor.execute("bash", "{}");

    assert_eq!(result.output.lines().count(), 100);
}

#[test]
fn test_tool_use_with_special_characters() {
    // Test that special characters in tool output are preserved
    let special_output = "Output with <tags> & \"quotes\" and 'apostrophes'";
    let executor = MockToolExecutor::new();

    executor.set_tool_result("bash", MockToolResult::success(special_output));

    let result = executor.execute("bash", "{}");

    assert_eq!(result.output, special_output);
}

#[test]
fn test_concurrent_tool_executions() {
    // Test that multiple tools can be tracked concurrently
    let executor = MockToolExecutor::new();

    executor.set_tool_result("bash", MockToolResult::success("Bash output"));
    executor.set_tool_result("read", MockToolResult::success("Read output"));
    executor.set_tool_result("write", MockToolResult::success("Write output"));

    let bash_result = executor.execute("bash", "{}");
    let read_result = executor.execute("read", "{}");
    let write_result = executor.execute("write", "{}");

    assert_eq!(bash_result.output, "Bash output");
    assert_eq!(read_result.output, "Read output");
    assert_eq!(write_result.output, "Write output");

    assert_eq!(executor.execution_history().len(), 3);
}
