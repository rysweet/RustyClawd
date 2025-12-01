# RustyClawd Validation Test Suite

Comprehensive test suite for the RustyClawd validation system following Test-Driven Development (TDD) methodology.

## Philosophy

These tests follow the **Testing Pyramid**:
- **60% Unit Tests** - Fast, focused tests of individual scripts
- **30% Integration Tests** - Multiple components working together
- **10% E2E Tests** - Complete workflows from start to finish

All tests are written to **FAIL** initially since the implementation doesn't exist yet. They will pass once the scripts are properly implemented.

## Test Files

### Unit Tests (60%)

1. **test_bootstrap.sh** - Tests for `bootstrap.sh`
   - OpenSSL dependency detection
   - Package installation (apt/dnf)
   - RustyClawd build verification
   - Error handling and user guidance
   - 14 test cases

2. **test_validate_phases.sh** - Tests for `validate.sh` phases
   - Phase 1: 5 parallel workstreams
   - Phase 2: Synthesis and dependencies
   - Phase 3: Test plan generation
   - Phase 4: Test execution
   - Phase 5: Report synthesis
   - 18 test cases

3. **test_report_generation.sh** - Tests for `synthesize_report.sh`
   - Artifact discovery and reading
   - Markdown structure generation
   - Section ordering and formatting
   - Missing artifact handling
   - 17 test cases

### Integration Tests (30%)

4. **test_integration.sh** - Multi-component integration
   - Bootstrap → validate pipeline
   - Parallel execution safety
   - Agent invocation and prompts
   - Artifact flow between phases
   - 14 test cases

### E2E Tests (10%)

5. **test_e2e.sh** - Complete workflow validation
   - Full validation from scratch
   - Error recovery scenarios
   - Real-world usage patterns
   - 15 test cases

### Test Infrastructure

6. **test_helpers.sh** - Shared test utilities
   - Assertion functions (assert_success, assert_failure, etc.)
   - Command mocking support
   - Test environment setup/teardown
   - Color-coded output

## Running Tests

### Run All Tests
```bash
cd /home/azureuser/src/RustyClawd/.claude/scenarios/rustyclawd-validation/tests
./run_all_tests.sh
```

### Run Specific Test Category
```bash
./run_all_tests.sh unit          # Unit tests only
./run_all_tests.sh integration   # Integration tests only
./run_all_tests.sh e2e          # E2E tests only
```

### Run Individual Test File
```bash
./test_bootstrap.sh              # Run bootstrap tests
./test_validate_phases.sh        # Run validation phase tests
./test_report_generation.sh      # Run report generation tests
./test_integration.sh            # Run integration tests
./test_e2e.sh                   # Run E2E tests
```

## Expected Behavior

### Before Implementation (NOW)
All tests should **FAIL** because:
- `bootstrap.sh` doesn't exist
- `validate.sh` doesn't exist
- `synthesize_report.sh` doesn't exist
- Agent prompts directory doesn't exist

### After Implementation
Tests should **PASS** when:
- Scripts are implemented correctly
- All required functionality is present
- Error handling works as expected
- Artifacts are generated properly

## Test Coverage Statistics

- **Total Test Cases**: ~78 tests
- **Unit Tests**: 49 tests (63%)
- **Integration Tests**: 14 tests (18%)
- **E2E Tests**: 15 tests (19%)

Distribution aligns with testing pyramid principles (60/30/10 target).

## Test Features

### Mocking
Tests use command mocking to simulate:
- `pkg-config` (OpenSSL detection)
- `cargo` (Rust build)
- `claude` (AI agent invocation)
- `apt-get`, `dnf` (package managers)

### Assertions
Comprehensive assertion functions:
- `assert_success` / `assert_failure` - Exit code validation
- `assert_output_contains` / `assert_output_not_contains` - Output validation
- `assert_file_exists` / `assert_file_contains` - File validation
- `assert_equals` - Value comparison

### Test Isolation
Each test runs in isolation:
- Temporary directories (`$TEST_TMPDIR`)
- Clean environment per test
- No test interdependencies

## Implementation Guidelines

When implementing the scripts, ensure:

1. **Script locations match test expectations**:
   - `.claude/scenarios/rustyclawd-validation/bootstrap.sh`
   - `.claude/scenarios/rustyclawd-validation/validate.sh`
   - `.claude/scenarios/rustyclawd-validation/synthesize_report.sh`

2. **Environment variables are respected**:
   - `ARTIFACTS_DIR` - Where to write output
   - `AGENT_PROMPTS_DIR` - Where to read prompts
   - `PROJECT_ROOT` - RustyClawd root directory

3. **Command-line flags work**:
   - `--check-only`, `--build`, `--install` (bootstrap.sh)
   - `--phase N`, `--all`, `--dry-run` (validate.sh)

4. **Error handling provides helpful messages**:
   - Missing dependencies → installation instructions
   - Failed agents → continue with remaining work
   - Missing files → clear error messages

5. **Parallel execution is safe**:
   - No race conditions in artifact writes
   - Each workstream writes to unique file
   - Atomic file operations

## Test-Driven Development Workflow

1. **Run tests** (they fail - expected!)
   ```bash
   ./run_all_tests.sh
   ```

2. **Implement minimal script** to make first test pass
   ```bash
   # Create bootstrap.sh with basic structure
   ```

3. **Run tests again** (more tests pass)
   ```bash
   ./test_bootstrap.sh
   ```

4. **Iterate** until all tests pass

5. **Refactor** with confidence (tests prevent regressions)

## Debugging Failed Tests

When tests fail, check:
1. Test output shows which assertion failed
2. `$TEST_OUTPUT` contains command output
3. `$TEST_TMPDIR` artifacts for inspection
4. Mock commands are being called correctly

Example debug pattern:
```bash
# Add to failing test:
echo "DEBUG: Output was: $TEST_OUTPUT"
echo "DEBUG: Files created:"
ls -la "$TEST_TMPDIR/artifacts"
```

## Contributing

When adding new functionality:
1. Write tests first (TDD!)
2. Ensure tests fail before implementation
3. Implement feature
4. Verify tests pass
5. Add to appropriate test file based on pyramid level

## Test Philosophy Alignment

These tests embody amplihack philosophy:
- **Ruthless Simplicity**: Pure bash, no external test frameworks
- **Zero-BS**: Tests verify real behavior, no stubs
- **Modular Design**: Each test file is self-contained
- **Fast Execution**: All tests run in < 30 seconds

## Success Metrics

Tests are successful when:
- ✓ All tests pass after implementation
- ✓ Tests run in < 30 seconds total
- ✓ Clear failure messages when things break
- ✓ No flaky tests (consistent results)
- ✓ Easy to add new tests

---

**Current Status**: Tests written, awaiting implementation. All tests should fail until scripts are built.

**Next Steps**: Implement `bootstrap.sh`, `validate.sh`, and `synthesize_report.sh` following TDD methodology.
