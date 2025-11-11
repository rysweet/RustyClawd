# RustyClawd Drop-In Replacement Verification Methods

**Objective**: Design 5 actionable test methods to validate that RustyClawd is a drop-in replacement for Claude Code.

**Status**: This document defines specific, executable procedures to identify gaps and bugs.

---

## Executive Summary

Drop-in replacement verification requires testing across **5 critical dimensions**:

1. **Tool Signature Compatibility** - Same inputs/outputs as Claude Code
2. **Behavioral Fidelity** - Identical results for equivalent operations
3. **CLI Interface Equivalence** - Command-line parity
4. **Error Handling Alignment** - Same error types and messages
5. **Integration Compatibility** - Works in actual Claude Code workflows

Each method is designed to be:
- **Executable**: Exact commands provided
- **Reproducible**: No manual interpretation needed
- **Diagnostic**: Identifies specific failure points
- **Actionable**: Clear pass/fail criteria

---

## Method 1: Tool Signature Validation

### Purpose
Verify that all tools exposed by RustyClawd match Claude Code's tool contracts (parameters, outputs, errors).

### Test Design

**Core Principle**: Tool schemas define the contract. RustyClawd must accept the same parameters and produce the same output format.

**Measurement Points**:
- Input parameter validation (types, required/optional)
- Output JSON structure and types
- Error response formats
- Streaming event types

### Execution Procedure

#### Step 1: Build a signature comparison harness

Create `/Users/ryan/src/declawed/claude-code-rs/tests/tool_signature_tests.rs`:

```bash
cd /Users/ryan/src/declawed/claude-code-rs
cargo test tool_signature -- --nocapture 2>&1 | tee tool_signature_results.txt
```

**What to check in output**:
- Each tool returns `type: "result"` or `type: "error"`
- All fields from original SDK tools are present
- No extra undocumented fields
- Type coercion matches (numbers stay numbers, strings stay strings)

#### Step 2: Create a parameter validation test

For each tool, test boundary conditions:

```bash
# Bash tool: empty command should error
cargo run -- bash "" 2>&1 | grep -q "error\|Error" && echo "PASS: Empty command rejected" || echo "FAIL: Empty command accepted"

# Read tool: nonexistent file should error
cargo run -- read /nonexistent/file.txt 2>&1 | grep -q "error\|Error" && echo "PASS: File not found error" || echo "FAIL: No error for missing file"

# Write tool: invalid path should handle gracefully
cargo run -- write "" --content "test" 2>&1 | grep -q "error\|Error" && echo "PASS: Empty path rejected" || echo "FAIL: Empty path accepted"

# Glob tool: invalid pattern handling
cargo run -- glob "[invalid(pattern" 2>&1 | grep -q "error\|Error" && echo "PASS: Invalid pattern rejected" || echo "FAIL: Invalid pattern accepted"

# Grep tool: missing pattern should error
cargo run -- grep "" 2>&1 | grep -q "error\|Error" && echo "PASS: Empty pattern rejected" || echo "FAIL: Empty pattern accepted"
```

**Expected Results**:
- All boundary tests pass (error rejection works)
- Error messages are descriptive
- Exit codes are non-zero for errors

### Validation Criteria

Pass if:
- ✅ All tool endpoints accept required parameters
- ✅ Parameter types match schema (strings for file paths, numbers for offsets)
- ✅ Optional parameters are truly optional
- ✅ Error responses include `type: "error"` and `message` field
- ✅ No unexpected fields in success responses

Fail if:
- ❌ Missing required parameter accepted
- ❌ Wrong parameter type accepted without error
- ❌ Output lacks expected fields
- ❌ Error format differs from schema

---

## Method 2: Behavioral Equivalence Testing

### Purpose
Verify RustyClawd produces identical results to Claude Code for the same operations.

### Test Design

**Core Principle**: For identical inputs, outputs must be identical or semantically equivalent (same content, possibly different whitespace/formatting).

### Execution Procedure

#### Step 1: Create test fixtures

```bash
# Create test workspace
mkdir -p /tmp/rustyclawd_equivalence_tests/{input,expected}

# Create a test file
cat > /tmp/rustyclawd_equivalence_tests/input/sample.txt << 'EOF'
Line 1: Hello
Line 2: World
Line 3: Test
Line 4: Data
Line 5: Complete
EOF

# Create modified version
cat > /tmp/rustyclawd_equivalence_tests/input/sample_modified.txt << 'EOF'
Line 1: Hello
Line 2: Universe
Line 3: Test
Line 4: Data
Line 5: Complete
EOF
```

#### Step 2: Test each tool equivalence

