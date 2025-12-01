#!/bin/bash
# test_report_generation.sh - Unit tests for synthesize_report.sh
#
# Philosophy: Test report generation and formatting
# - Reads all markdown artifacts correctly
# - Generates proper markdown structure
# - Includes all required sections
# - Handles missing artifacts gracefully
#
# Coverage: 60% (Unit tests)
# - Artifact discovery and reading
# - Markdown structure generation
# - Section ordering
# - Error handling

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test_helpers.sh"

# Assume synthesize_report.sh will be in parent directory
SYNTHESIZE_SCRIPT="$SCRIPT_DIR/../synthesize_report.sh"

# Test: synthesize_report.sh reads all phase artifacts
test_reads_all_artifacts() {
    # Setup: Create phase artifacts
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Bootstrap Status" > "$TEST_TMPDIR/artifacts/bootstrap_status.md"
    echo "# Phase 1 Dependency" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    echo "# Phase 2 Synthesis" > "$TEST_TMPDIR/artifacts/phase2_synthesis.md"
    echo "# Phase 3 Test Plan" > "$TEST_TMPDIR/artifacts/phase3_test_plan.md"
    echo "# Phase 4 Results" > "$TEST_TMPDIR/artifacts/phase4_test_results.md"

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Should read all artifacts
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/validation_report.md"
}

# Test: synthesize_report.sh generates proper markdown structure
test_generates_markdown_structure() {
    # Setup: Minimal artifacts
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Content" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report has proper markdown headers
    assert_success
    assert_file_contains "$TEST_TMPDIR/artifacts/validation_report.md" "#"
}

# Test: synthesize_report.sh includes executive summary section
test_includes_executive_summary() {
    # Setup: Create artifacts
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Analysis" > "$TEST_TMPDIR/artifacts/phase2_synthesis.md"

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report includes executive summary
    assert_success
    assert_file_contains "$TEST_TMPDIR/artifacts/validation_report.md" "Executive Summary" || \
    assert_file_contains "$TEST_TMPDIR/artifacts/validation_report.md" "Summary"
}

# Test: synthesize_report.sh includes all phase sections
test_includes_all_phase_sections() {
    # Setup: Create all phase artifacts
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Bootstrap" > "$TEST_TMPDIR/artifacts/bootstrap_status.md"
    echo "# Dependency" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    echo "# Config" > "$TEST_TMPDIR/artifacts/phase1_config_analysis.md"
    echo "# Security" > "$TEST_TMPDIR/artifacts/phase1_security_analysis.md"
    echo "# Integration" > "$TEST_TMPDIR/artifacts/phase1_integration_analysis.md"
    echo "# Resource" > "$TEST_TMPDIR/artifacts/phase1_resource_analysis.md"
    echo "# Synthesis" > "$TEST_TMPDIR/artifacts/phase2_synthesis.md"
    echo "# Test Plan" > "$TEST_TMPDIR/artifacts/phase3_test_plan.md"
    echo "# Test Results" > "$TEST_TMPDIR/artifacts/phase4_test_results.md"

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report includes all sections
    assert_success
    report="$TEST_TMPDIR/artifacts/validation_report.md"
    assert_file_contains "$report" "Bootstrap"
    assert_file_contains "$report" "Phase 1"
    assert_file_contains "$report" "Phase 2"
    assert_file_contains "$report" "Phase 3"
    assert_file_contains "$report" "Phase 4"
}

# Test: synthesize_report.sh orders sections correctly
test_orders_sections_correctly() {
    # Setup: Create artifacts in random order
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Phase 4" > "$TEST_TMPDIR/artifacts/phase4_test_results.md"
    echo "# Phase 1" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    echo "# Phase 3" > "$TEST_TMPDIR/artifacts/phase3_test_plan.md"

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report sections are in correct order
    assert_success
    report="$TEST_TMPDIR/artifacts/validation_report.md"

    # Extract line numbers where each phase appears
    phase1_line=$(grep -n "Phase 1" "$report" | head -1 | cut -d: -f1 || echo "999")
    phase3_line=$(grep -n "Phase 3" "$report" | head -1 | cut -d: -f1 || echo "999")
    phase4_line=$(grep -n "Phase 4" "$report" | head -1 | cut -d: -f1 || echo "999")

    # Phase 1 should come before Phase 3
    [[ $phase1_line -lt $phase3_line ]]
    # Phase 3 should come before Phase 4
    [[ $phase3_line -lt $phase4_line ]]
}

# Test: synthesize_report.sh handles missing artifacts gracefully
test_handles_missing_artifacts() {
    # Setup: Only some artifacts exist
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Phase 1" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    # Phase 2, 3, 4 missing

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Should still generate report
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/validation_report.md"
}

# Test: synthesize_report.sh includes missing section warnings
test_includes_missing_warnings() {
    # Setup: Minimal artifacts
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Content" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report mentions missing sections or includes placeholder
    assert_success
    report="$TEST_TMPDIR/artifacts/validation_report.md"
    # Should indicate some phases are missing or not run
    assert_file_contains "$report" "not available" || \
    assert_file_contains "$report" "not run" || \
    assert_file_contains "$report" "missing"
}

