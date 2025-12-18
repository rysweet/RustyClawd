# Module Specification: tmux Test Framework

**Module Type:** Test Infrastructure
**Layer:** Real Terminal E2E Tests
**Purpose:** Bash helpers for testing RustyClawd in real terminal environment

---

## Philosophy

**Single Responsibility:** Provide tmux session management for E2E testing

**Ruthless Simplicity:** Pure bash, no additional dependencies

**Zero-BS:** Tests real terminal interaction, not mocks

**Self-Contained:** All tmux helpers in one shell script

---

## Public API (Functions)

### Session Management

```bash
# Start RustyClawd in detached tmux session
#
# Arguments:
#   $1 - session_name: Unique name for tmux session
#   $2 - timeout: Startup timeout in seconds (default: 10)
#
# Returns:
#   0 on success, 1 on timeout
#
# Example:
#   start_rustyclawd_session "test-$$" 10
start_rustyclawd_session()
```

```bash
# Cleanup tmux session (kill and remove)
#
# Arguments:
#   $1 - session_name: Name of session to clean up
#
# Example:
#   cleanup_session "test-$$"
cleanup_session()
```

```bash
# Setup trap for automatic cleanup on exit
#
# Arguments:
#   $1 - session_name: Name of session to cleanup on exit
#
# Example:
#   trap_cleanup "test-$$"
trap_cleanup()
```

### Input Injection

```bash
# Send command to tmux session
#
# Arguments:
#   $1 - session_name: Target session
#   $2 - command: Text to send
#   $3 - wait_time: Seconds to wait after (default: 1)
#
# Example:
#   send_command "test-$$" "/analyze src/" 3
send_command()
```

```bash
# Send raw keys to tmux session
#
# Arguments:
#   $1 - session_name: Target session
#   $2 - keys: tmux key notation (e.g., "C-c" for Ctrl-C)
#
# Example:
#   send_keys "test-$$" "C-c"
send_keys()
```

### Output Capture

```bash
# Capture full pane output as text
#
# Arguments:
#   $1 - session_name: Target session
#
# Output:
#   Prints pane content to stdout
#
# Example:
#   output=$(capture_output "test-$$")
capture_output()
```

```bash
# Save output to file
#
# Arguments:
#   $1 - session_name: Target session
#   $2 - filename: Path to save output
#
# Example:
#   save_output "test-$$" "logs/test-output.txt"
save_output()
```

### Validation

```bash
# Verify output contains expected text
#
# Arguments:
#   $1 - session_name: Target session
#   $2 - expected: Text that should be present
#
# Returns:
#   0 if text found, 1 if not found
#
# Example:
#   verify_output_contains "test-$$" "Welcome"
verify_output_contains()
```

```bash
# Verify output matches regex
#
# Arguments:
#   $1 - session_name: Target session
#   $2 - regex: Extended regex pattern
#
# Returns:
#   0 if matches, 1 if not
#
# Example:
#   verify_output_matches "test-$$" "Found [0-9]+ modules"
verify_output_matches()
```

```bash
# Wait for specific text to appear
#
# Arguments:
#   $1 - session_name: Target session
#   $2 - text: Text to wait for
#   $3 - timeout: Maximum wait time in seconds
#
# Returns:
#   0 if text appears, 1 on timeout
#
# Example:
#   wait_for_text "test-$$" "Analyzing" 10
wait_for_text()
```

### Debugging

```bash
# Dump session information for debugging
#
# Arguments:
#   $1 - session_name: Target session
#
# Output:
#   Session info, window list, pane info
#
# Example:
#   dump_session_info "test-$$"
dump_session_info()
```

```bash
# Take screenshot of current pane state
#
# Arguments:
#   $1 - session_name: Target session
#   $2 - filename: Path to save screenshot
#
# Example:
#   take_screenshot "test-$$" "logs/failure.txt"
take_screenshot()
```

---

## Dependencies

### External Commands

- `tmux` - Terminal multiplexer (required)
- `grep` - Text search (standard)
- `sleep` - Timing delays (standard)
- `date` - Timestamps (standard)

### Environment Variables

