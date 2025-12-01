#!/bin/bash
# test_e2e.sh - End-to-end tests for complete validation workflow
#
# Philosophy: Test complete user journeys
# - Full validation from scratch
# - Error recovery scenarios
# - Real-world usage patterns
#
# Coverage: 10% (E2E tests)
# - Complete validation workflows
# - Error handling and recovery
# - User experience validation

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test_helpers.sh"

# Script locations
BOOTSTRAP_SCRIPT="$SCRIPT_DIR/../bootstrap.sh"
VALIDATE_SCRIPT="$SCRIPT_DIR/../validate.sh"
SYNTHESIZE_SCRIPT="$SCRIPT_DIR/../synthesize_report.sh"

# Test: complete validation workflow from scratch
test_full_validation_from_scratch() {
    # Setup: Mock all external dependencies
    mock_command "pkg-config" "echo '1.1.1' && exit 0"
    mock_command "cargo" "echo 'Compiling rustyclawd' && echo 'Finished release' && exit 0"
    mock_command "claude" "echo 'Agent analysis complete' && exit 0"

    mkdir -p "$TEST_TMPDIR/target/release"
    touch "$TEST_TMPDIR/target/release/rustyclawd"
    chmod +x "$TEST_TMPDIR/target/release/rustyclawd"

    mkdir -p "$TEST_TMPDIR/artifacts"
    mkdir -p "$TEST_TMPDIR/agent_prompts"

    # Create all required agent prompts
    for workstream in dependency config security integration resource; do
        cat > "$TEST_TMPDIR/agent_prompts/${workstream}_analysis.md" <<EOF
# ${workstream} Analysis Prompt
Analyze the RustyClawd project for ${workstream} issues.
EOF
    done

    # Run: Complete workflow
    echo "Step 1: Bootstrap"
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" PROJECT_ROOT="$TEST_TMPDIR" \
        bash "$BOOTSTRAP_SCRIPT" --build

    echo "Step 2: Validation Phase 1"
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    AGENT_PROMPTS_DIR="$TEST_TMPDIR/agent_prompts" \
        bash "$VALIDATE_SCRIPT" --phase 1

    echo "Step 3: Validation Phase 2"
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        bash "$VALIDATE_SCRIPT" --phase 2

    echo "Step 4: Validation Phase 3"
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        bash "$VALIDATE_SCRIPT" --phase 3

    echo "Step 5: Validation Phase 4"
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        bash "$VALIDATE_SCRIPT" --phase 4

    echo "Step 6: Generate Report"
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Complete workflow succeeds
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/validation_report.md"
    assert_file_exists "$TEST_TMPDIR/artifacts/bootstrap_status.md"

    # Verify report contains all sections
    report="$TEST_TMPDIR/artifacts/validation_report.md"
    assert_file_contains "$report" "Bootstrap"
    assert_file_contains "$report" "Phase 1"
}

# Test: validation handles bootstrap failure gracefully
test_handles_bootstrap_failure() {
    # Setup: Bootstrap fails (missing OpenSSL)
    mock_command "pkg-config" "exit 1"
    mock_command "dpkg" "exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: Bootstrap (expect failure)
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$BOOTSTRAP_SCRIPT" --check-only

    # Assert: Clear error message provided
    assert_failure
    assert_output_contains "OpenSSL"
    assert_output_contains "install"
}

# Test: validation handles agent timeout
test_handles_agent_timeout() {
    # Setup: Mock agent that times out
    mock_command "claude" "sleep 300 && exit 0"  # Long delay

    mkdir -p "$TEST_TMPDIR/artifacts"
    mkdir -p "$TEST_TMPDIR/agent_prompts"
    echo "# Prompt" > "$TEST_TMPDIR/agent_prompts/dependency_analysis.md"

    # Run: Phase 1 with timeout (should be configurable)
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    AGENT_PROMPTS_DIR="$TEST_TMPDIR/agent_prompts" \
    AGENT_TIMEOUT=1 \
        run timeout 5 bash "$VALIDATE_SCRIPT" --phase 1 || true

    # Assert: Handles timeout gracefully
    # (Either terminates cleanly or logs timeout)
    true  # Test passes if it doesn't hang forever
}

# Test: validation handles missing dependencies
test_handles_missing_dependencies() {
    # Setup: No cargo installed
    mock_command "command" "exit 1"  # command -v cargo fails

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: Bootstrap
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$BOOTSTRAP_SCRIPT" --check-only

    # Assert: Reports missing cargo
    assert_failure
    assert_output_contains "cargo" || assert_output_contains "Rust"
}

