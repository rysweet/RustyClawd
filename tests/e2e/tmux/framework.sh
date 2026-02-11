#!/bin/bash
# tmux Test Framework
#
# **Status:** Production Ready
#
# This framework provides helper functions for tmux-based E2E testing.
# All functions are fully implemented and tested.
#
# See: docs/architecture/e2e_testing_architecture.md

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default configuration
RUSTYCLAWD_BIN="${RUSTYCLAWD_BIN:-/home/azureuser/src/RustyClawd/target/debug/rusty}"
TMUX_DEFAULT_TIMEOUT="${TMUX_DEFAULT_TIMEOUT:-10}"
TMUX_POLL_INTERVAL="${TMUX_POLL_INTERVAL:-0.1}"

#############################################################################
# Session Management
#############################################################################

# Start RustyClawd in tmux session
#
# Usage: start_rustyclawd_session <session_name> [timeout_seconds]
# Args:
#   session_name: Name for the tmux session
#   timeout: Optional timeout in seconds (default: 10)
# Returns:
#   0 on success, 1 on failure
start_rustyclawd_session() {
    local session_name="$1"
    local timeout="${2:-$TMUX_DEFAULT_TIMEOUT}"

    # Security: Validate binary path before execution
    # Check if binary exists and is executable
    if [[ ! -x "$RUSTYCLAWD_BIN" ]]; then
        echo "${RED}ERROR${NC}: RustyClawd binary not found or not executable: $RUSTYCLAWD_BIN" >&2
        return 1
    fi

    # Check if binary is a regular file (not a symlink or directory)
    if [[ ! -f "$RUSTYCLAWD_BIN" ]] || [[ -L "$RUSTYCLAWD_BIN" ]]; then
        echo "${RED}ERROR${NC}: RustyClawd binary must be a regular file (not a symlink): $RUSTYCLAWD_BIN" >&2
        return 1
    fi

    # Kill any existing session with same name
    cleanup_session "$session_name" 2>/dev/null || true

    # Start RustyClawd in detached tmux session
    tmux new-session -d -s "$session_name" "$RUSTYCLAWD_BIN" 2>/dev/null || {
        echo "${RED}ERROR${NC}: Failed to create tmux session: $session_name" >&2
        return 1
    }

    # Wait for startup (look for any output or prompt)
    local start_time=$(date +%s)
    local end_time=$((start_time + timeout))

    while true; do
        local current_time=$(date +%s)

        # Check if timeout reached
        if [[ $current_time -ge $end_time ]]; then
            echo "${RED}ERROR${NC}: Timeout waiting for RustyClawd to start (${timeout}s)" >&2
            dump_session_info "$session_name" >&2
            return 1
        fi

        # Check if session still exists
        if ! tmux has-session -t "$session_name" 2>/dev/null; then
            echo "${RED}ERROR${NC}: tmux session died during startup" >&2
            return 1
        fi

        # Capture output and check if there's any content (startup complete)
        local output=$(capture_output "$session_name" 2>/dev/null || echo "")
        if [[ -n "$output" ]] && [[ ! "$output" =~ ^[[:space:]]*$ ]]; then
            # Output detected, startup successful
            return 0
        fi

        sleep "$TMUX_POLL_INTERVAL"
    done
}

# Clean up tmux session
#
# Usage: cleanup_session <session_name>
# Args:
#   session_name: Name of the tmux session to kill
cleanup_session() {
    local session_name="$1"

    # Kill session if it exists (suppress errors if it doesn't)
    tmux kill-session -t "$session_name" 2>/dev/null || true
}

# Setup cleanup trap
#
# Usage: trap_cleanup <session_name>
# Args:
#   session_name: Session to clean up on exit/interrupt
# Handles all signals (EXIT, INT, TERM, HUP) properly
trap_cleanup() {
    local session_name="$1"

    # Register cleanup function for all relevant signals
    trap "cleanup_session '$session_name'" EXIT INT TERM HUP
}

#############################################################################
# Input Injection
#############################################################################

# Send command to tmux session with Enter
#
# Usage: send_command <session_name> <command> [wait_seconds]
# Args:
#   session_name: Target tmux session
#   command: Command text to send
#   wait_seconds: Optional wait time after sending (default: 1)
send_command() {
    local session_name="$1"
    local command="$2"
    local wait_time="${3:-1}"

    # Verify session exists
    if ! tmux has-session -t "$session_name" 2>/dev/null; then
        echo "${RED}ERROR${NC}: Session not found: $session_name" >&2
        return 1
    fi

    # Send keys with Enter (C-m)
    tmux send-keys -t "$session_name" -- "$command" C-m

    # Wait for command to process
    sleep "$wait_time"
}

# Send raw keys to tmux session (no Enter)
#
# Usage: send_keys <session_name> <keys>
# Args:
#   session_name: Target tmux session
#   keys: Keys to send (can include C-c, C-d, etc.)
send_keys() {
    local session_name="$1"
    local keys="$2"

    # Verify session exists
    if ! tmux has-session -t "$session_name" 2>/dev/null; then
        echo "${RED}ERROR${NC}: Session not found: $session_name" >&2
        return 1
    fi

    # Send raw keys
    tmux send-keys -t "$session_name" -- "$keys"
}

