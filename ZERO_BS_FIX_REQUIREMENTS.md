# Zero-BS Philosophy Violations: Comprehensive Fix Requirements

## Document Metadata

- **Type**: Bug Fix / Refactoring
- **Priority**: Critical
- **Complexity**: Complex (3-5 days)
- **Estimated Effort**: 3-5 days
- **Approach**: TDD (Test-First Development)
- **Quality Score Target**: 100% (no suppressed warnings)

## Executive Summary

This specification addresses 14 zero-BS philosophy violations found across the RustyClawd CLI crate. The violations range from global warning suppressions to fake implementations, placeholder stubs, and incomplete functionality. The goal is to eliminate all technical dishonesty by either implementing proper working functionality or removing incomplete features entirely.

## Zero-BS Philosophy Core Principles

1. **No Fake Code**: Every function must work or not exist
2. **No Warning Suppressions**: Fix underlying issues, don't hide them
3. **No Placeholders**: Complete implementations or remove them
4. **Test Everything**: Write tests before implementation
5. **Quality First**: Maintainability over quick fixes

## Violations Summary

### Critical (2)
- **V1**: Global dead_code suppression in `/home/azureuser/src/RustyClawd/crates/cli/src/lib.rs`
- **V2**: Global unused_imports suppression in `/home/azureuser/src/RustyClawd/crates/cli/src/lib.rs`

### High (2)
- **V3**: Fake TUI API response "I'm here! (API integration coming soon)" in `/home/azureuser/src/RustyClawd/crates/cli/src/tui/ui.rs:529`
- **V4**: Stub builtin command implementations returning placeholder strings in `/home/azureuser/src/RustyClawd/crates/cli/src/commands/builtins.rs`

### Medium (3)
- **V5**: Plugin executor placeholder - command execution returns success without actually running in `/home/azureuser/src/RustyClawd/crates/cli/src/plugins/executor.rs:78-89`
- **V6**: Multiple unimplemented subcommands with placeholder returns in `commands/executor.rs`
- **V7**: Non-compiling example `/home/azureuser/src/RustyClawd/crates/cli/examples/verify_schemas.rs` (missing OpenSSL dependencies)

### Low (7)
- **V8**: Swallowed error conversions with generic error messages using `map_err(|e| ...)` (10 files)
- **V9**: TDD test stubs - 726 tests across 37 files (many may be incomplete)
- **V10**: Ignored test `test_matcher_mcp_full_pattern` in `/home/azureuser/src/RustyClawd/crates/cli/tests/hooks_doc_tests.rs:198`
- **V11**: Ignored test `test_scenario_mcp_tool_pattern_matching` in same file at line 1310
- **V12**: Example code using `.unwrap()` in `commands/executor.rs:37` (non-production context)
- **V13**: Terminal cleanup using `let _ = ` in TUI Drop impl at `tui/ui.rs:483`
- **V14**: Current directory fallback using `unwrap_or_else` in `commands/executor.rs:37`

---

## PART 1: CRITICAL VIOLATIONS (V1, V2)

### Violation V1 & V2: Global Warning Suppressions

**File**: `/home/azureuser/src/RustyClawd/crates/cli/src/lib.rs` (lines 6-7)

**Current State**:
```rust
#![allow(dead_code)]
#![allow(unused_imports)]
```

**Problem**: These global suppressions hide real issues across the entire crate.

**Objective**: Remove both suppressions and fix all underlying dead code and unused import warnings.

### Success Criteria

1. **Removal Verification**
   - [ ] Both `#![allow(dead_code)]` and `#![allow(unused_imports)]` removed from lib.rs
   - [ ] `cargo build` completes with zero warnings in CLI crate
   - [ ] `cargo clippy` shows no suppressed warnings

2. **Dead Code Resolution**
   - [ ] All genuinely unused code removed
   - [ ] All used code properly exported or made public where needed
   - [ ] Module visibility correctly configured
   - [ ] No false-positive dead code remains

3. **Unused Import Resolution**
   - [ ] All unused imports removed
   - [ ] All necessary imports retained
   - [ ] Import organization follows Rust conventions

### Implementation Strategy

#### Phase 1: Compile and Catalog
```bash
# Remove suppressions temporarily
cargo check 2>&1 | grep "warning: " | tee warnings.txt
# Categorize warnings by type and file
```

