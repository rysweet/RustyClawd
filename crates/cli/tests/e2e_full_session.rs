//! End-to-End Test: Full Interactive Session
//!
//! This test validates complete workflows from session startup to shutdown,
//! including multi-turn conversations and complete session lifecycle.
//!
//! Note: Hook integration tests will be added in a future phase when
//! hooks system is integrated with TestSession.

mod e2e;

use e2e::helpers::TestSession;
use e2e::mocks::MockResponse;

/// Test complete session lifecycle from start to end
#[tokio::test]
async fn test_session_start_to_end() {
    // 1. Setup: Create session
    let mut session = TestSession::builder()
        .with_mock_llm()
        .build()
        .await
        .expect("Failed to create session");

    // 2. Queue welcome response
    session
        .mock_llm()
        .queue_response(MockResponse::text("Welcome to RustyClawd!"));

    // 3. Send initial message
    session.send_input("Hello").await.unwrap();

    // 4. Verify: Response displayed
    assert!(
        session.tui_contains("Welcome to RustyClawd"),
        "Welcome message should appear"
    );

    // 5. Queue exit response
    session
        .mock_llm()
        .queue_response(MockResponse::text("Goodbye!"));

    // 6. Send exit command
    session.send_input("/exit").await.unwrap();

    // 7. Verify: Exit handled
    assert!(
        session.tui_contains("Goodbye"),
        "Exit message should appear"
    );

    // 8. Shutdown cleanly
    let result = session.shutdown().await;
    assert!(result.is_ok(), "Session should shutdown cleanly");
}

/// Test multi-turn conversation with context preservation
#[tokio::test]
async fn test_multi_turn_conversation() {
    let mut session = TestSession::builder()
        .with_mock_llm()
        .build()
        .await
        .unwrap();

    // Turn 1: Initial greeting
    session
        .mock_llm()
        .queue_response(MockResponse::text("Hello! I'm RustyClawd."));

    session.send_input("Hello").await.unwrap();

    assert!(
        session.tui_contains("RustyClawd"),
        "First response should appear"
    );

    // Turn 2: Question referencing context
    session
        .mock_llm()
        .queue_response(MockResponse::text(
            "My name is RustyClawd, as I just mentioned.",
        ));

    session.send_input("What's your name?").await.unwrap();

    // Verify context preserved across turns
    let context = session.get_llm_context();
    // Note: Currently only user messages are tracked, not assistant responses
    // In full implementation with real session state, we'd have all messages
    assert!(
        context.len() >= 2,
        "Should have at least 2 user messages, got {} messages",
        context.len()
    );
    assert!(
        context.iter().any(|msg| msg.contains("Hello")),
        "First turn should be in context"
    );

    // Turn 3: Another question
    session
        .mock_llm()
        .queue_response(MockResponse::text("I can help with Rust development."));

    session
        .send_input("What can you do?")
        .await
        .unwrap();

    // Verify all context preserved
    let context = session.get_llm_context();
    // Note: Currently only user messages are tracked
    assert!(
        context.len() >= 3,
        "Should have at least 3 user messages, got {} messages",
        context.len()
    );
}

/// Test hooks fire correctly (simplified version without real hooks system)
#[tokio::test]
async fn test_hooks_fire_correctly() {
    let mut session = TestSession::builder()
        .with_mock_llm()
        .build()
        .await
        .unwrap();

    // Queue response
    session
        .mock_llm()
        .queue_response(MockResponse::text("Test response"));

    // Send input (this would trigger hooks in full implementation)
    session.send_input("Test message").await.unwrap();

    // Verify: Session handled input/output correctly
    assert!(
        session.tui_contains("Test message"),
        "User input should appear"
    );
    assert!(
        session.tui_contains("Test response"),
        "Assistant response should appear"
    );

    // In future: Verify hooks using real HooksSystem
    // assert!(hooks.hook_fired("UserPromptSubmit"));
    // assert!(hooks.hook_fired("PreLLMCall"));
    // assert!(hooks.hook_fired("PostLLMCall"));
}

/// Test tool use workflow with LLM
#[tokio::test]
async fn test_tool_use_workflow() {
    let mut session = TestSession::builder()
        .with_mock_llm()
        .build()
        .await
        .unwrap();

    // Mock LLM requesting tool use
    session.mock_llm().queue_response(MockResponse::tool_use(
        "Read",
        "tool_call_123",
        serde_json::json!({"file_path": "README.md"}),
    ));

    // User requests file read
    session
        .send_input("Please read README.md")
        .await
        .unwrap();

    // Verify: Tool invocation recorded
    assert!(
        session.tool_was_invoked("Read"),
        "Read tool should be invoked"
    );

    // Verify: Tool context captured
    let tool_context = session.get_tool_context("Read");
    assert!(tool_context.is_some(), "Tool context should be captured");
    assert!(
        tool_context.unwrap().contains("README.md"),
        "Tool parameters should be preserved"
    );

    // In future: Verify tool execution and hooks
    // assert!(hooks.hook_fired("PreToolUse"));
    // assert!(hooks.hook_fired("PostToolUse"));
}

/// Test skill and command together in same session
#[tokio::test]
async fn test_skill_and_command_together() {
    use e2e::helpers::TestSkillEnvironment;

    let skill_env = TestSkillEnvironment::new()
        .with_skill("analyzer", "Analyze code")
        .build()
        .unwrap();

    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_skill_dir(skill_env.path().to_path_buf())
        .build()
        .await
        .unwrap();

    // First: Use slash command
    session
        .mock_llm()
        .queue_response(MockResponse::text("Debug mode enabled"));

    session.send_input("/debug").await.unwrap();

    assert!(
        session.tool_was_invoked("SlashCommand"),
        "SlashCommand should be invoked"
    );

    // Second: Reference skill in conversation
    session
        .mock_llm()
        .queue_response(MockResponse::text("Analysis complete"));

    session.send_input("Analyze this code").await.unwrap();

    // Verify both interactions preserved
    let context = session.get_llm_context();
    // Note: Currently only user messages are tracked
    assert!(
        context.len() >= 2,
        "Should have both slash command and skill interactions, got {} messages",
        context.len()
    );
    assert!(
        context.iter().any(|msg| msg.contains("/debug")),
        "Slash command should be in context"
    );
    assert!(
        context.iter().any(|msg| msg.contains("Analyze")),
        "Skill-related input should be in context"
    );
}

/// Test error handling in interactive session
#[tokio::test]
async fn test_error_handling() {
    let mut session = TestSession::builder()
        .with_mock_llm()
        .build()
        .await
        .unwrap();

    // Mock LLM error response
    session
        .mock_llm()
        .queue_response(MockResponse::error("API rate limit exceeded"));

    // Send request
    let result = session.send_input("Hello").await;

    // Verify: Error handled gracefully
    // (Currently our mock returns error as Result::Err)
    // In real implementation, errors would be shown in TUI
    assert!(result.is_err(), "Error response should propagate");

    // In future: Verify error hooks
    // assert!(hooks.hook_fired("ErrorOccurred"));
}
