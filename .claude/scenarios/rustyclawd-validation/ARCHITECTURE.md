# RustyClawd Validation System - Architecture

Ahoy! This document details the technical architecture o' the RustyClawd validation system. It explains how the system validates a Rust implementation o' Claude Code against official documentation through parallel agent orchestration.

## Design Philosophy

This system embodies amplihack's ruthless simplicity principles:

- **Minimal codebase**: ~200 lines total (vs 2,400-line Python alternative)
- **Bash-native parallelism**: Job control (`&` + `wait`) instead of orchestration frameworks
- **Direct agent invocation**: CLI-based agent calls, no complex coordination
- **Markdown artifacts**: Human-readable, version-controllable reports
- **External process leverage**: Uses existing tools (cargo, pytest) directly

## System Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                     validate.sh (Coordinator)                │
│                                                               │
│  Phase 0: Bootstrap (build fixes)                            │
│           ↓                                                   │
│  Phase 1: Investigation (5 parallel agents)                  │
│           ↓                                                   │
│  Phase 2: Gap Analysis (analyzer agent)                      │
│           ↓                                                   │
│  Phase 3: Test Plan (tester agent)                           │
│           ↓                                                   │
│  Phase 4: Test Execution (3 parallel test suites)            │
│           ↓                                                   │
│  Phase 5: Report Synthesis (architect agent)                 │
└─────────────────────────────────────────────────────────────┘
```

### Component Structure

```
.claude/scenarios/rustyclawd-validation/
├── bootstrap.sh              # Phase 0: Build environment fixes (~40 lines)
├── validate.sh               # Main coordinator (~100 lines)
├── synthesize_report.sh      # Report generation helper (~30 lines)
├── agent_prompts/            # Agent task templates (markdown)
│   ├── investigation.md      # Phase 1 prompts (5 agents)
│   ├── gap_analysis.md       # Phase 2 prompt
│   └── test_plan.md          # Phase 3 prompt
├── tests/
│   └── test_validation.sh    # System validation tests (~30 lines)
├── reports/                  # Generated validation reports
│   └── [TIMESTAMP]/          # Timestamped validation run
│       ├── investigation_report.md
│       ├── gap_analysis.md
│       ├── test_plan.md
│       ├── test_execution.md
│       └── validation_report.md
├── README.md                 # User documentation
├── ARCHITECTURE.md           # This file
└── USAGE.md                  # Detailed usage guide
```

**Total Lines**: ~200 lines of bash across all scripts

## Phase-by-Phase Architecture

### Phase 0: Bootstrap

**Purpose**: Fix build environment before validation starts

**Script**: `bootstrap.sh` (~40 lines)

**Responsibilities**:
- Detect operating system and package manager
- Install OpenSSL development libraries
- Verify installation success
- Test cargo build

**Process Flow**:

```bash
┌──────────────┐
│  Detect OS   │
└──────┬───────┘
       │
       ├─ macOS → brew install openssl@3
       ├─ Ubuntu/Debian → apt-get install libssl-dev pkg-config
       ├─ Fedora/RHEL → dnf install openssl-devel
       └─ Windows/WSL → apt-get install libssl-dev pkg-config
       │
┌──────▼──────────┐
│ Test cargo build│
└─────────────────┘
```

**Key Implementation**:

```bash
detect_os() {
    case "$(uname -s)" in
        Darwin*) echo "macos" ;;
        Linux*)
            if grep -qi microsoft /proc/version; then
                echo "wsl"
            elif [ -f /etc/debian_version ]; then
                echo "debian"
            elif [ -f /etc/redhat-release ]; then
                echo "redhat"
            fi ;;
    esac
}