#### Phase 2: Fix Dead Code
- Audit each module for actual usage
- Remove genuinely unused functions/types
- Add `pub` visibility where code is used but not exported
- Add unit tests for legitimately used internal code

#### Phase 3: Fix Unused Imports
- Remove imports not referenced in code
- Consolidate duplicate imports
- Organize imports: std, external crates, local modules

### Acceptance Criteria

- [ ] `cargo build --all-features` produces zero warnings
- [ ] `cargo clippy --all-features` produces zero warnings
- [ ] All modules compile and link successfully
- [ ] No functionality regression (all existing tests pass)
- [ ] Documentation builds without warnings

### Test Strategy

**Pre-Implementation Tests**:
```rust
#[test]
fn test_no_global_warning_suppressions() {
    let lib_rs = std::fs::read_to_string("src/lib.rs").unwrap();
    assert!(!lib_rs.contains("#![allow(dead_code)]"));
    assert!(!lib_rs.contains("#![allow(unused_imports)]"));
}
```

**Post-Fix Validation**:
```bash
cargo build 2>&1 | grep -c "warning: " | grep -q "^0$"
cargo clippy 2>&1 | grep -c "warning: " | grep -q "^0$"
```

---

## PART 2: HIGH PRIORITY VIOLATIONS (V3, V4)

### Violation V3: Fake TUI API Integration

**File**: `/home/azureuser/src/RustyClawd/crates/cli/src/tui/ui.rs` (lines 528-531)

**Current Code**:
```rust
// TODO: Integrate with Claude API
tui.add_message(ChatMessage::assistant(
    "I'm here! (API integration coming soon)".to_string()
));
```

**Problem**: Fake response misleads users into thinking API is working.

**Decision**: Implement real API integration OR remove TUI mode entirely.

### Success Criteria - Option A: Implement Real API

1. **API Integration**
   - [ ] TUI connects to real Claude API using rustyclawd-core client
   - [ ] User messages sent to API
   - [ ] Streaming responses displayed in real-time
   - [ ] Error handling for API failures
   - [ ] Loading indicators during API calls

2. **User Experience**
   - [ ] No fake messages
   - [ ] Clear error messages for API failures
   - [ ] Graceful handling of network issues
   - [ ] Support for all Claude models

3. **Testing**
   - [ ] Integration tests with mock API server
   - [ ] Unit tests for message handling
   - [ ] Manual testing with real API

### Success Criteria - Option B: Remove TUI Mode

1. **Removal**
   - [ ] Delete `/home/azureuser/src/RustyClawd/crates/cli/src/tui/` directory
   - [ ] Remove TUI dependencies from Cargo.toml
   - [ ] Remove TUI command-line flag
   - [ ] Update documentation

2. **Migration**
   - [ ] Document interactive mode as alternative
   - [ ] Ensure all TUI features available elsewhere

### Implementation Strategy (Option A: Implement)

#### Phase 1: Write Integration Tests (TDD)

**Test File**: `crates/cli/tests/tui_api_integration_tests.rs`

```rust
#[tokio::test]
async fn test_tui_sends_user_message_to_api() {
    // Given: Mock API server
    let mock_server = setup_mock_claude_api();

    // When: User sends message in TUI
    let tui = TuiState::new().unwrap();
    tui.send_message("Hello Claude").await.unwrap();

    // Then: API receives correct request
    assert_eq!(mock_server.received_messages(), vec!["Hello Claude"]);
}

#[tokio::test]
async fn test_tui_displays_api_response() {
    // Given: Mock API returning specific response
    let mock_api = mock_api_with_response("Hello! How can I help?");

    // When: User sends message
    let mut tui = TuiState::with_api(mock_api);
    tui.send_message("Hi").await.unwrap();

    // Then: Response displayed as assistant message
    let messages = tui.get_messages();
    assert_eq!(messages.last().unwrap().content, "Hello! How can I help?");
    assert!(matches!(messages.last().unwrap().role, MessageRole::Assistant));
}

#[tokio::test]
async fn test_tui_handles_api_error_gracefully() {
    // Given: Mock API that fails
    let mock_api = mock_api_with_error("Network timeout");

    // When: User sends message
    let mut tui = TuiState::with_api(mock_api);
    let result = tui.send_message("Test").await;

    // Then: Error displayed to user
    assert!(result.is_err());
    let error_msg = tui.get_messages().last().unwrap();
    assert!(error_msg.content.contains("Error"));
    assert!(error_msg.content.contains("Network timeout"));
}

#[tokio::test]
async fn test_tui_streams_api_response() {
    // Given: Mock API with streaming response
    let mock_api = mock_api_streaming("Hello", " world", "!");

    // When: User sends message
    let mut tui = TuiState::with_api(mock_api);
    tui.send_message("Test").await.unwrap();

    // Then: Response builds up incrementally
    let updates = tui.get_response_updates();
    assert_eq!(updates, vec!["Hello", "Hello world", "Hello world!"]);
}
```