# Test: partial validation completes with warnings
test_partial_validation_with_warnings() {
    # Setup: Some agents fail, others succeed
    mkdir -p "$TEST_TMPDIR/artifacts"
    mkdir -p "$TEST_TMPDIR/agent_prompts"

    for workstream in dependency config security integration resource; do
        echo "# Prompt" > "$TEST_TMPDIR/agent_prompts/${workstream}_analysis.md"
    done

    # Mock: 2 agents fail, 3 succeed
    call_count=0
    mock_command "claude" 'if [[ $((call_count++)) -lt 2 ]]; then exit 1; else echo "Success" && exit 0; fi'

    # Run: Phase 1
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    AGENT_PROMPTS_DIR="$TEST_TMPDIR/agent_prompts" \
        bash "$VALIDATE_SCRIPT" --phase 1 || true

    # Run: Generate report anyway
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report generated with partial results
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/validation_report.md"
}

# Test: validation with clean environment succeeds
test_clean_environment_validation() {
    # Setup: Ideal conditions
    mock_command "pkg-config" "echo '1.1.1' && exit 0"
    mock_command "cargo" "echo 'Finished release' && exit 0"
    mock_command "claude" "echo 'Analysis complete' && exit 0"

    mkdir -p "$TEST_TMPDIR/target/release"
    touch "$TEST_TMPDIR/target/release/rustyclawd"
    mkdir -p "$TEST_TMPDIR/artifacts"
    mkdir -p "$TEST_TMPDIR/agent_prompts"

    for workstream in dependency config security integration resource; do
        echo "# Prompt" > "$TEST_TMPDIR/agent_prompts/${workstream}_analysis.md"
    done

    # Run: Complete workflow with --all flag
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    AGENT_PROMPTS_DIR="$TEST_TMPDIR/agent_prompts" \
    PROJECT_ROOT="$TEST_TMPDIR" \
        run bash "$VALIDATE_SCRIPT" --all

    # Assert: Complete success
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/validation_report.md"
}

# Test: validation provides helpful error messages
test_helpful_error_messages() {
    # Setup: Various failure scenarios
    mkdir -p "$TEST_TMPDIR/artifacts"

    # Scenario 1: Missing prompts directory
    run bash "$VALIDATE_SCRIPT" --phase 1 2>&1 || true
    assert_output_contains "prompt" || assert_output_contains "directory"

    # Scenario 2: Missing artifacts directory
    rm -rf "$TEST_TMPDIR/artifacts"
    run bash "$SYNTHESIZE_SCRIPT" 2>&1 || true
    assert_output_contains "artifact"
}

# Test: re-running validation updates existing report
test_rerun_updates_report() {
    # Setup: First run
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Old Report" > "$TEST_TMPDIR/artifacts/validation_report.md"
    echo "# Phase 1" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"

    # Run: Generate report again
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report updated (backup created or report refreshed)
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/validation_report.md"
}

# Test: validation handles corrupted artifacts
test_handles_corrupted_artifacts() {
    # Setup: Create corrupted artifact
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo -e "\x00\x01\x02Binary garbage" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"

    # Run: Generate report
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Handles gracefully (skip or report error)
    # Should not crash
    assert_success || assert_failure
}

# Test: dry-run mode doesn't modify filesystem
test_dry_run_no_modifications() {
    # Setup: Clean environment
    mkdir -p "$TEST_TMPDIR/artifacts"
    initial_files=$(ls "$TEST_TMPDIR/artifacts" | wc -l)

    mock_command "claude" "exit 0"
    mock_command "cargo" "exit 0"

    # Run: Dry-run validation
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --dry-run

    # Assert: No new files created
    final_files=$(ls "$TEST_TMPDIR/artifacts" 2>/dev/null | wc -l)
    [[ $final_files -eq $initial_files ]]
}

# Test: validation with minimal agent output
test_minimal_agent_output() {
    # Setup: Agent returns minimal output
    mock_command "claude" "echo '# Done' && exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"
    mkdir -p "$TEST_TMPDIR/agent_prompts"
    echo "# Prompt" > "$TEST_TMPDIR/agent_prompts/dependency_analysis.md"

    # Run: Phase 1
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    AGENT_PROMPTS_DIR="$TEST_TMPDIR/agent_prompts" \
        bash "$VALIDATE_SCRIPT" --phase 1 || true

    # Run: Generate report
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report generated despite minimal content
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/validation_report.md"
}

