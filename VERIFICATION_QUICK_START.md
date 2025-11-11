# RustyClawd Verification - Quick Start Guide

## Overview

5 executable test methods to validate RustyClawd is a drop-in replacement for Claude Code.

**Total runtime**: ~5-10 minutes
**Pass rate target**: 100% (or document specific gaps)

---

## Method 1: Tool Signature Validation

**What it tests**: All tools exist and accept correct parameter types

**Run it**:
```bash
chmod +x /Users/ryan/src/declawed/claude-code-rs/tests/method1_tool_signatures.sh
/Users/ryan/src/declawed/claude-code-rs/tests/method1_tool_signatures.sh
```

**Pass criteria**:
- ✅ All 6 tools (bash, read, write, edit, glob, grep) respond with JSON
- ✅ Parameter types are validated (no string where number expected)
- ✅ Required parameters are enforced
- ✅ Optional parameters work together

**Example output**:
```
✓ Bash tool responds with JSON
✓ Bash captures stdout correctly
✓ Read tool returns data field
✓ Write creates file
...
PASSED: 10
FAILED: 0
```

---

## Method 2: Behavioral Equivalence Testing

**What it tests**: Results match Claude Code for identical operations

**Run it**:
```bash
chmod +x /Users/ryan/src/declawed/claude-code-rs/tests/method2_behavioral_equivalence.sh
/Users/ryan/src/declawed/claude-code-rs/tests/method2_behavioral_equivalence.sh
```

**Pass criteria**:
- ✅ Read returns same lines in same order
- ✅ Write creates/overwrites atomically
- ✅ Edit modifies exact strings
- ✅ Glob matches all files correctly
- ✅ Bash captures output and exit codes
- ✅ Grep finds all matches

**Example output**:
```
Testing: Read tool equivalence
✓ Read: Returns all 5 lines
✓ Read: First line correct
✓ Read: Last line correct
✓ Read: Offset skips correctly
...
PASSED: 20
FAILED: 0
```

---

## Method 3: CLI Interface Parity

**What it tests**: Command-line interface matches exactly

**Run it**:
```bash
chmod +x /Users/ryan/src/declawed/claude-code-rs/tests/method3_cli_parity.sh
/Users/ryan/src/declawed/claude-code-rs/tests/method3_cli_parity.sh
```

**Pass criteria**:
- ✅ All subcommands exist (bash, read, write, edit, glob, grep)
- ✅ All documented flags work (-h, --help, -V, --version)
- ✅ Positional arguments work
- ✅ Flag combinations work
- ✅ Invalid arguments are rejected

**Example output**:
```
Testing: Subcommand existence
✓ Subcommand: bash exists
✓ Subcommand: read exists
✓ Subcommand: write exists
✓ Flag: --help shows subcommands
✓ Bash flag: --timeout
...
PASSED: 25
FAILED: 0
```

---

## Method 4: Error Handling Alignment

**What it tests**: Error messages and formats are consistent

**Run it**:
```bash
chmod +x /Users/ryan/src/declawed/claude-code-rs/tests/method4_error_alignment.sh
/Users/ryan/src/declawed/claude-code-rs/tests/method4_error_alignment.sh
```

**Pass criteria**:
- ✅ All errors include `type: "error"` and `message` fields
- ✅ Missing arguments are detected
- ✅ Type mismatches are caught (e.g., string for number)
- ✅ File not found errors are clear
- ✅ Exit codes are non-zero for errors
- ✅ Tool recovers after errors

**Example output**:
```
Testing: Missing required arguments
✓ Bash: missing command
✓ Read: missing file path
✓ Write: missing content
✓ Edit: missing old-string
Testing: Invalid parameter type handling
✓ Read: negative offset
✓ Read: non-numeric offset
...
PASSED: 18
FAILED: 0
```

---

## Method 5: Integration Workflow Testing

**What it tests**: Multi-step workflows work seamlessly (the ultimate test)

**Run it**:
```bash
chmod +x /Users/ryan/src/declawed/claude-code-rs/tests/method5_integration_workflows.sh
/Users/ryan/src/declawed/claude-code-rs/tests/method5_integration_workflows.sh
```

**Pass criteria**:
- ✅ Multi-step workflows complete without data loss
- ✅ File operations are atomic and consistent
- ✅ Modifications persist across operations
- ✅ Search results are accurate in complex projects
- ✅ JSON output is valid throughout
- ✅ Errors don't break subsequent operations

**Example workflows tested**:
1. Create → Read → Verify
2. Create → Edit → Verify
3. Create multiple files → Find with glob → Search with grep
4. Execute bash command chains
5. Edit complex JSON → Verify syntax
6. Multi-step file processing
7. Error recovery
8. Nested directory handling
9. Output format consistency
10. Data integrity with special characters

**Example output**:
```
Workflow 1: Create file -> Read -> Verify content
✓ W1: Write file
✓ W1: Read contains function signature
✓ W1: Read captures all lines
Workflow 2: Create file -> Edit -> Verify
✓ W2: Write initial content
✓ W2: Edit first value
...
PASSED: 35
FAILED: 0
```

---

## Run All Tests at Once

```bash
chmod +x /Users/ryan/src/declawed/claude-code-rs/RUN_VERIFICATION.sh
/Users/ryan/src/declawed/claude-code-rs/RUN_VERIFICATION.sh
```