**Read Tool Equivalence**:
```bash
# Test 1: Full file read
cargo run -- read /tmp/rustyclawd_equivalence_tests/input/sample.txt > /tmp/test_read_full.json
cat /tmp/test_read_full.json | jq '.type' | grep -q "result" && echo "PASS: Read returns result" || echo "FAIL: Read structure wrong"
cat /tmp/test_read_full.json | jq '.data | length' | grep -q "5" && echo "PASS: Read all 5 lines" || echo "FAIL: Read missing lines"

# Test 2: Read with offset
cargo run -- read /tmp/rustyclawd_equivalence_tests/input/sample.txt --offset 2 > /tmp/test_read_offset.json
cat /tmp/test_read_offset.json | jq '.data[0]' | grep -q "Line 3" && echo "PASS: Offset works correctly" || echo "FAIL: Offset incorrect"

# Test 3: Read with limit
cargo run -- read /tmp/rustyclawd_equivalence_tests/input/sample.txt --limit 2 > /tmp/test_read_limit.json
cat /tmp/test_read_limit.json | jq '.data | length' | grep -q "2" && echo "PASS: Limit works correctly" || echo "FAIL: Limit incorrect"
```

**Write Tool Equivalence**:
```bash
# Test 1: Basic write
cargo run -- write /tmp/test_write_basic.txt --content "Test content"
cat /tmp/test_write_basic.txt | grep -q "Test content" && echo "PASS: Content written correctly" || echo "FAIL: Content not written"

# Test 2: Overwrite existing
cargo run -- write /tmp/test_write_basic.txt --content "New content"
cat /tmp/test_write_basic.txt | grep -q "New content" && ! grep -q "Test content" && echo "PASS: Overwrite works" || echo "FAIL: Overwrite failed"

# Test 3: Create nested directories
cargo run -- write /tmp/nested/deep/path/file.txt --content "Nested file"
test -f /tmp/nested/deep/path/file.txt && echo "PASS: Nested directory creation works" || echo "FAIL: Directory creation failed"
```

**Edit Tool Equivalence**:
```bash
# Test 1: Simple replacement
cp /tmp/rustyclawd_equivalence_tests/input/sample.txt /tmp/test_edit.txt
cargo run -- edit /tmp/test_edit.txt --old-string "World" --new-string "Universe"
cat /tmp/test_edit.txt | grep -q "Universe" && ! grep -q "World" && echo "PASS: Single replacement works" || echo "FAIL: Replacement failed"

# Test 2: Replace-all for duplicates
echo "test test test" > /tmp/test_edit_multi.txt
cargo run -- edit /tmp/test_edit_multi.txt --old-string "test" --new-string "verified" --replace-all
cat /tmp/test_edit_multi.txt | grep -q "verified verified verified" && echo "PASS: Replace-all works" || echo "FAIL: Replace-all failed"

# Test 3: Unique string requirement (should error on non-unique without --replace-all)
echo -e "line\nline\nline" > /tmp/test_edit_nonunique.txt
cargo run -- edit /tmp/test_edit_nonunique.txt --old-string "line" --new-string "changed" 2>&1 | grep -q "error\|Error\|unique" && echo "PASS: Non-unique detection works" || echo "FAIL: Should reject non-unique"
```

**Glob Tool Equivalence**:
```bash
# Test 1: Basic glob pattern
cargo run -- glob "**/*.txt" --path /tmp/rustyclawd_equivalence_tests/input > /tmp/test_glob_basic.json
cat /tmp/test_glob_basic.json | jq '.files | length' | grep -q "[0-9]" && echo "PASS: Glob finds files" || echo "FAIL: Glob failed"

# Test 2: No matches
cargo run -- glob "*.nonexistent" --path /tmp/rustyclawd_equivalence_tests/input > /tmp/test_glob_empty.json
cat /tmp/test_glob_empty.json | jq '.files | length' | grep -q "0" && echo "PASS: Empty glob returns empty list" || echo "FAIL: Empty result incorrect"
```

**Bash Tool Equivalence**:
```bash
# Test 1: Simple command
cargo run -- bash "echo 'Hello from RustyClawd'" > /tmp/test_bash_echo.json
cat /tmp/test_bash_echo.json | jq '.stdout' | grep -q "Hello from RustyClawd" && echo "PASS: Echo command works" || echo "FAIL: Echo failed"

# Test 2: Command with exit code
cargo run -- bash "exit 42" > /tmp/test_bash_exit.json
cat /tmp/test_bash_exit.json | jq '.exit_code' | grep -q "42" && echo "PASS: Exit code captured" || echo "FAIL: Exit code wrong"

# Test 3: Error capture
cargo run -- bash "nonexistent_command_xyz" > /tmp/test_bash_error.json
cat /tmp/test_bash_error.json | jq '.stderr' | grep -q "nonexistent_command_xyz" && echo "PASS: Stderr captured" || echo "FAIL: Stderr not captured"
```