#### Phase 2: Implement API Integration

**Modified File**: `crates/cli/src/tui/ui.rs`

```rust
use rustyclawd_core::{Client, Message, ClientConfig};

pub struct TuiState {
    // Existing fields...
    api_client: Client,
}

impl TuiState {
    pub async fn send_message(&mut self, content: &str) -> Result<()> {
        // Add user message
        self.add_message(ChatMessage::user(content.to_string()));

        // Build API messages
        let api_messages: Vec<Message> = self.messages
            .iter()
            .filter(|m| !matches!(m.role, MessageRole::System))
            .map(|m| self.to_api_message(m))
            .collect();

        // Call API with streaming
        let mut response_content = String::new();
        let mut stream = self.api_client.send_message_stream(&api_messages).await?;

        // Add placeholder for assistant message
        let msg_index = self.messages.len();
        self.add_message(ChatMessage::assistant(String::new()));

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(text) => {
                    response_content.push_str(&text);
                    // Update message in place
                    self.messages[msg_index].content = response_content.clone();
                    self.draw()?;
                }
                Err(e) => {
                    self.add_message(ChatMessage::system(
                        format!("Error: {}", e)
                    ));
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    fn to_api_message(&self, msg: &ChatMessage) -> Message {
        match msg.role {
            MessageRole::User => Message::user(msg.content.clone()),
            MessageRole::Assistant => Message::assistant(msg.content.clone()),
            MessageRole::System => Message::system(msg.content.clone()),
        }
    }
}
```

#### Phase 3: Update Main Event Loop

Replace fake response code with real API call:

```rust
// In run_tui() function, replace:
// tui.add_message(ChatMessage::assistant("I'm here! (API integration coming soon)".to_string()));

// With:
if let Err(e) = tui.send_message(input).await {
    tui.set_status(format!("Error: {}", e));
}
```

### Acceptance Criteria - Option A

- [ ] All fake responses removed
- [ ] Real API client integrated
- [ ] Streaming responses work
- [ ] Error handling implemented
- [ ] All integration tests pass
- [ ] Manual testing successful with real API
- [ ] No TODO comments in TUI code

### Acceptance Criteria - Option B

- [ ] TUI module completely removed
- [ ] No TUI references in codebase
- [ ] Documentation updated
- [ ] Users notified of removal

### Test Strategy

**Pre-Implementation**:
```rust
#[test]
fn test_no_fake_api_responses() {
    let ui_rs = std::fs::read_to_string("src/tui/ui.rs").unwrap();
    assert!(!ui_rs.contains("API integration coming soon"));
    assert!(!ui_rs.contains("I'm here!"));
}
```

---

### Violation V4: Stub Builtin Commands

**File**: `/home/azureuser/src/RustyClawd/crates/cli/src/commands/builtins.rs`

**Problem**: Many builtin commands return placeholder strings instead of doing real work.

**Examples**:
- `history_command()`: "Note: History display would be populated in interactive mode"
- `stats_command()`: All values are hardcoded zeros
- `agents_command()`: "(No agents configured)" without checking actual state
- 25+ similar stubs

**Decision**: Implement working commands OR mark as unimplemented with clear errors.

### Success Criteria

1. **Working Implementations**
   - [ ] /history shows real command history
   - [ ] /stats shows actual session statistics
   - [ ] /agents queries real agent registry
   - [ ] /hooks queries real hooks configuration
   - [ ] /plugins lists actual loaded plugins
   - [ ] All commands interact with real system state

2. **Honest Unimplemented Commands**
   - [ ] Commands not yet implemented return clear error
   - [ ] Error message: "Command /xxx not yet implemented"
   - [ ] No fake/placeholder data
   - [ ] Documentation marks as unimplemented

3. **State Management**
   - [ ] Session state tracked: messages, tokens, duration
   - [ ] History stored and retrievable
   - [ ] Statistics calculated from real data

