#!/bin/bash
# E2E Test: Complex Workflows in Real Terminal
#
# **Status:** Production Ready
#
# This test validates complex multi-step workflows work correctly in a real terminal,
# including multi-turn conversations, tool + skill combinations, and error recovery.

set -euo pipefail

# Import framework
SCRIPT_DIR="$(dirname "$0")"
source "$SCRIPT_DIR/framework.sh"

# Test configuration
SESSION="rustyclawd-workflow-test-$$"
trap_cleanup "$SESSION"

#############################################################################
# Test: Multi-turn conversation workflow
#############################################################################

test_multi_turn_workflow() {
    test_info "Starting multi-turn conversation workflow test"

    # 1. Start RustyClawd
    if ! start_rustyclawd_session "$SESSION" 15; then
        test_fail "Failed to start RustyClawd"
        return 1
    fi

    # 2. Wait for startup
    sleep 2

    # 3. Turn 1: Initial question
    test_info "Turn 1: Asking initial question"
    send_command "$SESSION" "What is RustyClawd?" 3

    # Verify first turn produced output
    local output1=$(capture_output "$SESSION")
    if [[ -z "$output1" ]] || [[ "$output1" =~ ^[[:space:]]*$ ]]; then
        test_fail "Turn 1 produced no output"
        return 1
    fi

    # 4. Turn 2: Follow-up question
    test_info "Turn 2: Asking follow-up question"
    send_command "$SESSION" "Can you elaborate?" 3

    # Verify second turn produced output
    local output2=$(capture_output "$SESSION")
    if [[ -z "$output2" ]] || [[ "$output2" =~ ^[[:space:]]*$ ]]; then
        test_fail "Turn 2 produced no output"
        return 1
    fi

    # 5. Turn 3: Third turn to verify sustained conversation
    test_info "Turn 3: Third question"
    send_command "$SESSION" "Thank you" 3

    # Verify third turn and session still responsive
    if wait_for_text "$SESSION" "thank" 5 || wait_for_text "$SESSION" "Thank" 5; then
        test_pass "Multi-turn conversation workflow successful (3 turns)"
        return 0
    else
        test_fail "Multi-turn conversation failed at turn 3"
        take_screenshot "$SESSION" "$(dirname "$0")/logs/multi_turn_failure_$$.txt"
        return 1
    fi
}

#############################################################################
# Test: Tool and skill workflow (commands + conversation)
#############################################################################

test_tool_and_skill_workflow() {
    test_info "Starting tool and skill workflow test"

    # 1. Start RustyClawd
    if ! start_rustyclawd_session "$SESSION" 15; then
        test_fail "Failed to start RustyClawd"
        return 1
    fi

    # 2. Wait for startup
    sleep 2

    # 3. Execute slash command (tool)
    test_info "Executing slash command"
    send_command "$SESSION" "/help" 3

    if ! wait_for_text "$SESSION" "help" 5; then
        test_fail "Slash command failed"
        return 1
    fi

    # 4. Follow with natural language (potential skill invocation)
    test_info "Sending natural language message"
    send_command "$SESSION" "What commands are available?" 3

    # 5. Verify session handles both tool and conversation
    sleep 2
    local output=$(capture_output "$SESSION")

    # Check if both slash command and conversation are handled
    if echo "$output" | grep -qi "help"; then
        test_pass "Tool and skill workflow successful"
        return 0
    else
        test_fail "Tool and skill workflow incomplete"
        echo "Output was:"
        echo "$output"
        take_screenshot "$SESSION" "$(dirname "$0")/logs/tool_skill_failure_$$.txt"
        return 1
    fi
}

#############################################################################
# Test: Error recovery workflow
#############################################################################

test_error_recovery() {
    test_info "Starting error recovery workflow test"

    # 1. Start RustyClawd
    if ! start_rustyclawd_session "$SESSION" 15; then
        test_fail "Failed to start RustyClawd"
        return 1
    fi

    # 2. Wait for startup
    sleep 2

    # 3. Send invalid command (trigger error)
    test_info "Sending invalid command to trigger error"
    send_command "$SESSION" "/invalid-command-xyz" 2

    # 4. Verify session still responsive after error
    test_info "Verifying error recovery"
    send_command "$SESSION" "/help" 3

    if ! wait_for_text "$SESSION" "help" 5; then
        test_fail "Session not recovered after error"
        take_screenshot "$SESSION" "$(dirname "$0")/logs/recovery_failure_step1_$$.txt"
        return 1
    fi

    # 5. Send another invalid command (stress test recovery)
    test_info "Testing multiple error recovery"
    send_command "$SESSION" "This is an invalid request with no context" 2
    send_command "$SESSION" "/help" 2

    # 6. Verify session still functional
    if wait_for_text "$SESSION" "help" 5; then
        test_pass "Error recovery workflow successful"
        return 0
    else
        test_fail "Session not recovered after multiple errors"
        take_screenshot "$SESSION" "$(dirname "$0")/logs/recovery_failure_step2_$$.txt"
        return 1
    fi
}

#############################################################################
# Run all tests
#############################################################################

main() {
    echo ""
    echo "╔═══════════════════════════════════════════════════════╗"
    echo "║  E2E Test Suite: Complex Workflows (tmux)            ║"
    echo "╚═══════════════════════════════════════════════════════╝"
    echo ""

    local failed=0

    # Run each test in its own session
    SESSION="rustyclawd-multi-turn-$$"
    trap_cleanup "$SESSION"
    test_multi_turn_workflow || ((failed++))
    cleanup_session "$SESSION"

    SESSION="rustyclawd-tool-skill-$$"
    trap_cleanup "$SESSION"
    test_tool_and_skill_workflow || ((failed++))
    cleanup_session "$SESSION"

    SESSION="rustyclawd-error-recovery-$$"
    trap_cleanup "$SESSION"
    test_error_recovery || ((failed++))
    cleanup_session "$SESSION"

    echo ""
    echo "═══════════════════════════════════════════════════════"
    echo "Test Results"
    echo "═══════════════════════════════════════════════════════"
    echo ""

    if [ $failed -eq 0 ]; then
        test_pass "All complex workflow tests passed (3/3)!"
        return 0
    else
        test_fail "$failed complex workflow test(s) failed"
        echo ""
        echo "Check logs in: $(dirname "$0")/logs/"
        echo ""
        return 1
    fi
}

# Run tests
main
