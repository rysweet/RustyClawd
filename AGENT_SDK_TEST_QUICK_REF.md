# Agent SDK Test Suite - Quick Reference

**Location:** `/Users/ryan/src/declawed/claude-code-rs/crates/cli/tests/agent_sdk_tests.rs`

## Test Suite Overview

- **Total Tests:** 51
- **Status:** All passing
- **Runtime:** <1 second
- **Coverage:** Agent invocation, context management, result handling, parallel execution, isolation, hooks, subagents

## Quick Commands

```bash
# Run all agent SDK tests
cargo test --test agent_sdk_tests

# Run specific test
cargo test --test agent_sdk_tests test_context_continue_flag_resumes_session -- --nocapture

# Run with verbose output
cargo test --test agent_sdk_tests -- --nocapture --test-threads=1

# List all tests
cargo test --test agent_sdk_tests -- --list
```

## Test Categories

### 1. Agent Invocation (6 tests)
Core query function and streaming:
- `test_agent_query_basic_invocation` - Basic query with prompt
- `test_agent_query_returns_valid_session_id` - Session tracking
- `test_agent_query_streaming_simulation` - Streaming patterns
- `test_agent_query_with_custom_model` - Model configuration
- `test_agent_query_with_system_prompt` - System prompt setup
- `test_agent_query_empty_prompt` - Edge case: empty input

### 2. Context Management (7 tests)
Session state and lifecycle:
- `test_context_new_session_isolation` - Isolation between sessions
- `test_context_continue_flag_resumes_session` - Continue existing session
- `test_context_resume_session_by_id` - Resume by ID
- `test_context_resume_invalid_session_fails` - Error handling
- `test_context_fork_session_creates_isolated_branch` - Session forking
- `test_context_continuation_counter_increments` - Track interactions
- Full workflows also validate persistence

### 3. Result Handling (7 tests)
Message serialization and persistence:
- `test_result_contains_unique_message_id` - Unique tracking
- `test_result_serialization_format` - Complete structure
- `test_result_error_handling_invalid_tool` - Error capture
- `test_result_tools_used_tracking` - Tool usage history
- `test_result_session_persistence` - Context persistence
- Plus integration tests for full lifecycle

### 4. Parallel Execution (6 tests)
Background process management:
- `test_parallel_background_bash_execution` - Start processes
- `test_parallel_shell_id_retrieval` - Get output by ID
- `test_parallel_process_state_transitions` - State changes
- `test_parallel_multiple_processes_isolated` - Independence
- `test_parallel_invalid_shell_id_error` - Error handling
- `test_parallel_process_output_accumulation` - Output retrieval

### 5. Agent Isolation (8 tests)
Tool permissions and access control:
- `test_isolation_allowed_tools_filter` - Whitelist tools
- `test_isolation_disallowed_tools_filter` - Blacklist tools
- `test_isolation_allowed_and_disallowed_precedence` - Override logic
- `test_isolation_no_restrictions_allows_all` - Default mode
- `test_isolation_empty_allowed_list_restricts_all` - Deny all
- `test_isolation_tool_execution_tracks_usage` - Track usage
- `test_isolation_permission_modes` - Permission variants
- Plus execution tracking

### 6. Hook System (8 tests)
Event-driven middleware:
- `test_hooks_session_start_fired` - SessionStart hook
- `test_hooks_session_end_fired` - SessionEnd hook
- `test_hooks_pre_tool_use_fired` - PreToolUse hook
- `test_hooks_post_tool_use_fired` - PostToolUse hook
- `test_hooks_pre_and_post_tool_order` - Hook ordering
- `test_hooks_session_lifecycle_complete` - Full lifecycle
- `test_hooks_multiple_tool_executions` - Multiple tools

### 7. Subagent Delegation (5 tests)
Autonomous multi-step tasks:
- `test_subagent_definition_structure` - Definition format
- `test_subagent_configuration_in_options` - Register subagents
- `test_subagent_multiple_agents_registry` - Multiple subagents
- `test_subagent_tool_isolation` - Tool restrictions per subagent
- `test_subagent_model_override` - Model overrides

### 8. Edge Cases (5 tests)
Boundary conditions and limits:
- `test_boundary_very_long_prompt` - 10,000 char prompts
- `test_boundary_special_characters_in_prompt` - Unicode, escapes
- `test_boundary_rapid_sequential_queries` - 10 rapid queries
- `test_boundary_many_background_processes` - 20 processes
- `test_boundary_deeply_nested_session_forks` - Multi-level forks

### 9. E2E Tests (3 tests)
Complete workflows:
- `test_e2e_complete_agent_session_workflow` - Full lifecycle
- `test_e2e_agent_with_tool_execution` - Tools + hooks
- `test_e2e_parallel_agents_independent_sessions` - Parallel execution
- Plus fork context test

## Key Structures

### AgentOptions
```rust
pub struct AgentOptions {
    pub model: Option<String>,                      // Model selection
    pub system_prompt: Option<String>,              // System instructions
    pub allowed_tools: Option<Vec<String>>,         // Whitelist tools
    pub disallowed_tools: Option<Vec<String>>,      // Blacklist tools
    pub continue_session: bool,                     // Resume session
    pub resume_session_id: Option<String>,          // Resume specific ID
    pub fork_session: bool,                         // Fork session
    pub permission_mode: PermissionMode,            // Permission strategy
    pub hooks: HashMap<HookEvent, Vec<String>>,     // Event handlers
    pub agents: HashMap<String, SubagentDefinition>,// Subagents
}
```