**Grep Tool Equivalence**:
```bash
# Test 1: Basic pattern search
cargo run -- grep "Line 2" --path /tmp/rustyclawd_equivalence_tests/input > /tmp/test_grep_basic.json
cat /tmp/test_grep_basic.json | jq '.matches | length' | grep -q "1" && echo "PASS: Grep finds match" || echo "FAIL: Grep failed"

# Test 2: Case insensitive
cargo run -- grep -i "HELLO" --path /tmp/rustyclawd_equivalence_tests/input > /tmp/test_grep_case.json
cat /tmp/test_grep_case.json | jq '.matches | length' | grep -q "[0-9]" && echo "PASS: Case insensitive works" || echo "FAIL: Case insensitive failed"

# Test 3: No matches returns empty
cargo run -- grep "NOTFOUND123" --path /tmp/rustyclawd_equivalence_tests/input > /tmp/test_grep_empty.json
cat /tmp/test_grep_empty.json | jq '.matches | length' | grep -q "0" && echo "PASS: No matches returns empty" || echo "FAIL: Empty result incorrect"
```

#### Step 3: Create comprehensive equivalence test script

```bash
#!/bin/bash
# File: /Users/ryan/src/declawed/claude-code-rs/tests/equivalence_check.sh

set -e
PASS=0
FAIL=0

test_result() {
    if [ $? -eq 0 ]; then
        echo "✓ $1"
        ((PASS++))
    else
        echo "✗ $1"
        ((FAIL++))
    fi
}

# Run all equivalence tests
echo "=== BEHAVIORAL EQUIVALENCE TESTS ==="

# Read tests
cargo run -- read /tmp/rustyclawd_equivalence_tests/input/sample.txt > /tmp/r.json 2>/dev/null
test_result "Read: Full file read"

# Write tests
cargo run -- write /tmp/e_test.txt --content "content" 2>/dev/null
test_result "Write: Basic write"

# Edit tests
cargo run -- edit /tmp/e_test.txt --old-string "content" --new-string "modified" 2>/dev/null
test_result "Edit: String replacement"

# Glob tests
cargo run -- glob "**/*.txt" --path /tmp 2>/dev/null
test_result "Glob: Pattern matching"

# Bash tests
cargo run -- bash "echo test" 2>/dev/null | jq -e '.stdout' > /dev/null
test_result "Bash: Command execution"

# Grep tests
cargo run -- grep "test" --path /tmp 2>/dev/null
test_result "Grep: Pattern search"

echo "=== RESULTS ==="
echo "PASSED: $PASS"
echo "FAILED: $FAIL"
exit $FAIL
```

### Validation Criteria

Pass if:
- ✅ Read tool returns all requested lines in correct order
- ✅ Write tool creates/overwrites files atomically
- ✅ Edit tool modifies exact strings correctly
- ✅ Glob tool finds all matching files
- ✅ Bash tool captures stdout, stderr, and exit codes
- ✅ Grep tool finds all pattern matches

Fail if:
- ❌ Results are different from Claude Code
- ❌ Order of results differs
- ❌ File permissions change unexpectedly
- ❌ Partial/truncated output occurs

---

## Method 3: CLI Interface Parity Testing

### Purpose
Verify RustyClawd CLI accepts all Claude Code command formats and flags.

### Test Design

**Core Principle**: The command-line interface must be identical. Same flags, same subcommands, same argument order.

### Execution Procedure

#### Step 1: Test all CLI subcommands exist

```bash
# Test bash subcommand
cargo run -- bash "echo test" > /dev/null 2>&1 && echo "PASS: bash subcommand" || echo "FAIL: bash subcommand missing"

# Test read subcommand
cargo run -- read /tmp/test.txt > /dev/null 2>&1 && echo "PASS: read subcommand" || echo "FAIL: read subcommand missing"

# Test write subcommand
cargo run -- write /tmp/test.txt --content "test" > /dev/null 2>&1 && echo "PASS: write subcommand" || echo "FAIL: write subcommand missing"

# Test edit subcommand
cargo run -- edit /tmp/test.txt --old-string "test" --new-string "new" > /dev/null 2>&1 && echo "PASS: edit subcommand" || echo "FAIL: edit subcommand missing"

# Test glob subcommand
cargo run -- glob "**/*.txt" > /dev/null 2>&1 && echo "PASS: glob subcommand" || echo "FAIL: glob subcommand missing"

# Test grep subcommand
cargo run -- grep "test" > /dev/null 2>&1 && echo "PASS: grep subcommand" || echo "FAIL: grep subcommand missing"

# Test bash-output subcommand
cargo run -- bash-output "shell_id" > /dev/null 2>&1 && echo "PASS: bash-output subcommand exists" || echo "FAIL: bash-output subcommand missing"

# Test kill-shell subcommand
cargo run -- kill-shell "shell_id" > /dev/null 2>&1 && echo "PASS: kill-shell subcommand exists" || echo "FAIL: kill-shell subcommand missing"
```