install_deps() {
    local os="$1"
    case "$os" in
        macos) brew install openssl@3 ;;
        debian|wsl) sudo apt-get install -y libssl-dev pkg-config ;;
        redhat) sudo dnf install -y openssl-devel ;;
    esac
}
```

**Exit Conditions**:
- Success (0): OpenSSL installed, cargo builds successfully
- Failure (1): Installation failed or cargo build still broken

### Phase 1: Investigation

**Purpose**: Parallel analysis of RustyClawd implementation by specialized agents

**Coordinator**: `validate.sh` phase 1 section (~20 lines)

**Responsibilities**:
- Launch 5 agents in parallel
- Collect agent outputs
- Merge findings into investigation report
- Handle agent failures gracefully

**Parallel Agents**:

1. **Tester Agent** - Test coverage analysis
2. **Reviewer Agent** - Code quality and philosophy compliance
3. **Architect Agent** - Design patterns and architecture review
4. **Cargo Test Executor** - Run Rust unit tests
5. **Pytest Executor** - Run Python integration tests

**Process Flow**:

```bash
                    ┌──────────────┐
                    │ Launch Phase │
                    └──────┬───────┘
                           │
        ┌──────────────────┼──────────────────┬──────────────┬──────────────┐
        │                  │                  │              │              │
   ┌────▼────┐      ┌──────▼──────┐    ┌─────▼─────┐  ┌─────▼─────┐  ┌────▼────┐
   │ Tester  │      │  Reviewer   │    │ Architect │  │   Cargo   │  │ Pytest  │
   │  Agent  │      │   Agent     │    │   Agent   │  │   Tests   │  │  Tests  │
   └────┬────┘      └──────┬──────┘    └─────┬─────┘  └─────┬─────┘  └────┬────┘
        │                  │                  │              │              │
        │                  │                  │              │              │
        └──────────────────┴──────────────────┴──────────────┴──────────────┘
                                      │
                               ┌──────▼──────┐
                               │    wait     │
                               │  (collect)  │
                               └──────┬──────┘
                                      │
                             ┌────────▼─────────┐
                             │ Merge findings   │
                             │ into report      │
                             └──────────────────┘
```

**Key Implementation**:

```bash
# Launch agents in parallel
run_investigation() {
    local output_dir="$1"

    # Launch each agent as background job
    invoke_agent "tester" "investigation.md" > "$output_dir/tester.md" 2>&1 &
    local pid_tester=$!

    invoke_agent "reviewer" "investigation.md" > "$output_dir/reviewer.md" 2>&1 &
    local pid_reviewer=$!

    invoke_agent "architect" "investigation.md" > "$output_dir/architect.md" 2>&1 &
    local pid_architect=$!

    cargo test > "$output_dir/cargo_tests.txt" 2>&1 &
    local pid_cargo=$!

    pytest tests/ > "$output_dir/pytest.txt" 2>&1 &
    local pid_pytest=$!

    # Wait for all to complete
    wait $pid_tester $pid_reviewer $pid_architect $pid_cargo $pid_pytest

    # Merge results
    merge_investigation_findings "$output_dir"
}
```

**Parallelism Strategy**:
- All 5 tasks run concurrently using bash background jobs (`&`)
- `wait` command blocks until all complete
- Process IDs tracked for potential timeout management
- Independent outputs to separate files prevent race conditions

**Agent Prompt Template**: `agent_prompts/investigation.md`

```markdown
# Investigation Task: [AGENT_TYPE]

Analyze RustyClawd implementation focusing on your specialty:

## Context
- Repository: /path/to/RustyClawd
- Official docs: /path/to/claude-code-docs

## Your Focus
[Tester: Test coverage and test quality]
[Reviewer: Code quality and philosophy compliance]
[Architect: Design patterns and architecture]

## Deliverables
1. Key findings (3-5 bullet points)
2. Critical issues (prioritized)
3. Recommendations (actionable)

## Output Format
Markdown with clear sections.
```

**Output**: `investigation_report.md` (merged findings from all 5 agents)

### Phase 2: Gap Analysis

**Purpose**: Compare official documentation against actual implementation

**Coordinator**: `validate.sh` phase 2 section (~15 lines)

**Responsibilities**:
- Invoke analyzer agent with gap analysis prompt
- Feed investigation report as context
- Identify doc/code discrepancies
- Prioritize gaps by severity

**Process Flow**:

```bash
┌────────────────────────┐
│ Load investigation     │
│ report (Phase 1 output)│
└──────────┬─────────────┘
           │
           │
