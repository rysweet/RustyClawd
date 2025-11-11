#!/bin/bash
# Master Verification Suite: RustyClawd Drop-In Replacement
# Runs all 5 verification methods sequentially

set -e

PROJECT_DIR="/Users/ryan/src/declawed/claude-code-rs"
cd "$PROJECT_DIR"

echo ""
echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  RUSTYCLAWD DROP-IN REPLACEMENT VERIFICATION SUITE            ║"
echo "║  5 Methods to Validate Complete Compatibility                  ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

TOTAL_PASS=0
TOTAL_FAIL=0
RESULTS=()

run_verification() {
    local method_num=$1
    local method_name=$2
    local script=$3

    echo ""
    echo "════════════════════════════════════════════════════════════════"
    echo "[$method_num/5] $method_name"
    echo "════════════════════════════════════════════════════════════════"
    echo ""

    if bash "$script" 2>&1 | tee "/tmp/m${method_num}_results.log"; then
        echo ""
        echo "✓ METHOD $method_num PASSED"
        RESULTS+=("✓ M$method_num: $method_name")
        return 0
    else
        echo ""
        echo "✗ METHOD $method_num FAILED"
        RESULTS+=("✗ M$method_num: $method_name")
        return 1
    fi
}

# Run all 5 methods
set +e  # Continue even if tests fail

run_verification 1 "Tool Signature Validation" "$PROJECT_DIR/tests/method1_tool_signatures.sh"
M1_RESULT=$?

run_verification 2 "Behavioral Equivalence Testing" "$PROJECT_DIR/tests/method2_behavioral_equivalence.sh"
M2_RESULT=$?

run_verification 3 "CLI Interface Parity" "$PROJECT_DIR/tests/method3_cli_parity.sh"
M3_RESULT=$?

run_verification 4 "Error Handling Alignment" "$PROJECT_DIR/tests/method4_error_alignment.sh"
M4_RESULT=$?

run_verification 5 "Integration Workflow Testing" "$PROJECT_DIR/tests/method5_integration_workflows.sh"
M5_RESULT=$?

# Summary
echo ""
echo "════════════════════════════════════════════════════════════════"
echo "FINAL VERIFICATION RESULTS"
echo "════════════════════════════════════════════════════════════════"
echo ""

for result in "${RESULTS[@]}"; do
    echo "$result"
done

echo ""
echo "Detailed logs available:"
echo "  - /tmp/m1_results.log (Tool Signatures)"
echo "  - /tmp/m2_results.log (Behavioral Equivalence)"
echo "  - /tmp/m3_results.log (CLI Parity)"
echo "  - /tmp/m4_results.log (Error Alignment)"
echo "  - /tmp/m5_results.log (Integration Workflows)"
echo ""

# Determine overall status
if [ $M1_RESULT -eq 0 ] && [ $M2_RESULT -eq 0 ] && [ $M3_RESULT -eq 0 ] && [ $M4_RESULT -eq 0 ] && [ $M5_RESULT -eq 0 ]; then
    echo "════════════════════════════════════════════════════════════════"
    echo "✓ ALL VERIFICATIONS PASSED"
    echo "════════════════════════════════════════════════════════════════"
    echo ""
    echo "RustyClawd is verified as a drop-in replacement for Claude Code."
    echo ""
    exit 0
else
    echo "════════════════════════════════════════════════════════════════"
    echo "✗ SOME VERIFICATIONS FAILED"
    echo "════════════════════════════════════════════════════════════════"
    echo ""
    echo "Review the logs above to identify specific failures."
    echo "Gaps found may require implementation or bugfixes."
    echo ""
    exit 1
fi
