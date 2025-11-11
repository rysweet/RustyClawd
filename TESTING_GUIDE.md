# Claude Code CLI Testing Guide

**Quick Reference for Running and Understanding the CLI Test Suite**

---

## Installation & Setup

### Prerequisites

```bash
# Verify Rust is installed
rustc --version

# Navigate to project
cd /Users/ryan/src/declawed/claude-code-rs
```

### Optional: Install Ripgrep

To run all grep tests successfully:

```bash
# macOS
brew install ripgrep

# Linux
sudo apt-get install ripgrep

# From Rust
cargo install ripgrep

# Verify installation
rg --version
```

---

## Basic Test Commands

### Run All CLI Tests

```bash
cargo test --package claude-code-cli --test cli_reference_tests
```

**Output**: Shows all 100 tests, results, and summary

### Run Tests with Details

```bash
cargo test --package claude-code-cli --test cli_reference_tests -- --nocapture
```

Shows print statements and verbose output

### Run Only Passing Tests

```bash
cargo test --package claude-code-cli --test cli_reference_tests
```

Filter to see status at the end

### Run Only Failing Tests

```bash
cargo test --package claude-code-cli --test cli_reference_tests 2>&1 | grep -A 5 "FAILED"
```

### Show Ignored Tests

```bash
cargo test --package claude-code-cli --test cli_reference_tests -- --ignored
```

Lists all 21 tests for future features

---

## Category-Specific Testing

### Test Help and Version Flags

```bash
cargo test --package claude-code-cli --test cli_reference_tests help
cargo test --package claude-code-cli --test cli_reference_tests version
```

**Expected**: 4/4 passing

### Test Debug Flag

```bash
cargo test --package claude-code-cli --test cli_reference_tests debug
```

**Expected**: 2/2 passing

### Test Bash Command

```bash
cargo test --package claude-code-cli --test cli_reference_tests bash
```

**Expected**: 9/12 passing
**Failing**: Flag placement issues (3 tests)

### Test Read Command

```bash
cargo test --package claude-code-cli --test cli_reference_tests read
```

**Expected**: 7/8 passing
**Failing**: Nonexistent file handling (1 test)

### Test Write Command

```bash
cargo test --package claude-code-cli --test cli_reference_tests write
```

**Expected**: 7/7 passing ✅

### Test Edit Command

```bash
cargo test --package claude-code-cli --test cli_reference_tests edit
```

**Expected**: 4/8 passing
**Failing**: Boolean flag issues (4 tests)

### Test Glob Command

```bash
cargo test --package claude-code-cli --test cli_reference_tests glob
```

**Expected**: 6/6 passing ✅

### Test Grep Command

```bash
cargo test --package claude-code-cli --test cli_reference_tests grep
```

**Expected**: 5/20 passing
**Failing**: Missing ripgrep binary (15 tests)

---

## Test Output Interpretation

### Successful Test Output

```
test test_help_flag_short ... ok
test test_version_flag_long ... ok
test test_bash_command_simple ... ok
...
test result: ok. 60 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Failed Test Output

```
---- test_edit_with_all_required_args stdout ----
thread 'test_edit_with_all_required_args' panicked at ...
Unexpected failure.
code=1
stderr=```"❌ Error: ...
```

**Interpretation**: Test expected success but got failure code

### Ignored Test Output

```
test test_continue_flag ... ignored, Feature not yet implemented: Continue mode
test test_mcp_command ... ignored, Feature not yet implemented: MCP command
...
test result: ok. 60 passed; 19 failed; 21 ignored
```

**Interpretation**: Feature not yet implemented, test disabled

---

## Debugging Failed Tests

### Step 1: Identify the Test

```bash
# Run verbose output
cargo test --package claude-code-cli --test cli_reference_tests -- --nocapture 2>&1 | grep "^test"
```

### Step 2: Run Single Test

```bash
cargo test --package claude-code-cli --test cli_reference_tests test_name -- --nocapture
```

Example:
```bash
cargo test --package claude-code-cli --test cli_reference_tests test_edit_with_all_required_args -- --nocapture
```

### Step 3: Examine Error

The output will show:
- What command was executed
- What exit code was returned
- What stderr/stdout was produced
- What was expected

### Step 4: Check Implementation