┌──────────▼──────────────┐
│ Invoke analyzer agent   │
│ with gap analysis prompt│
└──────────┬──────────────┘
           │
           │
┌──────────▼──────────────┐
│ Compare:                │
│ - Official Claude docs  │
│ - RustyClawd impl       │
│ - Investigation findings│
└──────────┬──────────────┘
           │
           │
┌──────────▼──────────────┐
│ Generate gap_analysis.md│
│ - Missing features      │
│ - Extra features        │
│ - Discrepancies         │
└─────────────────────────┘
```

**Key Implementation**:

```bash
run_gap_analysis() {
    local input_report="$1"
    local output_file="$2"

    # Build context from investigation report
    local context
    context=$(cat "$input_report")

    # Invoke analyzer agent
    invoke_agent "analyzer" "gap_analysis.md" \
        --context "$context" \
        --docs "/path/to/claude-code-docs" \
        --impl "/path/to/RustyClawd" \
        > "$output_file"
}
```

**Agent Prompt Template**: `agent_prompts/gap_analysis.md`

```markdown
# Gap Analysis Task

Compare RustyClawd implementation against official Claude Code documentation.

## Inputs
- Investigation report: [provided as context]
- Official docs: /path/to/claude-code-docs
- Implementation: /path/to/RustyClawd

## Analysis Dimensions
1. Features in docs but missing in code
2. Features in code but not in docs
3. Behavioral discrepancies
4. API signature differences
5. Configuration mismatches

## Output Structure
### Critical Gaps (High Priority)
### Medium Gaps
### Low Priority Gaps
### Documentation Gaps (need doc updates)
### Recommendations (prioritized action items)
```

**Output**: `gap_analysis.md` (structured comparison report)

### Phase 3: Test Plan Generation

**Purpose**: Create comprehensive test strategy based on gap analysis

**Coordinator**: `validate.sh` phase 3 section (~10 lines)

**Responsibilities**:
- Invoke tester agent with test plan prompt
- Feed gap analysis as input
- Generate test cases for identified gaps
- Prioritize test scenarios

**Process Flow**:

```bash
┌─────────────────────┐
│ Load gap_analysis.md│
└──────────┬──────────┘
           │
           │
┌──────────▼──────────────┐
│ Invoke tester agent     │
│ with test plan prompt   │
└──────────┬──────────────┘
           │
           │
┌──────────▼──────────────┐
│ Generate test_plan.md   │
│ - Unit test scenarios   │
│ - Integration scenarios │
│ - E2E test cases        │
│ - Priority rankings     │
└─────────────────────────┘
```

**Key Implementation**:

```bash
run_test_plan() {
    local gap_analysis="$1"
    local output_file="$2"

    # Build context from gap analysis
    local context
    context=$(cat "$gap_analysis")

    # Invoke tester agent
    invoke_agent "tester" "test_plan.md" \
        --context "$context" \
        > "$output_file"
}
```

**Agent Prompt Template**: `agent_prompts/test_plan.md`

```markdown
# Test Plan Generation Task

Create comprehensive test plan for RustyClawd validation.

## Inputs
- Gap analysis: [provided as context]
- Investigation findings: [reference Phase 1]

## Test Strategy
Generate test cases covering:
1. Unit tests (60% of effort)
2. Integration tests (30% of effort)
3. End-to-end tests (10% of effort)

## Test Categories
- Critical path validation
- Gap coverage (from Phase 2)
- Regression prevention
- Performance benchmarks
- Security validation