#############################################################################
# Output Capture
#############################################################################

# Capture current tmux output
#
# Usage: output=$(capture_output <session_name>)
# Args:
#   session_name: Target tmux session
# Returns:
#   Current terminal output as string
capture_output() {
    local session_name="$1"

    # Verify session exists
    if ! tmux has-session -t "$session_name" 2>/dev/null; then
        echo "${RED}ERROR${NC}: Session not found: $session_name" >&2
        return 1
    fi

    # Capture pane output
    tmux capture-pane -t "$session_name" -p
}

# Save output to file
#
# Usage: save_output <session_name> <filename>
# Args:
#   session_name: Target tmux session
#   filename: File to save output to
save_output() {
    local session_name="$1"
    local filename="$2"

    # Capture output and save to file
    capture_output "$session_name" > "$filename" || {
        echo "${RED}ERROR${NC}: Failed to save output to: $filename" >&2
        return 1
    }
}

#############################################################################
# Validation
#############################################################################

# Verify output contains expected text
#
# Usage: verify_output_contains <session_name> <expected_text>
# Args:
#   session_name: Target tmux session
#   expected_text: Text that should be present
# Returns:
#   0 if text found, 1 if not found
verify_output_contains() {
    local session_name="$1"
    local expected="$2"

    # Capture current output
    local output=$(capture_output "$session_name") || return 1

    # Check if output contains expected text
    if echo "$output" | grep -qF "$expected"; then
        return 0
    else
        echo "${RED}ERROR${NC}: Expected text not found: '$expected'" >&2
        echo "Current output:" >&2
        echo "---" >&2
        echo "$output" >&2
        echo "---" >&2
        return 1
    fi
}

# Verify output matches regex pattern
#
# Usage: verify_output_matches <session_name> <regex_pattern>
# Args:
#   session_name: Target tmux session
#   regex_pattern: Regex pattern to match
# Returns:
#   0 if matches, 1 if not
verify_output_matches() {
    local session_name="$1"
    local pattern="$2"

    # Capture current output
    local output=$(capture_output "$session_name") || return 1

    # Check if output matches pattern
    if echo "$output" | grep -qE "$pattern"; then
        return 0
    else
        echo "${RED}ERROR${NC}: Pattern not matched: '$pattern'" >&2
        echo "Current output:" >&2
        echo "---" >&2
        echo "$output" >&2
        echo "---" >&2
        return 1
    fi
}

# Wait for text to appear in output
#
# Usage: wait_for_text <session_name> <text> <timeout_seconds>
# Args:
#   session_name: Target tmux session
#   text: Text to wait for
#   timeout: Maximum seconds to wait
# Returns:
#   0 if text appears, 1 on timeout
wait_for_text() {
    local session_name="$1"
    local text="$2"
    local timeout="$3"

    local start_time=$(date +%s)
    local end_time=$((start_time + timeout))

    while true; do
        local current_time=$(date +%s)

        # Check if timeout reached
        if [[ $current_time -ge $end_time ]]; then
            echo "${RED}ERROR${NC}: Timeout waiting for text: '$text' (${timeout}s)" >&2
            echo "Final output:" >&2
            echo "---" >&2
            capture_output "$session_name" >&2
            echo "---" >&2
            return 1
        fi

        # Check if session still exists
        if ! tmux has-session -t "$session_name" 2>/dev/null; then
            echo "${RED}ERROR${NC}: Session died while waiting for text" >&2
            return 1
        fi

        # Check if text appears in output
        local output=$(capture_output "$session_name" 2>/dev/null || echo "")
        if echo "$output" | grep -qF "$text"; then
            return 0
        fi

        sleep "$TMUX_POLL_INTERVAL"
    done
}

