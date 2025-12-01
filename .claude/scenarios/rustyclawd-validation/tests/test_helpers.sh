#!/bin/bash
# test_helpers.sh - Shared test utilities for RustyClawd validation tests
#
# Philosophy: Ruthlessly simple testing utilities
# - No external dependencies (pure bash)
# - Clear assertion messages
# - Mock command support
#
# Usage:
#   source test_helpers.sh
#   test_my_function() {
#       run command_to_test
#       assert_success
#       assert_output_contains "expected text"
#   }

set -euo pipefail

# Test state variables
TEST_OUTPUT=""
TEST_EXIT_CODE=0
TEST_TMPDIR=""
MOCK_COMMANDS=()

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test statistics
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Setup test environment
setup_test_env() {
    TEST_TMPDIR=$(mktemp -d)
    export PATH="$TEST_TMPDIR/mocks:$PATH"
    mkdir -p "$TEST_TMPDIR/mocks"
}

# Cleanup test environment
teardown_test_env() {
    if [[ -n "$TEST_TMPDIR" ]] && [[ -d "$TEST_TMPDIR" ]]; then
        rm -rf "$TEST_TMPDIR"
    fi
    TEST_TMPDIR=""
}

# Run a command and capture output
run() {
    TEST_OUTPUT=$("$@" 2>&1) || TEST_EXIT_CODE=$?
    TEST_EXIT_CODE=${TEST_EXIT_CODE:-0}
}

# Assertions
assert_success() {
    if [[ $TEST_EXIT_CODE -ne 0 ]]; then
        echo -e "${RED}FAIL${NC}: Expected success (exit 0), got exit code $TEST_EXIT_CODE"
        echo "Output: $TEST_OUTPUT"
        return 1
    fi
}

assert_failure() {
    if [[ $TEST_EXIT_CODE -eq 0 ]]; then
        echo -e "${RED}FAIL${NC}: Expected failure (exit != 0), got exit code 0"
        echo "Output: $TEST_OUTPUT"
        return 1
    fi
}

assert_exit_code() {
    local expected=$1
    if [[ $TEST_EXIT_CODE -ne $expected ]]; then
        echo -e "${RED}FAIL${NC}: Expected exit code $expected, got $TEST_EXIT_CODE"
        echo "Output: $TEST_OUTPUT"
        return 1
    fi
}

assert_output_contains() {
    local expected=$1
    if ! echo "$TEST_OUTPUT" | grep -q "$expected"; then
        echo -e "${RED}FAIL${NC}: Output does not contain: $expected"
        echo "Actual output: $TEST_OUTPUT"
        return 1
    fi
}

assert_output_not_contains() {
    local unexpected=$1
    if echo "$TEST_OUTPUT" | grep -q "$unexpected"; then
        echo -e "${RED}FAIL${NC}: Output should not contain: $unexpected"
        echo "Actual output: $TEST_OUTPUT"
        return 1
    fi
}

assert_file_exists() {
    local filepath=$1
    if [[ ! -f "$filepath" ]]; then
        echo -e "${RED}FAIL${NC}: File does not exist: $filepath"
        return 1
    fi
}

assert_file_not_exists() {
    local filepath=$1
    if [[ -f "$filepath" ]]; then
        echo -e "${RED}FAIL${NC}: File should not exist: $filepath"
        return 1
    fi
}

assert_file_contains() {
    local filepath=$1
    local expected=$2
    if [[ ! -f "$filepath" ]]; then
        echo -e "${RED}FAIL${NC}: File does not exist: $filepath"
        return 1
    fi
    if ! grep -q "$expected" "$filepath"; then
        echo -e "${RED}FAIL${NC}: File $filepath does not contain: $expected"
        echo "File contents:"
        cat "$filepath"
        return 1
    fi
}

assert_dir_exists() {
    local dirpath=$1
    if [[ ! -d "$dirpath" ]]; then
        echo -e "${RED}FAIL${NC}: Directory does not exist: $dirpath"
        return 1
    fi
}

assert_equals() {
    local expected=$1
    local actual=$2
    if [[ "$expected" != "$actual" ]]; then
        echo -e "${RED}FAIL${NC}: Expected '$expected', got '$actual'"
        return 1
    fi
}

# Mocking functions
mock_command() {
    local cmd_name=$1
    local cmd_behavior=$2

    local mock_path="$TEST_TMPDIR/mocks/$cmd_name"
    cat > "$mock_path" <<EOF
#!/bin/bash
$cmd_behavior
EOF
    chmod +x "$mock_path"
    MOCK_COMMANDS+=("$cmd_name")
}

# Clear all mocks
clear_mocks() {
    for cmd in "${MOCK_COMMANDS[@]}"; do
        rm -f "$TEST_TMPDIR/mocks/$cmd"
    done
    MOCK_COMMANDS=()
}

# Test runner
run_test() {
    local test_name=$1
    local test_function=$2

    TESTS_RUN=$((TESTS_RUN + 1))

    # Setup
    setup_test_env
    TEST_OUTPUT=""
    TEST_EXIT_CODE=0

    echo -n "Running: $test_name ... "

    # Run test
    if $test_function; then
        echo -e "${GREEN}PASS${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi

    # Teardown
    teardown_test_env
}

# Test suite runner
run_test_suite() {
    local suite_name=$1
    shift
    local tests=("$@")

    echo ""
    echo "========================================"
    echo "Test Suite: $suite_name"
    echo "========================================"

    TESTS_RUN=0
    TESTS_PASSED=0
    TESTS_FAILED=0

    for test_func in "${tests[@]}"; do
        run_test "$test_func" "$test_func"
    done

    echo ""
    echo "========================================"
    echo "Results: $TESTS_PASSED passed, $TESTS_FAILED failed, $TESTS_RUN total"
    echo "========================================"

    if [[ $TESTS_FAILED -gt 0 ]]; then
        return 1
    fi
}

# Print test summary
print_summary() {
    echo ""
    echo "========================================"
    echo "TOTAL: $TESTS_PASSED passed, $TESTS_FAILED failed, $TESTS_RUN total"
    echo "========================================"

    if [[ $TESTS_FAILED -gt 0 ]]; then
        echo -e "${RED}Some tests failed!${NC}"
        exit 1
    else
        echo -e "${GREEN}All tests passed!${NC}"
        exit 0
    fi
}