## Output Format
Markdown checklist with:
- [ ] Test case description
- Priority: [High/Medium/Low]
- Type: [Unit/Integration/E2E]
- Expected outcome
```

**Output**: `test_plan.md` (structured test strategy)

### Phase 4: Test Execution

**Purpose**: Execute test suites in parallel and collect results

**Coordinator**: `validate.sh` phase 4 section (~25 lines)

**Responsibilities**:
- Run Rust unit tests (cargo test)
- Run Python integration tests (pytest)
- Run custom integration tests
- Collect and format results

**Process Flow**:

```bash
                    ┌──────────────┐
                    │ Launch Phase │
                    └──────┬───────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────────┐  ┌──────▼──────┐   ┌──────▼──────────┐
   │ cargo test  │  │   pytest    │   │  Integration    │
   │  (Rust)     │  │  (Python)   │   │   test suite    │
   └────┬────────┘  └──────┬──────┘   └──────┬──────────┘
        │                  │                  │
        └──────────────────┴──────────────────┘
                           │
                    ┌──────▼──────┐
                    │    wait     │
                    └──────┬──────┘
                           │
                  ┌────────▼─────────┐
                  │ Format results   │
                  │ into test_exec.md│
                  └──────────────────┘
```

**Key Implementation**:

```bash
run_test_execution() {
    local output_dir="$1"

    # Launch test suites in parallel
    cargo test --all > "$output_dir/cargo_results.txt" 2>&1 &
    local pid_cargo=$!

    pytest tests/ --verbose > "$output_dir/pytest_results.txt" 2>&1 &
    local pid_pytest=$!

    ./tests/integration_tests.sh > "$output_dir/integration_results.txt" 2>&1 &
    local pid_integration=$!

    # Wait for all
    wait $pid_cargo $pid_pytest $pid_integration

    # Parse and format results
    format_test_results "$output_dir"
}

format_test_results() {
    local dir="$1"
    local output="$dir/test_execution.md"

    echo "# Test Execution Results" > "$output"
    echo "" >> "$output"

    # Parse cargo test output
    echo "## Cargo Tests (Rust)" >> "$output"
    grep -E "test result:|passed|failed" "$dir/cargo_results.txt" >> "$output"

    # Parse pytest output
    echo "" >> "$output"
    echo "## Pytest Tests (Python)" >> "$output"
    grep -E "passed|failed|FAILED|ERROR" "$dir/pytest_results.txt" >> "$output"

    # Parse integration results
    echo "" >> "$output"
    echo "## Integration Tests" >> "$output"
    cat "$dir/integration_results.txt" >> "$output"
}
```

**Parallelism Strategy**:
- Three test suites run concurrently
- Each writes to separate output file
- Results parsed and formatted into unified report
- Test failures don't block other suites

**Output**: `test_execution.md` (combined test results)

### Phase 5: Report Synthesis

**Purpose**: Generate executive summary validation report

**Script**: `synthesize_report.sh` (~30 lines)

**Coordinator**: `validate.sh` phase 5 section (~10 lines)

**Responsibilities**:
- Collect all phase outputs
- Invoke architect agent for synthesis
- Generate validation score
- Create actionable recommendations

**Process Flow**:

```bash
┌─────────────────────────────────────┐
│ Collect Phase 1-4 outputs:          │
│ - investigation_report.md           │
│ - gap_analysis.md                   │
│ - test_plan.md                      │
│ - test_execution.md                 │
└──────────────┬──────────────────────┘
               │
               │
┌──────────────▼──────────────────────┐
│ Invoke architect agent              │
│ with synthesis prompt               │
└──────────────┬──────────────────────┘
               │
               │
┌──────────────▼──────────────────────┐
│ Generate validation_report.md       │
│ - Executive summary                 │
│ - Validation score (0-100)          │
│ - Critical issues                   │
│ - Prioritized recommendations       │
│ - Links to detailed phase reports   │
└─────────────────────────────────────┘
```

**Key Implementation**:

```bash
#!/bin/bash
# synthesize_report.sh

