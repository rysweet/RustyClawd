#!/bin/bash
# synthesize_report.sh - Generate final validation report from phase artifacts
#
# Philosophy: Ruthless simplicity
# - Read all markdown artifacts
# - Generate structured markdown report
# - Handle missing artifacts gracefully
# - Create proper table of contents
#
# Exit codes:
#   0 - Success (report generated)
#   1 - Failure (artifacts missing or generation failed)

set -euo pipefail

# Configuration
ARTIFACTS_DIR="${ARTIFACTS_DIR:-./reports}"
OUTPUT_FILE="${OUTPUT_FILE:-$ARTIFACTS_DIR/validation_report.md}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check if artifacts directory exists
if [ ! -d "$ARTIFACTS_DIR" ]; then
    echo -e "${RED}Error: Artifacts directory not found: $ARTIFACTS_DIR${NC}"
    echo "artifacts"
    exit 1
fi

# Backup existing report
if [ -f "$OUTPUT_FILE" ]; then
    backup_file="${OUTPUT_FILE}.bak"
    cp "$OUTPUT_FILE" "$backup_file"
    echo "Created backup: $backup_file"
fi

# Helper function to check if artifact exists
artifact_exists() {
    local artifact=$1
    [ -f "$ARTIFACTS_DIR/$artifact" ]
}

# Helper function to read artifact content
read_artifact() {
    local artifact=$1
    if artifact_exists "$artifact"; then
        cat "$ARTIFACTS_DIR/$artifact"
    else
        echo "**Note**: $artifact not available (phase not run)"
    fi
}

# Helper function to extract section content
extract_section() {
    local artifact=$1
    if artifact_exists "$artifact"; then
        if [ -s "$ARTIFACTS_DIR/$artifact" ]; then
            cat "$ARTIFACTS_DIR/$artifact"
        else
            echo "**Note**: $artifact is empty"
        fi
    else
        echo "**Note**: $artifact missing or not run"
    fi
}

# Generate executive summary
generate_executive_summary() {
    echo "## Executive Summary"
    echo ""
    echo "This validation report synthesizes findings from all validation phases:"
    echo ""

    local phases_run=0
    for phase_artifact in phase1_dependency_analysis.md phase2_synthesis.md phase3_test_plan.md phase4_test_results.md; do
        if artifact_exists "$phase_artifact"; then
            phases_run=$((phases_run + 1))
        fi
    done

    echo "- **Phases Completed**: $phases_run/4"

    if artifact_exists "phase4_test_results.md"; then
        echo "- **Test Status**: Tests executed (see Phase 4)"
    fi

    if artifact_exists "bootstrap_status.md"; then
        echo "- **Build Status**: Bootstrap completed"
    fi

    echo ""
}

# Generate table of contents
generate_toc() {
    echo "## Table of Contents"
    echo ""
    echo "1. [Executive Summary](#executive-summary)"

    if artifact_exists "bootstrap_status.md"; then
        echo "2. [Bootstrap Results](#bootstrap-results)"
    fi

    echo "3. [Phase 1: Investigation](#phase-1-investigation)"
    echo "4. [Phase 2: Synthesis](#phase-2-synthesis)"
    echo "5. [Phase 3: Test Plan](#phase-3-test-plan)"
    echo "6. [Phase 4: Test Results](#phase-4-test-results)"
    echo ""
}

# Generate report
generate_report() {
    cat > "$OUTPUT_FILE" <<EOF
# RustyClawd Validation Report

**Generated**: $(date '+%Y-%m-%d %H:%M:%S')
**Artifacts Directory**: $ARTIFACTS_DIR

---

EOF

    # Table of Contents
    generate_toc >> "$OUTPUT_FILE"

    # Executive Summary
    generate_executive_summary >> "$OUTPUT_FILE"

    # Bootstrap Results
    if artifact_exists "bootstrap_status.md"; then
        echo "---" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        echo "## Bootstrap Results" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        extract_section "bootstrap_status.md" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    fi

    # Phase 1: Investigation
    echo "---" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    echo "## Phase 1: Investigation" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"

    if artifact_exists "phase1_dependency_analysis.md"; then
        echo "### Dependency Analysis" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        extract_section "phase1_dependency_analysis.md" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    fi

    if artifact_exists "phase1_config_analysis.md"; then
        echo "### Config Analysis" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        extract_section "phase1_config_analysis.md" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    fi

    if artifact_exists "phase1_security_analysis.md"; then
        echo "### Security Analysis" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        extract_section "phase1_security_analysis.md" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    fi

    if artifact_exists "phase1_integration_analysis.md"; then
        echo "### Integration Analysis" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        extract_section "phase1_integration_analysis.md" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    fi

    if artifact_exists "phase1_resource_analysis.md"; then
        echo "### Resource Analysis" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        extract_section "phase1_resource_analysis.md" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    fi

    # Phase 2: Synthesis
    echo "---" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    echo "## Phase 2: Synthesis" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    extract_section "phase2_synthesis.md" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"

    # Phase 3: Test Plan
    echo "---" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    echo "## Phase 3: Test Plan" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    extract_section "phase3_test_plan.md" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"

    # Phase 4: Test Results
    echo "---" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    echo "## Phase 4: Test Results" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    extract_section "phase4_test_results.md" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"

    # Summary Statistics
    echo "---" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    echo "## Summary" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
    echo "Validation report complete. Review findings above for detailed analysis." >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
}

# Main execution
echo "Synthesizing validation report..."
echo "Reading artifacts from: $ARTIFACTS_DIR"

generate_report

if [ -f "$OUTPUT_FILE" ]; then
    echo -e "${GREEN}✓ Report generated: $OUTPUT_FILE${NC}"
    exit 0
else
    echo -e "${RED}✗ Failed to generate report${NC}"
    exit 1
fi