### Implementation Strategy

#### Phase 1: Audit Commands (Categorize)

**Category A - Remove** (Not in scope for MVP):
- /review, /sandbox, /doctor, /compact, /init, /permissions
- /trace, /log, /checkpoint, /restore, /undo, /redo
- Decision: Remove functions, return `None` from execute()

**Category B - Implement** (Core functionality):
- /help, /exit, /clear, /history, /stats
- /version, /status, /config, /model
- Decision: Full implementation with real state

**Category C - Defer** (Plugin system needed):
- /mcp, /agents, /hooks, /plugins, /tools
- Decision: Return honest "not yet implemented" error

#### Phase 2: Write Tests First (TDD)

**Test File**: `crates/cli/tests/builtin_commands_tests.rs`

```rust
use rustyclawd::commands::{builtins::BuiltinCommands, parser::Command};

// Category A: Removed commands
#[test]
fn test_removed_commands_not_builtin() {
    assert!(!BuiltinCommands::is_builtin("review"));
    assert!(!BuiltinCommands::is_builtin("sandbox"));
    assert!(!BuiltinCommands::is_builtin("doctor"));
    assert!(!BuiltinCommands::is_builtin("compact"));
}

#[test]
fn test_removed_commands_return_none() {
    let cmd = Command::new("review".to_string(), None);
    assert_eq!(BuiltinCommands::execute(&cmd), None);
}

// Category B: Implemented commands
#[test]
fn test_history_returns_real_history() {
    let mut session = SessionState::new();
    session.add_to_history("/help");
    session.add_to_history("/clear");

    let cmd = Command::new("history".to_string(), None);
    let result = BuiltinCommands::execute_with_state(&cmd, &session).unwrap();

    assert!(result.contains("/help"));
    assert!(result.contains("/clear"));
}

#[test]
fn test_stats_shows_real_data() {
    let mut session = SessionState::new();
    session.increment_message_count();
    session.increment_message_count();
    session.add_tokens(150);
    session.add_tokens(200);

    let cmd = Command::new("stats".to_string(), None);
    let result = BuiltinCommands::execute_with_state(&cmd, &session).unwrap();

    assert!(result.contains("Messages: 2"));
    assert!(result.contains("Total tokens: 350"));
}

#[test]
fn test_version_returns_real_version() {
    let cmd = Command::new("version".to_string(), None);
    let result = BuiltinCommands::execute(&cmd).unwrap();

    // Should contain actual version from Cargo.toml
    assert!(result.contains(env!("CARGO_PKG_VERSION")));
}

// Category C: Deferred commands
#[test]
fn test_agents_command_returns_not_implemented() {
    let cmd = Command::new("agents".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Command /agents not yet implemented"
    );
}

#[test]
fn test_hooks_command_returns_not_implemented() {
    let cmd = Command::new("hooks".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_err());
}
```

#### Phase 3: Create Session State Module

**New File**: `crates/cli/src/session.rs`

```rust
use std::time::Instant;

pub struct SessionState {
    command_history: Vec<String>,
    message_count: usize,
    total_tokens: usize,
    session_start: Instant,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            command_history: Vec::new(),
            message_count: 0,
            total_tokens: 0,
            session_start: Instant::now(),
        }
    }

    pub fn add_to_history(&mut self, command: String) {
        self.command_history.push(command);
    }

    pub fn get_history(&self) -> &[String] {
        &self.command_history
    }

    pub fn increment_message_count(&mut self) {
        self.message_count += 1;
    }

    pub fn add_tokens(&mut self, tokens: usize) {
        self.total_tokens += tokens;
    }

    pub fn get_stats(&self) -> SessionStats {
        SessionStats {
            messages: self.message_count,
            commands: self.command_history.len(),
            tokens: self.total_tokens,
            duration_secs: self.session_start.elapsed().as_secs(),
        }
    }
}

pub struct SessionStats {
    pub messages: usize,
    pub commands: usize,
    pub tokens: usize,
    pub duration_secs: u64,
}
```

#### Phase 4: Implement Real Commands

**Modified File**: `crates/cli/src/commands/builtins.rs`

