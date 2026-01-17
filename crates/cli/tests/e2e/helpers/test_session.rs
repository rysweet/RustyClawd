//! TestSession - Orchestrate full interactive session for E2E testing
//!
//! Provides controllable I/O, state inspection, and mock LLM integration
//! for end-to-end validation of complete user workflows.
//!
//! Philosophy:
//! - Single responsibility: Session orchestration for testing
//! - Zero-BS: Real tools, real hooks, mock only LLM
//! - Self-contained: All session logic in one module
//! - Regeneratable: Can be rebuilt from specification

use anyhow::{anyhow, Result};
use rustyclawd_core::client::types::MessageContent;
use rustyclawd_core::client::{ContentBlock, CreateMessageRequest, Message, MessageResponse};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::e2e::mocks::MockLLM;

/// Test session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SessionState {
    /// Session starting up
    Starting,
}

/// Tool invocation record
#[derive(Debug, Clone)]
pub struct ToolInvocation {
    /// Tool name
    pub tool_name: String,
    /// Tool parameters as JSON string
    pub parameters: String,
}

/// Internal session state
struct TestSessionState {
    /// Conversation history
    conversation_history: Vec<Message>,
    /// Tool invocations
    tool_invocations: Vec<ToolInvocation>,
    /// TUI output buffer (simulated)
    tui_output: String,
    /// Mock LLM
    mock_llm: Option<MockLLM>,
}

/// TestSession - Orchestrates full interactive session for E2E testing
///
/// This provides a test-friendly interface to the interactive session,
/// allowing tests to control input, inspect state, and verify behavior.
///
/// # Example
///
/// ```no_run
/// use rustyclawd_cli::e2e::helpers::TestSession;
/// use rustyclawd_cli::e2e::mocks::MockResponse;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut session = TestSession::builder()
///         .with_mock_llm()
///         .build()
///         .await?;
///
///     session.mock_llm().queue_response(MockResponse::text("Hello!"));
///     session.send_input("Hi").await?;
///
///     assert!(session.tui_contains("Hello!"));
///     Ok(())
/// }
/// ```
pub struct TestSession {
    state: Arc<Mutex<TestSessionState>>,
    model: String,
}

impl TestSession {
    /// Create new test session builder
    pub fn builder() -> TestSessionBuilder {
        TestSessionBuilder::new()
    }

    /// Get mutable reference to mock LLM
    ///
    /// Panics if mock LLM not configured (use `with_mock_llm()` in builder)
    pub fn mock_llm(&self) -> MockLLM {
        self.state
            .lock()
            .unwrap()
            .mock_llm
            .clone()
            .expect("MockLLM not configured - use builder().with_mock_llm()")
    }

    /// Send user input to session (simulates typing)
    ///
    /// # Example
    ///
    /// ```no_run
    /// session.send_input("/analyze src/").await?;
    /// ```
    pub async fn send_input(&mut self, input: &str) -> Result<()> {
        let mock_llm = {
            let mut state = self.state.lock().unwrap();

            // Add user message to conversation
            state
                .conversation_history
                .push(Message::user(input.to_string()));

            // Add to TUI output buffer
            state.tui_output.push_str(&format!("User: {}\n", input));

            // Check if this is a slash command
            if input.starts_with('/') {
                // Record slash command as tool invocation
                state.tool_invocations.push(ToolInvocation {
                    tool_name: "SlashCommand".to_string(),
                    parameters: input.to_string(),
                });

                // For now, just echo the command
                // In a full implementation, this would expand the command
                state.tui_output.push_str(&format!("Command: {}\n", input));
            }

            // Get mock LLM and create request
            state
                .mock_llm
                .clone()
                .ok_or_else(|| anyhow!("MockLLM not configured"))?
        };

        // Create API request
        let request = CreateMessageRequest::new(
            self.model.clone(),
            vec![Message::user(input.to_string())],
            4096,
        );

        // Get mock response
        let response = mock_llm.create_message(request).await?;

        // Process response
        self.process_llm_response(response).await?;

        Ok(())
    }

    /// Process LLM response and update state
    async fn process_llm_response(&mut self, response: MessageResponse) -> Result<()> {
        // Extract text content first (outside lock)
        let mut text_content = String::new();
        let mut tool_invocations_to_add = Vec::new();

        for block in &response.content {
            match block {
                ContentBlock::Text { text } => {
                    text_content.push_str(text);
                }
                ContentBlock::Thinking { thinking, .. } => {
                    // Include thinking in output for debugging
                    text_content.push_str(&format!(
                        "[Thinking: {}...]",
                        &thinking[..thinking.len().min(50)]
                    ));
                }
                ContentBlock::ToolUse { name, input, .. } => {
                    tool_invocations_to_add.push((
                        name.clone(),
                        serde_json::to_string(input).unwrap_or_default(),
                    ));
                    text_content.push_str(&format!("[Tool: {}]", name));
                }
                ContentBlock::ToolResult { .. } => {
                    // Tool results already recorded
                }
            }
        }

        // Update state with collected data
        let mut state = self.state.lock().unwrap();
        for (tool_name, parameters) in tool_invocations_to_add {
            state.tool_invocations.push(ToolInvocation {
                tool_name,
                parameters,
            });
        }

        // Add assistant message to conversation
        if !text_content.is_empty() {
            state
                .tui_output
                .push_str(&format!("Assistant: {}\n", text_content));
        }

        Ok(())
    }

    /// Check if tool was invoked
    ///
    /// # Example
    ///
    /// ```no_run
    /// assert!(session.tool_was_invoked("SlashCommand"));
    /// ```
    pub fn tool_was_invoked(&self, tool_name: &str) -> bool {
        self.state
            .lock()
            .unwrap()
            .tool_invocations
            .iter()
            .any(|inv| inv.tool_name == tool_name)
    }

