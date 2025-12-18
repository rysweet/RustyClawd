# Module Specification: TestSession

**Module Type:** Test Infrastructure
**Layer:** E2E Test Helpers
**Purpose:** Orchestrate full interactive session testing with controllable I/O

---

## Philosophy

**Single Responsibility:** Manage complete interactive session lifecycle for E2E testing

**Regeneratable:** Can be rebuilt from this spec without breaking dependent tests

**Self-Contained:** All session orchestration logic in one module

**Standard Library Preference:** Use std where possible, tokio for async only

---

## Public API (Studs)

### Main Type

```rust
/// Orchestrates full interactive session for E2E testing
///
/// Provides controllable I/O, state inspection, and mock LLM integration
/// for end-to-end validation of complete user workflows.
pub struct TestSession {
    // Private implementation details
}
```

### Builder Pattern

```rust
/// Builder for configuring TestSession
pub struct TestSessionBuilder {
    // Configuration fields
}

impl TestSessionBuilder {
    /// Use mock LLM client instead of real API
    pub fn with_mock_llm(self) -> Self;

    /// Use real TUI rendering (TestBackend)
    pub fn with_real_tui(self) -> Self;

    /// Inject custom hooks system
    pub fn with_hooks(self, hooks: HooksSystem) -> Self;

    /// Set skill directory path
    pub fn with_skill_dir(self, path: PathBuf) -> Self;

    /// Set working directory for session
    pub fn with_working_dir(self, path: PathBuf) -> Self;

    /// Enable test mode (no external API calls)
    pub fn with_test_mode(self, enabled: bool) -> Self;

    /// Build configured TestSession
    pub async fn build(self) -> Result<TestSession>;
}
```

### Session Interaction

```rust
impl TestSession {
    /// Create new session builder
    pub fn builder() -> TestSessionBuilder;

    /// Send user input to session (simulates typing)
    ///
    /// # Example
    /// ```rust
    /// session.send_input("/analyze src/").await?;
    /// ```
    pub async fn send_input(&mut self, input: &str) -> Result<()>;

    /// Inject mock LLM text response
    ///
    /// # Example
    /// ```rust
    /// session.inject_llm_response("Analysis: Found 42 modules").await?;
    /// ```
    pub async fn inject_llm_response(&mut self, response: &str) -> Result<()>;

    /// Inject mock LLM tool use
    ///
    /// # Example
    /// ```rust
    /// session.inject_llm_tool_use(
    ///     "Read",
    ///     json!({"file_path": "README.md"})
    /// ).await?;
    /// ```
    pub async fn inject_llm_tool_use(
        &mut self,
        tool_name: &str,
        params: serde_json::Value
    ) -> Result<()>;

    /// Check if specific tool was invoked
    ///
    /// # Example
    /// ```rust
    /// assert!(session.tool_was_invoked("SlashCommand"));
    /// ```
    pub fn tool_was_invoked(&self, tool_name: &str) -> bool;

    /// Get all messages sent to LLM
    ///
    /// Returns vector of message contents for verification.
    pub fn get_llm_context(&self) -> Vec<String>;

    /// Check if TUI output contains text
    ///
    /// # Example
    /// ```rust
    /// assert!(session.tui_contains("Welcome to RustyClawd"));
    /// ```
    pub fn tui_contains(&self, text: &str) -> bool;

    /// Get tool invocation context/parameters
    ///
    /// Returns the context passed to tool when it was invoked.
    pub fn get_tool_context(&self, tool_name: &str) -> Option<String>;

    /// Add conversation turn (user + assistant messages)
    ///
    /// Used to establish conversation history before test actions.
    ///
    /// # Example
    /// ```rust
    /// session.add_conversation_turn(
    ///     "What's in main.rs?",
    ///     "main.rs contains the entry point..."
    /// ).await?;
    /// ```
    pub async fn add_conversation_turn(
        &mut self,
        user_msg: &str,
        assistant_msg: &str
    ) -> Result<()>;

    /// Wait for tool execution to complete
    ///
    /// Blocks until tool result available or timeout.
    pub async fn wait_for_tool_result(&mut self) -> Result<ToolResult>;

    /// Get captured TUI output as string
    pub fn get_tui_output(&self) -> String;

    /// Get hooks system for verification
    pub fn hooks(&self) -> &HooksSystem;

    /// Shutdown session cleanly
    pub async fn shutdown(self) -> Result<()>;
}
```

---

## Dependencies

### External Crates

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
ratatui = "0.26"
serde_json = "1.0"
anyhow = "1.0"
```

### Internal Modules

- `crates/cli/src/interactive.rs` - InteractiveSession
- `crates/api_client/` - ApiClient trait
- `crates/hooks/` - HooksSystem
- `crates/cli/src/tools/` - Tool executor
- `tests/tui_test_harness.rs` - TUI TestBackend
- `tests/e2e/mocks/mock_llm.rs` - MockLLM

---

## Implementation Notes

### Key Design Decisions

**1. Builder Pattern**
- Rationale: Flexible configuration without constructor explosion
- Alternatives: Multiple constructors, config struct
- Trade-off: Slightly more verbose, much more flexible

**2. Async API**
- Rationale: Matches real interactive session behavior
- Alternatives: Synchronous with blocking
- Trade-off: Requires tokio runtime, more realistic

**3. Mock vs Real Components**
- Mock: LLM client (external API, non-deterministic)
- Real: TUI, Tools, Hooks (internal, deterministic)
- Rationale: Test real behavior where possible