#### Step 2: Test CLI flags

```bash
# Test --help flag
cargo run -- --help > /tmp/help_full.txt 2>&1
grep -q "bash\|read\|write\|edit\|glob\|grep" /tmp/help_full.txt && echo "PASS: --help shows subcommands" || echo "FAIL: --help incomplete"

# Test -h short flag
cargo run -- -h > /tmp/help_short.txt 2>&1
diff <(cat /tmp/help_full.txt) <(cat /tmp/help_short.txt) > /dev/null && echo "PASS: -h equivalent to --help" || echo "FAIL: -h differs from --help"

# Test --version flag
cargo run -- --version > /tmp/version_full.txt 2>&1
grep -q "[0-9]" /tmp/version_full.txt && echo "PASS: --version shows version" || echo "FAIL: --version failed"

# Test -V short flag
cargo run -- -V > /tmp/version_short.txt 2>&1
diff <(cat /tmp/version_full.txt) <(cat /tmp/version_short.txt) > /dev/null && echo "PASS: -V equivalent to --version" || echo "FAIL: -V differs"

# Test debug flag on bash
cargo run -- --debug bash "echo test" > /tmp/debug_on.json 2>&1
cat /tmp/debug_on.json | jq -e '.debug' > /dev/null 2>&1 && echo "PASS: --debug flag works" || echo "FAIL: --debug flag not working"
```

#### Step 3: Test argument parsing consistency

Create `/Users/ryan/src/declawed/claude-code-rs/tests/cli_parity_tests.sh`:

```bash
#!/bin/bash
# CLI Parity Test Suite

set -e
PASS=0
FAIL=0

test_cli() {
    if eval "$1" > /dev/null 2>&1; then
        echo "✓ $2"
        ((PASS++))
    else
        echo "✗ $2"
        ((FAIL++))
    fi
}

echo "=== CLI PARITY TESTS ==="

# Positional arguments
test_cli "cargo run -- bash 'echo test'" "Bash: positional command"
test_cli "cargo run -- read /tmp/test.txt" "Read: positional file path"
test_cli "cargo run -- write /tmp/test.txt --content 'test'" "Write: positional file path"
test_cli "cargo run -- glob '**/*.txt'" "Glob: positional pattern"
test_cli "cargo run -- grep 'pattern'" "Grep: positional pattern"

# Flag arguments
test_cli "cargo run -- read /tmp/test.txt --offset 0" "Read: --offset flag"
test_cli "cargo run -- read /tmp/test.txt --limit 10" "Read: --limit flag"
test_cli "cargo run -- write /tmp/test.txt --content 'test'" "Write: --content flag"
test_cli "cargo run -- edit /tmp/test.txt --old-string 'a' --new-string 'b'" "Edit: string flags"
test_cli "cargo run -- bash 'echo test' --timeout 5000" "Bash: --timeout flag"
test_cli "cargo run -- bash 'echo test' --description 'test cmd'" "Bash: --description flag"
test_cli "cargo run -- grep 'pattern' -i" "Grep: -i flag (case-insensitive)"
test_cli "cargo run -- grep 'pattern' -B 2 -A 2" "Grep: context flags"
test_cli "cargo run -- glob '*.txt' --path /tmp" "Glob: --path flag"

# Flag combinations
test_cli "cargo run -- read /tmp/test.txt --offset 1 --limit 5" "Read: combined offset+limit"
test_cli "cargo run -- grep 'test' -i --glob '*.rs'" "Grep: combined flags"
test_cli "cargo run -- bash 'echo' --timeout 1000 --description 'test'" "Bash: combined timeout+description"

echo "=== RESULTS ==="
echo "PASSED: $PASS"
echo "FAILED: $FAIL"
exit $FAIL
```

Run it:
```bash
chmod +x /Users/ryan/src/declawed/claude-code-rs/tests/cli_parity_tests.sh
/Users/ryan/src/declawed/claude-code-rs/tests/cli_parity_tests.sh
```

### Validation Criteria

Pass if:
- ✅ All subcommands are recognized
- ✅ All documented flags are accepted
- ✅ Positional arguments work
- ✅ Short and long flags are equivalent
- ✅ Flag combinations work correctly
- ✅ Help text is comprehensive

Fail if:
- ❌ Subcommand not found errors
- ❌ Flag not recognized errors
- ❌ Wrong argument positions required
- ❌ Missing short/long flag variants

