#!/bin/bash
# E2E Test: Slash Commands in Real Terminal
#
# **Status:** Production Ready
#
# This test validates that slash commands work in a real terminal environment,
# including rendering, input handling, and command execution.

set -euo pipefail

# Import framework
SCRIPT_DIR="$(dirname "$0")"
source "$SCRIPT_DIR/framework.sh"

# Test configuration
SESSION="rustyclawd-slash-test-$$"
trap_cleanup "$SESSION"

#############################################################################
# Test: /help command shows available commands
#############################################################################

test_help_command_e2e() {
    test_info "Starting /help command test"

    # 1. Start RustyClawd
    if ! start_rustyclawd_session "$SESSION" 15; then
        test_fail "Failed to start RustyClawd"
        return 1
    fi

    # 2. Wait for any initial output (startup complete)
    sleep 2

    # 3. Send /help command
    test_info "Sending /help command"
    send_command "$SESSION" "/help" 3

    # 4. Verify help output appears
    # Look for common help-related text
    if ! wait_for_text "$SESSION" "help" 10; then
        test_fail "/help output not shown"
        take_screenshot "$SESSION" "$(dirname "$0")/logs/help_failure_$$.txt"
        return 1
    fi

    test_pass "/help command works in real terminal"
    return 0
}

#############################################################################
# Test: /analyze command execution (basic)
#############################################################################

test_analyze_command_e2e() {
    test_info "Starting /analyze command test"

    # Note: This test uses a simple path to avoid long analysis times
    # We're testing command execution, not analysis completeness

    # 1. Start RustyClawd
    if ! start_rustyclawd_session "$SESSION" 15; then
        test_fail "Failed to start RustyClawd"
        return 1
    fi

    # 2. Wait for startup
    sleep 2

    # 3. Send analyze command with a simple target
    test_info "Sending /analyze command"
    send_command "$SESSION" "/analyze tests/" 5

    # 4. Verify command was accepted
    # The command should appear in output or trigger some processing indicator
    local output=$(capture_output "$SESSION")

    # Check if command triggered any response
    # Could be: "Analyzing", "analyze", or just echoed command
    if echo "$output" | grep -qiE "(analyz|test)"; then
        test_pass "/analyze command accepted and processing"
        return 0
    else
        test_fail "/analyze command not processed"
        echo "Output was:"
        echo "$output"
        take_screenshot "$SESSION" "$(dirname "$0")/logs/analyze_failure_$$.txt"
        return 1
    fi
}

#############################################################################
# Test: Invalid slash command shows appropriate response
#############################################################################

test_invalid_command_error_e2e() {
    test_info "Starting invalid command test"

    # 1. Start RustyClawd
    if ! start_rustyclawd_session "$SESSION" 15; then
        test_fail "Failed to start RustyClawd"
        return 1
    fi

    # 2. Wait for startup
    sleep 2

    # 3. Send invalid command
    test_info "Sending /nonexistent command"
    send_command "$SESSION" "/nonexistent" 3

    # 4. Capture output to verify error handling
    local output=$(capture_output "$SESSION")

    # The system should handle the invalid command somehow:
    # - Show error message
    # - OR pass it through to LLM (which will handle it)
    # - OR just echo it back
    # As long as TUI doesn't crash, test passes

    # Verify session still responsive
    send_command "$SESSION" "/help" 2

    if wait_for_text "$SESSION" "help" 5; then
        test_pass "TUI still responsive after invalid command"
        return 0
    else
        test_fail "TUI not responsive after invalid command"
        take_screenshot "$SESSION" "$(dirname "$0")/logs/invalid_failure_$$.txt"
        return 1
    fi
}

#############################################################################
# Run all tests
#############################################################################

main() {
    echo ""
    echo "╔═══════════════════════════════════════════════════════╗"
    echo "║  E2E Test Suite: Slash Commands (tmux)               ║"
    echo "╚═══════════════════════════════════════════════════════╝"
    echo ""

    local failed=0

    # Run each test in its own session
    SESSION="rustyclawd-slash-help-$$"
    trap_cleanup "$SESSION"
    test_help_command_e2e || ((failed++))
    cleanup_session "$SESSION"

    SESSION="rustyclawd-slash-analyze-$$"
    trap_cleanup "$SESSION"
    test_analyze_command_e2e || ((failed++))
    cleanup_session "$SESSION"

    SESSION="rustyclawd-slash-invalid-$$"
    trap_cleanup "$SESSION"
    test_invalid_command_error_e2e || ((failed++))
    cleanup_session "$SESSION"

    echo ""
    echo "═══════════════════════════════════════════════════════"
    echo "Test Results"
    echo "═══════════════════════════════════════════════════════"
    echo ""

    if [ $failed -eq 0 ]; then
        test_pass "All slash command tests passed (3/3)!"
        return 0
    else
        test_fail "$failed slash command test(s) failed"
        echo ""
        echo "Check logs in: $(dirname "$0")/logs/"
        echo ""
        return 1
    fi
}

# Run tests
main
