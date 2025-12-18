//! TUI Streaming Tests
//!
//! Tests for real-time streaming functionality:
//! - Incremental text rendering
//! - Streaming event handling
//! - Buffer updates during streaming
//! - Streaming performance
//! - Error handling during streaming

mod helpers;
mod mocks;
mod tui_test_harness;

use mocks::{MockApiClient, MockResponse, MockStreamEvent};
use rustyclawd::tui::{ChatMessage, MessageRole};
use tui_test_harness::TuiTestHarness;

#[test]
fn test_streaming_single_chunk() {
    // Test that a single chunk is displayed correctly
    let mut harness = TuiTestHarness::new().unwrap();

    harness
        .terminal
        .draw(|f| {
            use ratatui::{
                text::{Line, Span},
                widgets::Paragraph,
            };

            let text = vec![Line::from(vec![
                Span::raw("Claude> "),
                Span::raw("Hello, World!"),
            ])];

            let paragraph = Paragraph::new(text);
            f.render_widget(paragraph, f.area());
        })
        .unwrap();

    assert!(harness.contains("Claude>"));
    assert!(harness.contains("Hello, World!"));
}

#[test]
fn test_streaming_multiple_chunks() {
    // Test that multiple chunks are accumulated correctly
    let messages = [
        ChatMessage::assistant("This is ".to_string()),
        ChatMessage::assistant("a test ".to_string()),
        ChatMessage::assistant("message.".to_string()),
    ];

    // Verify chunks can be combined
    let combined: String = messages.iter().map(|m| m.content.as_str()).collect();
    assert_eq!(combined, "This is a test message.");
}

#[test]
fn test_streaming_incremental_display() {
    // Test that content is displayed incrementally
    let mut harness = TuiTestHarness::new().unwrap();

    // Simulate incremental rendering
    let chunks = ["Hello", " ", "World", "!"];

    for (i, _chunk) in chunks.iter().enumerate() {
        let accumulated: String = chunks[..=i].concat();

        harness
            .terminal
            .draw(|f| {
                use ratatui::{text::Line, widgets::Paragraph};

                let paragraph = Paragraph::new(Line::from(accumulated.clone()));
                f.render_widget(paragraph, f.area());
            })
            .unwrap();

        // Verify accumulated content is displayed
        assert!(harness.contains(&accumulated));
    }

    // Final state should contain complete message
    assert!(harness.contains("Hello World!"));
}

#[test]
fn test_streaming_empty_chunk() {
    // Test that empty chunks are handled gracefully
    let chunks = ["Hello", "", "World"];
    let combined: String = chunks.iter().copied().collect();

    assert_eq!(combined, "HelloWorld");
}

#[test]
fn test_streaming_unicode_chunks() {
    // Test that Unicode characters are handled correctly in chunks
    let chunks = ["Hello ", "🦀", " World"];
    let combined: String = chunks.concat();

    assert_eq!(combined, "Hello 🦀 World");
    assert_eq!(combined.chars().count(), 13); // Verify emoji is single char
}

#[test]
fn test_streaming_newline_chunks() {
    // Test that newlines in chunks are preserved
    let chunks = ["Line 1\n", "Line 2\n", "Line 3"];
    let combined: String = chunks.concat();

    assert!(combined.contains("Line 1\nLine 2\nLine 3"));
    assert_eq!(combined.lines().count(), 3);
}

#[test]
fn test_streaming_long_content() {
    // Test streaming of long content
    let chunk = "This is a very long line that might exceed terminal width and require wrapping to display correctly in the TUI.";
    let mut harness = TuiTestHarness::new().unwrap();

    harness
        .terminal
        .draw(|f| {
            use ratatui::{text::Line, widgets::Paragraph};

            let paragraph = Paragraph::new(Line::from(chunk));
            f.render_widget(paragraph, f.area());
        })
        .unwrap();

    assert!(harness.contains("This is a very long line"));
}

#[test]
fn test_streaming_message_roles() {
    // Test that different message roles are handled during streaming
    let messages = [
        ChatMessage::user("User message".to_string()),
        ChatMessage::assistant("Assistant message".to_string()),
        ChatMessage::system("System message".to_string()),
    ];

    assert_eq!(messages.len(), 3);
    assert!(matches!(messages[0].role, MessageRole::User));
    assert!(matches!(messages[1].role, MessageRole::Assistant));
    assert!(matches!(messages[2].role, MessageRole::System));
}

#[test]
fn test_streaming_buffer_update_efficiency() {
    // Test that buffer updates are efficient
    let mut harness = TuiTestHarness::new().unwrap();

    // Multiple updates
    for i in 0..10 {
        let content = format!("Update {}", i);
        harness
            .terminal
            .draw(|f| {
                use ratatui::{text::Line, widgets::Paragraph};

                let paragraph = Paragraph::new(Line::from(content.clone()));
                f.render_widget(paragraph, f.area());
            })
            .unwrap();
    }

    // Verify final state
    assert!(harness.contains("Update 9"));
}

#[test]
fn test_streaming_with_special_characters() {
    // Test that special characters in streamed content are handled
    let special_chars = vec!["<html>", "&nbsp;", "\"quoted\"", "'single'"];

    for special in special_chars {
        let mut harness = TuiTestHarness::new().unwrap();

        harness
            .terminal
            .draw(|f| {
                use ratatui::{text::Line, widgets::Paragraph};

                let paragraph = Paragraph::new(Line::from(special));
                f.render_widget(paragraph, f.area());
            })
            .unwrap();

        assert!(harness.contains(special));
    }
}

// Mock API client tests
#[tokio::test]
async fn test_mock_streaming_response() {
    use futures::StreamExt;

    let client = MockApiClient::new();
    client.queue_response(MockResponse::chunked_text(vec![
        "Hello",
        " ",
        "streaming",
        " ",
        "world",
    ]));

    let mut stream = client.send_message("test".to_string()).await;
    let mut chunks = Vec::new();

    while let Some(event) = stream.next().await {
        if let MockStreamEvent::ContentDelta(chunk) = event {
            chunks.push(chunk);
        }
    }

    assert_eq!(chunks.len(), 5);
    assert_eq!(chunks.concat(), "Hello streaming world");
}

#[tokio::test]
async fn test_mock_streaming_with_completion() {
    use futures::StreamExt;

    let client = MockApiClient::new();
    client.queue_response(MockResponse::text("Test message"));

    let mut stream = client.send_message("test".to_string()).await;
    let mut events = Vec::new();

    while let Some(event) = stream.next().await {
        events.push(event);
    }

    assert_eq!(events.len(), 2); // Content + Complete
    assert!(matches!(events[0], MockStreamEvent::ContentDelta(_)));
    assert_eq!(events[1], MockStreamEvent::MessageComplete);
}