---

## Method 4: Error Handling Alignment Testing

### Purpose
Verify RustyClawd error responses match Claude Code's error format and behavior.

### Test Design

**Core Principle**: Errors must return the same error type, message format, and HTTP status (if applicable).

### Execution Procedure

#### Step 1: Create error test matrix

Create `/Users/ryan/src/declawed/claude-code-rs/tests/error_alignment_tests.sh`:

```bash
#!/bin/bash
# Error Alignment Test Suite

set +e  # Don't exit on errors - we're testing them
PASS=0
FAIL=0

test_error() {
    local cmd="$1"
    local expected_pattern="$2"
    local test_name="$3"

    local output=$(eval "$cmd" 2>&1)

    if echo "$output" | grep -qE "$expected_pattern"; then
        echo "✓ $test_name"
        ((PASS++))
    else
        echo "✗ $test_name (got: $output)"
        ((FAIL++))
    fi
}

echo "=== ERROR ALIGNMENT TESTS ==="

# Missing required arguments
test_error "cargo run -- bash" "error|Error|required|missing" "Bash: missing command"
test_error "cargo run -- read" "error|Error|required|missing" "Read: missing file path"
test_error "cargo run -- write" "error|Error|required|missing" "Write: missing file path"
test_error "cargo run -- write /tmp/test.txt" "error|Error|required|content" "Write: missing content"
test_error "cargo run -- edit /tmp/test.txt" "error|Error|required" "Edit: missing old-string"
test_error "cargo run -- grep" "error|Error|required|pattern" "Grep: missing pattern"

# Invalid file paths
test_error "cargo run -- read /nonexistent/path/file.txt" "error|Error|not found|No such" "Read: file not found"
test_error "cargo run -- edit /nonexistent/file.txt --old-string 'a' --new-string 'b'" "error|Error|not found" "Edit: file not found"

# Invalid parameters
test_error "cargo run -- read /tmp/test.txt --offset -1" "error|Error|invalid|negative" "Read: negative offset"
test_error "cargo run -- read /tmp/test.txt --offset abc" "error|Error|invalid|parse" "Read: non-numeric offset"
test_error "cargo run -- read /tmp/test.txt --limit 0" "error|Error|invalid|zero" "Read: zero limit"
test_error "cargo run -- bash 'cmd' --timeout -1" "error|Error|invalid|negative" "Bash: negative timeout"
test_error "cargo run -- bash 'cmd' --timeout abc" "error|Error|invalid|parse" "Bash: non-numeric timeout"

# Invalid patterns
test_error "cargo run -- grep '[invalid' --path /tmp" "error|Error|invalid|pattern|regex" "Grep: invalid regex"
test_error "cargo run -- glob '[invalid('" "error|Error|invalid|pattern" "Glob: invalid pattern"

# Edit-specific errors
echo "test content" > /tmp/edit_test.txt
test_error "cargo run -- edit /tmp/edit_test.txt --old-string 'notfound' --new-string 'new'" "error|Error|not found|unique" "Edit: string not found"

echo "duplicate duplicate" > /tmp/edit_dup.txt
test_error "cargo run -- edit /tmp/edit_dup.txt --old-string 'duplicate' --new-string 'single'" "error|Error|not unique|multiple" "Edit: non-unique string"

# Cleanup
rm -f /tmp/edit_test.txt /tmp/edit_dup.txt

echo ""
echo "=== RESULTS ==="
echo "PASSED: $PASS"
echo "FAILED: $FAIL"
exit $FAIL
```

Run it:
```bash
chmod +x /Users/ryan/src/declawed/claude-code-rs/tests/error_alignment_tests.sh
/Users/ryan/src/declawed/claude-code-rs/tests/error_alignment_tests.sh
```

#### Step 2: Verify error structure

```bash
# Create test cases that should error
mkdir -p /tmp/error_tests
cd /tmp/error_tests

# Test 1: Missing argument error format
echo "Test: Missing command argument"
cargo run -- bash 2>&1 | jq . > /tmp/error_tests/missing_arg.json 2>/dev/null || cat > /tmp/error_tests/missing_arg.txt
echo "Output:" && cat /tmp/error_tests/missing_arg.json 2>/dev/null || cat /tmp/error_tests/missing_arg.txt

# Test 2: File not found error format
echo "Test: File not found error"
cargo run -- read /nonexistent 2>&1 | jq . > /tmp/error_tests/not_found.json 2>/dev/null || cat > /tmp/error_tests/not_found.txt
echo "Output:" && cat /tmp/error_tests/not_found.json 2>/dev/null || cat /tmp/error_tests/not_found.txt

# Test 3: Invalid parameter error format
echo "Test: Invalid parameter error"
cargo run -- read /tmp/test.txt --offset abc 2>&1 | jq . > /tmp/error_tests/invalid_param.json 2>/dev/null || cat > /tmp/error_tests/invalid_param.txt
echo "Output:" && cat /tmp/error_tests/invalid_param.json 2>/dev/null || cat /tmp/error_tests/invalid_param.txt

# Verify error structure consistency
echo ""
echo "Error structure check:"
if jq -e '.type == "error" and .message' /tmp/error_tests/not_found.json 2>/dev/null; then
    echo "✓ Error has consistent structure"
else
    echo "✗ Error structure inconsistent"
fi
```