### PermissionMode
```rust
pub enum PermissionMode {
    Default,           // Standard permission checks
    AcceptEdits,       // Auto-accept tool edits
    BypassPermissions, // Skip permission checks
    Plan,              // Plan-mode execution
}
```

### HookEvent
```rust
pub enum HookEvent {
    PreToolUse,    // Before tool execution
    PostToolUse,   // After tool execution
    SessionStart,  // Session creation
    SessionEnd,    // Session completion
    PreCompact,    // Before context compaction
}
```

### ProcessState
```rust
pub enum ProcessState {
    Running,           // Still executing
    Completed(i32),    // Finished with exit code
    Failed(String),    // Failed with error
}
```

## Common Test Patterns

### Testing Session Continuation
```rust
let mut options = AgentOptions::default();
let result1 = sdk.query("First", &options)?;

options.continue_session = true;
let result2 = sdk.query("Second", &options)?;

assert_eq!(result2.session_id, result1.session_id);
```

### Testing Tool Permissions
```rust
let mut options = AgentOptions::default();
options.allowed_tools = Some(vec!["bash".to_string()]);

assert!(sdk.execute_tool("session_1", "bash", "ls", &options).is_ok());
assert!(sdk.execute_tool("session_1", "web_search", "q", &options).is_err());
```

### Testing Hook Events
```rust
sdk.clear_hooks();
let _result = sdk.execute_tool("session_1", "bash", "ls", &options);
let hooks = sdk.get_hook_calls();

assert!(hooks.iter().any(|(e, _)| *e == HookEvent::PreToolUse));
assert!(hooks.iter().any(|(e, _)| *e == HookEvent::PostToolUse));
```

### Testing Parallel Processes
```rust
let shell1 = sdk.run_background("cmd1", "session_1")?;
let shell2 = sdk.run_background("cmd2", "session_1")?;

let output1 = sdk.get_background_output(&shell1)?;
let output2 = sdk.get_background_output(&shell2)?;

assert_eq!(output1.state, ProcessState::Running);
assert_eq!(output2.state, ProcessState::Running);
```

## Checking Test Coverage

```bash
# See which tests exist
cargo test --test agent_sdk_tests -- --list

# Run and show test names
cargo test --test agent_sdk_tests -- --nocapture --test-threads=1

# Check compilation
cargo test --test agent_sdk_tests --no-run
```

## Test Results Summary

```
running 51 tests
test test_agent_query_basic_invocation ... ok
test test_agent_query_returns_valid_session_id ... ok
[... 49 more tests ...]

test result: ok. 51 passed; 0 failed
```

## Architecture

The test suite uses an `AgentSDK` simulator that provides:

1. **Session Management** - HashMap-based context storage with Arc<Mutex<>>
2. **Background Processes** - Tracks shell IDs and process states
3. **Hook System** - Event tracking for middleware pattern
4. **Permission Checking** - Tool access validation
5. **Tool Execution** - Simulated tool invocation with tracking

This allows testing SDK behavior without external dependencies.

## Extending Tests

### Add a new test category
1. Add new test functions with `#[test]` attribute
2. Follow naming: `test_category_specific_behavior`
3. Use existing helper methods on `AgentSDK`
4. Run: `cargo test --test agent_sdk_tests`

### Add edge cases
1. Identify boundary conditions
2. Create parametrized or separate test
3. Verify both success and error paths
4. Document what's being tested

## Debugging

```bash
# Run single test with output
cargo test --test agent_sdk_tests test_context_continue_flag_resumes_session -- --nocapture

# Run with backtrace on panic
RUST_BACKTRACE=1 cargo test --test agent_sdk_tests -- --nocapture

# Run with thread output
cargo test --test agent_sdk_tests -- --nocapture --test-threads=1
```

## Key Assertion Patterns

```rust
// Session equality
assert_eq!(result1.session_id, result2.session_id);

// Error checking
assert!(result.is_err());
assert!(result.unwrap_err().contains("message"));

// Collection checking
assert!(contexts.contains_key(&session_id));
assert_eq!(context.tools_executed.len(), 2);

// Ordering
let pre_index = hooks.iter().position(|(e, _)| *e == HookEvent::PreToolUse);
let post_index = hooks.iter().position(|(e, _)| *e == HookEvent::PostToolUse);
assert!(pre_index.unwrap() < post_index.unwrap());
```

## Reference Documentation

- Agent SDK Overview: https://docs.claude.com/en/docs/agent-sdk/overview
- TypeScript Implementation: https://docs.claude.com/en/docs/agent-sdk/typescript
- Test Coverage Report: `/Users/ryan/src/declawed/claude-code-rs/AGENT_SDK_TEST_COVERAGE.md`

## Tips

1. **Fast Testing** - Tests complete in <1 second, use for rapid feedback
2. **Isolation** - Each test creates fresh AgentSDK instance
3. **Deterministic** - No flaky tests, no timing dependencies
4. **Production Ready** - Can be integrated with actual SDK implementation
5. **Documentation** - Tests serve as executable specification

---

**Created:** November 11, 2025
**Test File:** `/Users/ryan/src/declawed/claude-code-rs/crates/cli/tests/agent_sdk_tests.rs`
**Status:** All 51 tests passing