synthesize_validation_report() {
    local reports_dir="$1"
    local output_file="$2"

    # Collect all phase outputs
    local context=""
    context+="# Investigation Findings\n"
    context+=$(cat "$reports_dir/investigation_report.md")
    context+="\n\n# Gap Analysis\n"
    context+=$(cat "$reports_dir/gap_analysis.md")
    context+="\n\n# Test Plan\n"
    context+=$(cat "$reports_dir/test_plan.md")
    context+="\n\n# Test Results\n"
    context+=$(cat "$reports_dir/test_execution.md")

    # Calculate validation score
    local score
    score=$(calculate_validation_score "$reports_dir")

    # Invoke architect for synthesis
    invoke_agent "architect" "synthesis_prompt.md" \
        --context "$context" \
        --score "$score" \
        > "$output_file"
}

calculate_validation_score() {
    local dir="$1"
    local score=0

    # Doc coverage (30 points)
    local doc_coverage
    doc_coverage=$(grep -o "Coverage: [0-9]*%" "$dir/gap_analysis.md" | grep -o "[0-9]*")
    score=$((score + doc_coverage * 30 / 100))

    # Test pass rate (40 points)
    local test_passes
    test_passes=$(grep "passed" "$dir/test_execution.md" | wc -l)
    local test_total
    test_total=$(grep -E "passed|failed" "$dir/test_execution.md" | wc -l)
    score=$((score + test_passes * 40 / test_total))

    # Philosophy compliance (30 points)
    local compliance
    compliance=$(grep -o "Compliance: [0-9]*%" "$dir/investigation_report.md" | grep -o "[0-9]*")
    score=$((score + compliance * 30 / 100))

    echo "$score"
}
```

**Validation Score Formula**:
```
Total Score (0-100) =
    (Doc Coverage % × 0.30) +
    (Test Pass Rate % × 0.40) +
    (Philosophy Compliance % × 0.30)
```

**Output**: `validation_report.md` (executive summary)

## Agent Invocation Pattern

All agent invocations follow a uniform pattern:

```bash
invoke_agent() {
    local agent_type="$1"    # e.g., "tester", "reviewer", "architect"
    local prompt_file="$2"   # e.g., "investigation.md"
    shift 2

    # Build agent invocation command
    # (Assumes Claude Code CLI with agent subcommand)
    claude-code agent "$agent_type" \
        --prompt "agent_prompts/$prompt_file" \
        "$@"
}
```

**Key Features**:
- Standardized interface across all agents
- Prompt templates in `agent_prompts/` directory
- Additional arguments passed through (`"$@"`)
- Clean separation of orchestration and agent logic

## Data Flow

```
┌──────────────────┐
│   Bootstrap      │
│  (OpenSSL deps)  │
└────────┬─────────┘
         │
         ▼
┌────────────────────────────────┐
│   Phase 1: Investigation       │
│   5 parallel agents            │
└────────┬───────────────────────┘
         │
         │ investigation_report.md
         ▼
┌────────────────────────────────┐
│   Phase 2: Gap Analysis        │
│   analyzer agent               │
└────────┬───────────────────────┘
         │
         │ gap_analysis.md
         ▼
┌────────────────────────────────┐
│   Phase 3: Test Plan           │
│   tester agent                 │
└────────┬───────────────────────┘
         │
         │ test_plan.md
         ▼
┌────────────────────────────────┐
│   Phase 4: Test Execution      │
│   3 parallel test suites       │
└────────┬───────────────────────┘
         │
         │ test_execution.md
         ▼
┌────────────────────────────────┐
│   Phase 5: Report Synthesis    │
│   architect agent              │
└────────┬───────────────────────┘
         │
         │ validation_report.md
         ▼
┌────────────────────────────────┐
│   Complete Validation          │
│   All reports in ./reports/    │
└────────────────────────────────┘
```

**Artifact Format**: All artifacts are markdown files with consistent structure:

```markdown
# [Phase Name] Report

**Generated**: 2025-12-01 14:30:22
**Phase**: [1-5]
**Status**: [Success/Failed]

