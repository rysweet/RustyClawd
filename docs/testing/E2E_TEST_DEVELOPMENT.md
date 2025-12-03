# E2E Test Development Guide

**Audience:** Developers writing new E2E tests
**Status:** Production Guide
**Last Updated:** 2025-12-03

---

## Table of Contents

1. [Getting Started: Your First E2E Test in 15 Minutes](#getting-started-your-first-e2e-test-in-15-minutes)
2. [Overview](#overview)
3. [When to Write Each Type](#when-to-write-each-type)
4. [Part 1: Writing Programmatic E2E Tests](#part-1-writing-programmatic-e2e-tests)
5. [Part 2: Writing tmux E2E Tests](#part-2-writing-tmux-e2e-tests)
6. [Part 3: Writing YAML Scenario Tests](#part-3-writing-yaml-scenario-tests)

---

## Getting Started: Your First E2E Test in 15 Minutes

**New to E2E testing?** Follow this tutorial to write and run yer first test!

### Step 1: Create Test File (2 minutes)

Create a new test file in the programmatic tests directory:

```bash
# Create test file
touch tests/e2e/programmatic/test_my_feature.rs
```

**Location:** `/home/azureuser/src/RustyClawd/tests/e2e/programmatic/test_my_feature.rs`

### Step 2: Write Simple Test (5 minutes)

Add this complete example to yer test file:

```rust
// tests/e2e/programmatic/test_my_feature.rs

use rustyclawd_test_helpers::{TestSession, MockLLM};
use tokio;

#[tokio::test]
async fn test_help_command_integration() {
    // 1. Setup - Create test environment
    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_real_tui()
        .build()
        .await
        .expect("Failed to create test session");

    // 2. Queue mock LLM response
    session.mock_llm().add_response("Here's the help text...");

    // 3. Action - User types /help
    session.send_input("/help").await
        .expect("Failed to send input");

    // 4. Verify - Check TUI shows help
    session.await_tui_update().await;
    assert!(session.tui_contains("help"),
        "TUI should display help text");

    println!("✅ Test passed! Help command works.");
}
```

### Step 3: Run Test Locally (3 minutes)

```bash
# Run your new test
cargo test --test test_my_feature

# Expected output:
# running 1 test
# test test_help_command_integration ... ok
# ✅ Test passed! Help command works.
```

### Step 4: Verify It Passes (2 minutes)

If ye see `ok` and the success message, yer test is workin'! If not:
- Check syntax errors: `cargo check`
- Read error messages carefully
- Verify mock LLM response queued before input

### Step 5: Add to CI (3 minutes)

Yer test automatically runs in CI once committed! No extra configuration needed.

```bash
# Stage and commit
git add tests/e2e/programmatic/test_my_feature.rs
git commit -m "test: Add E2E test for help command"

# Push and watch CI
git push
gh pr checks  # Monitor CI status
```

**Congratulations!** Ye just wrote yer first E2E test in 15 minutes! 🎉

Now continue readin' to learn more advanced patterns...

---

## Overview

This guide shows ye how to write new End-to-End tests for RustyClawd. E2E tests validate complete user workflows from start to finish, ensuring TRUE 100% parity with Claude Code.

**Three Ways to Write E2E Tests:**

1. **Programmatic Tests (Rust)** - Fast, integration-level, best for component interactions
2. **tmux Tests (Bash)** - Real terminal, best for rendering and keyboard input validation
3. **YAML Scenarios** - Declarative, best for complex multi-step workflows

---

## When to Write Each Type

| Use Case | Best Test Type | Why |
|----------|---------------|-----|
| New SlashCommand integration | Programmatic | Fast, tests TUI + command interaction |
| New Skills feature | Programmatic | Tests context propagation, skill loading |
| TUI rendering change | tmux | Validates actual terminal output |
| New keyboard shortcut | tmux | Tests real input handling |
| Complex workflow (e.g., DDD) | YAML Scenario | Reusable, declarative, documents workflow |
| Multi-agent interaction | YAML Scenario | Captures complex state transitions |
| Error recovery flow | YAML Scenario | Tests failure + recovery path |

**Golden Rule:** Start with programmatic tests (fastest feedback), add tmux tests for rendering validation, create YAML scenarios for reusable documentation.

---

## Part 1: Writing Programmatic E2E Tests

### Test Structure

```rust
// tests/e2e/programmatic/test_your_feature.rs

use rustyclawd_test_helpers::{TestSession, MockLLM, TestSkillEnvironment};
use tokio;

#[tokio::test]
async fn test_your_feature_integration() {
    // 1. Setup - Create test environment
    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_real_tui()
        .with_real_tools()
        .build()
        .await
        .expect("Failed to create test session");

    // 2. Action - Perform user workflow
    session.send_input("Your user input here").await
        .expect("Failed to send input");

    // 3. Verify - Check expected outcomes
    assert!(session.tool_was_invoked("ToolName"),
        "Expected ToolName to be invoked");
    assert!(session.tui_contains("Expected text"),
        "Expected text not found in TUI");

    // 4. Cleanup - Session dropped automatically
}
```

### Example: New SlashCommand Test

```rust
/// Test that /mystats command integrates with TUI correctly
#[tokio::test]
async fn test_mystats_command_tui_integration() {
    // Setup: Create session with mock LLM
    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_real_tui()
        .build()
        .await
        .unwrap();

    // Setup: Queue mock LLM response
    session.mock_llm().add_response(
        "Your statistics show 42 commands executed."
    );

    // Action: User types /mystats
    session.send_input("/mystats").await.unwrap();

    // Verify: SlashCommandTool invoked
    assert!(session.tool_was_invoked("SlashCommand"),
        "/mystats should invoke SlashCommandTool");

    // Verify: Command expanded in LLM context
    let llm_context = session.get_llm_context();
    assert!(llm_context.contains("show my statistics"),
        "Command prompt should be expanded");

    // Verify: TUI shows command
    assert!(session.tui_contains("/mystats"),
        "TUI should display command");

    // Wait for LLM response to render
    session.await_llm_response().await.unwrap();

    // Verify: TUI shows response
    assert!(session.tui_contains("42 commands"),
        "TUI should show statistics");
}
```

### Example: Skills Context Test

```rust
/// Test that skills receive full conversation context
#[tokio::test]
async fn test_skill_receives_conversation_context() {
    // Setup: Create test skill
    let skill_env = TestSkillEnvironment::new()
        .with_skill(
            "code-reviewer",
            "Review the code mentioned in the conversation"
        )
        .build();

    // Setup: Session with skill directory
    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_skill_dir(skill_env.path())
        .build()
        .await
        .unwrap();

    // Setup: Establish conversation context
    session.add_conversation_turn(
        "User: Here's my function: fn add(a: i32, b: i32) -> i32 { a + b }",
        "Assistant: I see your add function. It looks correct."
    ).await.unwrap();

    // Action: User invokes skill
    session.send_input("Use code-reviewer skill on that function").await.unwrap();

    // Verify: SkillTool invoked
    assert!(session.tool_was_invoked("Skill"),
        "Skill tool should be invoked");

    // Verify: Skill has access to prior context
    let tool_context = session.get_tool_context("Skill").unwrap();
    assert!(tool_context.contains("fn add"),
        "Skill should have access to function in conversation");

    // Verify: Skill prompt injected
    let llm_context = session.get_llm_context();
    assert!(llm_context.contains("Review the code"),
        "Skill prompt should be injected into LLM context");
}
```

### Key Patterns for Programmatic Tests

**Pattern 1: Queuing Mock Responses**

```rust
// Queue text response
session.mock_llm().add_response("LLM response text here");

// Queue tool use
session.mock_llm().add_tool_use("Read", json!({
    "file_path": "README.md"
}));

// Queue error
session.mock_llm().add_error(ApiError::RateLimit);
```

**Important**: The queue-based response system blocks if the queue is empty. If your test hangs, verify you've queued enough responses for all expected LLM calls.

**Pattern 2: TUI State Assertions**

```rust
// Wait for TUI to update
session.await_tui_update().await;

// Check text present
assert!(session.tui_contains("expected text"));

// Check text NOT present
assert!(!session.tui_contains("unexpected text"));

// Get full TUI state for debugging
let tui_state = session.get_tui_state();
println!("TUI state: {}", tui_state);
```

**Pattern 3: Tool Invocation Checks**

```rust
// Check tool was invoked
assert!(session.tool_was_invoked("Read"));

// Check tool NOT invoked
assert!(!session.tool_was_invoked("Write"));

// Get tool parameters
let params = session.get_tool_params("Read").unwrap();
assert_eq!(params["file_path"], "README.md");

// Get tool result
let result = session.get_tool_result("Read").unwrap();
assert!(result.is_ok());
```

**Pattern 4: Conversation Context**

```rust
// Add prior conversation turns
session.add_conversation_turn(
    "User: What's in main.rs?",
    "Assistant: main.rs contains the entry point..."
).await;

// Verify context preserved
let context = session.get_llm_context();
assert!(context.contains("entry point"));
```

### Common Pitfalls

**Pitfall 1: Forgetting to queue LLM responses**
```rust
// ❌ BAD - session hangs waiting for LLM
session.send_input("Hello").await;

// ✅ GOOD - queue response first
session.mock_llm().add_response("Hi there!");
session.send_input("Hello").await;
```

**Pitfall 2: Not waiting for async operations**
```rust
// ❌ BAD - assertion runs before TUI updates
session.send_input("test").await;
assert!(session.tui_contains("test"));

// ✅ GOOD - wait for TUI to update
session.send_input("test").await;
session.await_tui_update().await;
assert!(session.tui_contains("test"));
```

**Pitfall 3: Not cleaning up test resources**
```rust
// ❌ BAD - skill directory leaked
let skill_env = TestSkillEnvironment::new()
    .with_skill("test", "prompt")
    .build();
// Test ends, directory not cleaned

// ✅ GOOD - TestSkillEnvGuard cleans up on drop
let skill_env = TestSkillEnvironment::new()
    .with_skill("test", "prompt")
    .build();  // Returns RAII guard
// Guard dropped at end of scope, directory cleaned
```

---

## Part 2: Writing tmux E2E Tests

### Test Structure

```bash
#!/bin/bash
# tests/e2e/tmux/test_your_feature.sh

set -euo pipefail

# Import framework
source "$(dirname "$0")/framework.sh"

# Test configuration
SESSION="rustyclawd-test-$$"  # Standard naming convention
trap_cleanup "$SESSION"

# Test function
test_your_feature() {
    echo "Test: Your feature description"

    # 1. Start RustyClawd in tmux
    start_rustyclawd_session "$SESSION" || exit 1

    # 2. Wait for startup
    if ! verify_output_contains "$SESSION" "Welcome"; then
        echo "FAIL: Startup failed"
        exit 1
    fi

    # 3. Send input
    send_command "$SESSION" "your input" 2

    # 4. Verify output
    if ! verify_output_contains "$SESSION" "expected output"; then
        echo "FAIL: Expected output not found"
        exit 1
    fi

    echo "PASS: Your feature works"
}

# Run test
test_your_feature

echo "✅ All tests passed"
```

### Example: New Slash Command Test

```bash
#!/bin/bash
# tests/e2e/tmux/test_mystats_command.sh

set -euo pipefail
source "$(dirname "$0")/framework.sh"

SESSION="rustyclawd-test-$$"  # Use standard naming: rustyclawd-test-$$
trap_cleanup "$SESSION"

test_mystats_command() {
    echo "Test: /mystats command in real terminal"

    # Start RustyClawd
    start_rustyclawd_session "$SESSION" 10 || exit 1

    # Verify welcome screen
    if ! verify_output_contains "$SESSION" "Welcome to RustyClawd"; then
        echo "FAIL: Welcome message not shown"
        exit 1
    fi

    # Execute /mystats command
    send_command "$SESSION" "/mystats" 3

    # Wait for command processing
    if ! wait_for_text "$SESSION" "Statistics" 10; then
        echo "FAIL: Command not processed"
        capture_output "$SESSION"  # Debug output
        exit 1
    fi

    # Verify statistics displayed
    if ! verify_output_contains "$SESSION" "commands executed"; then
        echo "FAIL: Statistics not displayed"
        exit 1
    fi

    # Verify TUI still responsive
    send_command "$SESSION" "test" 1
    if ! verify_output_contains "$SESSION" "test"; then
        echo "FAIL: TUI not responsive after command"
        exit 1
    fi

    echo "PASS: /mystats command works in terminal"
}

test_mystats_command
echo "✅ /mystats test passed"
```

### Example: Keyboard Input Test

```bash
test_keyboard_shortcuts() {
    echo "Test: Ctrl+C cancels in-progress operation"

    start_rustyclawd_session "$SESSION" || exit 1

    # Start long-running operation
    send_command "$SESSION" "/analyze large_codebase" 1

    # Wait for operation to start
    sleep 2

    # Send Ctrl+C
    tmux send-keys -t "$SESSION" C-c
    sleep 1

    # Verify operation cancelled
    if ! verify_output_contains "$SESSION" "Cancelled"; then
        echo "FAIL: Operation not cancelled"
        exit 1
    fi

    # Verify TUI still functional
    send_command "$SESSION" "hello" 1
    if ! verify_output_contains "$SESSION" "hello"; then
        echo "FAIL: TUI not responsive after cancel"
        exit 1
    fi

    echo "PASS: Ctrl+C cancels operation"
}
```

### tmux Framework Functions

**Session Management:**
```bash
# Start RustyClawd in tmux session
start_rustyclawd_session <session_name> [timeout_seconds]

# Clean up session
cleanup_session <session_name>

# Setup cleanup trap
trap_cleanup <session_name>
```

**Input:**
```bash
# Send command with Enter
send_command <session_name> "command text" [wait_seconds]

# Send raw keys (no Enter)
send_keys <session_name> "keys"

# Send control characters
tmux send-keys -t <session_name> C-c  # Ctrl+C
tmux send-keys -t <session_name> C-d  # Ctrl+D
```

**Output:**
```bash
# Capture current terminal output
capture_output <session_name>

# Save output to file
save_output <session_name> "filename.txt"
```

**Validation:**
```bash
# Check if text present
verify_output_contains <session_name> "expected text"

# Check with regex
verify_output_matches <session_name> "regex pattern"

# Wait for text to appear (with timeout)
wait_for_text <session_name> "text" <timeout_seconds>
```

**Debugging:**
```bash
# Print session info
dump_session_info <session_name>

# Take screenshot for debugging
take_screenshot <session_name> "screenshot.txt"
```

### Common tmux Test Patterns

**Pattern 1: Wait for Async Operations**
```bash
# ❌ BAD - immediate check fails
send_command "$SESSION" "/analyze src/" 1
verify_output_contains "$SESSION" "Analysis complete"

# ✅ GOOD - wait for operation
send_command "$SESSION" "/analyze src/" 1
wait_for_text "$SESSION" "Analysis complete" 30
verify_output_contains "$SESSION" "42 files"
```

**Pattern 2: Debug Failed Tests**
```bash
# Add debug output on failure
if ! verify_output_contains "$SESSION" "expected"; then
    echo "FAIL: Expected text not found"
    echo "=== Actual output ==="
    capture_output "$SESSION"
    echo "===================="
    exit 1
fi
```

**Pattern 3: Test Cleanup**
```bash
# Always use trap for cleanup with standard naming
SESSION="rustyclawd-test-$$"
trap_cleanup "$SESSION"

# Trap ensures cleanup even on test failure or Ctrl+C
```

---

## Part 3: Writing YAML Scenario Tests

### Scenario Structure

```yaml
# tests/e2e/scenarios/your-scenario.yaml
scenario:
  name: "Your Feature Workflow"
  description: "End-to-end test of your feature from user perspective"
  type: tui
  tags: [feature-name, workflow, core]

  environment:
    terminal_size:
      width: 100
      height: 30
    timeout: 30s

  steps:
    - action: launch
      description: "Start RustyClawd"
      target: "cargo run --release --bin rustyclawd"
      timeout: 10s

    - action: wait_for_text
      description: "Wait for welcome screen"
      contains: "Welcome"
      timeout: 5s

    - action: send_input
      description: "User input"
      text: "your command here"
      submit: true

    - action: wait_for_text
      description: "Wait for response"
      contains: "expected response"
      timeout: 10s

    - action: capture_screenshot
      description: "Save final state"
      filename: "your-scenario-result.txt"

    - action: send_input
      description: "Exit cleanly"
      text: "/exit"
      submit: true

  assertions:
    - type: text_present
      value: "Welcome"
      description: "Welcome screen shown"

    - type: text_present
      value: "expected response"
      description: "Feature response displayed"

    - type: exit_clean
      description: "Session exits without errors"
```

### Example: Multi-Turn Conversation Scenario

```yaml
# tests/e2e/scenarios/multi-turn-conversation.yaml
scenario:
  name: "Multi-Turn Conversation with Context"
  description: "Validate context preserved across multiple turns"
  type: tui
  tags: [conversation, context, core-workflow]

  environment:
    terminal_size:
      width: 120
      height: 40
    timeout: 60s

  steps:
    # Turn 1: Ask about file
    - action: launch
      description: "Start RustyClawd"
      target: "cargo run --bin rustyclawd"
      timeout: 10s

    - action: wait_for_text
      description: "Wait for ready state"
      contains: "Welcome"
      timeout: 5s

    - action: send_input
      description: "Ask about README"
      text: "What's in the README.md file?"
      submit: true

    - action: wait_for_text
      description: "Wait for Read tool execution"
      contains: "README"
      timeout: 15s

    # Turn 2: Reference prior context
    - action: send_input
      description: "Ask follow-up question"
      text: "What are the key points from that file?"
      submit: true

    - action: wait_for_text
      description: "Wait for summary"
      contains: "key points"
      timeout: 15s

    # Turn 3: Another context reference
    - action: send_input
      description: "Third turn referencing prior"
      text: "Create a new file based on those ideas"
      submit: true

    - action: wait_for_text
      description: "Wait for Write tool"
      contains: "Created"
      timeout: 15s

    # Cleanup
    - action: capture_screenshot
      description: "Capture final conversation"
      filename: "multi-turn-result.txt"

    - action: send_input
      description: "Exit"
      text: "/exit"
      submit: true

  assertions:
    - type: text_present
      value: "README"
      description: "First turn processed"

    - type: text_present
      value: "key points"
      description: "Second turn used context"

    - type: text_present
      value: "Created"
      description: "Third turn used context"

    - type: exit_clean
      description: "Clean exit"
```

### Example: Error Recovery Scenario

```yaml
# tests/e2e/scenarios/error-recovery.yaml
scenario:
  name: "Error Handling and Recovery"
  description: "Validate graceful error handling and recovery"
  type: tui
  tags: [error-handling, recovery, robustness]

  environment:
    terminal_size:
      width: 100
      height: 30
    timeout: 45s

  steps:
    - action: launch
      description: "Start RustyClawd"
      target: "cargo run --bin rustyclawd"
      timeout: 10s

    - action: wait_for_text
      description: "Wait for ready"
      contains: "Welcome"
      timeout: 5s

    # Trigger error: Invalid command
    - action: send_input
      description: "Invalid slash command"
      text: "/nonexistent"
      submit: true

    - action: wait_for_text
      description: "Error message shown"
      contains: "Unknown command"
      timeout: 5s

    # Verify recovery: Normal command works
    - action: send_input
      description: "Valid command after error"
      text: "/help"
      submit: true

    - action: wait_for_text
      description: "Help shown"
      contains: "Available commands"
      timeout: 5s

    # Trigger another error: Tool failure
    - action: send_input
      description: "Request nonexistent file"
      text: "Read the file /nonexistent/path.txt"
      submit: true

    - action: wait_for_text
      description: "Tool error shown"
      contains: "not found"
      timeout: 10s

    # Verify recovery again
    - action: send_input
      description: "Normal request after tool error"
      text: "What's 2+2?"
      submit: true

    - action: wait_for_text
      description: "Normal response"
      contains: "4"
      timeout: 10s

    - action: send_input
      description: "Exit"
      text: "/exit"
      submit: true

  assertions:
    - type: text_present
      value: "Unknown command"
      description: "Invalid command error shown"

    - type: text_present
      value: "Available commands"
      description: "Recovered from command error"

    - type: text_present
      value: "not found"
      description: "Tool error shown"

    - type: text_present
      value: "4"
      description: "Recovered from tool error"

    - type: exit_clean
      description: "Session still functional after errors"
```

### Scenario Actions Reference

**Launch:**
```yaml
- action: launch
  description: "What this does"
  target: "command to run"
  timeout: "10s"
```

**Wait for Text:**
```yaml
- action: wait_for_text
  description: "What we're waiting for"
  contains: "text to find"
  timeout: "5s"
```

**Send Input:**
```yaml
- action: send_input
  description: "What input does"
  text: "text to type"
  submit: true  # Press Enter
```

**Capture Screenshot:**
```yaml
- action: capture_screenshot
  description: "Why capturing"
  filename: "output.txt"
```

**Sleep (use sparingly):**
```yaml
- action: sleep
  description: "Why waiting"
  duration: "2s"
```

### Scenario Assertions Reference

**Text Present:**
```yaml
- type: text_present
  value: "text to find"
  description: "Why this matters"
```

**Text Not Present:**
```yaml
- type: text_not_present
  value: "text that shouldn't be there"
  description: "Why absence matters"
```

**Exit Clean:**
```yaml
- type: exit_clean
  description: "Session should exit gracefully"
```

**File Exists:**
```yaml
- type: file_exists
  path: "/path/to/file"
  description: "File should be created"
```

### Running Scenarios

```bash
# Run single scenario
cargo run --bin scenario_runner run --file your-scenario.yaml

# Run all scenarios in directory
cargo run --bin scenario_runner run --dir scenarios/

# Run scenarios with specific tag
cargo run --bin scenario_runner run --dir scenarios/ --tag core-workflow

# Verbose output for debugging
cargo run --bin scenario_runner run --file your-scenario.yaml --verbose
```

---

## Best Practices

### General Principles

1. **Test One Thing:** Each test validates one specific workflow or integration
2. **Clear Names:** Test names describe what's being validated
3. **Good Failure Messages:** When test fails, message explains what went wrong
4. **Independent Tests:** Tests don't depend on other tests' side effects
5. **Fast Feedback:** Programmatic tests < 10s, tmux tests < 30s, scenarios < 1 min

### Programmatic Test Best Practices

```rust
// ✅ GOOD: Clear, focused test
#[tokio::test]
async fn test_slash_command_updates_tui() {
    let mut session = setup_basic_session().await;
    session.mock_llm().add_response("Response");

    session.send_input("/help").await.unwrap();

    assert!(session.tui_contains("/help"),
        "TUI should show typed command");
}

// ❌ BAD: Tests multiple unrelated things
#[tokio::test]
async fn test_everything() {
    // Tests slash commands, skills, tools, errors...
    // Hard to debug when it fails
}
```

### tmux Test Best Practices

```bash
# ✅ GOOD: Generous timeouts, clear checks
test_feature() {
    start_rustyclawd_session "$SESSION" 10 || exit 1

    send_command "$SESSION" "/command" 2

    if ! wait_for_text "$SESSION" "expected" 15; then
        echo "FAIL: Expected text not found after 15s"
        capture_output "$SESSION"  # Debug info
        exit 1
    fi

    echo "PASS: Feature works"
}

# ❌ BAD: Tight timeouts, no debug output
test_feature() {
    start_rustyclawd_session "$SESSION" 2
    send_command "$SESSION" "/command" 0
    verify_output_contains "$SESSION" "expected"
    # Will fail intermittently due to timing
}
```

### Scenario Best Practices

```yaml
# ✅ GOOD: Descriptive, reasonable timeouts
scenario:
  name: "Clear Feature Name"
  description: "What this scenario validates"

  steps:
    - action: send_input
      description: "Why sending this input"
      text: "input"
      submit: true

    - action: wait_for_text
      description: "What this means"
      contains: "text"
      timeout: 10s  # Reasonable timeout

  assertions:
    - type: text_present
      value: "text"
      description: "Why this assertion matters"

# ❌ BAD: Vague, tight timeouts
scenario:
  name: "Test"  # Not descriptive
  steps:
    - action: send_input
      text: "input"
      # No description

    - action: wait_for_text
      contains: "text"
      timeout: 1s  # Too tight
```

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Testing Implementation Details

```rust
// ❌ BAD: Testing internal state
#[tokio::test]
async fn test_internal_buffer_size() {
    let session = TestSession::new().await;
    let buffer = session.get_internal_buffer();
    assert_eq!(buffer.capacity(), 1024);
    // Breaks when buffer implementation changes
}

// ✅ GOOD: Testing observable behavior
#[tokio::test]
async fn test_large_message_handling() {
    let mut session = TestSession::new().await;
    let large_msg = "x".repeat(10000);

    session.send_input(&large_msg).await.unwrap();

    assert!(session.tui_contains("x".repeat(100)),
        "Should handle large messages");
    // Tests what users experience
}
```

### Anti-Pattern 2: Flaky Timing

```bash
# ❌ BAD: Fixed sleep without verification
send_command "$SESSION" "/analyze" 1
sleep 5  # Hope it's done
verify_output_contains "$SESSION" "Analysis"

# ✅ GOOD: Wait with timeout and verification
send_command "$SESSION" "/analyze" 1
wait_for_text "$SESSION" "Analysis" 30  # Generous timeout
verify_output_contains "$SESSION" "42 files"
```

### Anti-Pattern 3: Unclear Failure Messages

```rust
// ❌ BAD: Useless failure message
assert!(session.tui_contains("text"));

// ✅ GOOD: Explains what failed and why
assert!(session.tui_contains("Welcome message"),
    "TUI should show welcome message after startup. \
     Actual TUI state: {}", session.get_tui_state());
```

### Anti-Pattern 4: Test Interdependence

```rust
// ❌ BAD: Tests depend on execution order
#[tokio::test]
async fn test_1_create_file() {
    // Creates test.txt
}

#[tokio::test]
async fn test_2_read_file() {
    // Assumes test.txt exists from test_1
    // BREAKS if test_1 not run first
}

// ✅ GOOD: Independent tests
#[tokio::test]
async fn test_create_file() {
    // Test creates its own test.txt
    // Cleans up at end
}

#[tokio::test]
async fn test_read_file() {
    // Test creates its own test.txt
    // Doesn't depend on other tests
}
```

---

## Debugging Failed Tests

### Programmatic Test Debugging

```rust
#[tokio::test]
async fn test_debug_example() {
    let mut session = TestSession::builder()
        .with_debug(true)  // Enable debug logging
        .build()
        .await
        .unwrap();

    // Add debug output
    eprintln!("TUI state: {}", session.get_tui_state());
    eprintln!("LLM context: {:?}", session.get_llm_context());
    eprintln!("Tools invoked: {:?}", session.get_invoked_tools());

    // Rest of test...
}
```

### tmux Test Debugging

```bash
# Run test with bash debug mode
bash -x test_feature.sh

# Add debug output in test
send_command "$SESSION" "/command" 2

# Capture and print output
echo "=== TUI Output ==="
capture_output "$SESSION"
echo "=================="

# Attach to session interactively
tmux attach -t "$SESSION"
# Ctrl+B, D to detach
```

### Scenario Debugging

```bash
# Run with verbose output
cargo run --bin scenario_runner run \
    --file scenario.yaml \
    --verbose \
    --save-screenshots

# Check screenshot files
ls -lh tests/e2e/scenarios/screenshots/

# Run scenario steps manually
# 1. Start tmux session
tmux new-session -s debug

# 2. Run RustyClawd
cargo run --bin rustyclawd

# 3. Execute scenario steps by hand
# 4. Observe what actually happens
```

---

## Examples by Feature Type

### Testing New Tools

```rust
#[tokio::test]
async fn test_new_tool_integration() {
    let mut session = TestSession::builder()
        .with_mock_llm()
        .with_real_tools()  // Includes your new tool
        .build()
        .await
        .unwrap();

    // Queue LLM to use your tool
    session.mock_llm().add_tool_use("YourNewTool", json!({
        "param": "value"
    }));

    session.send_input("Invoke my new tool").await.unwrap();

    // Verify tool executed
    assert!(session.tool_was_invoked("YourNewTool"));

    // Verify tool result returned to LLM
    let result = session.get_tool_result("YourNewTool").unwrap();
    assert!(result.is_ok());
}
```

### Testing New Hooks

```rust
#[tokio::test]
async fn test_new_hook_fires() {
    let hooks = HooksSystem::new_with_real_hooks();

    let mut session = TestSession::builder()
        .with_hooks(hooks.clone())
        .build()
        .await
        .unwrap();

    session.send_input("Trigger hook").await.unwrap();

    // Verify your new hook fired
    assert!(hooks.hook_fired("YourNewHook"),
        "YourNewHook should fire on this action");
}
```

### Testing Error Handling

```rust
#[tokio::test]
async fn test_graceful_error_handling() {
    let mut session = TestSession::new().await;

    // Trigger error
    session.mock_llm().add_error(ApiError::RateLimit);
    session.send_input("Request").await.unwrap();

    // Verify error displayed gracefully
    session.await_tui_update().await;
    assert!(session.tui_contains("Rate limit"),
        "Should show user-friendly error");

    // Verify session still functional
    session.mock_llm().add_response("OK");
    session.send_input("Next request").await.unwrap();

    assert!(session.tui_contains("OK"),
        "Should recover from error");
}
```

---

## Performance Considerations

### Keep Tests Fast

**Target Times:**
- Programmatic: < 10 seconds each
- tmux: < 30 seconds each
- Scenarios: < 1 minute each

**Optimization Strategies:**

1. **Use MockLLM (don't call real API)**
   - Programmatic tests: Always use mock
   - tmux tests: Can use mock via environment variable
   - Scenarios: Use mock for most tests, real API for smoke tests

2. **Minimize sleep/wait times**
   - Use `wait_for_text` with reasonable timeouts
   - Don't add unnecessary delays

3. **Parallel execution where possible**
   - Programmatic tests: Run in parallel (Rust native)
   - tmux tests: Sequential (terminal conflicts)
   - Scenarios: Sequential (use tmux)

4. **Cleanup resources**
   - Kill tmux sessions promptly
   - Clean up test files/directories
   - Drop test sessions (RAII cleanup)

---

## CI Integration

### Running Tests in CI

```yaml
# .github/workflows/e2e-tests.yml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e-programmatic:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run programmatic E2E tests
        run: cargo test --test e2e_programmatic

  e2e-tmux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install tmux
        run: sudo apt-get install -y tmux

      - name: Build RustyClawd
        run: cargo build --release

      - name: Run tmux E2E tests
        run: |
          cd tests/e2e/tmux
          bash run_all.sh

  e2e-scenarios:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Install dependencies
        run: |
          sudo apt-get install -y tmux
          pip3 install pyyaml

      - name: Run scenario tests
        run: |
          cargo build --bin scenario_runner
          cd tests/e2e/scenarios
          cargo run --bin scenario_runner run --dir .
```

---

## Questions?

**Architecture:** `docs/architecture/e2e_testing_architecture.md`
**Module Specs:** `docs/specs/`
**User Guide:** [`E2E_TESTING.md`](./E2E_TESTING.md)
**Parity Validation:** [`PARITY_VALIDATION.md`](./PARITY_VALIDATION.md)

---

**Last Updated:** 2025-12-03
**Test Framework Version:** 1.0
**Parity Level:** TRUE 100%