```rust
impl BuiltinCommands {
    pub fn is_builtin(name: &str) -> bool {
        matches!(
            name,
            // Keep only implemented commands
            "help" | "exit" | "quit" | "clear" |
            "history" | "stats" | "version" |
            "status" | "config" | "model"
        )
    }

    pub fn execute(cmd: &Command) -> Result<String> {
        match cmd.name.as_str() {
            "help" => Ok(Self::help(&cmd.args_str)),
            "exit" | "quit" => Ok(Self::exit_command()),
            "clear" => Ok(Self::clear_command()),
            "version" => Ok(Self::version_command()),
            _ => Err(anyhow!("Command /{} not yet implemented", cmd.name)),
        }
    }

    pub fn execute_with_state(
        cmd: &Command,
        state: &SessionState
    ) -> Result<String> {
        match cmd.name.as_str() {
            "history" => Ok(Self::history_command(state)),
            "stats" => Ok(Self::stats_command(state)),
            "status" => Ok(Self::status_command(state)),
            _ => Self::execute(cmd),
        }
    }

    fn history_command(state: &SessionState) -> String {
        let history = state.get_history();
        if history.is_empty() {
            return "No command history yet.".to_string();
        }

        let mut output = String::from("Command History:\n");
        for (i, cmd) in history.iter().enumerate() {
            output.push_str(&format!("  {}. {}\n", i + 1, cmd));
        }
        output
    }

    fn stats_command(state: &SessionState) -> String {
        let stats = state.get_stats();
        format!(
            "Session Statistics:\n\
             - Messages: {}\n\
             - Commands executed: {}\n\
             - Total tokens: {}\n\
             - Session duration: {}s",
            stats.messages,
            stats.commands,
            stats.tokens,
            stats.duration_secs
        )
    }

    fn status_command(state: &SessionState) -> String {
        let stats = state.get_stats();
        format!(
            "System Status:\n\
             - Connection: active\n\
             - Model: {}\n\
             - Messages this session: {}\n\
             - Uptime: {}s",
            env!("CARGO_PKG_VERSION"),
            stats.messages,
            stats.duration_secs
        )
    }
}
```

### Acceptance Criteria

- [ ] All placeholder strings removed
- [ ] Real session state tracked
- [ ] /history shows actual history
- [ ] /stats shows real statistics
- [ ] Unimplemented commands return clear errors
- [ ] No fake data anywhere
- [ ] All tests pass

---

## PART 3: MEDIUM PRIORITY VIOLATIONS (V5, V6, V7)

### Violation V5: Plugin Executor Placeholder

**File**: `/home/azureuser/src/RustyClawd/crates/cli/src/plugins/executor.rs` (lines 78-89)

**Current Code**:
```rust
// For now, return success with command information
// Full subprocess execution would require:
// 1. Determining interpreter (node, python, shell, etc.)
// 2. Spawning subprocess with proper environment
// 3. Passing arguments via stdin/command line
// 4. Capturing and parsing output
Ok(PluginExecutionResult {
    success: true,
    output: format!("Command '{}' executed successfully", command.name),
    errors: vec![],
    duration_ms: duration,
})
```

**Problem**: Returns fake success without executing command.

**Decision**: Either implement real subprocess execution OR remove execute_command() method entirely.

### Success Criteria - Option A: Implement

1. **Subprocess Execution**
   - [ ] Detect interpreter from plugin manifest (node, python, bash)
   - [ ] Spawn subprocess with proper environment
   - [ ] Pass arguments correctly
   - [ ] Capture stdout/stderr
   - [ ] Handle timeouts
   - [ ] Return real exit codes

2. **Error Handling**
   - [ ] Interpreter not found errors
   - [ ] Execution timeout errors
   - [ ] Permission denied errors
   - [ ] Invalid argument errors

3. **Security**
   - [ ] Input validation
   - [ ] Path sanitization
   - [ ] Environment variable isolation

### Success Criteria - Option B: Remove

- [ ] Delete `execute_command()` method
- [ ] Remove from public API
- [ ] Update documentation
- [ ] Remove tests that use it

### Implementation Strategy (Option A)

#### Phase 1: Write Tests

