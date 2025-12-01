# RustyClawd Validation System

Arrr! This be a comprehensive validation tool fer RustyClawd - a Rust implementation o' Claude Code. It validates the implementation against official documentation through parallel agent orchestration and automated testin'.

## Quick Start

```bash
# Run complete validation (includes bootstrap)
cd .claude/scenarios/rustyclawd-validation
./validate.sh

# Check build status only (bootstrap)
./bootstrap.sh

# Run specific phase
./validate.sh --phase 2  # Gap analysis only
```

## What It Does

This validation system:

1. **Fixes the build environment** - Installs OpenSSL dependencies automatically
2. **Investigates implementation** - Parallel analysis by multiple specialized agents
3. **Identifies gaps** - Compares implementation against official docs
4. **Generates test plans** - Creates comprehensive test coverage strategy
5. **Executes tests** - Runs Rust tests, Python tests, and integration tests in parallel
6. **Synthesizes reports** - Produces human-readable validation reports

## Prerequisites

- **Bash 4.0+** - Job control support for parallelism
- **Rust toolchain** - cargo, rustc
- **Python 3.8+** - For pytest tests
- **Claude Code** - Must be runnable in the environment
- **Git** - Repository access

## Installation

The validation system be already installed in yer RustyClawd repository:

```
.claude/scenarios/rustyclawd-validation/
├── bootstrap.sh              # Phase 0: Build fixes
├── validate.sh               # Main coordinator
├── synthesize_report.sh      # Report generation
├── agent_prompts/            # Agent task templates
│   ├── investigation.md
│   ├── gap_analysis.md
│   └── test_plan.md
└── tests/
    └── test_validation.sh    # System tests
```

No additional installation be needed!

## Usage

### Complete Validation (All Phases)

Run the full validation pipeline from start to finish:

```bash
./validate.sh
```

This executes all six phases:
- Phase 0: Bootstrap (build environment fixes)
- Phase 1: Investigation (parallel agent analysis)
- Phase 2: Gap Analysis (doc vs implementation)
- Phase 3: Test Plan Generation
- Phase 4: Test Execution (parallel)
- Phase 5: Report Synthesis

**Output**: Creates timestamped reports in `./reports/YYYY-MM-DD_HH-MM-SS/`

### Run Specific Phases

Skip to a particular phase when ye already have previous results:

```bash
# Run only gap analysis (assumes Phase 1 completed)
./validate.sh --phase 2

# Run from Phase 3 onwards
./validate.sh --phase 3
```

### Bootstrap Only (Fix Build)

If RustyClawd won't build, run bootstrap first:

```bash
./bootstrap.sh
```

This installs OpenSSL development libraries on yer platform:
- **macOS**: `brew install openssl@3`
- **Ubuntu/Debian**: `apt-get install libssl-dev pkg-config`
- **Fedora/RHEL**: `dnf install openssl-devel`

### Check System Health

Verify the validation system itself works:

```bash
cd tests
./test_validation.sh
```

## Understanding the Reports

After validation completes, reports be stored in `./reports/[TIMESTAMP]/`:

### Phase 1: Investigation Report (`investigation_report.md`)

Contains findings from five parallel investigations:

```markdown
# Investigation Report

## Tester Analysis
- Test coverage findings
- Missing test scenarios
- Test quality assessment

## Reviewer Analysis
- Code quality issues
- Philosophy compliance
- Architecture concerns

## Architect Analysis
- Design patterns
- Module structure
- Integration points

## Cargo Test Results
- Unit test pass/fail status
- Performance metrics
- Rust-specific issues

## Pytest Results
- Python test coverage
- Integration test status
- Cross-language testing gaps
```

### Phase 2: Gap Analysis Report (`gap_analysis.md`)

Compares official docs against actual implementation:

```markdown
# Gap Analysis Report

## Documentation Coverage
- Features documented but not implemented
- Features implemented but not documented
- Discrepancies between docs and code

## Critical Gaps
- High-priority missing functionality
- Breaking inconsistencies
- Security concerns

## Recommendations
- Prioritized fixes
- Documentation updates
- Architecture improvements
```

### Phase 3: Test Plan (`test_plan.md`)

Comprehensive test strategy:

```markdown
# Test Plan

## Unit Tests
- [ ] Module X - core functionality
- [ ] Module Y - error handling
- [ ] Module Z - edge cases

## Integration Tests
- [ ] Component A ↔ Component B
- [ ] External service integration
- [ ] End-to-end workflows

## Test Priorities
1. Critical path testing
2. Security validation
3. Performance benchmarks
```

### Phase 4: Test Execution Results (`test_execution.md`)

Combined results from all test runs:

```markdown
# Test Execution Results

## Cargo Tests (Rust)
- Passed: 234
- Failed: 5
- Skipped: 2

## Pytest Tests (Python)
- Passed: 89
- Failed: 1
- Coverage: 87%

## Integration Tests
- Scenarios passed: 12/15
- Performance: Within thresholds
```

### Phase 5: Final Validation Report (`validation_report.md`)

Executive summary synthesizing all findings:

```markdown
# RustyClawd Validation Report

## Executive Summary
Overall validation status and key findings.

## Validation Score
- Documentation Coverage: 85%
- Test Coverage: 78%
- Philosophy Compliance: 92%
- **Overall Score: 82/100**

## Critical Issues
1. [High] Feature X missing implementation
2. [Medium] Test coverage gaps in Module Y

## Recommendations
Prioritized action items for improving validation score.

## Appendices
Links to detailed phase reports.
```

