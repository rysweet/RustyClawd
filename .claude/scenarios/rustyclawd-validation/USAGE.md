# RustyClawd Validation System - Usage Guide

Ahoy matey! This be the comprehensive user guide fer the RustyClawd validation system. It covers everythin' from basic usage to advanced workflows.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Basic Usage](#basic-usage)
- [Phase-by-Phase Usage](#phase-by-phase-usage)
- [Command-Line Options](#command-line-options)
- [Common Use Cases](#common-use-cases)
- [Output Interpretation](#output-interpretation)
- [Advanced Workflows](#advanced-workflows)
- [Troubleshooting Guide](#troubleshooting-guide)
- [Performance Tuning](#performance-tuning)
- [Best Practices](#best-practices)

## Prerequisites

Before settin' sail with the validation system, ensure ye have:

### Required Tools

```bash
# Check bash version (need 4.0+)
bash --version
# Output: GNU bash, version 4.0 or higher

# Check Rust toolchain
cargo --version
# Output: cargo 1.70.0 or higher

rustc --version
# Output: rustc 1.70.0 or higher

# Check Python
python3 --version
# Output: Python 3.8.0 or higher

# Check pytest
pytest --version
# Output: pytest 7.0.0 or higher
```

### Optional Tools

```bash
# Git (for repository access)
git --version

# Claude Code CLI (for agent invocation)
claude-code --version
```

### System Requirements

- **CPU**: 2+ cores recommended (4+ for optimal parallelism)
- **RAM**: 4GB minimum, 8GB recommended
- **Disk**: 500MB free space for reports
- **OS**: macOS, Linux, or WSL2 on Windows

## Basic Usage

### First-Time Validation

The simplest way to run a complete validation:

```bash
# Navigate to validation directory
cd .claude/scenarios/rustyclawd-validation

# Run complete validation
./validate.sh
```

**What happens**:
1. Bootstrap runs automatically (installs OpenSSL deps)
2. Five agents analyze the codebase in parallel (3-5 min)
3. Gap analysis identifies discrepancies (1-2 min)
4. Test plan generated (1-2 min)
5. Tests executed in parallel (5-10 min)
6. Final report synthesized (1 min)

**Output location**: `./reports/[TIMESTAMP]/validation_report.md`

### Quick Status Check

Check if RustyClawd builds successfully:

```bash
./bootstrap.sh
```

This only runs Phase 0 (build environment setup).

### Re-validation After Fixes

After fixin' issues from a previous validation:

```bash
# Skip bootstrap, run from investigation onwards
./validate.sh --skip-bootstrap
```

## Phase-by-Phase Usage

### Phase 0: Bootstrap (Build Fixes)

**Purpose**: Fix build environment before validation

**Command**:
```bash
./bootstrap.sh
```

**Duration**: 30-60 seconds (one-time setup)

**What it does**:
- Detects your operating system
- Installs OpenSSL development libraries
- Verifies cargo can build RustyClawd

**Success output**:
```
[Bootstrap] Detecting OS: macos
[Bootstrap] Installing OpenSSL dependencies...
[Bootstrap] Running: brew install openssl@3
[Bootstrap] Testing cargo build...
[Bootstrap] ✓ Build successful
```

**Failure output**:
```
[Bootstrap] Detecting OS: macos
[Bootstrap] Installing OpenSSL dependencies...
[Bootstrap] ✗ Installation failed
[Bootstrap] Error: Homebrew not found
[Bootstrap] Please install Homebrew: https://brew.sh
```

**Manual invocation**:
```bash
# From validate.sh
./validate.sh --phase 0
```

### Phase 1: Investigation

**Purpose**: Parallel analysis by specialized agents

**Command**:
```bash
./validate.sh --phase 1
```

**Duration**: 3-5 minutes (5 parallel agents)

**What it does**:
- Launches 5 agents in parallel:
  - Tester: Analyzes test coverage
  - Reviewer: Checks code quality
  - Architect: Reviews architecture
  - Cargo test: Runs Rust unit tests
  - Pytest: Runs Python integration tests
- Collects findings into unified report

**Progress output**:
```
[Phase 1] Investigation: Launching 5 parallel agents...
[Phase 1]   - Tester agent started (PID 12345)
[Phase 1]   - Reviewer agent started (PID 12346)
[Phase 1]   - Architect agent started (PID 12347)
[Phase 1]   - Cargo tests started (PID 12348)
[Phase 1]   - Pytest started (PID 12349)
[Phase 1] Waiting for agents to complete...
[Phase 1] Tester complete (45s)
[Phase 1] Reviewer complete (78s)
[Phase 1] Architect complete (134s)
[Phase 1] Cargo tests complete (89s)
[Phase 1] Pytest complete (102s)
[Phase 1] Merging findings...
[Phase 1] ✓ Complete: investigation_report.md
```

**Output**: `./reports/[TIMESTAMP]/investigation_report.md`

**Sections in report**:
- Tester Analysis (test coverage, quality)
- Reviewer Analysis (code quality, philosophy)
- Architect Analysis (design patterns, structure)
- Cargo Test Results (unit test outcomes)
- Pytest Results (integration test outcomes)

### Phase 2: Gap Analysis

**Purpose**: Compare docs vs implementation

**Command**:
```bash
./validate.sh --phase 2
```

**Duration**: 1-2 minutes

**Prerequisites**: Phase 1 must have completed (needs investigation report)

**What it does**:
- Loads Phase 1 investigation report
- Compares official Claude Code docs to RustyClawd implementation
- Identifies missing features, extra features, discrepancies
- Prioritizes gaps by severity

**Progress output**:
```
[Phase 2] Gap Analysis: Starting...
[Phase 2] Loading investigation report...
[Phase 2] Invoking analyzer agent...
[Phase 2] Comparing official docs vs implementation...
[Phase 2] Identifying discrepancies...
[Phase 2] Prioritizing gaps...
[Phase 2] ✓ Complete: gap_analysis.md
```

**Output**: `./reports/[TIMESTAMP]/gap_analysis.md`

**Sections in report**:
- Documentation Coverage Summary
- Critical Gaps (high priority)
- Medium Priority Gaps
- Low Priority Gaps
- Documentation Gaps (need doc updates)
- Prioritized Recommendations

### Phase 3: Test Plan Generation

**Purpose**: Create comprehensive test strategy

**Command**:
```bash
./validate.sh --phase 3
```

**Duration**: 1-2 minutes

**Prerequisites**: Phase 2 must have completed (needs gap analysis)

**What it does**:
- Loads Phase 2 gap analysis
- Invokes tester agent to generate test plan
- Creates test cases targeting identified gaps
- Prioritizes test scenarios

**Progress output**:
```
[Phase 3] Test Plan: Starting...
[Phase 3] Loading gap analysis...
[Phase 3] Invoking tester agent...
[Phase 3] Generating test scenarios...
[Phase 3] ✓ Complete: test_plan.md
```

**Output**: `./reports/[TIMESTAMP]/test_plan.md`

**Sections in report**:
- Unit Test Scenarios (60% of effort)
- Integration Test Scenarios (30% of effort)
- End-to-End Test Scenarios (10% of effort)
- Test Priority Rankings
- Expected Outcomes

### Phase 4: Test Execution

**Purpose**: Execute test suites in parallel

**Command**:
```bash
./validate.sh --phase 4
```

**Duration**: 5-10 minutes (3 parallel test suites)

**Prerequisites**: Phase 3 recommended (provides test plan context)

**What it does**:
- Launches 3 test suites in parallel:
  - Cargo test: Rust unit tests
  - Pytest: Python integration tests
  - Integration tests: End-to-end scenarios
- Collects and formats results

**Progress output**:
```
[Phase 4] Test Execution: Starting...
[Phase 4]   - Cargo tests started (PID 23456)
[Phase 4]   - Pytest started (PID 23457)
[Phase 4]   - Integration tests started (PID 23458)
[Phase 4] Waiting for tests to complete...
[Phase 4] Cargo tests complete (234 passed, 5 failed) - 289s
[Phase 4] Pytest complete (89 passed, 1 failed) - 156s
[Phase 4] Integration tests complete (12/15 passed) - 423s
[Phase 4] Formatting results...
[Phase 4] ✓ Complete: test_execution.md
```

**Output**: `./reports/[TIMESTAMP]/test_execution.md`

**Sections in report**:
- Cargo Test Results (Rust)
- Pytest Results (Python)
- Integration Test Results
- Test Summary Statistics
- Failed Test Details

### Phase 5: Report Synthesis

**Purpose**: Generate executive summary

**Command**:
```bash
./validate.sh --phase 5
```

**Duration**: 1 minute

**Prerequisites**: Phases 1-4 must have completed

**What it does**:
- Collects all phase reports (1-4)
- Calculates validation score (0-100)
- Invokes architect agent for synthesis
- Generates executive summary with recommendations

**Progress output**:
```
[Phase 5] Report Synthesis: Starting...
[Phase 5] Collecting phase reports...
[Phase 5] Calculating validation score...
[Phase 5] Score: 82/100 (Doc: 85%, Tests: 78%, Philosophy: 92%)
[Phase 5] Invoking architect agent...
[Phase 5] Generating executive summary...
[Phase 5] ✓ Complete: validation_report.md
```

**Output**: `./reports/[TIMESTAMP]/validation_report.md`

**Sections in report**:
- Executive Summary
- Validation Score Breakdown
- Critical Issues (prioritized)
- Medium Issues
- Low Issues
- Prioritized Recommendations
- Next Steps
- Appendices (links to phase reports)

## Command-Line Options

### Complete Reference

```bash
./validate.sh [OPTIONS]

Options:
  --phase N             Run specific phase (0-5)
  --skip-bootstrap      Skip Phase 0 (build fixes)
  --parallel N          Max parallel jobs (default: auto-detect)
  --timeout N           Agent timeout in seconds (default: 600)
  --output DIR          Custom output directory (default: ./reports/[TIMESTAMP])
  --verbose             Show detailed execution logs
  --quiet               Suppress progress messages
  --help                Display help message
  --version             Show version information
```

### Option Details

#### `--phase N`

Run a specific phase only:

```bash
# Run only gap analysis
./validate.sh --phase 2

# Run from test plan onwards
./validate.sh --phase 3
```

**Valid values**: 0, 1, 2, 3, 4, 5

**Note**: Later phases require earlier phases to have completed (for input context).

#### `--skip-bootstrap`

Skip Phase 0 (build environment setup):

```bash
./validate.sh --skip-bootstrap
```

**Use when**:
- OpenSSL already installed
- Bootstrap already run successfully
- Re-running validation after fixes

#### `--parallel N`

Limit concurrent jobs:

```bash
# Limit to 2 parallel jobs
./validate.sh --parallel 2

# Single-threaded execution
./validate.sh --parallel 1
```

**Default**: Auto-detects CPU cores

**Use when**:
- System resources limited
- Debugging agent issues
- Controlling system load

#### `--timeout N`

Set agent timeout in seconds:

```bash
# 20-minute timeout
./validate.sh --timeout 1200

# 5-minute timeout (aggressive)
./validate.sh --timeout 300
```

**Default**: 600 seconds (10 minutes)

**Use when**:
- Agents timing out prematurely
- Large codebases requiring more time
- Fast validation needed

#### `--output DIR`

Specify custom output directory:

```bash
# Use specific directory
./validate.sh --output ./my-validation-run

# Continue previous run
./validate.sh --phase 3 --output ./reports/2025-12-01_10-00-00
```

**Default**: `./reports/[TIMESTAMP]` (auto-generated)

#### `--verbose`

Show detailed execution logs:

```bash
./validate.sh --verbose
```

**Output includes**:
- Agent invocation commands
- Full stdout/stderr from agents
- Timing information
- Debug messages

#### `--quiet`

Suppress progress messages:

```bash
./validate.sh --quiet
```

**Use when**:
- Running in CI/CD
- Scripted automation
- Only want final results

## Common Use Cases

### Use Case 1: First-Time Validation

**Scenario**: Never validated RustyClawd before

**Commands**:
```bash
cd .claude/scenarios/rustyclawd-validation
./validate.sh --verbose
```

**Expected time**: 15-20 minutes

**What to check**:
- Bootstrap succeeds (OpenSSL installed)
- All agents complete successfully
- Validation report generated

### Use Case 2: Quick Build Check

**Scenario**: Just want to verify RustyClawd builds

**Commands**:
```bash
./bootstrap.sh
```

**Expected time**: 30-60 seconds

**Success criteria**:
```
[Bootstrap] ✓ Build successful
```

### Use Case 3: Re-validation After Fixes

**Scenario**: Fixed issues from previous validation, want to re-validate

**Commands**:
```bash
# Skip bootstrap, run full validation
./validate.sh --skip-bootstrap
```

**Expected time**: 12-15 minutes

**What to check**:
- Validation score improved
- Previously failed tests now pass
- Critical issues resolved

### Use Case 4: Gap Analysis Only

**Scenario**: Want to understand doc/code discrepancies without full validation

**Commands**:
```bash
# Run Phase 1 (investigation)
./validate.sh --phase 1

# Run Phase 2 (gap analysis)
./validate.sh --phase 2
```

**Expected time**: 4-7 minutes

**Output**: `gap_analysis.md` with detailed comparison

### Use Case 5: Test Execution Only

**Scenario**: Have test plan, just need to run tests

**Commands**:
```bash
./validate.sh --phase 4
```

**Expected time**: 5-10 minutes

**What to check**:
- Test pass/fail rates
- Performance metrics
- Failed test details

### Use Case 6: Incremental Validation

**Scenario**: Fixing issues one phase at a time

**Commands**:
```bash
# Day 1: Investigation + gap analysis
./validate.sh --phase 1
./validate.sh --phase 2

# Fix critical gaps...

# Day 2: Re-run gap analysis + generate test plan
./validate.sh --phase 2
./validate.sh --phase 3

# Implement missing tests...

# Day 3: Execute tests + synthesize report
./validate.sh --phase 4
./validate.sh --phase 5
```

### Use Case 7: CI/CD Integration

**Scenario**: Automated validation in continuous integration

**Commands**:
```bash
#!/bin/bash
# .github/workflows/validate.yml

# Run validation quietly
./validate.sh --quiet --output ./ci-validation

# Check validation score
score=$(grep "Overall Score:" ./ci-validation/validation_report.md | grep -o "[0-9]*")

if [ "$score" -lt 80 ]; then
    echo "Validation score too low: $score/100 (threshold: 80)"
    exit 1
fi

echo "Validation passed: $score/100"
```

### Use Case 8: Debugging Agent Issues

**Scenario**: Agent failing or producing unexpected results

**Commands**:
```bash
# Run single agent with verbose output
./validate.sh --phase 1 --parallel 1 --verbose --timeout 1200

# Examine agent output
cat ./reports/[TIMESTAMP]/logs/tester_agent.log
```

## Output Interpretation

### Understanding Validation Scores

Validation score is calculated as:

```
Total Score (0-100) =
    (Doc Coverage % × 0.30) +
    (Test Pass Rate % × 0.40) +
    (Philosophy Compliance % × 0.30)
```

**Score ranges**:
- **90-100**: Excellent - Production ready
- **80-89**: Good - Minor issues to address
- **70-79**: Fair - Significant gaps exist
- **60-69**: Poor - Major work needed
- **Below 60**: Critical - Substantial validation issues

### Reading Investigation Reports

**Tester Analysis Section**:
```markdown
## Tester Analysis

**Test Coverage**: 78%
**Test Quality**: Good
**Missing Scenarios**: 12

### Key Findings
- Unit test coverage strong (85%)
- Integration test coverage weak (45%)
- Missing edge case tests for error handling
```

**What to look for**:
- Coverage percentage (aim for 80%+)
- Missing test scenarios (prioritize critical paths)
- Test quality issues (flaky tests, poor assertions)

**Reviewer Analysis Section**:
```markdown
## Reviewer Analysis

**Code Quality**: B+
**Philosophy Compliance**: 92%
**Issues Found**: 8 (2 critical, 3 medium, 3 low)

### Critical Issues
1. Module X violates single responsibility principle
2. Excessive abstraction in Module Y
```

**What to look for**:
- Philosophy compliance percentage (aim for 90%+)
- Critical issues (highest priority fixes)
- Patterns of non-compliance (systematic problems)

**Architect Analysis Section**:
```markdown
## Architect Analysis

**Architecture Quality**: Good
**Modularity Score**: 85%
**Integration Points**: 12 (all documented)

### Design Concerns
- Circular dependency between Module A and B
- Missing abstraction boundary in Component X
```

**What to look for**:
- Modularity score (aim for 80%+)
- Circular dependencies (architectural debt)
- Missing boundaries (potential coupling issues)

### Reading Gap Analysis Reports

**Documentation Coverage Section**:
```markdown
## Documentation Coverage

**Overall Coverage**: 85%

### Features in Docs but Missing in Code
1. [HIGH] Feature X (documented in section 3.2)
2. [MEDIUM] Feature Y (documented in section 4.1)

### Features in Code but Not in Docs
1. Module Z (undocumented utility)
2. Function foo() (public API, no docs)

### Discrepancies
1. API signature mismatch: authenticate() takes 3 params (docs say 2)
2. Behavior difference: timeout default is 30s (docs say 60s)
```

**Priority levels**:
- **HIGH**: Critical missing functionality or breaking discrepancies
- **MEDIUM**: Important but non-blocking gaps
- **LOW**: Minor documentation inconsistencies

**What to prioritize**:
1. HIGH priority missing features (functionality gaps)
2. Breaking discrepancies (API mismatches)
3. MEDIUM priority features (important enhancements)
4. Documentation gaps (doc-only fixes)

### Reading Test Execution Reports

**Cargo Test Results**:
```markdown
## Cargo Tests (Rust)

**Passed**: 234
**Failed**: 5
**Skipped**: 2
**Duration**: 289s

### Failed Tests
1. test_api::test_authentication_flow
   Error: assertion failed: response.status == 200
   Got: 401

2. test_config::test_env_var_loading
   Error: environment variable not found
```

**What to look for**:
- Pass rate percentage (aim for 95%+)
- Failed test categories (isolate problem areas)
- Duration (performance regression indicator)

**Pytest Results**:
```markdown
## Pytest Tests (Python)

**Passed**: 89
**Failed**: 1
**Coverage**: 87%
**Duration**: 156s

### Failed Tests
1. test_integration::test_rust_python_bridge
   Error: Rust module failed to load
   Traceback: ...
```

**What to look for**:
- Coverage percentage (aim for 80%+)
- Integration test failures (cross-component issues)
- Performance issues (slow tests)

### Reading Validation Reports

**Executive Summary**:
```markdown
## Executive Summary

RustyClawd demonstrates strong implementation quality with **82/100**
validation score. The codebase shows excellent philosophy compliance (92%)
and good documentation coverage (85%). Test coverage is adequate (78%) but
has room for improvement in integration testing.

**Key Achievements**:
- Core functionality fully implemented
- Architecture follows brick philosophy
- Strong unit test coverage

**Areas for Improvement**:
- Integration test coverage (45% vs 80% target)
- 5 failing unit tests in authentication module
- 2 critical missing features (documented but not implemented)
```

**What it tells you**:
- Overall validation health (score)
- Strengths to maintain
- Weaknesses to address

**Prioritized Recommendations**:
```markdown
## Prioritized Recommendations

### Critical (Do First)
1. Fix 5 failing authentication tests (Est: 4 hours)
2. Implement missing Feature X (Est: 2 days)
3. Resolve circular dependency Module A ↔ B (Est: 1 day)

### High Priority (Do Next)
4. Increase integration test coverage to 80% (Est: 3 days)
5. Document undocumented Module Z (Est: 2 hours)

### Medium Priority (Schedule Soon)
6. Implement missing Feature Y (Est: 1 day)
7. Refactor Module C for better modularity (Est: 1 day)
```

**How to use**:
- Start with Critical items (highest impact)
- Estimate effort (included in recommendations)
- Track progress across validation runs

## Advanced Workflows

### Custom Agent Configuration

Create custom agent prompts in `agent_prompts/`:

```markdown
<!-- agent_prompts/security_audit.md -->

# Security Audit Task

Perform comprehensive security audit of RustyClawd.

## Focus Areas
- Input validation
- Authentication/authorization
- Cryptographic implementations
- Dependency vulnerabilities

## Deliverables
Security audit report with risk ratings.
```

Invoke in custom phase:

```bash
# Add to validate.sh
invoke_agent "security" "security_audit.md" > "$output/security_audit.md"
```

### Parallel Custom Validation

Run custom validation alongside standard phases:

```bash
# validate.sh - Phase 1 enhancement
run_investigation() {
    local output_dir="$1"

    # Standard agents
    invoke_agent "tester" "investigation.md" > "$output_dir/tester.md" &
    invoke_agent "reviewer" "investigation.md" > "$output_dir/reviewer.md" &

    # Custom agents
    invoke_agent "security" "security_audit.md" > "$output_dir/security.md" &
    invoke_agent "performance" "perf_analysis.md" > "$output_dir/performance.md" &

    wait
}
```

### Validation Result Tracking

Track validation scores over time:

```bash
#!/bin/bash
# track_scores.sh

extract_score() {
    local report="$1"
    grep "Overall Score:" "$report" | grep -o "[0-9]*"
}

log_score() {
    local timestamp="$1"
    local score="$2"
    echo "$timestamp,$score" >> validation_history.csv
}

# Run validation
./validate.sh --output ./latest-validation

# Extract and log score
score=$(extract_score ./latest-validation/validation_report.md)
log_score "$(date +%Y-%m-%d)" "$score"

# Generate trend report
python3 analyze_trends.py validation_history.csv
```

### Integration with Git Hooks

Run validation on pre-push:

```bash
#!/bin/bash
# .git/hooks/pre-push

echo "Running RustyClawd validation..."

cd .claude/scenarios/rustyclawd-validation
./validate.sh --skip-bootstrap --quiet --output ./pre-push-validation

score=$(grep "Overall Score:" ./pre-push-validation/validation_report.md | grep -o "[0-9]*")

if [ "$score" -lt 75 ]; then
    echo "Validation score too low: $score/100 (minimum: 75)"
    echo "Please address validation issues before pushing."
    exit 1
fi

echo "Validation passed: $score/100"
```

## Troubleshooting Guide

### Problem: Bootstrap Fails to Install OpenSSL

**Symptoms**:
```
[Bootstrap] ✗ Installation failed
[Bootstrap] Error: Package manager command failed
```

**Solutions**:

1. **macOS**: Ensure Homebrew is installed
```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
./bootstrap.sh
```

2. **Ubuntu/Debian**: Update package index
```bash
sudo apt-get update
./bootstrap.sh
```

3. **Manual installation**:
```bash
# macOS
brew install openssl@3

# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# Fedora/RHEL
sudo dnf install openssl-devel
```

### Problem: Agents Timeout During Phase 1

**Symptoms**:
```
[Phase 1] Tester agent timeout after 600s
[Phase 1] ✗ Investigation incomplete
```

**Solutions**:

1. **Increase timeout**:
```bash
./validate.sh --timeout 1200  # 20 minutes
```

2. **Run agents sequentially** (debugging):
```bash
./validate.sh --parallel 1 --verbose
```

3. **Check agent logs**:
```bash
cat ./reports/[TIMESTAMP]/logs/tester_agent.log
```

### Problem: Missing Phase Reports

**Symptoms**:
```
[Phase 3] Error: Investigation report not found
[Phase 3] Expected: ./reports/.../investigation_report.md
```

**Solutions**:

1. **Run earlier phases first**:
```bash
./validate.sh --phase 1
./validate.sh --phase 2
./validate.sh --phase 3
```

2. **Specify correct output directory**:
```bash
./validate.sh --phase 3 --output ./reports/2025-12-01_10-00-00
```

3. **Start fresh**:
```bash
./validate.sh  # Run all phases from beginning
```

### Problem: Test Execution Failures

**Symptoms**:
```
[Phase 4] Cargo tests failed (exit code 101)
[Phase 4] Pytest failed (exit code 1)
```

**Solutions**:

1. **Check individual test failures**:
```bash
cargo test --verbose
pytest tests/ --verbose
```

2. **Review test execution report**:
```bash
cat ./reports/[TIMESTAMP]/test_execution.md
```

3. **Run tests outside validation**:
```bash
# Isolate cargo test issues
cargo test --all

# Isolate pytest issues
pytest tests/ --verbose --tb=short
```

### Problem: Permission Errors

**Symptoms**:
```
[Phase 1] Error: Permission denied writing to ./reports/
bash: ./validate.sh: Permission denied
```

**Solutions**:

1. **Make scripts executable**:
```bash
chmod +x validate.sh bootstrap.sh synthesize_report.sh
```

2. **Create reports directory with write permissions**:
```bash
mkdir -p ./reports
chmod 755 ./reports
```

3. **Check ownership**:
```bash
ls -la ./reports
# Should be owned by your user
```

### Problem: Out of Disk Space

**Symptoms**:
```
[Phase 5] Error: No space left on device
```

**Solutions**:

1. **Clean old validation reports**:
```bash
# Keep only last 5 validations
ls -t ./reports/ | tail -n +6 | xargs -I {} rm -rf ./reports/{}
```

2. **Check disk usage**:
```bash
du -sh ./reports/*
df -h .
```

3. **Use external directory**:
```bash
./validate.sh --output /path/to/external/storage/validation
```

## Performance Tuning

### Optimizing Parallel Execution

**Auto-detect CPU cores** (default):
```bash
./validate.sh
# Uses all available cores
```

**Manual tuning**:
```bash
# High-performance system (8+ cores)
./validate.sh --parallel 5

# Resource-constrained (2 cores)
./validate.sh --parallel 2

# Single-threaded (debugging)
./validate.sh --parallel 1
```

### Reducing Validation Time

**Skip bootstrap** (if already run):
```bash
./validate.sh --skip-bootstrap
# Saves 30-60 seconds
```

**Run specific phases only**:
```bash
# Only run gap analysis (assuming Phase 1 complete)
./validate.sh --phase 2
# Saves 12-15 minutes
```

**Use aggressive timeouts**:
```bash
./validate.sh --timeout 300  # 5 minutes per agent
# Risk: Agents may timeout prematurely
```

### Memory Optimization

**Limit parallel jobs** (reduces memory):
```bash
./validate.sh --parallel 2
# Trades speed for memory usage
```

**Run phases separately**:
```bash
./validate.sh --phase 1
# Wait for Phase 1 to complete
./validate.sh --phase 2
# Spreads memory usage over time
```

## Best Practices

### Regular Validation

Run validation regularly:

```bash
# Weekly validation
0 0 * * 0 cd /path/to/rustyclawd/.claude/scenarios/rustyclawd-validation && ./validate.sh --quiet
```

### Track Validation Trends

Maintain validation history:

```bash
# validation_history.csv
2025-11-01,78
2025-11-08,82
2025-11-15,85
2025-11-22,87
```

### Prioritize Fixes

Use validation reports to prioritize work:

1. **Critical issues first** (blocking problems)
2. **High-priority gaps** (important missing features)
3. **Test coverage improvements** (long-term quality)
4. **Documentation updates** (low effort, high value)

### Validate Before Major Changes

Always validate before and after:

```bash
# Before
./validate.sh --output ./before-refactor

# Make changes...

# After
./validate.sh --output ./after-refactor

# Compare
diff ./before-refactor/validation_report.md ./after-refactor/validation_report.md
```

### Archive Validation Reports

Keep historical validation reports:

```bash
# Archive old reports
mkdir -p ./archives/2025-11
mv ./reports/2025-11-* ./archives/2025-11/

# Compress archives
tar -czf validation-archives-2025-11.tar.gz ./archives/2025-11/
```

## Related Documentation

- [README.md](./README.md) - Overview and quick start
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Technical design details
- [agent_prompts/](./agent_prompts/) - Agent task templates

---

**Last Updated**: 2025-12-01
**Version**: 1.0.0
**Maintained By**: RustyClawd Validation Team

For technical architecture details, see [ARCHITECTURE.md](./ARCHITECTURE.md).
For quick reference, see [README.md](./README.md).