```rust
#[tokio::test]
async fn test_execute_command_runs_real_subprocess() {
    let executor = PluginExecutor::new();
    let plugin = test_plugin_with_command("echo", "echo 'test output'");
    executor.register(plugin.metadata);

    let result = executor.execute_command(
        "test-plugin",
        "echo",
        json!({"message": "test"})
    ).await.unwrap();

    assert!(result.success);
    assert_eq!(result.output.trim(), "test output");
    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_execute_command_captures_stderr() {
    let executor = PluginExecutor::new();
    let plugin = test_plugin_with_command("error", "echo 'error' >&2; exit 1");
    executor.register(plugin.metadata);

    let result = executor.execute_command(
        "test-plugin",
        "error",
        json!({})
    ).await;

    assert!(result.is_err() || !result.unwrap().success);
}

#[tokio::test]
async fn test_execute_command_respects_timeout() {
    let executor = PluginExecutor::new();
    let plugin = test_plugin_with_command("slow", "sleep 10");
    executor.register(plugin.metadata);

    let start = Instant::now();
    let result = executor.execute_command_with_timeout(
        "test-plugin",
        "slow",
        json!({}),
        Duration::from_secs(1)
    ).await;

    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(result.is_err());
}
```

#### Phase 2: Implement Real Execution

```rust
use tokio::process::Command;
use tokio::time::timeout;

impl PluginExecutor {
    pub async fn execute_command(
        &self,
        plugin_id: &str,
        command_name: &str,
        args: serde_json::Value,
    ) -> Result<PluginExecutionResult, String> {
        let start = Instant::now();

        let plugin = self.plugins.get(plugin_id)
            .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

        if !plugin.enabled {
            return Err("Plugin is disabled".to_string());
        }

        let command_def = plugin.manifest.commands.iter()
            .find(|c| c.name == command_name)
            .ok_or_else(|| format!("Command not found: {}", command_name))?;

        // Determine interpreter
        let interpreter = self.detect_interpreter(&plugin.manifest)?;

        // Build command path
        let script_path = plugin.path.join(&command_def.path);

        // Spawn subprocess
        let mut cmd = Command::new(&interpreter);
        cmd.arg(&script_path)
            .current_dir(&plugin.path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Set environment
        cmd.env("PLUGIN_ID", plugin_id);

        // Execute with timeout
        let timeout_duration = Duration::from_millis(
            command_def.timeout.unwrap_or(60000)
        );

        let child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        // Write args to stdin
        if let Some(mut stdin) = child.stdin {
            let args_json = serde_json::to_string(&args)
                .map_err(|e| format!("Failed to serialize args: {}", e))?;
            stdin.write_all(args_json.as_bytes()).await
                .map_err(|e| format!("Failed to write args: {}", e))?;
        }

        // Wait with timeout
        let output = timeout(timeout_duration, child.wait_with_output())
            .await
            .map_err(|_| "Command timed out".to_string())?
            .map_err(|e| format!("Command execution failed: {}", e))?;

        let duration = start.elapsed().as_millis() as u64;

        Ok(PluginExecutionResult {
            success: output.status.success(),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
            errors: if output.status.success() {
                vec![]
            } else {
                vec![String::from_utf8_lossy(&output.stderr).to_string()]
            },
            duration_ms: duration,
        })
    }

    fn detect_interpreter(&self, manifest: &PluginManifest) -> Result<String, String> {
        let main_file = &manifest.main;

        if main_file.ends_with(".js") || main_file.ends_with(".mjs") {
            Ok("node".to_string())
        } else if main_file.ends_with(".py") {
            Ok("python3".to_string())
        } else if main_file.ends_with(".sh") {
            Ok("bash".to_string())
        } else {
            Err(format!("Cannot determine interpreter for {}", main_file))
        }
    }
}
```

### Acceptance Criteria

- [ ] Real subprocess execution works
- [ ] All interpreters supported (node, python, bash)
- [ ] Timeout handling implemented
- [ ] Error messages are real
- [ ] No fake success responses
- [ ] All tests pass

---

### Violation V6: Unimplemented Subcommands

**File**: Multiple in `commands/executor.rs`

**Problem**: Methods using `unwrap()` and placeholder returns.

**Decision**: Remove unimplemented subcommands or implement properly.

### Acceptance Criteria

- [ ] All `unwrap()` calls removed or justified
- [ ] Error handling uses proper Result types
- [ ] No placeholder implementations

---

### Violation V7: Non-Compiling Example

**File**: `/home/azureuser/src/RustyClawd/crates/cli/examples/verify_schemas.rs`

**Problem**: Example doesn't compile due to missing OpenSSL system dependencies.

**Decision**: Either fix dependencies OR move to integration test that's conditionally compiled.

### Success Criteria - Option A: Fix Dependencies

- [ ] Document system dependencies in README
- [ ] Add conditional compilation for examples
- [ ] Example compiles on all platforms

