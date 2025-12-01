#!/bin/bash
# run_all_tests.sh - Master test runner for RustyClawd validation
#
# Philosophy: Run complete test suite following testing pyramid
# - 60% Unit tests
# - 30% Integration tests
# - 10% E2E tests
#
# Usage:
#   ./run_all_tests.sh           # Run all tests
#   ./run_all_tests.sh unit      # Run only unit tests
#   ./run_all_tests.sh integration # Run only integration tests
#   ./run_all_tests.sh e2e       # Run only E2E tests

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results
TOTAL_TESTS=0
TOTAL_PASSED=0
TOTAL_FAILED=0

# Test categories
UNIT_TESTS=(
    "test_bootstrap.sh"
    "test_validate_phases.sh"
    "test_report_generation.sh"
)

INTEGRATION_TESTS=(
    "test_integration.sh"
)

E2E_TESTS=(
    "test_e2e.sh"
)

run_test_file() {
    local test_file=$1
    local test_path="$SCRIPT_DIR/$test_file"

    if [[ ! -f "$test_path" ]]; then
        echo -e "${YELLOW}SKIP${NC}: $test_file (not found)"
        return 0
    fi

    echo ""
    echo -e "${BLUE}===========================================${NC}"
    echo -e "${BLUE}Running: $test_file${NC}"
    echo -e "${BLUE}===========================================${NC}"

    if bash "$test_path"; then
        echo -e "${GREEN}✓ PASSED${NC}: $test_file"
        return 0
    else
        echo -e "${RED}✗ FAILED${NC}: $test_file"
        return 1
    fi
}

run_test_suite() {
    local suite_name=$1
    shift
    local tests=("$@")

    echo ""
    echo -e "${BLUE}============================================${NC}"
    echo -e "${BLUE}Test Suite: $suite_name${NC}"
    echo -e "${BLUE}============================================${NC}"

    local suite_passed=0
    local suite_failed=0

    for test in "${tests[@]}"; do
        if run_test_file "$test"; then
            suite_passed=$((suite_passed + 1))
        else
            suite_failed=$((suite_failed + 1))
        fi
    done

    TOTAL_PASSED=$((TOTAL_PASSED + suite_passed))
    TOTAL_FAILED=$((TOTAL_FAILED + suite_failed))
    TOTAL_TESTS=$((TOTAL_TESTS + suite_passed + suite_failed))

    echo ""
    echo -e "${BLUE}$suite_name Results: $suite_passed passed, $suite_failed failed${NC}"
}

print_final_summary() {
    echo ""
    echo -e "${BLUE}============================================${NC}"
    echo -e "${BLUE}FINAL SUMMARY${NC}"
    echo -e "${BLUE}============================================${NC}"
    echo "Total Tests: $TOTAL_TESTS"
    echo -e "Passed: ${GREEN}$TOTAL_PASSED${NC}"
    echo -e "Failed: ${RED}$TOTAL_FAILED${NC}"

    if [[ $TOTAL_FAILED -eq 0 ]]; then
        echo ""
        echo -e "${GREEN}🎉 All tests passed!${NC}"
        exit 0
    else
        echo ""
        echo -e "${RED}💥 Some tests failed!${NC}"
        exit 1
    fi
}

# Main execution
main() {
    local test_category=${1:-all}

    echo -e "${BLUE}============================================${NC}"
    echo -e "${BLUE}RustyClawd Validation Test Suite${NC}"
    echo -e "${BLUE}============================================${NC}"
    echo "Test Category: $test_category"
    echo "Testing Pyramid: 60% Unit / 30% Integration / 10% E2E"

    case "$test_category" in
        unit)
            run_test_suite "Unit Tests (60%)" "${UNIT_TESTS[@]}"
            ;;
        integration)
            run_test_suite "Integration Tests (30%)" "${INTEGRATION_TESTS[@]}"
            ;;
        e2e)
            run_test_suite "E2E Tests (10%)" "${E2E_TESTS[@]}"
            ;;
        all)
            run_test_suite "Unit Tests (60%)" "${UNIT_TESTS[@]}"
            run_test_suite "Integration Tests (30%)" "${INTEGRATION_TESTS[@]}"
            run_test_suite "E2E Tests (10%)" "${E2E_TESTS[@]}"
            ;;
        *)
            echo -e "${RED}Unknown test category: $test_category${NC}"
            echo "Usage: $0 [unit|integration|e2e|all]"
            exit 1
            ;;
    esac

    print_final_summary
}

main "$@"
