#!/bin/bash
# test_validate_phases.sh - Unit tests for validate.sh phases
#
# Philosophy: Test phase orchestration and dependencies
# - Each phase executes in correct order
# - Phase dependencies are enforced
# - Parallel execution works correctly
# - Agent invocations are proper
#
# Coverage: 60% (Unit tests)
# - Phase 1: 5 parallel workstreams
# - Phase 2: Dependencies on Phase 1
# - Phase 3: Test plan generation
# - Phase 4: Parallel test execution
# - Phase 5: Report synthesis

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test_helpers.sh"

# Assume validate.sh will be in parent directory
VALIDATE_SCRIPT="$SCRIPT_DIR/../validate.sh"

# Test: validate.sh runs Phase 1 with 5 parallel workstreams
test_phase1_runs_5_workstreams() {
    # Mock: claude command for agent invocation
    mock_command "claude" "echo 'Agent output' && exit 0"

    # Run: validate.sh phase 1
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 1

    # Assert: Should invoke 5 agents in parallel
    assert_success
    assert_output_contains "workstream"
}

# Test: validate.sh Phase 1 creates separate artifacts for each workstream
test_phase1_creates_workstream_artifacts() {
    # Mock: claude command
    mock_command "claude" "echo 'Analysis complete' && exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: validate.sh phase 1
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 1

    # Assert: Should create 5 artifact files
    assert_success
    # Workstreams: dependency, config, security, integration, resource
    assert_file_exists "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    assert_file_exists "$TEST_TMPDIR/artifacts/phase1_config_analysis.md"
    assert_file_exists "$TEST_TMPDIR/artifacts/phase1_security_analysis.md"
    assert_file_exists "$TEST_TMPDIR/artifacts/phase1_integration_analysis.md"
    assert_file_exists "$TEST_TMPDIR/artifacts/phase1_resource_analysis.md"
}

# Test: validate.sh Phase 2 waits for Phase 1 completion
test_phase2_depends_on_phase1() {
    # Mock: Phase 1 not complete (missing artifacts)
    mock_command "claude" "exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: validate.sh phase 2 (without phase 1 run)
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 2

    # Assert: Should fail or warn about missing Phase 1
    assert_failure
    assert_output_contains "Phase 1"
}

# Test: validate.sh Phase 2 synthesizes Phase 1 results
test_phase2_synthesizes_results() {
    # Mock: Phase 1 complete
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Dependency Analysis" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    echo "# Config Analysis" > "$TEST_TMPDIR/artifacts/phase1_config_analysis.md"
    echo "# Security Analysis" > "$TEST_TMPDIR/artifacts/phase1_security_analysis.md"
    echo "# Integration Analysis" > "$TEST_TMPDIR/artifacts/phase1_integration_analysis.md"
    echo "# Resource Analysis" > "$TEST_TMPDIR/artifacts/phase1_resource_analysis.md"

    mock_command "claude" "echo 'Synthesis complete' && exit 0"

    # Run: validate.sh phase 2
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 2

    # Assert: Should create synthesis artifact
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/phase2_synthesis.md"
}

# Test: validate.sh Phase 3 generates test plan
test_phase3_generates_test_plan() {
    # Mock: Phase 2 complete
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Synthesis" > "$TEST_TMPDIR/artifacts/phase2_synthesis.md"

    mock_command "claude" "echo '## Test Plan\n- Test 1\n- Test 2' && exit 0"

    # Run: validate.sh phase 3
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 3

    # Assert: Should create test plan
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/phase3_test_plan.md"
}

# Test: validate.sh Phase 3 depends on Phase 2
test_phase3_depends_on_phase2() {
    # Mock: Phase 2 not complete
    mock_command "claude" "exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: validate.sh phase 3 (without phase 2)
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 3

    # Assert: Should fail about missing Phase 2
    assert_failure
    assert_output_contains "Phase 2"
}

# Test: validate.sh Phase 4 executes tests in parallel
test_phase4_executes_tests_parallel() {
    # Mock: Phase 3 complete with test plan
    mkdir -p "$TEST_TMPDIR/artifacts"
    cat > "$TEST_TMPDIR/artifacts/phase3_test_plan.md" <<EOF
# Test Plan
1. Test dependency installation
2. Test configuration validation
3. Test security checks
EOF

    mock_command "claude" "echo 'Test passed' && exit 0"

    # Run: validate.sh phase 4
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 4

    # Assert: Should run tests in parallel
    assert_success
    assert_output_contains "test"
}

# Test: validate.sh Phase 4 creates test result artifacts
test_phase4_creates_test_results() {
    # Mock: Phase 3 complete
    mkdir -p "$TEST_TMPDIR/artifacts"
    cat > "$TEST_TMPDIR/artifacts/phase3_test_plan.md" <<EOF
# Test Plan
1. Test A
2. Test B
EOF

    mock_command "claude" "echo 'PASS' && exit 0"

    # Run: validate.sh phase 4
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 4

    # Assert: Should create test results
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/phase4_test_results.md"
}

# Test: validate.sh Phase 4 depends on Phase 3
test_phase4_depends_on_phase3() {
    # Mock: Phase 3 not complete
    mock_command "claude" "exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: validate.sh phase 4 (without phase 3)
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 4

    # Assert: Should fail about missing Phase 3
    assert_failure
    assert_output_contains "Phase 3"
}

