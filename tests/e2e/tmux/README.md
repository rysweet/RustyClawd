# Phase 2: tmux E2E Tests (Bash)

**Status:** FAILING (Expected) - Waiting for framework implementation

This directory contains bash-based E2E tests that use tmux to test RustyClawd in a real terminal environment.

## Why tmux?

tmux provides:
- Real terminal rendering validation
- Actual keyboard input handling
- True terminal output capture
- Simple bash scripting (no complex dependencies)
- Standard on Linux/macOS CI environments

## Test Files

### `test_slash_command_e2e.sh`
Tests slash commands in real terminal:
- `/analyze` command execution
- `/debug` command execution
- Invalid command error handling
- Real terminal rendering

**Tests:**
- `test_analyze_command_e2e` - /analyze in real terminal
- `test_debug_command_e2e` - /debug execution
- `test_invalid_command_error_e2e` - Error display

### `test_skills_e2e.sh`
Tests skills in real terminal:
- Skill invocation
- Context usage
- Error handling for missing skills

**Tests:**
- `test_skill_invocation_e2e` - Real skill execution
- `test_skill_context_usage_e2e` - Context propagation
- `test_missing_skill_error_e2e` - Error handling

### `test_complex_workflow.sh`
Tests complex multi-step workflows:
- Multi-turn conversations
- Tool execution (Read/Write)
- Error recovery

**Tests:**
- `test_multi_turn_conversation_e2e` - Context preservation
- `test_tool_execution_workflow_e2e` - Real tool I/O
- `test_error_recovery_workflow_e2e` - Error handling

## Framework

### `framework.sh`
Shared helper functions (STUB - not implemented):

**Session Management:**
- `start_rustyclawd_session` - Launch in tmux
- `cleanup_session` - Kill session
- `trap_cleanup` - Cleanup on exit/interrupt

**Input:**
- `send_command` - Send command with Enter
- `send_keys` - Send raw keys

**Output:**
- `capture_output` - Capture terminal output
- `save_output` - Save to file

**Validation:**
- `verify_output_contains` - Check for text
- `verify_output_matches` - Regex matching
- `wait_for_text` - Wait with timeout

**Debugging:**
- `dump_session_info` - Session info
- `take_screenshot` - Capture state

## Running Tests

**Current Status:** All tests fail with "Framework not implemented" errors.

**To run tests (they will fail):**
```bash
# Run single test
bash tests/e2e/tmux/test_slash_command_e2e.sh

# Run all tests
bash tests/e2e/tmux/test_slash_command_e2e.sh
bash tests/e2e/tmux/test_skills_e2e.sh
bash tests/e2e/tmux/test_complex_workflow.sh

# Or create a run_all.sh script (after implementation)
```

**Expected output:**
```
═══════════════════════════════════════════════════════
Test: /analyze command in real terminal
═══════════════════════════════════════════════════════

❌ FAIL: Framework not implemented - cannot run test

This test SHOULD:
  1. Start RustyClawd in tmux session
  2. Wait for welcome message
  ...
```

## Implementation Order

1. **Implement framework.sh functions** (Task 2.1)
   - Session management
   - Input injection
   - Output capture
   - Validation helpers

2. **Update test scripts** (Tasks 2.2-2.4)
   - Remove stub failure messages
   - Uncomment actual test code
   - Verify tests pass

3. **CI Integration**
   - Add to `.github/workflows/e2e-tests.yml`
   - Ensure tmux available in CI
   - Configure artifact collection on failure

## Success Criteria

Phase 2 succeeds when:
- [ ] All framework functions implemented
- [ ] All 3 test scripts passing
- [ ] Tests run successfully in CI
- [ ] Zero flakiness
- [ ] Manual verification: Real terminal rendering correct

## Debugging Tests

```bash
# Enable bash debug mode
bash -x tests/e2e/tmux/test_slash_command_e2e.sh

# Attach to tmux session manually (for debugging)
tmux attach -t rustyclawd-test-$$
# Press Ctrl+B, D to detach
```

## Common Patterns

```bash
# Start session
start_rustyclawd_session "$SESSION" 10 || exit 1

# Wait for specific text
wait_for_text "$SESSION" "Welcome" 30 || {
    test_fail "Timeout waiting for welcome"
    capture_output "$SESSION"  # Debug output
    exit 1
}

# Send input and verify
send_command "$SESSION" "/analyze src/" 3
verify_output_contains "$SESSION" "analysis" || {
    test_fail "Expected output not found"
    take_screenshot "$SESSION" "failure.txt"
    exit 1
}
```

## Documentation

- **Architecture:** `docs/architecture/e2e_testing_architecture.md`
- **Development Guide:** `docs/testing/E2E_TEST_DEVELOPMENT.md`
- **Framework Spec:** `docs/specs/tmux_framework_spec.md`

## Next Steps

After Phase 2:
- Move to Phase 3: YAML scenario tests
- Achieve 95% Claude Code parity

**Target:** 95% parity after Phase 2
