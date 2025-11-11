# Verification Results Interpretation Guide

**Purpose**: Understand what test results mean and how to act on them

---

## Perfect Run Example

```
════════════════════════════════════════════════════════════════
METHOD 1: TOOL SIGNATURE VALIDATION
════════════════════════════════════════════════════════════════
Testing: Tool signature existence
✓ Bash tool responds with JSON
✓ Bash captures stdout correctly
✓ Read tool returns data field
✓ Read captures file content
✓ Write creates file
✓ Write stores content correctly
✓ Edit modifies content correctly
✓ Glob returns files array
✓ Glob finds matching files
✓ Bash rejects empty command
✓ Read rejects nonexistent file
...
============================================================
RESULTS: METHOD 1
============================================================
PASSED: 10
FAILED: 0

✓ All tool signatures validated successfully
```

**Interpretation**: All tools exist and accept correct parameter types. No issues found.

---

## Partial Failures Example

```
════════════════════════════════════════════════════════════════
METHOD 2: BEHAVIORAL EQUIVALENCE TESTING
════════════════════════════════════════════════════════════════
Testing: Glob tool equivalence
✓ Glob: Finds matching files
✓ Glob: Nested pattern finds more files
✓ Glob: Empty result for no matches
✗ Glob: Returns consistent ordering
  Expected: Same order on repeated runs
  Actual: Files in different order

Testing: Bash tool equivalence
✓ Bash: Captures stdout
✗ Bash: Captures stderr
  Expected: stderr field contains error output
  Actual: stderr field empty
...
============================================================
RESULTS: METHOD 2
============================================================
PASSED: 18
FAILED: 2
```

**Interpretation**:
- Glob files are not consistently ordered (may need sorting)
- Bash stderr capture not working (need to verify stderr redirection)

**Action**:
1. Check glob implementation for sorting logic
2. Review bash tool stderr handling
3. File issues for fixing

---

## Critical Failure Example

```
════════════════════════════════════════════════════════════════
METHOD 3: CLI INTERFACE PARITY TESTING
════════════════════════════════════════════════════════════════
Testing: Subcommand existence
✓ Subcommand: bash exists
✓ Subcommand: read exists
✓ Subcommand: write exists
✓ Subcommand: edit exists
✓ Subcommand: glob exists
✗ Subcommand: grep exists
  Error: error: unknown subcommand 'grep'
...
============================================================
RESULTS: METHOD 3
============================================================
PASSED: 24
FAILED: 1
```

**Interpretation**:
- Grep tool is not registered as a CLI subcommand
- This is critical - tool exists but isn't exposed

**Action**:
1. Check CLI parser in `crates/cli/src/main.rs`
2. Ensure grep is added to clap subcommands
3. Rebuild and test

---

## Error Handling Issue Example

```
════════════════════════════════════════════════════════════════
METHOD 4: ERROR HANDLING ALIGNMENT TESTING
════════════════════════════════════════════════════════════════
Testing: Missing required arguments
✓ Bash: missing command
✓ Read: missing file path
✓ Write: missing content
Testing: Invalid parameter type handling
✓ Read: negative offset
✗ Read: non-numeric offset
  Expected: error or Error in output
  Actual: Accepted 'abc' as offset value
...
============================================================
RESULTS: METHOD 4
============================================================
PASSED: 16
FAILED: 2
```

**Interpretation**:
- Parameter validation is incomplete
- Non-numeric values are accepted where numbers expected
- This could cause silent failures or crashes

**Action**:
1. Check offset parameter parsing in read tool
2. Add type validation before use
3. Return clear error if parse fails

---

## Workflow Integration Issue Example

```
════════════════════════════════════════════════════════════════
METHOD 5: INTEGRATION WORKFLOW TESTING
════════════════════════════════════════════════════════════════
Workflow 1: Create file -> Read -> Verify content
✓ W1: Write file
✓ W1: Read contains function signature
✓ W1: Read captures all lines
Workflow 2: Create file -> Edit -> Verify
✓ W2: Write initial content
✓ W2: Edit first value
✓ W2: Edit second value
✓ W2: Verify first edit persisted
✗ W2: Verify second edit persisted
  Expected: "multiplier = 3" in file
  Actual: "multiplier = 2" (original value)
Workflow 3: Multi-file-create-glob-grep
✗ W3: Glob finds 3 Rust files
  Expected: 3 files
  Actual: 1 file (only first created file visible)
...
============================================================
RESULTS: METHOD 5
============================================================
PASSED: 25
FAILED: 10
```

**Interpretation**:
- Multiple edits don't persist (second edit lost)
- File creation or globbing has timing/visibility issue
- Suggests atomicity or flush problems

**Action**:
1. Check if edits are flushing to disk
2. Verify write synchronization
3. Check if glob waits for file updates