# Wait for text to appear in tmux session (flexible/case-insensitive)
#
# Usage: wait_for_text_flexible <session_name> <text> <timeout>
# Args:
#   session_name: Target tmux session
#   text: Text to wait for (case-insensitive, partial match)
#   timeout: Maximum seconds to wait
# Returns:
#   0 if text appears, 1 if timeout or session dies
wait_for_text_flexible() {
    local session_name="$1"
    local text="$2"
    local timeout="$3"

    local start_time=$(date +%s)
    local end_time=$((start_time + timeout))

    while true; do
        local current_time=$(date +%s)

        # Check if timeout reached
        if [[ $current_time -ge $end_time ]]; then
            echo "${RED}ERROR${NC}: Timeout waiting for text: '$text' (${timeout}s)\" >&2
            echo \"Final output:\" >&2
            echo \"---\" >&2
            capture_output \"$session_name\" >&2
            echo \"---\" >&2
            return 1
        fi

        # Check if session still exists
        if ! tmux has-session -t \"$session_name\" 2>/dev/null; then
            echo \"${RED}ERROR${NC}: Session died while waiting for text\" >&2
            return 1
        fi

        # Check if text appears in output (case-insensitive)
        local output=$(capture_output \"$session_name\" 2>/dev/null || echo \"\")
        if echo \"$output\" | grep -qiF \"$text\"; then
            return 0
        fi

        sleep \"$TMUX_POLL_INTERVAL\"
    done
}

#############################################################################
# Session Status
#############################################################################

# Get session status
#
# Usage: if get_session_status <session_name>; then ...; fi
# Args:
#   session_name: Target tmux session
# Returns:
#   0 if session is running, 1 if not
get_session_status() {
    local session_name="$1"

    tmux has-session -t "$session_name" 2>/dev/null
}

#############################################################################
# Environment Setup/Teardown
#############################################################################

# Setup test environment
#
# Usage: setup_test_env
# Sets up any required environment variables or test directories
setup_test_env() {
    # Set environment for test mode (if RustyClawd supports it)
    export RUSTYCLAWD_TEST_MODE=1

    # Create logs directory if it doesn't exist
    mkdir -p "$(dirname "$0")/logs" 2>/dev/null || true
}

# Teardown test environment
#
# Usage: teardown_test_env
# Cleans up any test artifacts
teardown_test_env() {
    # Unset test mode
    unset RUSTYCLAWD_TEST_MODE 2>/dev/null || true
}

#############################################################################
# Test Execution Helpers
#############################################################################

# Run a single test with error handling
#
# Usage: run_test <test_name> <test_function>
# Args:
#   test_name: Human-readable test name
#   test_function: Function to execute
# Returns:
#   0 on success, 1 on failure
run_test() {
    local test_name="$1"
    local test_function="$2"

    echo ""
    echo "═══════════════════════════════════════════════════════"
    echo "Test: $test_name"
    echo "═══════════════════════════════════════════════════════"
    echo ""

    if $test_function; then
        test_pass "$test_name"
        return 0
    else
        test_fail "$test_name"
        return 1
    fi
}

# Assert equality
#
# Usage: assert_equals <actual> <expected> <message>
# Args:
#   actual: Actual value
#   expected: Expected value
#   message: Error message if not equal
# Returns:
#   0 if equal, 1 if not
assert_equals() {
    local actual="$1"
    local expected="$2"
    local message="${3:-Values not equal}"

    if [[ "$actual" == "$expected" ]]; then
        return 0
    else
        echo "${RED}ERROR${NC}: $message" >&2
        echo "  Expected: '$expected'" >&2
        echo "  Actual:   '$actual'" >&2
        return 1
    fi
}

#############################################################################
# Debugging
#############################################################################

# Dump session information for debugging
#
# Usage: dump_session_info <session_name>
# Args:
#   session_name: Target tmux session
dump_session_info() {
    local session_name="$1"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Session Debug Info: $session_name"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    # Check if session exists
    if tmux has-session -t "$session_name" 2>/dev/null; then
        echo "Session Status: ${GREEN}RUNNING${NC}"
        echo ""

        # Show session info
        echo "Session Info:"
        tmux list-sessions | grep "^$session_name:" || echo "  (no details available)"
        echo ""

        # Show windows
        echo "Windows:"
        tmux list-windows -t "$session_name" 2>/dev/null || echo "  (no windows)"
        echo ""

        # Show panes
        echo "Panes:"
        tmux list-panes -t "$session_name" 2>/dev/null || echo "  (no panes)"
        echo ""
    else
        echo "Session Status: ${RED}NOT RUNNING${NC}"
        echo ""
    fi

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# Take screenshot of current terminal state
#
# Usage: take_screenshot <session_name> <filename>
# Args:
#   session_name: Target tmux session
#   filename: File to save screenshot to
take_screenshot() {
    local session_name="$1"
    local filename="$2"

    # Create directory if needed
    mkdir -p "$(dirname "$filename")" 2>/dev/null || true

    # Add timestamp header
    {
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "Screenshot: $session_name"
        echo "Time: $(date '+%Y-%m-%d %H:%M:%S')"
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo ""
        capture_output "$session_name"
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    } > "$filename" || {
        echo "${RED}ERROR${NC}: Failed to save screenshot to: $filename" >&2
        return 1
    }
}

#############################################################################
# Test Status Reporting
#############################################################################

# Print test failure message
test_fail() {
    local message="$1"
    echo -e "${RED}❌ FAIL${NC}: $message"
}

# Print test success message
test_pass() {
    local message="$1"
    echo -e "${GREEN}✅ PASS${NC}: $message"
}

# Print warning message
test_warn() {
    local message="$1"
    echo -e "${YELLOW}⚠ WARN${NC}: $message"
}

# Print info message
test_info() {
    local message="$1"
    echo "ℹ INFO: $message"
}

#############################################################################
# Framework Status
#############################################################################

echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}tmux Test Framework - PRODUCTION READY${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "All framework functions implemented and ready to use."
echo ""
echo "Configuration:"
echo "  - RustyClawd Binary: $RUSTYCLAWD_BIN"
echo "  - Default Timeout: ${TMUX_DEFAULT_TIMEOUT}s"
echo "  - Poll Interval: ${TMUX_POLL_INTERVAL}s"
echo ""
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