### State Management

```rust
// Internal state structure (private)
struct TestSessionState {
    conversation_history: Vec<Message>,
    tool_invocations: Vec<ToolInvocation>,
    llm_responses: VecDeque<MockResponse>,
    tui_harness: TuiTestHarness,
    hooks_system: HooksSystem,
    mock_llm: MockLLM,
}
```

### Error Handling

- Use `Result<T, anyhow::Error>` for all operations
- Provide context with `.context()` on errors
- Clean up resources in Drop implementation
- Fail fast on configuration errors

### Testing Strategy

**Unit Tests (60%):**
- Test builder configuration
- Test state tracking
- Test mock LLM queue

**Integration Tests (30%):**
- Test with real TUI harness
- Test with real tools
- Test with real hooks

**E2E Tests (10%):**
- Test complete workflows
- Test multiple turns
- Test error scenarios

---

## Test Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default_config() {
        let builder = TestSession::builder();
        // Verify default configuration
    }

    #[test]
    fn test_tool_invocation_tracking() {
        let mut session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .unwrap();

        session.record_tool_invocation("Read", "{}");
        assert!(session.tool_was_invoked("Read"));
        assert!(!session.tool_was_invoked("Write"));
    }

    #[test]
    fn test_llm_context_accumulation() {
        let mut session = TestSession::builder()
            .with_mock_llm()
            .build()
            .await
            .unwrap();

        session.add_conversation_turn("Hello", "Hi!").await.unwrap();
        session.add_conversation_turn("What's up?", "Not much").await.unwrap();

        let context = session.get_llm_context();
        assert_eq!(context.len(), 4); // 2 turns = 4 messages
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_slash_command_integration() {
    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_real_tui()
        .build()
        .await
        .unwrap();

    // Send slash command
    session.send_input("/analyze src/").await.unwrap();

    // Verify SlashCommandTool invoked
    assert!(session.tool_was_invoked("SlashCommand"));

    // Verify command in TUI
    assert!(session.tui_contains("/analyze"));
}
```

---

## Usage Examples

### Example 1: Simple Message Test

```rust
#[tokio::test]
async fn test_simple_message() {
    let mut session = TestSession::builder()
        .with_mock_llm()
        .build()
        .await?;

    // User sends message
    session.send_input("Hello, Claude!").await?;

    // Mock response
    session.inject_llm_response("Hello! How can I help?").await?;

    // Verify in TUI
    assert!(session.tui_contains("Hello! How can I help?"));
}
```

### Example 2: Tool Use Test

```rust
#[tokio::test]
async fn test_tool_execution() {
    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_hooks(HooksSystem::new())
        .build()
        .await?;

    // User requests file read
    session.send_input("Read README.md").await?;

    // Mock LLM tool use
    session.inject_llm_tool_use(
        "Read",
        json!({"file_path": "README.md"})
    ).await?;

    // Verify tool invoked
    assert!(session.tool_was_invoked("Read"));

    // Verify PreToolUse hook
    assert!(session.hooks().hook_fired("PreToolUse"));

    // Wait for result
    let result = session.wait_for_tool_result().await?;
    assert!(result.is_success());
}
```

### Example 3: Multi-Turn Conversation

```rust
#[tokio::test]
async fn test_multi_turn_conversation() {
    let mut session = TestSession::builder()
        .with_mock_llm()
        .build()
        .await?;

    // Turn 1
    session.add_conversation_turn(
        "What's in main.rs?",
        "main.rs contains the entry point..."
    ).await?;

    // Turn 2
    session.add_conversation_turn(
        "Can you explain the CLI args?",
        "The CLI accepts several arguments..."
    ).await?;

    // Verify context preserved
    let context = session.get_llm_context();
    assert!(context.len() >= 4); // At least 2 turns
    assert!(context.iter().any(|msg| msg.contains("entry point")));
}
```

---

## Performance Considerations

**Session Startup:**
- Target: < 100ms for test session creation
- Real session startup slower due to I/O
- Mock LLM eliminates network latency

**Memory Usage:**
- Store full conversation history (typical: 10-20 messages)
- TUI buffer (80x24 cells = ~2KB)
- Tool invocation records (minimal)
- Total: < 1MB per session

**Cleanup:**
- Automatic cleanup via Drop
- Explicit shutdown for graceful termination
- No resource leaks

---

## Future Enhancements

**Phase 2 (If Needed):**
- Support for multiple concurrent sessions
- Recording/replay of interactions
- Integration with real API (optional)
- Performance metrics collection

**NOT Planned:**
- Windows-specific testing (Phase 1 focuses on Linux/macOS)
- GUI testing (TUI only)
- Network simulation (mock LLM sufficient)

---

## Contract Verification

**This module succeeds when:**

1. ✅ TestSession can orchestrate full interactive session
2. ✅ Mock LLM provides controllable responses
3. ✅ TUI state can be inspected and verified
4. ✅ Tool invocations are tracked correctly
5. ✅ Hooks system integration works
6. ✅ Conversation context preserved across turns
7. ✅ Clean startup and shutdown
8. ✅ All tests pass consistently

**This module fails if:**

- Session state becomes inconsistent
- Mock LLM doesn't match real API behavior
- TUI verification doesn't work
- Resource leaks occur
- Tests are flaky or unreliable

---

## See Also

- [E2E Testing Architecture](../architecture/e2e_testing_architecture.md)
- [MockLLM Specification](mock_llm_spec.md)
- [TuiTestHarness](../../tests/tui_test_harness.rs)