```bash
# Test mode flag (disable real API calls)
RUSTYCLAWD_TEST_MODE=1

# Custom cargo build flags
RUSTYCLAWD_BUILD_FLAGS="--release"

# Default timeout for operations (seconds)
TMUX_DEFAULT_TIMEOUT=10
```

---

## Implementation Notes

### Key Design Decisions

**1. Pure Bash**
- Rationale: Maximum simplicity, no dependencies
- Alternatives: Python, Rust test harness
- Trade-off: Less type safety, more portable

**2. Session Naming**
- Use PID suffix (`test-$$`) to avoid conflicts
- Cleanup in trap ensures no zombie sessions
- Unique names allow parallel test execution

**3. Timeout Handling**
- All wait operations have timeouts
- Prevent hanging tests
- Clear error messages on timeout

### Error Handling

```bash
# Standard error handling pattern
set -euo pipefail  # Exit on error, undefined vars, pipe failures

# Function template with error handling
function_name() {
    local param=$1

    # Validate inputs
    if [[ -z "$param" ]]; then
        echo "ERROR: Missing required parameter" >&2
        return 1
    fi

    # Perform operation
    if ! some_command; then
        echo "ERROR: Operation failed" >&2
        return 1
    fi

    return 0
}
```

### Logging

```bash
# Log to both stdout and file
log() {
    local level=$1
    shift
    local message="$@"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    echo "[$timestamp] $level: $message" | tee -a "$LOG_FILE"
}

# Usage
log "INFO" "Starting test"
log "ERROR" "Test failed"
```

---

## Test Requirements

### Unit Tests (Framework Functions)

```bash
# test_tmux_framework.sh

test_session_creation() {
    local session="test-unit-$$"

    if start_rustyclawd_session "$session" 5; then
        echo "PASS: Session created"
        cleanup_session "$session"
        return 0
    else
        echo "FAIL: Session creation failed"
        return 1
    fi
}

test_output_capture() {
    local session="test-capture-$$"

    # Start session
    start_rustyclawd_session "$session" 5 || return 1

    # Capture output
    local output=$(capture_output "$session")

    # Verify output not empty
    if [[ -n "$output" ]]; then
        echo "PASS: Output captured"
        cleanup_session "$session"
        return 0
    else
        echo "FAIL: No output captured"
        cleanup_session "$session"
        return 1
    fi
}

test_text_verification() {
    local session="test-verify-$$"

    # Start session
    start_rustyclawd_session "$session" 5 || return 1

    # Wait for welcome
    if wait_for_text "$session" "Welcome" 5; then
        echo "PASS: Text verification works"
        cleanup_session "$session"
        return 0
    else
        echo "FAIL: Text not found"
        cleanup_session "$session"
        return 1
    fi
}
```

### Integration Tests (With RustyClawd)

```bash
# test_rustyclawd_integration.sh

test_startup_and_welcome() {
    local session="test-startup-$$"
    trap_cleanup "$session"

    # Start RustyClawd
    if ! start_rustyclawd_session "$session" 10; then
        echo "FAIL: RustyClawd failed to start"
        return 1
    fi

    # Verify welcome message
    if ! verify_output_contains "$session" "Welcome"; then
        echo "FAIL: Welcome message not shown"
        return 1
    fi

    echo "PASS: Startup and welcome"
    return 0
}

test_slash_command_execution() {
    local session="test-slash-$$"
    trap_cleanup "$session"

    # Start session
    start_rustyclawd_session "$session" 10 || return 1

    # Send slash command
    send_command "$session" "/analyze src/" 3

    # Verify processing
    if ! wait_for_text "$session" "Analyzing" 10; then
        echo "FAIL: Slash command not processed"
        return 1
    fi

    echo "PASS: Slash command execution"
    return 0
}
```

---

## Usage Examples

### Example 1: Basic Test Script