## Executive Summary
[3-5 bullet points]

## Detailed Findings
[Structured sections]

## Recommendations
[Prioritized action items]

## Appendices
[Supporting data]
```

## Parallelism Architecture

### Bash Job Control Strategy

The system achieves maximum parallelism through bash's native job control:

```bash
# Pattern 1: Launch independent jobs
task1 &
pid1=$!

task2 &
pid2=$!

task3 &
pid3=$!

# Wait for all to complete
wait $pid1 $pid2 $pid3
```

**Benefits**:
- No external dependencies (pure bash)
- Automatic CPU core utilization
- Simple error handling (exit codes preserved)
- Clean process isolation

**Limitations**:
- No built-in timeout handling (requires additional logic)
- No automatic retry mechanisms
- Manual coordination for shared resources

### Parallelism Points

**Phase 1** (5-way parallelism):
```
tester & reviewer & architect & cargo_test & pytest
```

**Phase 4** (3-way parallelism):
```
cargo_test & pytest & integration_tests
```

**Total Throughput**: Up to 5 concurrent operations

## Error Handling Strategy

### Graceful Degradation

Each phase implements graceful degradation:

```bash
run_phase() {
    local phase_num="$1"
    local output_dir="$2"

    echo "[Phase $phase_num] Starting..."

    if ! execute_phase "$phase_num" "$output_dir"; then
        echo "[Phase $phase_num] FAILED - continuing with partial results"
        create_partial_report "$phase_num" "$output_dir"
        return 1
    fi

    echo "[Phase $phase_num] SUCCESS"
    return 0
}
```

**Philosophy**: Never fail completely - always produce partial results

### Phase Independence

Phases are designed to handle missing inputs:

```bash
run_gap_analysis() {
    local investigation_report="$1"

    if [ ! -f "$investigation_report" ]; then
        echo "WARNING: Investigation report missing - using limited context"
        investigation_report="/dev/null"
    fi

    # Continue with available data
    invoke_analyzer "$investigation_report"
}
```

### Error Classification

Errors are classified by severity:

1. **Critical** (Bootstrap failures) - Cannot proceed
2. **Major** (Phase failures) - Can proceed with degraded results
3. **Minor** (Agent timeouts) - Retry or skip

## Performance Characteristics

### Time Complexity

```
T_total = max(T_phase1_parallel) +
          T_phase2 +
          T_phase3 +
          max(T_phase4_parallel) +
          T_phase5

Where:
  T_phase1_parallel = max(T_tester, T_reviewer, T_architect, T_cargo, T_pytest)
  T_phase4_parallel = max(T_cargo, T_pytest, T_integration)
```

**Typical Values**:
- `T_phase1_parallel` ≈ 3-5 minutes (longest agent)
- `T_phase2` ≈ 1-2 minutes
- `T_phase3` ≈ 1-2 minutes
- `T_phase4_parallel` ≈ 5-10 minutes (test execution)
- `T_phase5` ≈ 1 minute

**Total**: ~15-20 minutes

### Space Complexity

Report storage scales linearly with validation runs:

```
Space = N_validations × (
    investigation_report_size +
    gap_analysis_size +
    test_plan_size +
    test_execution_size +
    validation_report_size
)

Typical per-run: ~500KB - 2MB (markdown text)
```

### Scalability

**CPU Cores**: System automatically utilizes available cores via bash job control
**Memory**: Minimal - bash scripts + agent processes
**Disk I/O**: Moderate - markdown file writes only

## Testing Strategy

### System Validation Tests

Located in `tests/test_validation.sh`:

```bash
#!/bin/bash
# test_validation.sh

test_bootstrap() {
    echo "Testing bootstrap..."
    ./bootstrap.sh
    [ $? -eq 0 ] && echo "✓ Bootstrap works" || echo "✗ Bootstrap failed"
}

