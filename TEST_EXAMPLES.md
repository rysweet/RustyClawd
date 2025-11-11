# Interactive Mode Test Suite - Code Examples

## Test File Location
```
/Users/ryan/src/declawed/claude-code-rs/crates/cli/tests/interactive_mode_tests.rs
```

## Example Tests

### 1. Input Parsing Tests

#### Parse Bash Command
```rust
#[test]
fn test_parse_bash_command_with_exclamation() {
    // Input prefixed with ! should be recognized as bash command
    let input = "!ls -la /tmp";
    let parsed = parse_input(input);

    assert_eq!(parsed.input_type, InputType::BashCommand);
    assert_eq!(parsed.content, "ls -la /tmp");
    assert_eq!(parsed.raw_input, input);
}
```

#### Parse Slash Command with Arguments
```rust
#[test]
fn test_parse_slash_command_with_arguments() {
    // Slash commands can have arguments
    let input = "/terminal-setup bash";
    let parsed = parse_input(input);

    assert_eq!(parsed.input_type, InputType::SlashCommand);
    assert_eq!(parsed.command_name, Some("terminal-setup".to_string()));
    assert_eq!(parsed.arguments, vec!["bash"]);
}
```

#### Parse Multiline Input
```rust
#[test]
fn test_parse_multiline_input_with_backslash() {
    // Multiline support with \ + Enter
    let input = "define a function \\\nwith multiple lines";
    let parsed = parse_input(input);

    assert_eq!(parsed.is_multiline, true);
    assert!(parsed.content.contains("define a function"));
    assert!(parsed.content.contains("with multiple lines"));
}
```

### 2. Session History Tests

#### Add Messages to History
```rust
#[test]
fn test_session_history_preserves_order() {
    let session = InteractiveSession::new();

    for i in 0..5 {
        session.process_input(&format!("message {}", i)).unwrap();
    }

    assert_eq!(session.history_len(), 5);
    let history = session.get_history();
    assert_eq!(history.first().unwrap().content, "message 0");
    assert_eq!(history.last().unwrap().content, "message 4");
}
```

#### Clear History
```rust
#[test]
fn test_session_clear_history_command() {
    let session = InteractiveSession::new();

    // Add some history
    for i in 0..3 {
        session.process_input(&format!("msg {}", i)).unwrap();
    }

    assert_eq!(session.history_len(), 3);

    // Clear history via slash command
    session.process_input("/clear").unwrap();
    assert_eq!(session.history_len(), 0);
}
```

### 3. Multi-turn Conversation Tests

#### Multi-turn User/Assistant Exchange
```rust
#[test]
fn test_multi_turn_user_assistant_exchange() {
    let session = InteractiveSession::new();

    // Turn 1
    let user_input = session.process_input("What is a trait?").unwrap();
    assert_eq!(user_input.input_type, InputType::Prompt);
    session.add_assistant_response("A trait is a collection of methods...");

    // Turn 2
    session.process_input("Can I implement multiple traits?").unwrap();
    session.add_assistant_response("Yes, absolutely. You can implement...");

    // Turn 3
    session.process_input("Show me an example").unwrap();
    session.add_assistant_response("Here is an example...");

    assert_eq!(session.conversation_turn_count(), 3);

    // Verify context is maintained
    let ctx = session.get_conversation_context();
    let messages = ctx.get_all_messages();
    assert_eq!(messages.len(), 3);
}
```

### 4. Command History Navigation Tests

#### Navigate Command History
```rust
#[test]
fn test_command_history_navigation_up_arrow() {
    let history = CommandHistory::new();

    history.add_command("ls -la");
    history.add_command("cd src");
    history.add_command("cargo build");

    assert_eq!(history.navigate_up(), Some("cargo build".to_string()));
    assert_eq!(history.navigate_up(), Some("cd src".to_string()));
    assert_eq!(history.navigate_up(), Some("ls -la".to_string()));
    assert_eq!(history.navigate_up(), None); // At beginning
}
```

#### Reverse Search
```rust
#[test]
fn test_command_history_reverse_search() {
    let history = CommandHistory::new();

    history.add_command("cargo build");
    history.add_command("cargo test");
    history.add_command("cargo check");
    history.add_command("cargo build --release");

    let matches = history.search("cargo build");
    assert!(matches.contains(&"cargo build".to_string()));
    assert!(matches.contains(&"cargo build --release".to_string()));
    assert_eq!(matches.len(), 2);
}
```

### 5. Background Task Tests

#### Track Background Tasks
```rust
#[test]
fn test_background_task_tracking() {
    let bg_tasks = BackgroundTaskTracker::new();

    let task_id = bg_tasks.register_task("ls -la");
    assert!(task_id.len() > 0);

    assert!(bg_tasks.is_running(&task_id));

    bg_tasks.complete_task(&task_id, "file1\nfile2");
    assert!(!bg_tasks.is_running(&task_id));
}
```

#### Output Buffering
```rust
#[test]
fn test_background_task_output_buffering() {
    let bg_tasks = BackgroundTaskTracker::new();

    let task_id = bg_tasks.register_task("find . -type f");
    bg_tasks.buffer_output(&task_id, "chunk1");
    bg_tasks.buffer_output(&task_id, "chunk2");

    let output = bg_tasks.get_buffered_output(&task_id);
    assert_eq!(output, Some("chunk1chunk2".to_string()));
}
```

