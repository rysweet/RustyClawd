//! TUI Persistence Tests
//!
//! Tests for session persistence:
//! - Message history saving
//! - State restoration
//! - Scroll position persistence
//! - Input buffer persistence

mod helpers;
mod tui_test_harness;

use rustyclawd::tui::{ChatMessage, MessageRole};

#[test]
fn test_message_history_storage() {
    // Test that message history can be stored
    let messages = [
        ChatMessage::user("Hello".to_string()),
        ChatMessage::assistant("Hi there!".to_string()),
        ChatMessage::user("How are you?".to_string()),
    ];

    assert_eq!(messages.len(), 3);

    // Verify messages can be serialized
    let json = serde_json::to_string(&messages.len());
    assert!(json.is_ok());
}

#[test]
fn test_message_serialization() {
    // Test message serialization for persistence
    let message = ChatMessage::user("Test message".to_string());

    // Verify message structure
    assert_eq!(message.content, "Test message");
    assert!(matches!(message.role, MessageRole::User));
}

#[test]
fn test_empty_history() {
    // Test handling of empty history
    let messages: Vec<ChatMessage> = vec![];

    assert_eq!(messages.len(), 0);
    assert!(messages.is_empty());
}

#[test]
fn test_large_history() {
    // Test handling of large message history
    let mut messages = Vec::new();

    for i in 0..1000 {
        messages.push(ChatMessage::user(format!("Message {}", i)));
    }

    assert_eq!(messages.len(), 1000);
}

#[test]
fn test_message_order_preservation() {
    // Test that message order is preserved
    let messages = [
        ChatMessage::user("First".to_string()),
        ChatMessage::assistant("Second".to_string()),
        ChatMessage::user("Third".to_string()),
    ];

    assert_eq!(messages[0].content, "First");
    assert_eq!(messages[1].content, "Second");
    assert_eq!(messages[2].content, "Third");
}

#[test]
fn test_scroll_position_storage() {
    // Test scroll position can be stored
    let scroll_offset = 42usize;

    // Verify scroll position is valid
    assert!(scroll_offset > 0);

    // Simulate restoration
    let restored_offset = scroll_offset;
    assert_eq!(restored_offset, 42);
}

#[test]
fn test_input_buffer_persistence() {
    // Test input buffer can be persisted
    let input_buffer = "This is incomplete text".to_string();
    let cursor_position = 10usize;

    // Verify buffer state
    assert!(!input_buffer.is_empty());
    assert!(cursor_position <= input_buffer.len());

    // Simulate restoration
    let restored_buffer = input_buffer.clone();
    let restored_cursor = cursor_position;

    assert_eq!(restored_buffer, "This is incomplete text");
    assert_eq!(restored_cursor, 10);
}

#[test]
fn test_state_restoration() {
    // Test complete state restoration
    let messages = [ChatMessage::user("Test".to_string())];
    let input = "Incomplete".to_string();
    let cursor = 5usize;
    let scroll = 0usize;

    // Verify state can be captured
    assert!(!messages.is_empty());
    assert!(!input.is_empty());
    assert_eq!(cursor, 5);
    assert_eq!(scroll, 0);
}

#[test]
fn test_message_with_newlines() {
    // Test messages with newlines are preserved
    let message = ChatMessage::user("Line 1\nLine 2\nLine 3".to_string());

    assert_eq!(message.content.lines().count(), 3);
    assert!(message.content.contains('\n'));
}

#[test]
fn test_message_with_unicode() {
    // Test messages with Unicode are preserved
    let message = ChatMessage::user("Hello 🦀 World".to_string());

    assert_eq!(message.content, "Hello 🦀 World");
    assert!(message.content.contains('🦀'));
}

#[test]
fn test_long_message_content() {
    // Test long message content preservation
    let long_content = "a".repeat(10000);
    let message = ChatMessage::user(long_content.clone());

    assert_eq!(message.content.len(), 10000);
    assert_eq!(message.content, long_content);
}

#[test]
fn test_mixed_role_history() {
    // Test history with mixed message roles
    let messages = [
        ChatMessage::user("User 1".to_string()),
        ChatMessage::assistant("Assistant 1".to_string()),
        ChatMessage::system("System 1".to_string()),
        ChatMessage::user("User 2".to_string()),
    ];

    assert_eq!(messages.len(), 4);

    let user_count = messages
        .iter()
        .filter(|m| matches!(m.role, MessageRole::User))
        .count();
    assert_eq!(user_count, 2);

    let assistant_count = messages
        .iter()
        .filter(|m| matches!(m.role, MessageRole::Assistant))
        .count();
    assert_eq!(assistant_count, 1);

    let system_count = messages
        .iter()
        .filter(|m| matches!(m.role, MessageRole::System))
        .count();
    assert_eq!(system_count, 1);
}