## Command-Line Options

```bash
./validate.sh [OPTIONS]

Options:
  --phase N          Run specific phase (0-5)
  --skip-bootstrap   Skip Phase 0 (build fixes)
  --parallel N       Max parallel jobs (default: auto-detected)
  --timeout N        Agent timeout in seconds (default: 600)
  --output DIR       Custom output directory
  --verbose          Show detailed execution logs
  --help             Display this help message
```

## Examples

### Example 1: First-Time Validation

```bash
# Fresh validation of RustyClawd
cd .claude/scenarios/rustyclawd-validation
./validate.sh --verbose

# Output:
# [Phase 0] Bootstrap: Installing OpenSSL dependencies...
# [Phase 1] Investigation: Launching 5 parallel agents...
# [Phase 1] Complete: investigation_report.md generated
# [Phase 2] Gap Analysis: Comparing docs vs implementation...
# [Phase 2] Complete: gap_analysis.md generated
# [Phase 3] Test Plan: Generating comprehensive test strategy...
# [Phase 3] Complete: test_plan.md generated
# [Phase 4] Test Execution: Running parallel test suites...
# [Phase 4] Complete: test_execution.md generated
# [Phase 5] Report Synthesis: Creating final validation report...
# [Phase 5] Complete: validation_report.md generated
#
# Validation complete! Reports in: ./reports/2025-12-01_14-30-22/
```

### Example 2: Incremental Validation

After fixing issues from a previous run, skip bootstrap and re-validate:

```bash
./validate.sh --skip-bootstrap --phase 2

# Output:
# [Phase 2] Gap Analysis: Starting from existing investigation...
# [Phase 2] Complete: gap_analysis.md updated
# [Phase 3] Test Plan: Refreshing test strategy...
# ...
```

### Example 3: Quick Test-Only Run

Run just the test execution phase:

```bash
./validate.sh --phase 4

# Output:
# [Phase 4] Test Execution: Running tests...
# [Cargo] Running Rust unit tests...
# [Pytest] Running Python integration tests...
# [Integration] Running end-to-end scenarios...
# [Phase 4] Complete: test_execution.md generated
```

## Troubleshooting

### Build Failures

**Problem**: `cargo build` fails with OpenSSL errors

**Solution**: Run bootstrap explicitly:

```bash
./bootstrap.sh
# Then retry validation
./validate.sh
```

### Agent Timeouts

**Problem**: Agents timing out during investigation phase

**Solution**: Increase timeout:

```bash
./validate.sh --timeout 1200  # 20 minutes
```

### Parallel Job Issues

**Problem**: System overload from too many parallel jobs

**Solution**: Limit parallelism:

```bash
./validate.sh --parallel 2  # Max 2 concurrent jobs
```

### Missing Reports

**Problem**: Previous phase reports not found

**Solution**: Either run all phases from start, or specify output directory with existing reports:

```bash
./validate.sh --phase 3 --output ./reports/2025-12-01_10-00-00
```

### Permission Errors

**Problem**: Cannot write to reports directory

**Solution**: Ensure write permissions:

```bash
chmod +x validate.sh bootstrap.sh synthesize_report.sh
mkdir -p ./reports
chmod 755 ./reports
```

## Architecture Overview

The validation system follows ruthless simplicity principles:

- **~200 lines total** - Minimal bash scripts
- **Maximum parallelism** - Bash job control (`&` + `wait`)
- **Direct agent invocation** - No complex orchestration frameworks
- **Markdown artifacts** - Human-readable, version-controllable outputs
- **External process execution** - Leverages existing tools (cargo, pytest)

For detailed architecture information, see [ARCHITECTURE.md](./ARCHITECTURE.md).

## Performance

Typical validation times on a 4-core system:

- **Phase 0 (Bootstrap)**: 30-60 seconds (one-time)
- **Phase 1 (Investigation)**: 3-5 minutes (5 parallel agents)
- **Phase 2 (Gap Analysis)**: 1-2 minutes
- **Phase 3 (Test Plan)**: 1-2 minutes
- **Phase 4 (Test Execution)**: 5-10 minutes (parallel test suites)
- **Phase 5 (Report Synthesis)**: 1 minute

**Total**: ~15-20 minutes for complete validation

## Related Documentation

- [ARCHITECTURE.md](./ARCHITECTURE.md) - Technical design and implementation details
- [USAGE.md](./USAGE.md) - Comprehensive user guide with advanced usage
- [agent_prompts/](./agent_prompts/) - Agent task templates and prompt engineering

## Philosophy Alignment

This validation system embodies amplihack principles:

- **Ruthless Simplicity**: ~200 lines vs 2,400-line Python alternative
- **Brick Architecture**: Self-contained phases with clear contracts
- **Zero-BS**: No stubs, all scripts fully functional
- **Parallel by Default**: Maximum throughput with bash job control
- **Regeneratable**: Each phase can be re-run independently

## Support

For issues or questions:

1. Check [Troubleshooting](#troubleshooting) section
2. Review [USAGE.md](./USAGE.md) for detailed usage patterns
3. Examine [ARCHITECTURE.md](./ARCHITECTURE.md) for technical details
4. Check phase-specific logs in `./reports/[TIMESTAMP]/logs/`

## License

Part of the RustyClawd project. See repository root for license information.

---

**Last Updated**: 2025-12-01
**Version**: 1.0.0
**Maintained By**: RustyClawd Validation Team