### 6. Session Management Tests

#### Session Lifecycle
```rust
#[test]
fn test_session_creation_and_initialization() {
    let session = InteractiveSession::new();

    assert_eq!(session.status(), SessionStatus::Active);
    assert_eq!(session.history_len(), 0);
    assert_eq!(session.conversation_turn_count(), 0);
}
```

#### EOF Signal
```rust
#[test]
fn test_session_handles_eof_signal() {
    let session = InteractiveSession::new();

    session.process_input("some work").unwrap();
    assert_eq!(session.status(), SessionStatus::Active);

    session.send_eof();
    assert_eq!(session.status(), SessionStatus::Closed);
}
```

#### Screen Clear Preserves History
```rust
#[test]
fn test_session_clears_screen_preserves_history() {
    let session = InteractiveSession::new();

    session.process_input("message 1").unwrap();
    session.process_input("message 2").unwrap();

    session.clear_screen();

    // Screen is cleared but history preserved
    assert_eq!(session.history_len(), 2);
    assert_eq!(session.screen_cleared(), true);
}
```

### 7. Mode Toggle Tests

#### Extended Thinking Toggle
```rust
#[test]
fn test_session_toggle_extended_thinking() {
    let session = InteractiveSession::new();

    assert_eq!(session.extended_thinking_enabled(), false);

    session.toggle_extended_thinking();
    assert_eq!(session.extended_thinking_enabled(), true);

    session.toggle_extended_thinking();
    assert_eq!(session.extended_thinking_enabled(), false);
}
```

#### Permission Mode Switching
```rust
#[test]
fn test_session_switch_permission_modes() {
    let session = InteractiveSession::new();

    assert_eq!(session.get_permission_mode(), PermissionMode::Normal);

    session.set_permission_mode(PermissionMode::AutoAccept);
    assert_eq!(session.get_permission_mode(), PermissionMode::AutoAccept);

    session.set_permission_mode(PermissionMode::Plan);
    assert_eq!(session.get_permission_mode(), PermissionMode::Plan);
}
```

### 8. Error Resilience Tests

#### Session Survives Input Errors
```rust
#[test]
fn test_session_survives_input_errors() {
    let session = InteractiveSession::new();

    let result1 = session.process_input("valid input");
    assert!(result1.is_ok());

    let _result2 = session.process_input(""); // Empty input
    // Session should handle gracefully
    assert_eq!(session.status(), SessionStatus::Active);

    let result3 = session.process_input("more valid input");
    assert!(result3.is_ok());
}
```

### 9. End-to-End Session Tests

#### Full Interactive Workflow
```rust
#[test]
fn test_full_interactive_session_workflow() {
    let session = InteractiveSession::new_in_dir("/project");

    // 1. User starts session
    assert_eq!(session.status(), SessionStatus::Active);

    // 2. Multi-turn conversation
    session.process_input("Help me create a test").unwrap();
    session.process_input("What patterns should I use?").unwrap();

    // 3. Execute bash commands
    session.process_input("!cargo test").unwrap();

    // 4. Navigate history
    let history = session.get_history();
    assert!(history.len() >= 3);

    // 5. Session remains active
    assert_eq!(session.status(), SessionStatus::Active);
}
```

#### Session with All Input Modes
```rust
#[test]
fn test_session_with_all_input_modes() {
    let session = InteractiveSession::new();

    // Standard prompt
    session.process_input("Define a function").unwrap();

    // Bash command
    session.process_input("!echo test").unwrap();

    // Slash command
    session.process_input("/clear").unwrap();

    // Memory shortcut
    session.process_input("#Important note").unwrap();

    // File mention
    session.process_input("@src/lib.rs").unwrap();

    let history = session.get_history();
    assert!(history.len() >= 3); // At least 3 items
}
```

## Running These Tests

```bash
# Run all tests
cargo test --test interactive_mode_tests

# Run specific test
cargo test --test interactive_mode_tests test_parse_bash_command_with_exclamation

# Run with output
cargo test --test interactive_mode_tests -- --nocapture

# Run category
cargo test --test interactive_mode_tests test_parse_
cargo test --test interactive_mode_tests test_session_
```

## Test Structure Pattern

All tests follow the Arrange-Act-Assert pattern:

```rust
#[test]
fn test_something() {
    // ARRANGE - Set up test state
    let session = InteractiveSession::new();
    session.process_input("setup").unwrap();

    // ACT - Perform the action being tested
    session.process_input("action").unwrap();

    // ASSERT - Verify the result
    assert_eq!(session.status(), SessionStatus::Active);
}
```

## Mock Types Available

- `InteractiveSession` - Main session orchestrator
- `ConversationContext` - Multi-turn conversation state
- `CommandHistory` - Command history with navigation
- `OutputController` - Output mode control
- `BackgroundTaskTracker` - Background task management
- `SessionMessage` - Individual history messages
- `ParsedInput` - Parsed input representation

## Running Tests with Coverage Insights

```bash
# See which tests fail
cargo test --test interactive_mode_tests 2>&1 | grep "test\|FAILED"

# Run in single thread for debugging
cargo test --test interactive_mode_tests -- --test-threads=1 --nocapture

# Show all assertions
RUST_BACKTRACE=1 cargo test --test interactive_mode_tests -- --nocapture
```