---

## Scoring Interpretation

### Test Results Scoring

```
108/108 (100%)  - Perfect: Drop-in ready
105/108 (97%)   - Excellent: Minor edge cases only
100/108 (93%)   - Good: Core functionality working
95/108 (88%)    - Acceptable: Gaps documented
85/108 (79%)    - Concerning: Significant gaps
70/108 (65%)    - Poor: Major work needed
<70/108 (<65%)  - Critical: Not production ready
```

### Pass Rate by Severity

**Critical (Must Fix)**:
- Any Method 1 failures (tools don't exist)
- Any Method 3 failures (CLI broken)
- >5 Method 5 failures (workflows broken)

**Important (Should Fix)**:
- Method 2 failures (wrong results)
- Method 4 failures (error handling broken)

**Minor (Nice to Fix)**:
- Edge case failures
- Performance issues
- Cosmetic output differences

---

## Quick Reference: Failure → Action

### "Method 1 FAILED"
```
Issue: Tool infrastructure problem
Action:
1. Check which tool failed: cargo run -- <tool> --help
2. Verify tool is implemented: ls crates/tools/src/<tool>.rs
3. Check if exported in lib.rs
4. Rebuild: cargo build --release
```

### "Method 2 FAILED"
```
Issue: Tool behavior doesn't match Claude Code
Action:
1. Identify failing test case
2. Run manually: cargo run -- <tool> <args>
3. Compare output to expected
4. Check test: grep "✗" /tmp/m2_results.log
5. Debug implementation logic
```

### "Method 3 FAILED"
```
Issue: CLI interface problem
Action:
1. Test subcommand: cargo run -- <subcommand> --help
2. Check clap parser: grep -r "<subcommand>" crates/cli/src/
3. Verify args are defined
4. Run help: cargo run -- --help
```

### "Method 4 FAILED"
```
Issue: Error handling incomplete
Action:
1. Test error case manually: cargo run -- <args_that_fail>
2. Check error message: Review stdout and stderr
3. Verify error type: Look for error! macro in code
4. Add validation if missing: Implement error check
```

### "Method 5 FAILED"
```
Issue: Multi-step workflows broken
Action:
1. Identify failing workflow
2. Run each step individually
3. Check which step fails
4. Debug state between operations
5. Verify file I/O and persistence
```

---

## Log File Analysis

Logs are saved in `/tmp/m[1-5]_results.log`

### Extracting Specific Failures

```bash
# See only failures
grep "✗" /tmp/m2_results.log

# Count failures by type
grep "✗" /tmp/m2_results.log | wc -l

# Get failure details
grep -A 2 "✗" /tmp/m2_results.log

# Find which test failed
grep "✗" /tmp/m3_results.log | grep -i "flag"

# Count passes
grep "✓" /tmp/m1_results.log | wc -l
```

### Creating Failure Report

```bash
cat > /Users/ryan/src/declawed/claude-code-rs/FAILURE_ANALYSIS.md << 'EOF'
# Verification Failure Analysis

## Summary
- Total Failed: 5 tests
- Pass Rate: 103/108 (95.4%)
- Severity: Low to Medium

## Failing Tests

### Method 2: Behavioral Equivalence
1. **Glob: Returns consistent ordering**
   - Issue: Results in different order on repeated runs
   - Impact: Non-deterministic behavior
   - Fix: Sort results before returning

### Method 4: Error Alignment
1. **Read: non-numeric offset**
   - Issue: Accepts "abc" as offset instead of erroring
   - Impact: Silent failures or crashes
   - Fix: Add try_parse() check on offset parameter

## Recommended Priority
1. Fix non-numeric offset (high priority - breaks tool)
2. Fix glob ordering (medium - non-deterministic)

## Next Steps
1. Implement fixes
2. Re-run verification suite
3. Update documentation
EOF
```

---

## Expected vs Actual Analysis

When a test fails, understand the gap:

### Example 1: Offset Parameter
```
Test: Read: non-numeric offset

Expected Behavior:
- Input: cargo run -- read /file.txt --offset abc
- Output: Error message with "invalid", "numeric", "must be"
- Exit Code: Non-zero

Actual Behavior:
- Input: cargo run -- read /file.txt --offset abc
- Output: (partial file content)
- Exit Code: 0

Gap Analysis:
- Parameter is not validated as numeric type
- Error handling path not triggered
- Tool accepts invalid input silently

Root Cause:
- offset is parsed as String, not u32
- No try_parse() check before use

Fix:
- Change offset: String to offset: u32
- Let serde/clap handle conversion
- Or: Add explicit parse check with ? operator
```

### Example 2: Glob Ordering
```
Test: Glob: Returns consistent ordering

Expected Behavior:
- Input: cargo run -- glob "*.txt" --path /tmp
- Output 1: ["file1.txt", "file2.txt", "file3.txt"]
- Output 2: ["file1.txt", "file2.txt", "file3.txt"]
- Outputs are identical

Actual Behavior:
- Input: cargo run -- glob "*.txt" --path /tmp
- Output 1: ["file1.txt", "file2.txt", "file3.txt"]
- Output 2: ["file3.txt", "file1.txt", "file2.txt"]
- Outputs differ

Gap Analysis:
- Results are not sorted
- HashMap iteration order is random
- Results depend on filesystem ordering

Root Cause:
- Using HashMap.values() without sort
- Or: results collected from BTreeMap which has insertion order

Fix:
- Sort results before returning:
  results.sort();
  results.sort_by_key(|f| f.mtime());  // If mtime sorting needed
```

---

## Priority-Based Fix Strategy

### Phase 1: Critical (Hours 1-2)
```
✗ Subcommand doesn't exist
✗ Tool crashes on execution
✗ Broken CLI parsing
✗ Wrong output format (not JSON)

Action: Fix immediately - tool unusable
```

### Phase 2: Important (Hours 2-4)
```
✗ Invalid parameters accepted
✗ Error handling missing
✗ Wrong results returned
✗ Workflows don't complete

Action: Fix before production use
```

### Phase 3: Nice-to-Have (Hours 4+)
```
✗ Inconsistent ordering
✗ Edge case handling
✗ Performance issues
✗ Cosmetic output differences

Action: Fix for completeness, not blocking
```

---

## Regression Testing

After fixing failures:

```bash
# Run all methods again
/Users/ryan/src/declawed/claude-code-rs/RUN_VERIFICATION.sh

# Compare results
diff /tmp/m2_results_old.log /tmp/m2_results.log

# Verify specific fix
/Users/ryan/src/declawed/claude-code-rs/tests/method2_behavioral_equivalence.sh | grep "Glob:"

# Run subset of tests
cargo run -- bash "test"  # Verify bash works
cargo run -- read /tmp/test.txt --offset 5  # Verify fixed offset
```

---

## Documentation Update Process

When fixing failures:

1. **Record the issue**:
   ```bash
   # In code comment
   // TODO: Fix glob sorting (Issue: non-deterministic output)
   ```

2. **Document the fix**:
   ```bash
   # In git commit
   git commit -m "Fix glob output ordering for deterministic results

   - Sorted results by filename before returning
   - Fixes Method 2 test failure
   - Ensures consistent ordering across runs"
   ```

3. **Update verification report**:
   ```bash
   echo "- FIXED: Glob returns consistent ordering" >> VERIFICATION_FIXES.log
   ```

4. **Re-run tests**:
   ```bash
   /Users/ryan/src/declawed/claude-code-rs/tests/method2_behavioral_equivalence.sh
   ```

---

## Final Verification Checklist

After all tests pass:

```bash
cat > /Users/ryan/src/declawed/claude-code-rs/VERIFICATION_CHECKLIST.md << 'EOF'
# Final Verification Checklist

## Tests Passed
- [x] Method 1: Tool Signature Validation (10/10)
- [x] Method 2: Behavioral Equivalence Testing (20/20)
- [x] Method 3: CLI Interface Parity (25/25)
- [x] Method 4: Error Handling Alignment (18/18)
- [x] Method 5: Integration Workflow Testing (35/35)

## Coverage Summary
- [x] All 6 core tools verified
- [x] All CLI flags working
- [x] Error handling complete
- [x] Real-world workflows tested
- [x] Edge cases handled

## Quality Metrics
- [x] 108/108 tests passing (100%)
- [x] No critical failures
- [x] All core functionality working
- [x] Production ready

## Sign-Off
- Date: [DATE]
- Verified by: [NAME]
- Recommendation: APPROVED as drop-in replacement
EOF
```

---

## Support Resources

**When stuck**:
1. Review `/Users/ryan/src/declawed/claude-code-rs/VERIFICATION_METHODS.md` (detailed spec)
2. Check `/Users/ryan/src/declawed/claude-code-rs/VERIFICATION_QUICK_START.md` (quick reference)
3. Review test code: `grep -A 5 "✗ Test name" /tmp/m_results.log`
4. Debug manually: `cargo run -- <tool> <args>`

**Common Issues**:
- Permissions: `chmod +x *.sh`
- Paths: Use absolute paths, not relative
- JSON: Install jq with `brew install jq`
- Rust: Update with `rustup update`

---

## Conclusion

Test failures are expected - they indicate what needs work. Use this guide to:
1. Understand what failed
2. Find the root cause
3. Implement the fix
4. Verify the fix works

When all tests pass: **RustyClawd is verified as a drop-in replacement!**