test_agent_invocation() {
    echo "Testing agent invocation..."
    invoke_agent "tester" "investigation.md" > /dev/null
    [ $? -eq 0 ] && echo "✓ Agent invocation works" || echo "✗ Agent failed"
}

test_parallel_execution() {
    echo "Testing parallel execution..."
    sleep 1 & sleep 1 & sleep 1 &
    wait
    [ $? -eq 0 ] && echo "✓ Parallelism works" || echo "✗ Parallel failed"
}

# Run all tests
test_bootstrap
test_agent_invocation
test_parallel_execution
```

### Integration Tests

The system includes self-validation:

```bash
./validate.sh --self-test
```

Validates:
- All scripts are executable
- Agent prompts exist
- Output directories are writable
- Required tools (cargo, pytest) are available

## Extension Points

### Adding New Phases

To add a new validation phase:

1. **Create phase function** in `validate.sh`:
```bash
run_phase_6_new_analysis() {
    local output_dir="$1"
    # Implementation
}
```

2. **Add prompt template** in `agent_prompts/new_analysis.md`

3. **Update coordinator** to call new phase:
```bash
case "$phase" in
    ...
    6) run_phase_6_new_analysis "$output_dir" ;;
esac
```

### Adding New Agents

To add new agent to existing phase:

1. **Update phase function** to launch new agent:
```bash
invoke_agent "new_agent" "prompt.md" > "$output/new_agent.md" 2>&1 &
local pid_new=$!
```

2. **Add to wait list**:
```bash
wait $pid1 $pid2 $pid_new
```

3. **Update report merger** to include new findings

### Custom Test Suites

To add custom test suite to Phase 4:

1. **Create test script** (e.g., `custom_tests.sh`)

2. **Update test execution**:
```bash
./custom_tests.sh > "$output_dir/custom_results.txt" 2>&1 &
local pid_custom=$!
```

3. **Update result formatting** to parse new output

## Comparison to Alternatives

### vs Python Implementation (2,400 lines)

| Aspect | Bash System | Python Alternative |
|--------|-------------|-------------------|
| **Lines of Code** | ~200 | ~2,400 (12x) |
| **Dependencies** | Bash 4.0+ | Python 3.8+, asyncio, aiohttp, etc. |
| **Parallelism** | Native job control | asyncio orchestration |
| **Agent Invocation** | Direct CLI calls | Framework coordination |
| **Artifacts** | Markdown files | JSON + markdown |
| **Complexity** | Minimal | High (frameworks, async) |

### vs Orchestration Frameworks

| Aspect | Bash System | Airflow/Prefect |
|--------|-------------|-----------------|
| **Setup Complexity** | None | High |
| **Maintenance** | Low | Medium |
| **Debugging** | Simple (bash -x) | Complex |
| **Flexibility** | High | Medium |
| **Learning Curve** | Minimal | Steep |

## Philosophy Alignment

This architecture embodies amplihack principles:

### Ruthless Simplicity
- ~200 lines total vs 2,400-line alternatives
- Bash-native parallelism (no frameworks)
- Direct agent invocation (no coordination layer)

### Brick Architecture
- Self-contained phases with clear contracts
- Each phase independently runnable
- Markdown artifacts as "studs" (interfaces)

### Zero-BS Implementation
- No stubs or placeholders
- All scripts fully functional
- Real external process execution

### Parallel by Default
- Maximum parallelism via bash job control
- 5-way parallelism (Phase 1), 3-way (Phase 4)

### Regeneratable Modules
- Each phase can be re-run independently
- Idempotent operations
- Clear input/output contracts

## Related Documentation

- [README.md](./README.md) - User guide and quick start
- [USAGE.md](./USAGE.md) - Detailed usage patterns and examples
- [agent_prompts/](./agent_prompts/) - Agent task templates

## Revision History

- **2025-12-01**: Initial architecture document
- **Version**: 1.0.0
- **Maintained By**: RustyClawd Validation Team

---

For implementation questions, see [USAGE.md](./USAGE.md).
For user-facing documentation, see [README.md](./README.md).