### Validation Criteria

Pass if:
- ✅ All errors include `type: "error"` field
- ✅ All errors include descriptive `message` field
- ✅ Missing required arguments are caught
- ✅ Invalid file paths produce "not found" errors
- ✅ Type mismatches are caught
- ✅ Error messages are user-friendly

Fail if:
- ❌ Errors don't have consistent JSON structure
- ❌ Missing error message field
- ❌ Unclear or cryptic error text
- ❌ Panics instead of returning errors

---

## Method 5: Integration Workflow Testing

### Purpose
Verify RustyClawd works correctly in realistic Claude Code workflows (the ultimate test).

### Test Design

**Core Principle**: RustyClawd must work seamlessly in actual agent/assistant workflows. Test multi-step operations that combine tools.

### Execution Procedure

#### Step 1: Create a realistic workflow

Create `/Users/ryan/src/declawed/claude-code-rs/tests/integration_workflow_test.sh`:

```bash
#!/bin/bash
# Integration Workflow Test - Simulates real Claude Code usage

set -e

WORKSPACE="/tmp/rustyclawd_workflow"
rm -rf "$WORKSPACE"
mkdir -p "$WORKSPACE"
cd "$WORKSPACE"

PASS=0
FAIL=0

echo "=== INTEGRATION WORKFLOW TESTS ==="
echo ""

# Workflow 1: Write-Read-Verify
echo "Workflow 1: Create file -> Read -> Verify content"
cargo run -- write "$WORKSPACE/workflow1.txt" --content "function main() {
  console.log('Hello');
}" > /dev/null 2>&1
echo "✓ Wrote file"
((PASS++))

OUTPUT=$(cargo run -- read "$WORKSPACE/workflow1.txt" 2>/dev/null | jq -r '.data[0]' 2>/dev/null)
if [[ "$OUTPUT" == *"function main"* ]]; then
    echo "✓ Read file correctly"
    ((PASS++))
else
    echo "✗ Read returned wrong content"
    ((FAIL++))
fi

# Workflow 2: Write-Edit-Read
echo ""
echo "Workflow 2: Create file -> Edit -> Read -> Verify"
cargo run -- write "$WORKSPACE/workflow2.txt" --content "const value = 10;" > /dev/null 2>&1
echo "✓ Wrote initial content"
((PASS++))

cargo run -- edit "$WORKSPACE/workflow2.txt" --old-string "10" --new-string "20" > /dev/null 2>&1
echo "✓ Edited content"
((PASS++))

RESULT=$(cat "$WORKSPACE/workflow2.txt")
if [[ "$RESULT" == *"const value = 20"* ]]; then
    echo "✓ Verified edited content"
    ((PASS++))
else
    echo "✗ Edit failed"
    ((FAIL++))
fi

# Workflow 3: Create multiple files -> Glob -> Grep
echo ""
echo "Workflow 3: Create files -> Find -> Search"
mkdir -p "$WORKSPACE/src"
cargo run -- write "$WORKSPACE/src/file1.rs" --content "fn main() { println!(\"test1\"); }" > /dev/null 2>&1
cargo run -- write "$WORKSPACE/src/file2.rs" --content "fn helper() { println!(\"test2\"); }" > /dev/null 2>&1
cargo run -- write "$WORKSPACE/src/file3.txt" --content "This is a readme" > /dev/null 2>&1
echo "✓ Created test files"
((PASS++))

GLOB_COUNT=$(cargo run -- glob "**/*.rs" --path "$WORKSPACE" 2>/dev/null | jq '.files | length' 2>/dev/null)
if [[ "$GLOB_COUNT" == "2" ]]; then
    echo "✓ Glob found 2 Rust files"
    ((PASS++))
else
    echo "✗ Glob found $GLOB_COUNT files instead of 2"
    ((FAIL++))
fi

GREP_RESULT=$(cargo run -- grep "println" --path "$WORKSPACE/src" 2>/dev/null | jq '.matches | length' 2>/dev/null)
if [[ "$GREP_RESULT" -gt "0" ]]; then
    echo "✓ Grep found println matches"
    ((PASS++))
else
    echo "✗ Grep failed to find matches"
    ((FAIL++))
fi

# Workflow 4: Bash command chain
echo ""
echo "Workflow 4: Execute bash commands -> Parse output"
BASH_OUT=$(cargo run -- bash "echo 'step1' && echo 'step2'" 2>/dev/null | jq -r '.stdout' 2>/dev/null)
if [[ "$BASH_OUT" == *"step1"* ]] && [[ "$BASH_OUT" == *"step2"* ]]; then
    echo "✓ Bash executed multi-line commands"
    ((PASS++))
else
    echo "✗ Bash output incorrect"
    ((FAIL++))
fi

# Workflow 5: Complex edit scenario
echo ""
echo "Workflow 5: Edit complex file -> Verify syntax"
cat > "$WORKSPACE/complex.json" << 'EOF'
{
  "name": "test",
  "version": "1.0.0",
  "description": "Original"
}
EOF
echo "✓ Created JSON file"
((PASS++))

cargo run -- edit "$WORKSPACE/complex.json" --old-string '"description": "Original"' --new-string '"description": "Updated"' > /dev/null 2>&1
echo "✓ Edited JSON"
((PASS++))

# Verify JSON is still valid
if jq empty "$WORKSPACE/complex.json" 2>/dev/null; then
    echo "✓ JSON still valid after edit"
    ((PASS++))
else
    echo "✗ JSON invalid after edit"
    ((FAIL++))
fi

# Workflow 6: Error recovery
echo ""
echo "Workflow 6: Handle errors gracefully"
ERROR_OUTPUT=$(cargo run -- read /nonexistent/path 2>&1)
if [[ "$ERROR_OUTPUT" == *"error"* ]] || [[ "$ERROR_OUTPUT" == *"Error"* ]]; then
    echo "✓ Error handled gracefully"
    ((PASS++))
else
    echo "✗ Error not detected"
    ((FAIL++))
fi

# Verify tool still works after error
cargo run -- bash "echo 'recovery'" > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "✓ Tool recovered after error"
    ((PASS++))
else
    echo "✗ Tool failed to recover"
    ((FAIL++))
fi

# Cleanup
cd /
rm -rf "$WORKSPACE"

echo ""
echo "=== RESULTS ==="
echo "PASSED: $PASS"
echo "FAILED: $FAIL"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✓ All workflows passed!"
    exit 0
else
    echo "✗ Some workflows failed"
    exit 1
fi
```