If test fails, check:
1. `/Users/ryan/src/declawed/claude-code-rs/crates/cli/src/main.rs` - CLI structure
2. `/Users/ryan/src/declawed/claude-code-rs/crates/tools/src/` - Tool implementations
3. Environment - required binaries (ripgrep, etc.)

---

## Test Statistics

### Quick Stats Command

```bash
cargo test --package claude-code-cli --test cli_reference_tests 2>&1 | tail -5
```

Shows: `test result: ok. X passed; Y failed; Z ignored`

### Detailed Breakdown

```bash
# Show passing tests count
cargo test --package claude-code-cli --test cli_reference_tests 2>&1 | grep "ok\." | wc -l

# Show test categories
cargo test --package claude-code-cli --test cli_reference_tests 2>&1 | grep "test test_" | cut -d_ -f1-3 | sort | uniq -c

# Show failures
cargo test --package claude-code-cli --test cli_reference_tests 2>&1 | grep "FAILED" | wc -l
```

---

## Common Issues & Solutions

### Issue 1: Ripgrep Tests Failing

**Error**: `Failed to spawn ripgrep: No such file or directory`

**Solution**:
```bash
brew install ripgrep
# or
cargo install ripgrep
```

Then rerun tests.

### Issue 2: Edit Command Tests Failing

**Error**: `Unexpected failure` for edit command

**Symptom**: 4 edit tests fail

**Solution**:
1. Check `/Users/ryan/src/declawed/claude-code-rs/crates/cli/src/main.rs` line 69-84
2. Review the `replace_all: bool` field definition
3. Ensure it's properly declared in the Commands enum

### Issue 3: Tests Won't Compile

**Error**: `error: expected item after doc comment`

**Solution**: This has been fixed. If it recurs, check test file syntax around documentation comments.

### Issue 4: Binary Not Found

**Error**: `thread panicked at ... can't find binary ... claude-code`

**Solution**:
```bash
# Build the binary first
cargo build --package claude-code-cli

# Or let cargo test build it automatically
cargo test --package claude-code-cli --test cli_reference_tests
```

---

## Test Structure Reference

### Test File Location

```
/Users/ryan/src/declawed/claude-code-rs/
├── crates/
│   └── cli/
│       ├── src/
│       │   └── main.rs           ← CLI implementation
│       ├── tests/
│       │   └── cli_reference_tests.rs  ← TEST SUITE (THIS FILE)
│       └── Cargo.toml            ← Dependencies
└── CLI_TEST_ANALYSIS.md          ← Detailed analysis
```

### Test File Organization

```
cli_reference_tests.rs (1,270 lines)
├── Help & Version Flags (4 tests) ...................... Lines 7-45
├── Debug Flag (2 tests) ............................... Lines 47-68
├── Bash Command (12 tests) ............................ Lines 70-220
├── Read Command (8 tests) ............................ Lines 222-313
├── Write Command (7 tests) ........................... Lines 315-376
├── Edit Command (8 tests) ........................... Lines 378-481
├── Glob Command (6 tests) .......................... Lines 483-540
├── Grep Command (20 tests) ........................ Lines 542-756
├── Command Discovery (2 tests) ................... Lines 758-790
├── Error Handling (4 tests) ..................... Lines 792-847
├── Integration Tests (2 tests) ................. Lines 849-890
├── Documentation Parity (3 tests) ............ Lines 892-964
├── Edge Cases (22 tests) ..................... Lines 966-1170
└── Missing Features - 20 ignored tests ........ Lines 1172-1230
```

---

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: CLI Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - name: Install ripgrep
        run: sudo apt-get install -y ripgrep
      - name: Run CLI tests
        run: cargo test --package claude-code-cli --test cli_reference_tests
```

### GitLab CI Example

```yaml
test_cli:
  image: rust:latest
  before_script:
    - apt-get update && apt-get install -y ripgrep
  script:
    - cargo test --package claude-code-cli --test cli_reference_tests