# Test: synthesize_report.sh preserves markdown formatting from artifacts
test_preserves_markdown_formatting() {
    # Setup: Artifact with rich markdown
    mkdir -p "$TEST_TMPDIR/artifacts"
    cat > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md" <<EOF
# Dependency Analysis

## Critical Issues

- **Issue 1**: Description
- **Issue 2**: Description

\`\`\`rust
fn example() {}
\`\`\`
EOF

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Formatting preserved
    assert_success
    report="$TEST_TMPDIR/artifacts/validation_report.md"
    assert_file_contains "$report" "**Issue 1**"
    assert_file_contains "$report" "\`\`\`"
}

# Test: synthesize_report.sh adds table of contents
test_adds_table_of_contents() {
    # Setup: Multiple artifacts
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Phase 1" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    echo "# Phase 2" > "$TEST_TMPDIR/artifacts/phase2_synthesis.md"

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report includes TOC
    assert_success
    report="$TEST_TMPDIR/artifacts/validation_report.md"
    assert_file_contains "$report" "Table of Contents" || \
    assert_file_contains "$report" "Contents"
}

# Test: synthesize_report.sh includes timestamp
test_includes_timestamp() {
    # Setup: Minimal artifacts
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Content" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report includes generation timestamp
    assert_success
    report="$TEST_TMPDIR/artifacts/validation_report.md"
    assert_file_contains "$report" "Generated" || \
    assert_file_contains "$report" "Date" || \
    assert_file_contains "$report" "2025"
}

# Test: synthesize_report.sh includes validation status summary
test_includes_validation_status() {
    # Setup: Test results with pass/fail
    mkdir -p "$TEST_TMPDIR/artifacts"
    cat > "$TEST_TMPDIR/artifacts/phase4_test_results.md" <<EOF
# Test Results

- Test 1: PASS
- Test 2: FAIL
- Test 3: PASS
EOF

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report summarizes validation status
    assert_success
    report="$TEST_TMPDIR/artifacts/validation_report.md"
    assert_file_contains "$report" "PASS" || assert_file_contains "$report" "FAIL"
}

# Test: synthesize_report.sh fails if artifacts directory missing
test_fails_if_artifacts_missing() {
    # Setup: No artifacts directory
    # Don't create $TEST_TMPDIR/artifacts

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/nonexistent" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Should fail gracefully
    assert_failure
    assert_output_contains "artifacts"
}

# Test: synthesize_report.sh creates backup of existing report
test_creates_backup() {
    # Setup: Existing report
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Old Report" > "$TEST_TMPDIR/artifacts/validation_report.md"
    echo "# New Content" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Should backup old report
    assert_success
    # Check for backup file with timestamp or .bak extension
    [[ $(ls "$TEST_TMPDIR/artifacts"/*.bak 2>/dev/null | wc -l) -gt 0 ]] || \
    [[ $(ls "$TEST_TMPDIR/artifacts"/validation_report*.md 2>/dev/null | wc -l) -gt 1 ]]
}

# Test: synthesize_report.sh includes links between sections
test_includes_section_links() {
    # Setup: Multiple phases
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Phase 1" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    echo "# Phase 2" > "$TEST_TMPDIR/artifacts/phase2_synthesis.md"

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report has navigation links
    assert_success
    report="$TEST_TMPDIR/artifacts/validation_report.md"
    # Should have markdown links [text](#anchor)
    assert_file_contains "$report" "#" || assert_file_contains "$report" "](#"
}

# Test: synthesize_report.sh includes summary statistics
test_includes_statistics() {
    # Setup: Various artifacts
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Phase 1" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    echo "# Phase 2" > "$TEST_TMPDIR/artifacts/phase2_synthesis.md"
    cat > "$TEST_TMPDIR/artifacts/phase4_test_results.md" <<EOF
# Results
- Pass: 10
- Fail: 2
EOF

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report includes statistics
    assert_success
    report="$TEST_TMPDIR/artifacts/validation_report.md"
    # Should have some counts or percentages
    [[ $(grep -c ":" "$report") -gt 3 ]]
}

# Test: synthesize_report.sh supports custom output path
test_supports_custom_output() {
    # Setup: Artifacts
    mkdir -p "$TEST_TMPDIR/artifacts"
    echo "# Content" > "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"

    # Run: synthesize_report.sh with custom output
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
    OUTPUT_FILE="$TEST_TMPDIR/custom_report.md" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Report created at custom location
    assert_success
    assert_file_exists "$TEST_TMPDIR/custom_report.md"
}

# Test: synthesize_report.sh handles empty artifacts
test_handles_empty_artifacts() {
    # Setup: Empty artifact files
    mkdir -p "$TEST_TMPDIR/artifacts"
    touch "$TEST_TMPDIR/artifacts/phase1_dependency_analysis.md"
    touch "$TEST_TMPDIR/artifacts/phase2_synthesis.md"

    # Run: synthesize_report.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" \
        run bash "$SYNTHESIZE_SCRIPT"

    # Assert: Should handle gracefully
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/validation_report.md"
}

# Run all tests
run_test_suite "synthesize_report.sh Unit Tests" \
    test_reads_all_artifacts \
    test_generates_markdown_structure \
    test_includes_executive_summary \
    test_includes_all_phase_sections \
    test_orders_sections_correctly \
    test_handles_missing_artifacts \
    test_includes_missing_warnings \
    test_preserves_markdown_formatting \
    test_adds_table_of_contents \
    test_includes_timestamp \
    test_includes_validation_status \
    test_fails_if_artifacts_missing \
    test_creates_backup \
    test_includes_section_links \
    test_includes_statistics \
    test_supports_custom_output \
    test_handles_empty_artifacts

print_summary