# Test: validate.sh Phase 5 synthesizes final report
test_phase5_synthesizes_report() {
    # Mock: All previous phases complete
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Phase 1" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    echo "# Phase 2" > "$TEST_TMPDIR/artifacts/phase2_synthesis.md"
    echo "# Phase 3" > "$TEST_TMPDIR/artifacts/phase3_test_plan.md"
    echo "# Phase 4" > "$TEST_TMPDIR/artifacts/phase4_test_results.md"

    mock_command "bash" "echo '# Final Report' > $TEST_TMPDIR/artifacts/validation_report.md && exit 0"

    # Run: validate.sh phase 5
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 5

    # Assert: Should create final report
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/validation_report.md"
}

# Test: validate.sh Phase 5 depends on Phase 4
test_phase5_depends_on_phase4() {
    # Mock: Phase 4 not complete
    mock_command "bash" "exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: validate.sh phase 5 (without phase 4)
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 5

    # Assert: Should fail about missing Phase 4
    assert_failure
    assert_output_contains "Phase 4"
}

# Test: validate.sh runs all phases sequentially
test_all_phases_run_sequentially() {
    # Mock: All commands
    mock_command "claude" "echo 'Agent output' && exit 0"
    mock_command "bash" "echo '# Report' && exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: validate.sh all phases
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --all

    # Assert: All phases execute
    assert_success
    assert_output_contains "Phase 1"
    assert_output_contains "Phase 2"
    assert_output_contains "Phase 3"
    assert_output_contains "Phase 4"
    assert_output_contains "Phase 5"
}

# Test: validate.sh handles agent failures in Phase 1
test_phase1_handles_agent_failures() {
    # Mock: One agent fails
    mock_command "claude" "exit 1"

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: validate.sh phase 1
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 1

    # Assert: Should continue despite failure
    assert_failure
    assert_output_contains "error"
}

# Test: validate.sh creates timestamped artifacts
test_creates_timestamped_artifacts() {
    # Mock: claude command
    mock_command "claude" "echo 'Analysis' && exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: validate.sh phase 1
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 1

    # Assert: Artifacts exist with timestamps or unique names
    assert_success
    # At least one artifact should exist
    [[ $(ls "$TEST_TMPDIR/artifacts" | wc -l) -gt 0 ]]
}

# Test: validate.sh provides progress updates
test_provides_progress_updates() {
    # Mock: claude command
    mock_command "claude" "echo 'Progress' && exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: validate.sh phase 1
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --phase 1

    # Assert: Should show progress messages
    assert_success
    assert_output_contains "Running" || assert_output_contains "Starting"
}

# Test: validate.sh uses correct agent prompts
test_uses_correct_agent_prompts() {
    # Mock: claude command that echoes the prompt file
    mock_command "claude" 'echo "Received: $*" && exit 0'

    mkdir -p "$TEST_TMPDIR/artifacts"
    mkdir -p "$TEST_TMPDIR/agent_prompts"
    echo "# Dependency Prompt" > "$TEST_TMPDIR/agent_prompts/dependency_analysis.md"

    # Run: validate.sh phase 1
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    AGENT_PROMPTS_DIR="$TEST_TMPDIR/agent_prompts" \
        run bash "$VALIDATE_SCRIPT" --phase 1

    # Assert: Should reference agent prompts
    assert_success
}

# Test: validate.sh supports dry-run mode
test_supports_dry_run() {
    # Mock: claude command
    mock_command "claude" "exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"

    # Run: validate.sh dry-run
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$VALIDATE_SCRIPT" --dry-run

    # Assert: Should not create artifacts
    assert_success
    assert_output_contains "dry"
}

# Test: validate.sh validates agent_prompts directory exists
test_validates_agent_prompts_exist() {
    # Mock: claude command
    mock_command "claude" "exit 0"

    mkdir -p "$TEST_TMPDIR/artifacts"
    # Don't create agent_prompts dir

    # Run: validate.sh phase 1
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    AGENT_PROMPTS_DIR="$TEST_TMPDIR/nonexistent" \
        run bash "$VALIDATE_SCRIPT" --phase 1

    # Assert: Should fail or warn about missing prompts
    assert_failure
    assert_output_contains "prompt"
}

# Run all tests
run_test_suite "validate.sh Phase Tests" \
    test_phase1_runs_5_workstreams \
    test_phase1_creates_workstream_artifacts \
    test_phase2_depends_on_phase1 \
    test_phase2_synthesizes_results \
    test_phase3_generates_test_plan \
    test_phase3_depends_on_phase2 \
    test_phase4_executes_tests_parallel \
    test_phase4_creates_test_results \
    test_phase4_depends_on_phase3 \
    test_phase5_synthesizes_report \
    test_phase5_depends_on_phase4 \
    test_all_phases_run_sequentially \
    test_phase1_handles_agent_failures \
    test_creates_timestamped_artifacts \
    test_provides_progress_updates \
    test_uses_correct_agent_prompts \
    test_supports_dry_run \
    test_validates_agent_prompts_exist

print_summary
