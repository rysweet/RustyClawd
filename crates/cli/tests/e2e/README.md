# Phase 1: Programmatic E2E Tests (Rust)

**Status:** FAILING (Expected) - Waiting for implementation

This directory contains Rust-based end-to-end integration tests that validate complete user workflows programmatically.

## Test Files

### `test_slash_command_tui_integration.rs`
Tests that slash commands work seamlessly with the TUI:
- SlashCommandTool invocation
- Command expansion
- TUI display updates
- Session state consistency

**Tests:**
- `test_analyze_command_tui_integration` - /analyze command flow
- `test_debug_command_expansion` - /debug command expansion
- `test_invalid_slash_command_error` - Error handling
- `test_slash_command_with_arguments` - Command arguments

### `test_skills_execution_context.rs`
Tests that skills execute with full conversation context:
- Skills load from disk
- Context propagation
- Skill prompt injection
- Multi-turn preservation

**Tests:**
- `test_skill_loads_correctly` - Skill file loading
- `test_skill_receives_conversation_context` - Context access
- `test_skill_executes_with_context` - Execution with context
- `test_missing_skill_file_error` - Error handling
- `test_skill_multi_turn_context_preservation` - Multi-turn

### `test_full_interactive_session.rs`
Tests complete session lifecycle:
- Startup and shutdown
- Hook execution order
- Multi-turn conversations
- Tool execution workflows
- Error recovery

**Tests:**
- `test_session_start_and_welcome` - Session startup
- `test_tool_execution_workflow` - Complete tool workflow
- `test_multi_turn_conversation` - Context preservation
- `test_session_shutdown` - Clean shutdown
- `test_hook_execution_order` - Hook ordering
- `test_error_recovery` - Error handling

## Infrastructure

### `helpers/`
Test helper modules (STUBS - not implemented):
- `TestSession` - Session orchestration
- `TestSkillEnvironment` - Test skill setup

### `mocks/`
Mock implementations (STUBS - not implemented):
- `MockLLM` - Controllable LLM client

## Running Tests

**Current Status:** Tests are marked with `#[ignore]` and will fail with:
```
thread 'test_name' panicked at 'not yet implemented: Implement TestSession first'
```

**To run tests (they will fail):**
```bash
# Run all E2E tests (they'll be skipped due to #[ignore])
cargo test --test e2e

# Run specific test file
cargo test --test test_slash_command_tui_integration

# Try to run ignored tests (they'll panic with "not implemented")
cargo test --test test_slash_command_tui_integration -- --ignored
```

## Implementation Order

1. **Implement helper modules** (Task 1.1)
   - `helpers/test_session.rs` - See `docs/specs/test_session_spec.md`
   - `mocks/mock_llm.rs` - See `docs/specs/mock_llm_spec.md`
   - `helpers/test_skill_env.rs` - Temporary skill directories

2. **Remove `#[ignore]` and `todo!()` from tests** (Tasks 1.2-1.4)
   - Uncomment test implementation code
   - Verify tests pass

3. **Fix bugs discovered** (Task 1.5)
   - Address test failures
   - Ensure zero flakiness

## Success Criteria

Phase 1 succeeds when:
- [ ] All helper modules implemented
- [ ] All 4 test files passing
- [ ] All existing integration tests still passing (no regressions)
- [ ] Zero test flakiness (3 consecutive clean runs)
- [ ] Manual verification: Real workflows work as expected

## Documentation

- **Architecture:** `docs/architecture/e2e_testing_architecture.md`
- **Development Guide:** `docs/testing/E2E_TEST_DEVELOPMENT.md`
- **Module Specs:** `docs/specs/test_session_spec.md`, `docs/specs/mock_llm_spec.md`

## Next Steps

After Phase 1:
- Move to Phase 2: tmux-based tests (real terminal interaction)
- Then Phase 3: YAML scenario tests (declarative workflows)

**Target:** 85% Claude Code parity after Phase 1