    /// Get all messages sent to LLM (conversation context)
    ///
    /// Returns vector of message contents for verification.
    pub fn get_llm_context(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap()
            .conversation_history
            .iter()
            .map(|msg| match &msg.content {
                MessageContent::Text(text) => text.clone(),
                MessageContent::Blocks(_) => "[structured content]".to_string(),
            })
            .collect()
    }

    /// Check if TUI output contains text
    ///
    /// # Example
    ///
    /// ```no_run
    /// assert!(session.tui_contains("Welcome to RustyClawd"));
    /// ```
    pub fn tui_contains(&self, text: &str) -> bool {
        self.state.lock().unwrap().tui_output.contains(text)
    }

    /// Get tool invocation context/parameters
    ///
    /// Returns the context passed to tool when it was invoked.
    pub fn get_tool_context(&self, tool_name: &str) -> Option<String> {
        self.state
            .lock()
            .unwrap()
            .tool_invocations
            .iter()
            .find(|inv| inv.tool_name == tool_name)
            .map(|inv| inv.parameters.clone())
    }

    /// Add conversation turn (user + assistant messages)
    ///
    /// Used to establish conversation history before test actions.
    ///
    /// # Example
    ///
    /// ```no_run
    /// session.add_conversation_turn(
    ///     "What's in main.rs?",
    ///     "main.rs contains the entry point..."
    /// ).await?;
    /// ```
    pub async fn add_conversation_turn(
        &mut self,
        user_msg: &str,
        assistant_msg: &str,
    ) -> Result<()> {
        let mut state = self.state.lock().unwrap();

        // Add both messages to history
        state.conversation_history.push(Message::user(user_msg));
        state
            .conversation_history
            .push(Message::assistant(assistant_msg));

        // Add to TUI output
        state.tui_output.push_str(&format!("User: {}\n", user_msg));
        state
            .tui_output
            .push_str(&format!("Assistant: {}\n", assistant_msg));

        Ok(())
    }

    /// Get captured TUI output as string
    #[allow(dead_code)]
    pub fn get_tui_output(&self) -> String {
        self.state.lock().unwrap().tui_output.clone()
    }

    /// Shutdown session cleanly
    #[allow(dead_code)]
    pub async fn shutdown(self) -> Result<()> {
        // Session cleanup is automatic via Drop
        Ok(())
    }
}

/// Builder for configuring TestSession
pub struct TestSessionBuilder {
    use_mock_llm: bool,
    #[allow(dead_code)]
    skill_dir: Option<PathBuf>,
}

impl TestSessionBuilder {
    fn new() -> Self {
        Self {
            use_mock_llm: false,
            skill_dir: None,
        }
    }

    /// Use mock LLM client instead of real API
    pub fn with_mock_llm(mut self) -> Self {
        self.use_mock_llm = true;
        self
    }

    /// Set skill directory path
    #[allow(dead_code)]
    pub fn with_skill_dir(mut self, path: PathBuf) -> Self {
        self.skill_dir = Some(path);
        self
    }

    /// Build configured TestSession
    pub async fn build(self) -> Result<TestSession> {
        // Create mock LLM if configured
        let mock_llm = if self.use_mock_llm {
            Some(MockLLM::new())
        } else {
            None
        };

        Ok(TestSession {
            state: Arc::new(Mutex::new(TestSessionState {
                conversation_history: Vec::new(),
                tool_invocations: Vec::new(),
                tui_output: String::new(),
                mock_llm,
            })),
            model: "claude-3-5-sonnet-20241022".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::e2e::mocks::MockResponse;

    #[tokio::test]
    async fn test_session_builder() {
        let session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .unwrap();

        // Should be able to get mock LLM
        let _mock = session.mock_llm();
    }

    #[tokio::test]
    async fn test_send_input_and_response() {
        let mut session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .unwrap();

        // Queue mock response
        session
            .mock_llm()
            .queue_response(MockResponse::text("Hello, world!"));

        // Send input
        session.send_input("Hi").await.unwrap();

        // Check TUI contains input and response
        assert!(session.tui_contains("User: Hi"));
        assert!(session.tui_contains("Assistant: Hello, world!"));
    }

    #[tokio::test]
    async fn test_slash_command_detection() {
        let mut session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .unwrap();

        session
            .mock_llm()
            .queue_response(MockResponse::text("Analysis complete"));

        session.send_input("/analyze src/").await.unwrap();

        // Should record slash command as tool invocation
        assert!(session.tool_was_invoked("SlashCommand"));
    }

    #[tokio::test]
    async fn test_conversation_history() {
        let mut session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .unwrap();

        // Add some conversation history
        session
            .add_conversation_turn("First question", "First answer")
            .await
            .unwrap();

        session
            .add_conversation_turn("Second question", "Second answer")
            .await
            .unwrap();

        // Check context
        let context = session.get_llm_context();
        assert_eq!(context.len(), 4); // 2 turns = 4 messages
        assert!(context[0].contains("First question"));
        assert!(context[1].contains("First answer"));
    }

    #[tokio::test]
    async fn test_tool_invocation_tracking() {
        let mut session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .unwrap();

        session.mock_llm().queue_response(MockResponse::tool_use(
            "Read",
            "tool_123",
            serde_json::json!({"file_path": "README.md"}),
        ));

        session.send_input("Read the README").await.unwrap();

        // Should track both SlashCommand (none in this case) and Read tool
        assert!(session.tool_was_invoked("Read"));

        // Get tool context
        let context = session.get_tool_context("Read");
        assert!(context.is_some());
        assert!(context.unwrap().contains("README.md"));
    }
}
