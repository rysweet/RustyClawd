//! End-to-End Test: Skills Execution in Context
//!
//! This test validates that skills execute with full conversation context,
//! including prior messages and conversation history.
//!
//! **Status:** FAILING - Waiting for TestSession and TestSkillEnvironment implementation
//!
//! **What This Tests:**
//! 1. Skills load correctly from disk
//! 2. Skills receive full conversation context
//! 3. Skill prompts injected into LLM context
//! 4. Tool parameters passed correctly
//! 5. Context preserved across turns

#[cfg(test)]
mod skills_execution_context_tests {
    // These imports will fail until the modules are implemented
    // use crate::e2e::helpers::{TestSession, TestSkillEnvironment};
    // use tokio;
    // use serde_json::json;

    /// Test that skills load correctly from disk
    #[tokio::test]
    #[ignore] // Ignored until TestSkillEnvironment implemented
    async fn test_skill_loads_correctly() {
        todo!("Implement TestSkillEnvironment first - see docs/specs/test_session_spec.md");

        /*
        // 1. Setup: Create test skill file
        let skill_env = TestSkillEnvironment::new()
            .with_skill("test-analyzer", "Perform deep analysis of code")
            .build();

        // 2. Setup: Session with skill directory
        let session = TestSession::builder()
            .with_mock_llm()
            .with_skill_dir(skill_env.path())
            .build()
            .await
            .unwrap();

        // 3. Verify: Skill file exists
        assert!(skill_env.path().join("test-analyzer.md").exists(),
            "Skill file should be created");

        // TestSkillEnvGuard drops here and cleans up
        */
    }

    /// Test that skills receive full conversation context
    #[tokio::test]
    #[ignore]
    async fn test_skill_receives_conversation_context() {
        todo!("Implement TestSession and TestSkillEnvironment first");

        /*
        // 1. Setup: Create test skill
        let skill_env = TestSkillEnvironment::new()
            .with_skill(
                "code-reviewer",
                "Review the code mentioned in the conversation"
            )
            .build();

        // 2. Setup: Session with conversation history
        let mut session = TestSession::builder()
            .with_mock_llm()
            .with_skill_dir(skill_env.path())
            .build()
            .await
            .unwrap();

        // 3. Establish context with prior messages
        session.add_conversation_turn(
            "User: Here's my function: fn add(a: i32, b: i32) -> i32 { a + b }",
            "Assistant: I see your add function. It looks correct."
        ).await.unwrap();

        // 4. Action: Invoke skill via natural language
        session.mock_llm().add_response("The function is well-written");
        session.send_input("Use code-reviewer skill on that function").await.unwrap();

        // 5. Verify: SkillTool invoked
        assert!(session.tool_was_invoked("Skill"),
            "Skill tool should be invoked");

        // 6. Verify: Skill has access to prior context
        let tool_context = session.get_tool_context("Skill").unwrap();
        assert!(tool_context.contains("fn add"),
            "Skill should have access to function in conversation");

        // 7. Verify: Skill prompt injected into LLM context
        let llm_context = session.get_llm_context();
        assert!(llm_context.iter().any(|msg| msg.contains("Review the code")),
            "Skill prompt should be injected into LLM context");
        */
    }

    /// Test that skill execution uses context correctly
    #[tokio::test]
    #[ignore]
    async fn test_skill_executes_with_context() {
        todo!("Implement TestSession and TestSkillEnvironment first");

        /*
        let skill_env = TestSkillEnvironment::new()
            .with_skill(
                "summarizer",
                "Summarize the key points from the discussion"
            )
            .build();

        let mut session = TestSession::builder()
            .with_mock_llm()
            .with_skill_dir(skill_env.path())
            .build()
            .await
            .unwrap();

        // Establish multi-turn conversation
        session.add_conversation_turn(
            "What's the testing strategy?",
            "We use 60% unit, 30% integration, 10% E2E tests."
        ).await.unwrap();

        session.add_conversation_turn(
            "What about mocking?",
            "We mock external APIs but use real internal components."
        ).await.unwrap();

        // Invoke summarizer skill
        session.mock_llm().add_response("Summary: Testing pyramid with strategic mocking");
        session.send_input("Use summarizer skill").await.unwrap();

        // Verify skill execution
        assert!(session.tool_was_invoked("Skill"));

        let llm_context = session.get_llm_context();
        assert!(llm_context.iter().any(|msg|
            msg.contains("testing strategy") ||
            msg.contains("60% unit")),
            "Skill should have access to full conversation");
        */
    }

    /// Test skill with missing file shows error
    #[tokio::test]
    #[ignore]
    async fn test_missing_skill_file_error() {
        todo!("Implement TestSession first");

        /*
        let mut session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .unwrap();

        session.send_input("Use nonexistent-skill").await.unwrap();

        // Should show error about missing skill
        assert!(session.tui_contains("not found") ||
                session.tui_contains("doesn't exist"),
            "Missing skill should show error");
        */
    }

    /// Test skill in multi-turn conversation preserves context
    #[tokio::test]
    #[ignore]
    async fn test_skill_multi_turn_context_preservation() {
        todo!("Implement TestSession and TestSkillEnvironment first");

        /*
        let skill_env = TestSkillEnvironment::new()
            .with_skill("analyzer", "Analyze the discussed topic")
            .build();

        let mut session = TestSession::builder()
            .with_mock_llm()
            .with_skill_dir(skill_env.path())
            .build()
            .await
            .unwrap();

        // Turn 1: Establish context
        session.add_conversation_turn(
            "Let's talk about error handling",
            "Error handling is critical for reliability"
        ).await.unwrap();

        // Turn 2: Use skill
        session.mock_llm().add_response("Analysis: Error handling patterns identified");
        session.send_input("Use analyzer skill").await.unwrap();

        assert!(session.tool_was_invoked("Skill"));

        // Turn 3: Follow-up should still have context
        session.mock_llm().add_response("The patterns are well-established");
        session.send_input("What did you find?").await.unwrap();

        let context = session.get_llm_context();
        assert!(context.iter().any(|msg| msg.contains("error handling")),
            "Context should be preserved after skill use");
        */
    }
}