This will:
1. Run all 5 methods sequentially
2. Display pass/fail for each
3. Save logs to `/tmp/m1_results.log` through `/tmp/m5_results.log`
4. Provide overall verdict

**Expected output**:
```
════════════════════════════════════════════════════════════════
FINAL VERIFICATION RESULTS
════════════════════════════════════════════════════════════════

✓ M1: Tool Signature Validation
✓ M2: Behavioral Equivalence Testing
✓ M3: CLI Interface Parity
✓ M4: Error Handling Alignment
✓ M5: Integration Workflow Testing

════════════════════════════════════════════════════════════════
✓ ALL VERIFICATIONS PASSED
════════════════════════════════════════════════════════════════

RustyClawd is verified as a drop-in replacement for Claude Code.
```

---

## Understanding Results

### Perfect Results (All Pass)
```
PASSED: 108
FAILED: 0
```
→ RustyClawd is production-ready as a drop-in replacement.

### Acceptable (Minor Gaps)
```
PASSED: 95
FAILED: 13
```
→ Review which specific tests failed. Some gaps may be:
- Features not yet implemented (optional tools)
- Warnings instead of errors (recoverable)
- Formatting differences (non-critical)

### Concerning (Major Gaps)
```
PASSED: 70
FAILED: 38
```
→ Significant compatibility issues. Prioritize:
1. Core tools (bash, read, write, edit, glob, grep)
2. Error handling
3. Output formats

---

## Interpreting Individual Failures

### If Method 1 fails
**Issue**: Tool doesn't exist or doesn't accept correct parameters
**Action**: Check tool implementation in `crates/tools/src/`

### If Method 2 fails
**Issue**: Output differs from Claude Code
**Action**: Compare expected vs actual JSON structure

### If Method 3 fails
**Issue**: CLI flag or subcommand missing
**Action**: Add to clap parser in `crates/cli/src/`

### If Method 4 fails
**Issue**: Error format or recovery is wrong
**Action**: Check error types and messaging in tool implementations

### If Method 5 fails
**Issue**: Workflows break in realistic scenarios
**Action**: Debug multi-step operations, check for state issues

---

## Troubleshooting

### Permission Denied
```bash
chmod +x /Users/ryan/src/declawed/claude-code-rs/tests/method*.sh
chmod +x /Users/ryan/src/declawed/claude-code-rs/RUN_VERIFICATION.sh
```

### Build Fails
```bash
cd /Users/ryan/src/declawed/claude-code-rs
cargo clean
cargo build --release
```

### No Such File
```bash
# Tests create temp files in /tmp
# Ensure /tmp exists and is writable
ls -la /tmp | head
chmod 755 /tmp
```

### Tests Hang
```bash
# Kill hanging processes
pkill -f "cargo run"
# Or use timeout wrapper
timeout 30 cargo run -- bash "echo test"
```

---

## Expected Test Times

| Method | Time | Count |
|--------|------|-------|
| 1. Signatures | 30-45s | 10 tests |
| 2. Equivalence | 60-90s | 20+ tests |
| 3. CLI Parity | 45-60s | 25+ tests |
| 4. Error Handling | 40-60s | 18+ tests |
| 5. Integration | 90-120s | 35+ tests |
| **Total** | **~5-10 min** | **108+ tests** |

---

## Documenting Gaps

If tests fail, create a gap report:

```bash
cat > /Users/ryan/src/declawed/claude-code-rs/VERIFICATION_GAPS.md << 'EOF'
# RustyClawd Verification Gaps Report

## Summary
- Date: $(date)
- Method 1: PASS (10/10)
- Method 2: FAIL (18/20) - Missing 2 tests
- Method 3: PASS (25/25)
- Method 4: PASS (18/18)
- Method 5: PASS (35/35)
- **Overall: 106/108 (98.1%)**

## Specific Failures
### Method 2 - Behavioral Equivalence
1. **Glob ordering** - Results are not sorted by mtime consistently
   - Expected: Sorted by modification time
   - Actual: Alphabetical order
   - Fix: Sort before returning

2. **Grep context** - Context flags (-B, -A) not working
   - Expected: Lines before/after match
   - Actual: Only matching lines returned
   - Fix: Implement context line capture

## Action Items
- [ ] Fix glob sorting to match mtime
- [ ] Implement grep context flags
- [ ] Re-run full verification
EOF
```

---

## Next Steps

1. **Run all tests**: `RUN_VERIFICATION.sh`
2. **Review results**: Check logs in `/tmp/m*_results.log`
3. **Document gaps**: Create `VERIFICATION_GAPS.md` if needed
4. **Fix issues**: Update tool implementations
5. **Re-verify**: Run tests again until 100% pass

---

## Files Included

- **VERIFICATION_METHODS.md** - Detailed 5-method specification (this guide)
- **RUN_VERIFICATION.sh** - Master test runner
- **tests/method1_tool_signatures.sh** - Tool signature validation
- **tests/method2_behavioral_equivalence.sh** - Behavior testing
- **tests/method3_cli_parity.sh** - CLI testing
- **tests/method4_error_alignment.sh** - Error testing
- **tests/method5_integration_workflows.sh** - Integration testing

---

## Success Criteria

✓ **Drop-in Replacement Verified** when:
- All 5 methods pass
- No critical gaps in core tools
- Output formats match
- Error handling is consistent
- Integration workflows work

Ready to use RustyClawd as a production replacement for Claude Code!