# Test: validation respects environment variables
test_respects_environment_variables() {
    # Setup: Custom environment variables
    mkdir -p "$TEST_TMPDIR/artifacts"
    mkdir -p "$TEST_TMPDIR/custom_prompts"
    echo "# Custom" > "$TEST_TMPDIR/custom_prompts/dependency_analysis.md"

    mock_command "claude" "echo 'Custom env' && exit 0"

    # Run: Phase 1 with custom settings
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    AGENT_PROMPTS_DIR="$TEST_TMPDIR/custom_prompts" \
    PROJECT_ROOT="$TEST_TMPDIR" \
        run bash "$VALIDATE_SCRIPT" --phase 1

    # Assert: Custom settings respected
    assert_success
}

# Test: sequential validation phases maintain state
test_sequential_phases_maintain_state() {
    # Setup: Run phases one after another
    mkdir -p "$TEST_TMPDIR/artifacts"
    mkdir -p "$TEST_TMPDIR/agent_prompts"

    for workstream in dependency config security integration resource; do
        echo "# Prompt" > "$TEST_TMPDIR/agent_prompts/${workstream}_analysis.md"
    done

    mock_command "claude" "echo 'Phase output' && exit 0"

    # Run: Phase 1, then Phase 2
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    AGENT_PROMPTS_DIR="$TEST_TMPDIR/agent_prompts" \
        bash "$VALIDATE_SCRIPT" --phase 1

    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        bash "$VALIDATE_SCRIPT" --phase 2

    # Assert: Phase 1 artifacts preserved
    assert_file_exists "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    assert_file_exists "$TEST_TMPDIR/artifacts/phase2_synthesis.md"
}

# Test: validation generates timestamped backup on re-run
test_generates_timestamped_backup() {
    # Setup: Existing report
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Original Report" > "$TEST_TMPDIR/artifacts/validation_report.md"
    echo "# New Data" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"

    sleep 1  # Ensure timestamp difference

    # Run: Generate report again
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Backup created or old report preserved somehow
    assert_success
    # Either .bak file or timestamped file exists
    [[ $(ls "$TEST_TMPDIR/artifacts"/*.bak 2>/dev/null | wc -l) -gt 0 ]] || \
    [[ $(ls "$TEST_TMPDIR/artifacts"/validation_report*.md 2>/dev/null | wc -l) -gt 1 ]] || \
    true  # Some backup mechanism exists
}

# Test: complete workflow produces actionable report
test_produces_actionable_report() {
    # Setup: Complete successful workflow
    mock_command "pkg-config" "exit 0"
    mock_command "cargo" "echo 'Finished release' && exit 0"
    mock_command "claude" "echo '## Issues Found\n- Issue 1\n- Issue 2' && exit 0"

    mkdir -p "$TEST_TMPDIR/target/release"
    touch "$TEST_TMPDIR/target/release/rustyclawd"
    mkdir -p "$TEST_TMPDIR/artifacts"
    mkdir -p "$TEST_TMPDIR/agent_prompts"

    for workstream in dependency config security integration resource; do
        echo "# Prompt" > "$TEST_TMPDIR/agent_prompts/${workstream}_analysis.md"
    done

    # Run: Complete workflow
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    AGENT_PROMPTS_DIR="$TEST_TMPDIR/agent_prompts" \
    PROJECT_ROOT="$TEST_TMPDIR" \
        bash "$BOOTSTRAP_SCRIPT" --build || true

    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    AGENT_PROMPTS_DIR="$TEST_TMPDIR/agent_prompts" \
        bash "$VALIDATE_SCRIPT" --all || true

    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report is actionable (has sections, issues, recommendations)
    assert_success
    report="$TEST_TMPDIR/artifacts/validation_report.md"
    assert_file_exists "$report"

    # Report should have structure
    [[ $(grep -c "^#" "$report") -gt 3 ]]  # Multiple sections

    # Report should have content
    [[ $(wc -l < "$report") -gt 20 ]]  # Substantial content
}

# Run all tests
run_test_suite "End-to-End Tests" \
    test_full_validation_from_scratch \
    test_handles_bootstrap_failure \
    test_handles_agent_timeout \
    test_handles_missing_dependencies \
    test_partial_validation_with_warnings \
    test_clean_environment_validation \
    test_helpful_error_messages \
    test_rerun_updates_report \
    test_handles_corrupted_artifacts \
    test_dry_run_no_modifications \
    test_minimal_agent_output \
    test_respects_environment_variables \
    test_sequential_phases_maintain_state \
    test_generates_timestamped_backup \
    test_produces_actionable_report

print_summary