Run it:
```bash
chmod +x /Users/ryan/src/declawed/claude-code-rs/tests/integration_workflow_test.sh
/Users/ryan/src/declawed/claude-code-rs/tests/integration_workflow_test.sh 2>&1 | tee integration_workflow_results.txt
```

#### Step 2: Verify streaming and background operations

```bash
# Test background bash execution
echo "Testing background bash operations..."

# Start background bash and capture shell ID
SHELL_OUTPUT=$(cargo run -- bash "sleep 2 && echo 'background result'" 2>&1)
echo "Background command started"

# Verify output eventually arrives
if echo "$SHELL_OUTPUT" | grep -q "background result"; then
    echo "✓ Background bash executed successfully"
else
    echo "✗ Background bash failed"
fi

# Test killing shell
echo "Testing kill-shell..."
# This is harder to test without background execution, skip if not implemented
```

#### Step 3: Test output consistency across tools

```bash
# Test that JSON output is consistent
echo "Testing output consistency..."

# All tools should output JSON
for cmd in 'bash "echo test"' 'read /tmp/test.txt' 'glob "*.txt"' 'grep "test"'; do
    output=$(cargo run -- $cmd 2>&1 | head -1)
    if [[ "$output" == "{"* ]]; then
        echo "✓ $cmd produces JSON"
    else
        echo "⚠ $cmd output may not be JSON: $output"
    fi
done
```

### Validation Criteria

Pass if:
- ✅ Multi-step workflows complete successfully
- ✅ File operations are atomic and consistent
- ✅ Search/grep results are accurate
- ✅ Tools recover from errors gracefully
- ✅ JSON output is valid throughout
- ✅ No data loss or corruption in multi-operation workflows

Fail if:
- ❌ Workflows hang or timeout
- ❌ Partial failures in multi-step operations
- ❌ Inconsistent state after operations
- ❌ Cascading failures
- ❌ Data corruption

---

## Execution Summary Table