### Success Criteria - Option B: Convert to Test

- [ ] Move to `tests/verify_schemas_test.rs`
- [ ] Add `#[cfg(test)]` guards
- [ ] Remove from examples directory

### Implementation Strategy

**Option A**:
```toml
# Cargo.toml
[dev-dependencies]
openssl = { version = "0.10", features = ["vendored"] }
```

**Option B**: Move to tests with proper guards.

---

## PART 4: LOW PRIORITY VIOLATIONS (V8-V14)

### Violation V8: Swallowed Error Context

**Files**: 10 files with `map_err(|e| ...)` losing error context

**Problem**: Generic error messages lose valuable debugging information.

**Solution**: Use `anyhow::Context` for error chain preservation.

### Implementation Strategy

Replace:
```rust
.map_err(|e| format!("Failed to load: {}", e))?
```

With:
```rust
.context("Failed to load plugin manifest")?
```

### Acceptance Criteria

- [ ] All `map_err` with string formatting replaced
- [ ] Error chains preserved
- [ ] Original errors accessible in debug output

---

### Violation V9: TDD Test Stubs

**Problem**: 726 tests across 37 files - some may be incomplete stubs.

**Solution**: Audit all tests, remove or complete stubs.

### Success Criteria

- [ ] All test functions have assertions
- [ ] No empty test bodies
- [ ] No `unimplemented!()` in tests
- [ ] Test coverage > 80%

---

### Violation V10 & V11: Ignored Tests

**File**: `/home/azureuser/src/RustyClawd/crates/cli/tests/hooks_doc_tests.rs`

**Tests**:
- Line 198: `test_matcher_mcp_full_pattern`
- Line 1310: `test_scenario_mcp_tool_pattern_matching`

**Problem**: Tests marked `#[ignore]` with TODO comments.

### Success Criteria

- [ ] Fix matcher implementation
- [ ] Remove `#[ignore]` attributes
- [ ] Tests pass
- [ ] TODO comments removed

### Implementation Strategy

The tests document that `mcp__.*__.*` pattern should match `mcp__server__tool` but currently fails. Fix the matcher logic in `HookMatcher::matches()`.

---

### Violation V12: Example Unwrap

**File**: `commands/executor.rs:37`

```rust
working_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
```

**Decision**: Keep as-is (reasonable fallback) but document why.

### Acceptance Criteria

- [ ] Add comment explaining fallback rationale
- [ ] No changes to code

---

### Violation V13: Terminal Cleanup Swallowed Error

**File**: `tui/ui.rs:483`

```rust
let _ = self.cleanup();
```

**Problem**: Drop impl ignores cleanup errors.

**Solution**: Log error instead of silently dropping.

### Implementation Strategy

```rust
impl Drop for TuiState {
    fn drop(&mut self) {
        if let Err(e) = self.cleanup() {
            eprintln!("Warning: Failed to cleanup terminal: {}", e);
        }
    }
}
```

### Acceptance Criteria

- [ ] Errors logged to stderr
- [ ] No silent failures
- [ ] Users aware of cleanup issues

---

### Violation V14: Current Directory Fallback

**File**: `commands/executor.rs:37`

**Decision**: Same as V12 - reasonable fallback, add comment.

---

## PART 5: DEFINITION OF DONE

### Overall Project Completion Criteria

**Code Quality**:
- [ ] Zero `#![allow(...)]` suppressions at crate level
- [ ] Zero compiler warnings
- [ ] Zero clippy warnings
- [ ] No TODO comments without tracking issues
- [ ] No fake implementations

**Testing**:
- [ ] All tests pass
- [ ] No `#[ignore]` tests without justification
- [ ] Test coverage > 80%
- [ ] Integration tests for all major features

**Documentation**:
- [ ] README updated with real features
- [ ] API documentation complete
- [ ] Examples compile and run
- [ ] Architecture decisions documented

**Philosophy Compliance**:
- [ ] Every function works or doesn't exist
- [ ] No placeholder strings
- [ ] No fake data
- [ ] Errors are honest and helpful
- [ ] Warning suppressions only where absolutely necessary with clear justification

---

## PART 6: PRIORITIZED IMPLEMENTATION PLAN

### Phase 1: Foundation (Day 1)
1. Fix V1 & V2 (global suppressions) - enables seeing all other issues
2. Create session state module for V4
3. Audit and categorize all 726 tests (V9)

