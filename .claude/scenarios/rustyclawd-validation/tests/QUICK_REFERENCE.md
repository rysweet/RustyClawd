# Quick Reference Guide

One-page reference for the RustyClawd validation test suite.

## Run Tests

```bash
cd .claude/scenarios/rustyclawd-validation/tests

# All tests
./run_all_tests.sh

# By category
./run_all_tests.sh unit          # 49 unit tests
./run_all_tests.sh integration   # 14 integration tests
./run_all_tests.sh e2e          # 15 E2E tests

# Individual files
./test_bootstrap.sh              # 14 tests
./test_validate_phases.sh        # 18 tests
./test_report_generation.sh      # 17 tests
./test_integration.sh            # 14 tests
./test_e2e.sh                   # 15 tests
```

## Test Files

| File | Tests | Coverage |
|------|-------|----------|
| test_bootstrap.sh | 14 | bootstrap.sh script |
| test_validate_phases.sh | 18 | validate.sh phases 1-5 |
| test_report_generation.sh | 17 | synthesize_report.sh |
| test_integration.sh | 14 | Multi-component workflows |
| test_e2e.sh | 15 | Complete workflows |
| **TOTAL** | **78** | **Complete system** |

## What Tests Cover

### Bootstrap Tests (14)
- OpenSSL detection (Fedora/Debian)
- Package installation with sudo
- RustyClawd build via cargo
- Build failure handling
- Artifact generation

### Validation Phase Tests (18)
- Phase 1: 5 parallel workstreams
- Phase 2: Synthesis with dependencies
- Phase 3: Test plan generation
- Phase 4: Parallel test execution
- Phase 5: Report synthesis
- Phase dependency enforcement

### Report Generation Tests (17)
- Artifact reading
- Markdown structure
- Section ordering
- Missing artifact handling
- Table of contents
- Timestamps and backups

### Integration Tests (14)
- Bootstrap → validate pipeline
- Parallel execution safety
- Agent prompt loading
- Artifact flow
- Concurrent write atomicity

### E2E Tests (15)
- Full validation from scratch
- Error recovery
- Bootstrap failure
- Agent timeout
- Missing dependencies
- Partial validation

## Implementation Checklist

### 1. bootstrap.sh (~100 lines)
- [ ] Detect OpenSSL (pkg-config)
- [ ] Install packages (apt/dnf)
- [ ] Build RustyClawd (cargo)
- [ ] Verify binary
- [ ] Generate artifacts
- [ ] Run: `./test_bootstrap.sh` (14 tests pass)

### 2. validate.sh (~150 lines)
- [ ] Phase 1: 5 parallel agents
- [ ] Phase 2: Synthesize results
- [ ] Phase 3: Generate test plan
- [ ] Phase 4: Execute tests
- [ ] Phase 5: Call synthesize_report.sh
- [ ] Run: `./test_validate_phases.sh` (18 tests pass)

### 3. synthesize_report.sh (~80 lines)
- [ ] Discover artifacts
- [ ] Read all markdown files
- [ ] Generate markdown report
- [ ] Order sections correctly
- [ ] Add table of contents
- [ ] Run: `./test_report_generation.sh` (17 tests pass)

### 4. agent_prompts/ (5 files)
- [ ] dependency_analysis.md
- [ ] config_analysis.md
- [ ] security_analysis.md
- [ ] integration_analysis.md
- [ ] resource_analysis.md
- [ ] Run: `./test_integration.sh` (14 tests pass)

### 5. Integration Complete
- [ ] Run: `./test_e2e.sh` (15 tests pass)
- [ ] Run: `./run_all_tests.sh` (78 tests pass)

## Common Commands

```bash
# Verify tests fail before implementation
./run_all_tests.sh
# Should see: 0 passed, 78 failed

# After implementing bootstrap.sh
./test_bootstrap.sh
# Should see: 14 passed, 0 failed

# After implementing all scripts
./run_all_tests.sh
# Should see: 78 passed, 0 failed
```

## Environment Variables

Tests respect these environment variables:
- `ARTIFACTS_DIR` - Where to write output
- `AGENT_PROMPTS_DIR` - Where to read prompts
- `PROJECT_ROOT` - RustyClawd root directory
- `AGENT_TIMEOUT` - Agent timeout in seconds

## Test Helpers

Available assertions in test_helpers.sh:
```bash
assert_success              # Exit code = 0
assert_failure              # Exit code != 0
assert_exit_code N          # Exit code = N
assert_output_contains "X"  # Output contains X
assert_file_exists PATH     # File exists
assert_file_contains PATH "X" # File contains X
assert_dir_exists PATH      # Directory exists
assert_equals A B           # A equals B
```

Available mocking:
```bash
mock_command "cargo" "echo 'Finished release' && exit 0"
clear_mocks
```

## Testing Pyramid

```
60% Unit Tests     - Fast, focused, isolated (49 tests)
30% Integration    - Multiple components (14 tests)
10% E2E Tests      - Complete workflows (15 tests)
```

## Quick Troubleshooting

**Tests fail with "No such file"**
- Expected! Implement the scripts first.

**Tests hang forever**
- Check for infinite loops in implementation
- Verify mocks are working

**Tests pass but shouldn't**
- Check assertion logic
- Verify mocks match real behavior

**Flaky tests (pass/fail randomly)**
- Check for race conditions
- Verify test isolation
- Check temp directory cleanup

## Documentation

- `README.md` - Complete test documentation
- `TEST_SUMMARY.md` - Test completion summary
- `TEST_STRUCTURE.md` - Visual test structure
- `QUICK_REFERENCE.md` - This file

## Success Metrics

- ✓ All 78 tests pass
- ✓ Tests run in < 30 seconds
- ✓ No flaky tests
- ✓ Clear error messages

## Current Status

**Phase**: Tests written, awaiting implementation
**Test Status**: All failing (expected - no implementation)
**Next Step**: Implement bootstrap.sh following TDD

---

**Last Updated**: 2025-12-01
**Test Count**: 78 tests across 5 test files
**Implementation Estimate**: ~330 lines + 5 prompt files
