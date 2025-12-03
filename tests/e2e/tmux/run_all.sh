#!/bin/bash
# Master Test Runner for tmux E2E Tests
#
# **Status:** Production Ready
#
# This script runs all tmux-based E2E tests and reports results.

set -euo pipefail

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test directory
SCRIPT_DIR="$(dirname "$0")"
cd "$SCRIPT_DIR"

# Log directory
LOG_DIR="$SCRIPT_DIR/logs"
mkdir -p "$LOG_DIR"

# Test tracking
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
START_TIME=$(date +%s)

#############################################################################
# Helper Functions
#############################################################################

print_header() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

print_section() {
    echo ""
    echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${YELLOW}$1${NC}"
    echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}"
    echo ""
}

run_test_suite() {
    local test_file="$1"
    local test_name=$(basename "$test_file" .sh)

    print_section "Running: $test_name"

    ((TOTAL_TESTS++))

    # Run test and capture output
    local log_file="$LOG_DIR/${test_name}_$(date +%Y%m%d_%H%M%S).log"

    if bash "$test_file" 2>&1 | tee "$log_file"; then
        echo -e "${GREEN}✅ PASSED${NC}: $test_name"
        ((PASSED_TESTS++))
        return 0
    else
        echo -e "${RED}❌ FAILED${NC}: $test_name"
        echo -e "${YELLOW}⚠  Log saved to:${NC} $log_file"
        ((FAILED_TESTS++))
        return 1
    fi
}

print_summary() {
    local end_time=$(date +%s)
    local duration=$((end_time - START_TIME))

    echo ""
    print_header "Test Run Summary"

    echo "Total Test Suites: $TOTAL_TESTS"
    echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
    echo -e "Failed: ${RED}$FAILED_TESTS${NC}"
    echo "Duration: ${duration}s"
    echo ""

    if [ $FAILED_TESTS -eq 0 ]; then
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${GREEN}🎉 ALL TESTS PASSED! Phase 2 Complete!${NC}"
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        return 0
    else
        echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo -e "${RED}❌ SOME TESTS FAILED${NC}"
        echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
        echo ""
        echo "Check logs in: $LOG_DIR"
        return 1
    fi
}

#############################################################################
# Main Execution
#############################################################################

main() {
    print_header "RustyClawd E2E Test Suite - Phase 2 (tmux)"

    echo "Starting tmux-based E2E tests..."
    echo "This will test RustyClawd in real terminal sessions."
    echo ""

    # Prerequisites check
    if ! command -v tmux &> /dev/null; then
        echo -e "${RED}ERROR${NC}: tmux not found. Please install tmux first."
        echo ""
        echo "Install instructions:"
        echo "  macOS:  brew install tmux"
        echo "  Ubuntu: sudo apt install tmux"
        echo "  Fedora: sudo dnf install tmux"
        exit 1
    fi

    # Check if RustyClawd binary exists
    if [ ! -x "/home/azureuser/src/RustyClawd/worktrees/feat/issue-103-e2e-testing/target/debug/claude" ]; then
        echo -e "${YELLOW}WARNING${NC}: RustyClawd binary not found at expected location."
        echo "Building RustyClawd first..."
        echo ""
        cd /home/azureuser/src/RustyClawd/worktrees/feat/issue-103-e2e-testing
        if ! cargo build 2>&1 | tail -20; then
            echo -e "${RED}ERROR${NC}: Failed to build RustyClawd"
            exit 1
        fi
        cd "$SCRIPT_DIR"
        echo ""
        echo -e "${GREEN}✅ Build successful${NC}"
        echo ""
    fi

    # Run test suites
    run_test_suite "./test_slash_command_e2e.sh"
    run_test_suite "./test_skills_e2e.sh"
    run_test_suite "./test_complex_workflow.sh"

    # Print summary
    print_summary
}

# Cleanup on exit
cleanup() {
    # Kill any lingering tmux sessions from tests
    tmux kill-server 2>/dev/null || true
}

trap cleanup EXIT

# Run main
main