### Phase 2: High-Value Fixes (Days 2-3)
4. Implement real builtin commands (V4)
5. Fix or remove TUI API integration (V3)
6. Fix plugin executor (V5) or remove if not needed

### Phase 3: Polish (Day 4)
7. Fix ignored tests (V10, V11)
8. Replace error swallowing (V8)
9. Fix example compilation (V7)
10. Add error logging to Drop (V13)

### Phase 4: Validation (Day 5)
11. Run full test suite
12. Manual testing of all features
13. Documentation update
14. Final review

---

## PART 7: RISK ASSESSMENT

### High Risk Items

**V3 (TUI API)**: Requires major refactoring or complete removal
- **Mitigation**: Decide quickly, implement mock API for testing

**V4 (Builtins)**: Touches core user-facing functionality
- **Mitigation**: Extensive testing, gradual rollout

**V5 (Plugin Executor)**: Security implications of subprocess execution
- **Mitigation**: Input validation, sandboxing, security review

### Medium Risk Items

**V1/V2 (Suppressions)**: May uncover many issues
- **Mitigation**: Fix incrementally by module

**V9 (Test Stubs)**: Large audit required
- **Mitigation**: Automated scanning for common patterns

### Low Risk Items

V8, V10, V11, V12, V13, V14: Isolated changes with clear solutions

---

## PART 8: QUALITY METRICS

### Success Metrics

1. **Warning Count**: 0
2. **Test Pass Rate**: 100%
3. **Code Coverage**: >80%
4. **Fake Implementations**: 0
5. **TODO Comments**: 0 (or tracked in issues)
6. **Ignored Tests**: 0 (or justified)

### Tracking

```bash
# Check warnings
cargo clippy --all-features 2>&1 | grep -c "warning:"

# Check test pass rate
cargo test --all-features 2>&1 | grep "test result"

# Check for fake strings
rg -i "(coming soon|placeholder|TODO|FIXME|stub)" --type rust

# Check ignored tests
rg "#\[ignore\]" --type rust
```

---

## PART 9: RESOURCES REQUIRED

### Dependencies
- Existing: rustyclawd-core, tokio, anyhow, thiserror
- New (if implementing): None

### Documentation
- Rust Async Book (for subprocess handling)
- Claude API documentation (for TUI integration)
- Plugin system architecture (for executor)

### Time Allocation
- Critical (V1-V2): 8 hours
- High (V3-V4): 16 hours
- Medium (V5-V7): 12 hours
- Low (V8-V14): 8 hours
- Testing & Validation: 8 hours
- **Total**: ~40-50 hours (3-5 days)

---

## APPENDIX A: TEST CHECKLIST

### Pre-Implementation Tests

```bash
# Baseline measurements
cargo clean
cargo build --all-features 2>&1 | tee build-before.log
cargo test --all-features 2>&1 | tee test-before.log
cargo clippy --all-features 2>&1 | tee clippy-before.log
```

### Post-Implementation Tests

```bash
# Final validation
cargo build --all-features 2>&1 | tee build-after.log
cargo test --all-features 2>&1 | tee test-after.log
cargo clippy --all-features 2>&1 | tee clippy-after.log

# Diff to verify improvements
diff build-before.log build-after.log
diff clippy-before.log clippy-after.log
```

---

## APPENDIX B: CODE REVIEW CHECKLIST

- [ ] No `#![allow(dead_code)]` or `#![allow(unused_imports)]`
- [ ] No `.unwrap()` except in tests or with clear justification
- [ ] No `let _ = ` silencing errors except in Drop with logging
- [ ] No "TODO:", "FIXME:", "coming soon", "placeholder" strings
- [ ] No `#[ignore]` tests without issue tracking
- [ ] All public functions documented
- [ ] All error paths tested
- [ ] All happy paths tested
- [ ] Integration tests for major features
- [ ] Examples compile and run

---

## CONCLUSION

This specification provides a comprehensive, actionable plan to eliminate all zero-BS philosophy violations in the RustyClawd CLI crate. By following the TDD approach, prioritizing critical issues, and maintaining rigorous quality standards, we will transform the codebase from containing fake implementations and suppressed warnings to being a model of honest, working software.

The estimated 3-5 day effort includes testing, documentation, and validation. The phased approach allows for incremental progress while maintaining a working codebase at each stage.

**Remember**: Fix it properly or remove it. No middle ground.