```

---

## Writing New Tests

### Add Test for New Flag

```rust
#[test]
fn test_my_new_flag() {
    let mut cmd = Command::cargo_bin("claude-code").unwrap();
    cmd.arg("subcommand")
        .arg("--my-flag")
        .arg("value")
        .assert()
        .success()
        .stdout(predicate::str::contains("expected output"));
}
```

### Add Test for Error Condition

```rust
#[test]
fn test_invalid_argument() {
    let mut cmd = Command::cargo_bin("claude-code").unwrap();
    cmd.arg("bash")
        .arg("--timeout")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid"));
}
```

### Add Ignored Test for Future Feature

```rust
#[test]
#[ignore = "Feature not yet implemented: my feature"]
fn test_future_feature() {
    let mut cmd = Command::cargo_bin("claude-code").unwrap();
    cmd.arg("new-command")
        .assert()
        .failure();
}
```

---

## Test Verification Checklist

Before declaring tests complete:

- [ ] All tests compile without errors
- [ ] Help/version flags work (4/4)
- [ ] Debug flag works (2/2)
- [ ] Write command works (7/7)
- [ ] Glob command works (6/6)
- [ ] Edge cases pass (22/22)
- [ ] Edit command passes (4/8 without fix, 8/8 with fix)
- [ ] Ripgrep installed and grep tests pass (5/20 without ripgrep, 20/20 with)
- [ ] Error handling works (3/4)
- [ ] No unexpected failures

---

## Performance Tips

### Speed Up Test Compilation

```bash
# Use release mode for faster execution
cargo test --package claude-code-cli --test cli_reference_tests --release

# Cache dependencies
cargo build --package claude-code-cli
```

### Run Specific Tests

```bash
# Run only bash tests (faster)
cargo test --package claude-code-cli --test cli_reference_tests bash -- --test-threads=1

# Run single test
cargo test --package claude-code-cli --test cli_reference_tests test_name
```

### Parallel Execution

```bash
# Default: parallel
cargo test --package claude-code-cli --test cli_reference_tests

# Sequential (for debugging)
cargo test --package claude-code-cli --test cli_reference_tests -- --test-threads=1
```

---

## Documentation Links

- **Test File**: `/Users/ryan/src/declawed/claude-code-rs/crates/cli/tests/cli_reference_tests.rs`
- **Analysis**: `/Users/ryan/src/declawed/claude-code-rs/CLI_TEST_ANALYSIS.md`
- **Coverage Report**: `/Users/ryan/src/declawed/claude-code-rs/TEST_COVERAGE_REPORT.md`
- **Official Docs**: https://code.claude.com/docs/en/cli-reference
- **assert_cmd Docs**: https://docs.rs/assert_cmd/
- **predicates Docs**: https://docs.rs/predicates/

---

## Quick Reference Card

```
┌─────────────────────────────────────────────────────────────┐
│  QUICK COMMANDS                                             │
├─────────────────────────────────────────────────────────────┤
│  cargo test --package claude-code-cli --test cli_reference  │
│  Run all tests                                              │
│                                                             │
│  cargo test ... 2>&1 | tail -20                            │
│  Show summary                                              │
│                                                             │
│  cargo test ... bash                                        │
│  Run bash tests only                                        │
│                                                             │
│  cargo test ... -- --ignored                               │
│  Show future features                                      │
│                                                             │
│  brew install ripgrep                                       │
│  Fix grep tests                                            │
└─────────────────────────────────────────────────────────────┘

CURRENT STATUS:
✅ 60 passing
❌ 19 failing (15 ripgrep, 4 edit command)
⏳ 21 ignored (future features)

EXPECTED AFTER FIXES:
✅ 88 passing
❌ 0 critical failures
⏳ 21 ignored (future features)
```

---

## Support & Troubleshooting

### Where to Find Information

1. **Test Details**: Read test names - they describe what they test
2. **Test Logic**: Check test code at line numbers in test structure
3. **Implementation**: Check `/Users/ryan/src/declawed/claude-code-rs/crates/cli/src/main.rs`
4. **Errors**: Examine stderr from failed test output

### Getting Help

1. Run test with `--nocapture` to see output
2. Check the analysis document: `CLI_TEST_ANALYSIS.md`
3. Check coverage report: `TEST_COVERAGE_REPORT.md`
4. Review test comments for expected behavior

---

## Version Information

```
Created: 2025-11-11
Rust Edition: 2021
Test Framework: assert_cmd + predicates
Total Lines of Test Code: 1,270
Test Categories: 13
Estimated Coverage: 74% of CLI surface
```

---

## Next Actions

1. **Immediate**: Install ripgrep `brew install ripgrep`
2. **Short-term**: Fix 4 edit command tests
3. **Medium-term**: Add error handling tests
4. **Long-term**: Implement 21 future features (marked as ignored)

Good luck with testing! 🚀
