//! End-to-End Test: SlashCommand TUI Integration
//!
//! This test validates that slash commands work seamlessly with the TUI,
//! including command expansion, tool invocation, and TUI rendering.
//!
//! **Status:** FAILING - Waiting for TestSession and MockLLM implementation
//!
//! **What This Tests:**
//! 1. SlashCommandTool invoked when user types `/analyze`
//! 2. Command prompt expanded and sent to LLM
//! 3. TUI displays command input
//! 4. TUI renders LLM response
//! 5. Session state remains consistent

#[cfg(test)]
mod slash_command_tui_integration_tests {
    use crate::e2e::helpers::TestSession;
    use crate::e2e::mocks::MockResponse;

    /// Test that /analyze command integrates correctly with TUI
    ///
    /// **Expected Behavior:**
    /// 1. User types `/analyze src/`
    /// 2. SlashCommandTool is invoked
    /// 3. Command is expanded to "Analyze the codebase at src/"
    /// 4. TUI shows `/analyze src/` in conversation
    /// 5. Mock LLM response "Analysis: Found 42 modules" appears in TUI
    #[tokio::test]
    async fn test_slash_command_displayed_in_tui() {
        // 1. Setup: Create test session with mock LLM
        let mut session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .expect("Failed to create test session");

        // 2. Queue mock LLM response
        session
            .mock_llm()
            .queue_response(MockResponse::text("Analysis: Found 42 modules"));

        // 3. Action: User types /analyze src/
        session
            .send_input("/analyze src/")
            .await
            .expect("Failed to send input");

        // 4. Verify: SlashCommandTool invoked
        assert!(
            session.tool_was_invoked("SlashCommand"),
            "/analyze should invoke SlashCommandTool"
        );

        // 5. Verify: TUI shows command
        assert!(
            session.tui_contains("/analyze src/"),
            "TUI should display typed command"
        );

        // 6. Verify: TUI shows response
        assert!(
            session.tui_contains("42 modules"),
            "TUI should show analysis results"
        );
    }

    /// Test that slash command expands correctly
    #[tokio::test]
    async fn test_slash_command_expansion() {
        let mut session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .unwrap();

        session
            .mock_llm()
            .queue_response(MockResponse::text("Debug mode enabled"));

        session.send_input("/debug").await.unwrap();

        // Verify: SlashCommandTool invoked
        assert!(
            session.tool_was_invoked("SlashCommand"),
            "/debug should invoke SlashCommandTool"
        );

        // Verify: Command appears in LLM context
        let context = session.get_llm_context();
        assert!(
            context.iter().any(|msg| msg.contains("/debug")),
            "Command should appear in LLM context"
        );
    }

    /// Test that slash command output appears in conversation
    #[tokio::test]
    async fn test_slash_command_output_in_conversation() {
        let mut session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .unwrap();

        session
            .mock_llm()
            .queue_response(MockResponse::text("Analysis complete"));

        session.send_input("/analyze src/").await.unwrap();

        // Verify both command and response appear in TUI
        let tui_output = session.get_tui_output();
        assert!(
            tui_output.contains("/analyze src/"),
            "TUI should show command"
        );
        assert!(
            tui_output.contains("Analysis complete"),
            "TUI should show response"
        );
    }

    /// Test that TUI state updates correctly
    #[tokio::test]
    async fn test_slash_command_tui_state_update() {
        let mut session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .unwrap();

        // Queue response
        session
            .mock_llm()
            .queue_response(MockResponse::text("Analysis result"));

        // Initial state - no output
        assert_eq!(
            session.get_tui_output().trim(),
            "",
            "TUI should be empty initially"
        );

        // Send command
        session.send_input("/analyze src/").await.unwrap();

        // State should update with both command and response
        let tui_output = session.get_tui_output();
        assert!(
            tui_output.contains("User: /analyze src/"),
            "TUI should show user input"
        );
        assert!(
            tui_output.contains("Command: /analyze src/"),
            "TUI should show command processing"
        );
        assert!(
            tui_output.contains("Assistant: Analysis result"),
            "TUI should show assistant response"
        );
    }
}