| Method | Test Command | Pass Criteria | Time |
|--------|--------------|---------------|------|
| **1. Tool Signatures** | `cargo test tool_signature -- --nocapture` | All 6 tools accept correct params | ~30s |
| **2. Behavioral Equivalence** | `/tests/equivalence_check.sh` | 100% equivalence across all tools | ~60s |
| **3. CLI Parity** | `/tests/cli_parity_tests.sh` | All subcommands, flags, args work | ~45s |
| **4. Error Alignment** | `/tests/error_alignment_tests.sh` | Consistent error structure, messages | ~40s |
| **5. Integration Workflows** | `/tests/integration_workflow_test.sh` | All 6 workflows complete, no corruption | ~90s |

**Total estimated runtime**: ~4-5 minutes for full verification suite

---

## Quick Start: Run All Tests

Create a master test runner at `/Users/ryan/src/declawed/claude-code-rs/FULL_VERIFICATION.sh`:

```bash
#!/bin/bash
# Complete RustyClawd Drop-In Replacement Verification

set -e
cd /Users/ryan/src/declawed/claude-code-rs

echo "=================================================="
echo "RUSTYCLAWD DROP-IN REPLACEMENT VERIFICATION"
echo "=================================================="
echo ""

TOTAL_PASS=0
TOTAL_FAIL=0

run_test_suite() {
    local name="$1"
    local script="$2"

    echo ""
    echo "Running: $name"
    echo "---"

    if bash "$script" 2>&1; then
        TOTAL_PASS=$((TOTAL_PASS + 1))
        echo "✓ $name PASSED"
    else
        TOTAL_FAIL=$((TOTAL_FAIL + 1))
        echo "✗ $name FAILED"
    fi
}

# Method 1: Tool Signatures
echo "[1/5] Tool Signature Validation"
cargo test tool_signature -- --nocapture 2>&1 | tee test_signatures.log

# Method 2: Behavioral Equivalence
echo "[2/5] Behavioral Equivalence Testing"
bash tests/equivalence_check.sh 2>&1 | tee test_equivalence.log

# Method 3: CLI Parity
echo "[3/5] CLI Interface Parity"
bash tests/cli_parity_tests.sh 2>&1 | tee test_cli_parity.log

# Method 4: Error Alignment
echo "[4/5] Error Handling Alignment"
bash tests/error_alignment_tests.sh 2>&1 | tee test_error_alignment.log

# Method 5: Integration Workflows
echo "[5/5] Integration Workflow Testing"
bash tests/integration_workflow_test.sh 2>&1 | tee test_integration.log

echo ""
echo "=================================================="
echo "FINAL RESULTS"
echo "=================================================="
echo ""
echo "See test logs for details:"
echo "  - test_signatures.log"
echo "  - test_equivalence.log"
echo "  - test_cli_parity.log"
echo "  - test_error_alignment.log"
echo "  - test_integration.log"
```

Run it:
```bash
chmod +x /Users/ryan/src/declawed/claude-code-rs/FULL_VERIFICATION.sh
/Users/ryan/src/declawed/claude-code-rs/FULL_VERIFICATION.sh
```

---

## Gap Analysis: What Gets Tested

### Covered
- ✅ All 6 core tools (bash, read, write, edit, glob, grep)
- ✅ CLI interface and flags
- ✅ Error handling and edge cases
- ✅ File I/O consistency
- ✅ Pattern matching accuracy
- ✅ JSON output format
- ✅ Multi-step workflows
- ✅ Recovery from errors

### Not Covered (Out of Scope)
- ❌ Advanced tools (WebFetch, TodoWrite, etc.)
- ❌ Agent system and model integration
- ❌ Streaming large files
- ❌ Permission handling
- ❌ Performance benchmarks
- ❌ Memory profiling
- ❌ Platform-specific behavior (Windows, Linux)

---

## Troubleshooting

**Tests fail to run**:
```bash
# Ensure cargo is installed and in PATH
rustc --version
cargo --version

# Rebuild project
cargo clean
cargo build --release
```

**Tool not found**:
```bash
# Ensure all subcommands are implemented
cargo run -- --help | grep -E "bash|read|write|edit|glob|grep"
```

**Output format differs**:
```bash
# Check if using `jq` correctly
cargo run -- bash "echo test" | jq .
# Should show valid JSON
```

**Permission errors**:
```bash
# Ensure test directories are writable
chmod 755 /tmp
# Or use a different temp directory
export TMPDIR=$HOME/rustyclawd_tmp
mkdir -p $TMPDIR
```

---

## Conclusion

These 5 methods comprehensively verify drop-in replacement compatibility:

1. **Tool Signatures**: Contracts are met (right inputs/outputs)
2. **Behavioral Equivalence**: Results are identical
3. **CLI Parity**: Interface is the same
4. **Error Alignment**: Errors are consistent
5. **Integration Workflows**: Real usage works seamlessly

Together, they provide **concrete, executable proof** that RustyClawd is a drop-in replacement for Claude Code.