```bash
#!/bin/bash
set -euo pipefail

# Import framework
source "$(dirname "$0")/framework.sh"

# Test configuration
SESSION="my-test-$$"
trap_cleanup "$SESSION"

# Start RustyClawd
echo "Starting RustyClawd..."
if ! start_rustyclawd_session "$SESSION" 10; then
    echo "ERROR: Failed to start RustyClawd"
    exit 1
fi

# Verify welcome
echo "Checking welcome message..."
if ! verify_output_contains "$SESSION" "Welcome"; then
    echo "ERROR: Welcome message not found"
    exit 1
fi

echo "✅ Test passed"
```

### Example 2: Multi-Step Test

```bash
#!/bin/bash
source framework.sh

SESSION="multi-step-$$"
trap_cleanup "$SESSION"

# Step 1: Start
start_rustyclawd_session "$SESSION" 10 || exit 1

# Step 2: Send command
send_command "$SESSION" "/analyze src/" 3

# Step 3: Wait for result
if ! wait_for_text "$SESSION" "modules" 20; then
    take_screenshot "$SESSION" "logs/failure.txt"
    echo "ERROR: Analysis failed"
    exit 1
fi

# Step 4: Verify details
if ! verify_output_matches "$SESSION" "Found [0-9]+ modules"; then
    echo "ERROR: Module count not shown"
    exit 1
fi

echo "✅ Multi-step test passed"
```

### Example 3: Error Recovery Test

```bash
#!/bin/bash
source framework.sh

SESSION="error-test-$$"
trap_cleanup "$SESSION"

# Start session
start_rustyclawd_session "$SESSION" 10 || exit 1

# Send invalid command
send_command "$SESSION" "/invalid-command" 2

# Verify error message
if ! verify_output_contains "$SESSION" "Unknown command"; then
    echo "ERROR: Error message not shown"
    exit 1
fi

# Verify session still responsive
send_command "$SESSION" "/help" 2

if ! verify_output_contains "$SESSION" "Available commands"; then
    echo "ERROR: Session not responsive after error"
    exit 1
fi

echo "✅ Error recovery test passed"
```

### Example 4: Performance Test

```bash
#!/bin/bash
source framework.sh

SESSION="perf-test-$$"
trap_cleanup "$SESSION"

# Measure startup time
START_TIME=$(date +%s)

start_rustyclawd_session "$SESSION" 10 || exit 1

if ! wait_for_text "$SESSION" "Welcome" 10; then
    echo "ERROR: Startup timeout"
    exit 1
fi

END_TIME=$(date +%s)
STARTUP_TIME=$((END_TIME - START_TIME))

echo "Startup time: ${STARTUP_TIME}s"

if [[ $STARTUP_TIME -gt 5 ]]; then
    echo "WARNING: Slow startup (> 5s)"
fi

echo "✅ Performance test completed"
```

---

## Performance Considerations

**Session Startup:**
- Typical: 2-3 seconds for RustyClawd to start
- Timeout: 10 seconds default
- No artificial delays

**Output Capture:**
- Instant (no I/O wait)
- Buffer size: 80x24 cells typical
- Use sparingly in tight loops

**Wait Operations:**
- Poll interval: 100ms default
- Configurable timeouts
- Early exit on match

---

## Future Enhancements

**Phase 2 (If Needed):**
- Parallel session support
- Video recording of sessions (script, scriptreplay)
- Performance metrics collection
- Screenshot comparison

**NOT Planned:**
- Windows support (WSL sufficient)
- GUI interaction (TUI only)
- Network simulation

---

## Contract Verification

**This module succeeds when:**

1. ✅ Session management works reliably
2. ✅ Input injection accurate
3. ✅ Output capture works
4. ✅ Validation functions reliable
5. ✅ Cleanup always happens (trap)
6. ✅ Error handling robust
7. ✅ Portable (Linux + macOS)
8. ✅ No external dependencies beyond tmux

**This module fails if:**

- Sessions leak (not cleaned up)
- Timing issues cause flakiness
- Output capture incomplete
- Works on one platform but not another
- Requires non-standard tools

---

## See Also

- [E2E Testing Architecture](../architecture/e2e_testing_architecture.md)
- [tmux Manual](https://man.openbsd.org/tmux)
- [Bash Best Practices](https://google.github.io/styleguide/shellguide.html)
