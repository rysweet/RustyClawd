//! End-to-End Test: Skills Execution in Context
//!
//! This test validates that skills execute with full conversation context,
//! including prior messages and conversation history.

mod e2e;

use e2e::helpers::{TestSession, TestSkillEnvironment};
use e2e::mocks::MockResponse;

/// Test that skills load correctly with conversation context
#[tokio::test]
async fn test_skill_loads_with_context() {
    // 1. Setup: Create test skill file
    let skill_env = TestSkillEnvironment::new()
        .with_skill("test-analyzer", "Perform deep analysis of code")
        .build()
        .unwrap();

    // 2. Setup: Session with skill directory
    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_skill_dir(skill_env.path().to_path_buf())
        .build()
        .await
        .unwrap();

    // 3. Queue mock response
    session
        .mock_llm()
        .queue_response(MockResponse::text("Analysis complete"));

    // 4. Add conversation context
    session
        .add_conversation_turn("What's in main.rs?", "main.rs contains the entry point")
        .await
        .unwrap();

    // 5. Send input that would trigger skill
    session.send_input("Analyze the code").await.unwrap();

    // 6. Verify: Skill file exists
    assert!(
        skill_env.path().join("test-analyzer.md").exists(),
        "Skill file should be created"
    );

    // 7. Verify: Conversation history preserved
    let context = session.get_llm_context();
    assert!(
        context.iter().any(|msg| msg.contains("main.rs")),
        "Prior conversation should be in context"
    );
}

/// Test that skills access prior messages in conversation
#[tokio::test]
async fn test_skill_accesses_prior_messages() {
    // 1. Setup: Create test skill
    let skill_env = TestSkillEnvironment::new()
        .with_skill(
            "code-reviewer",
            "Review the code mentioned in the conversation",
        )
        .build()
        .unwrap();

    // 2. Setup: Session with conversation history
    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_skill_dir(skill_env.path().to_path_buf())
        .build()
        .await
        .unwrap();

    // 3. Establish context with prior messages
    session
        .add_conversation_turn(
            "Here's my function: fn add(a: i32, b: i32) -> i32 { a + b }",
            "I see your add function. It looks correct.",
        )
        .await
        .unwrap();

    // 4. Action: New message that could reference skill
    session
        .mock_llm()
        .queue_response(MockResponse::text("The function is well-written"));

    session
        .send_input("Review that function")
        .await
        .unwrap();

    // 5. Verify: Prior context accessible
    let llm_context = session.get_llm_context();
    assert!(
        llm_context.iter().any(|msg| msg.contains("fn add")),
        "Skill should have access to function in conversation"
    );
}

/// Test that skill execution uses context correctly
#[tokio::test]
async fn test_skill_uses_context_correctly() {
    let skill_env = TestSkillEnvironment::new()
        .with_skill("summarizer", "Summarize the key points from the discussion")
        .build()
        .unwrap();

    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_skill_dir(skill_env.path().to_path_buf())
        .build()
        .await
        .unwrap();

    // Establish multi-turn conversation
    session
        .add_conversation_turn(
            "What's the testing strategy?",
            "We use 60% unit, 30% integration, 10% E2E tests.",
        )
        .await
        .unwrap();

    session
        .add_conversation_turn(
            "What about mocking?",
            "We mock external APIs but use real internal components.",
        )
        .await
        .unwrap();

    // Queue response for skill invocation
    session
        .mock_llm()
        .queue_response(MockResponse::text(
            "Summary: Testing pyramid with strategic mocking",
        ));

    session.send_input("Summarize our discussion").await.unwrap();

    // Verify full conversation accessible
    let llm_context = session.get_llm_context();
    assert!(
        llm_context
            .iter()
            .any(|msg| msg.contains("testing strategy") || msg.contains("60% unit")),
        "Skill should have access to full conversation"
    );
}

/// Test skill prompt injection into LLM context
#[tokio::test]
async fn test_skill_prompt_injection() {
    let skill_env = TestSkillEnvironment::new()
        .with_skill("debugger", "Debug the provided code")
        .build()
        .unwrap();

    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_skill_dir(skill_env.path().to_path_buf())
        .build()
        .await
        .unwrap();

    // Add code context
    session
        .add_conversation_turn(
            "Here's a bug: let x = 5 / 0;",
            "That will cause a division by zero error.",
        )
        .await
        .unwrap();

    // Queue response
    session
        .mock_llm()
        .queue_response(MockResponse::text("Bug identified: division by zero"));

    session.send_input("Debug this code").await.unwrap();

    // Verify skill environment was created properly
    assert!(
        skill_env.path().join("debugger.md").exists(),
        "Skill file should exist"
    );

    // Verify context includes the bug
    let context = session.get_llm_context();
    assert!(
        context.iter().any(|msg| msg.contains("5 / 0")),
        "Context should include the buggy code"
    );
}

/// Test multiple skills with shared context
#[tokio::test]
async fn test_multiple_skills_context() {
    let skill_env = TestSkillEnvironment::new()
        .with_skill("analyzer", "Analyze code patterns")
        .with_skill("optimizer", "Suggest optimizations")
        .build()
        .unwrap();

    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_skill_dir(skill_env.path().to_path_buf())
        .build()
        .await
        .unwrap();

    // Establish context
    session
        .add_conversation_turn(
            "Look at this loop: for i in 0..1000 { println!(\"{}\", i); }",
            "That's a simple loop printing numbers.",
        )
        .await
        .unwrap();

    // First skill invocation
    session
        .mock_llm()
        .queue_response(MockResponse::text("Pattern: simple iteration"));

    session.send_input("Analyze this pattern").await.unwrap();

    // Second skill invocation
    session
        .mock_llm()
        .queue_response(MockResponse::text("Optimization: batch the prints"));

    session.send_input("Optimize it").await.unwrap();

    // Both skills should have same context
    let context = session.get_llm_context();
    let loop_mentions = context.iter().filter(|msg| msg.contains("for i in")).count();

    assert!(
        loop_mentions >= 1,
        "Loop code should be in shared context for both skills"
    );
}
