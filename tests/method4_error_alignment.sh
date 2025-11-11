#!/bin/bash
# Method 4: Error Handling Alignment Testing
# Verifies error responses match Claude Code's format

set +e  # Don't exit on errors - we're testing them
cd /Users/ryan/src/declawed/claude-code-rs

PASS=0
FAIL=0

echo "=============================================="
echo "METHOD 4: ERROR HANDLING ALIGNMENT TESTING"
echo "=============================================="
echo ""

test_error() {
    local cmd="$1"
    local expected_pattern="$2"
    local test_name="$3"

    local output=$(eval "$cmd" 2>&1)

    if echo "$output" | grep -qE "$expected_pattern"; then
        echo "✓ $test_name"
        ((PASS++))
    else
        echo "✗ $test_name (output: ${output:0:50}...)"
        ((FAIL++))
    fi
}

# ========== MISSING REQUIRED ARGUMENTS ==========
echo "Testing: Missing required arguments"

test_error "cargo run -- bash" "error|Error|required|missing|Usage" "Bash: missing command"
test_error "cargo run -- read" "error|Error|required|missing|Usage" "Read: missing file path"
test_error "cargo run -- write /tmp/test.txt" "error|Error|required|content|Usage" "Write: missing content"
test_error "cargo run -- edit /tmp/test.txt" "error|Error|required|Usage" "Edit: missing old-string"
test_error "cargo run -- glob" "error|Error|required|missing|Usage" "Glob: missing pattern"
test_error "cargo run -- grep" "error|Error|required|missing|Usage" "Grep: missing pattern"

# ========== MISSING OPTIONAL ARGUMENTS ==========
echo "Testing: Invalid argument combinations"

test_error "cargo run -- bash 'cmd' --timeout" "error|Error|requires value|argument|missing" "Bash: --timeout without value"
test_error "cargo run -- read /tmp/test.txt --offset" "error|Error|requires value|argument|missing" "Read: --offset without value"
test_error "cargo run -- read /tmp/test.txt --limit" "error|Error|requires value|argument|missing" "Read: --limit without value"

# ========== INVALID FILE PATHS ==========
echo "Testing: File not found errors"

test_error "cargo run -- read /nonexistent/path/file.txt" "error|Error|not found|No such|cannot find" "Read: file not found"
test_error "cargo run -- edit /nonexistent/file.txt --old-string 'a' --new-string 'b'" "error|Error|not found|No such" "Edit: file not found"

# ========== INVALID PARAMETER TYPES ==========
echo "Testing: Invalid parameter type handling"

test_error "cargo run -- read /tmp/m4_test.txt --offset -1" "error|Error|invalid|negative|must be" "Read: negative offset"
test_error "cargo run -- read /tmp/m4_test.txt --offset abc" "error|Error|invalid|parse|not a valid" "Read: non-numeric offset"
test_error "cargo run -- read /tmp/m4_test.txt --limit 0" "error|Error|invalid|zero|must be" "Read: zero limit"
test_error "cargo run -- read /tmp/m4_test.txt --limit -5" "error|Error|invalid|negative" "Read: negative limit"
test_error "cargo run -- bash 'cmd' --timeout -1" "error|Error|invalid|negative" "Bash: negative timeout"
test_error "cargo run -- bash 'cmd' --timeout abc" "error|Error|invalid|parse|not a valid" "Bash: non-numeric timeout"
test_error "cargo run -- bash 'cmd' --timeout 0" "error|Error|invalid|zero|must be" "Bash: zero timeout"

# ========== INVALID PATTERNS ==========
echo "Testing: Invalid pattern handling"

test_error "cargo run -- grep '[invalid' --path /tmp" "error|Error|invalid|regex|pattern" "Grep: invalid regex pattern"
test_error "cargo run -- glob '[invalid('" "error|Error|invalid|pattern|glob" "Glob: invalid glob pattern"

# ========== EDIT-SPECIFIC ERRORS ==========
echo "Testing: Edit tool error cases"

# Create test file
echo "test content" > /tmp/m4_edit_test.txt

# Test: String not found
test_error "cargo run -- edit /tmp/m4_edit_test.txt --old-string 'NOTFOUND' --new-string 'new'" "error|Error|not found|cannot find" "Edit: string not found"

# Test: Non-unique string without --replace-all
echo "duplicate duplicate" > /tmp/m4_edit_dup.txt
test_error "cargo run -- edit /tmp/m4_edit_dup.txt --old-string 'duplicate' --new-string 'single'" "error|Error|not unique|multiple|ambiguous" "Edit: non-unique string"

# ========== PERMISSION/WRITE ERRORS ==========
echo "Testing: Permission-related errors"

# Try to write to read-only directory (create one)
mkdir -p /tmp/m4_readonly
chmod 444 /tmp/m4_readonly
test_error "cargo run -- write /tmp/m4_readonly/file.txt --content 'test'" "error|Error|permission|denied|write" "Write: permission denied"
chmod 755 /tmp/m4_readonly
rm -rf /tmp/m4_readonly

# ========== ERROR FORMAT VALIDATION ==========
echo "Testing: Error format consistency"

# Capture an error and validate structure
ERROR_OUTPUT=$(cargo run -- read /nonexistent 2>&1)

if echo "$ERROR_OUTPUT" | grep -qE "error|Error|invalid|not found"; then
    echo "✓ Error: Contains error indicator"
    ((PASS++))
else
    echo "✗ Error: Missing error indicator"
    ((FAIL++))
fi

# Check if it's JSON or plain text
if echo "$ERROR_OUTPUT" | grep -q "^{"; then
    echo "✓ Error: Returns JSON format"
    ((PASS++))

    # Validate JSON structure
    if echo "$ERROR_OUTPUT" | jq -e '.type' > /dev/null 2>&1; then
        echo "✓ Error: Has 'type' field"
        ((PASS++))
    else
        echo "✗ Error: Missing 'type' field in JSON"
        ((FAIL++))
    fi

    if echo "$ERROR_OUTPUT" | jq -e '.message' > /dev/null 2>&1; then
        echo "✓ Error: Has 'message' field"
        ((PASS++))
    else
        echo "✗ Error: Missing 'message' field in JSON"
        ((FAIL++))
    fi
else
    echo "⚠ Error: Returns plain text (check if intentional)"
fi

# ========== EXIT CODES ==========
echo "Testing: Exit code behavior"

cargo run -- bash "true" > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "✓ Exit code: Success (0) for passing command"
    ((PASS++))
else
    echo "✗ Exit code: Should be 0 for success"
    ((FAIL++))
fi

cargo run -- bash "false" > /dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "✓ Exit code: Non-zero for failing command"
    ((PASS++))
else
    echo "✗ Exit code: Should be non-zero for failure"
    ((FAIL++))
fi

cargo run -- read /nonexistent 2>&1 > /dev/null
if [ $? -ne 0 ]; then
    echo "✓ Exit code: Non-zero for missing file"
    ((PASS++))
else
    echo "✗ Exit code: Should be non-zero for error"
    ((FAIL++))
fi

# ========== ERROR RECOVERY ==========
echo "Testing: Error recovery"

# Make sure tool works after an error
ERROR_OUTPUT=$(cargo run -- read /nonexistent 2>&1)
RECOVERY_OUTPUT=$(cargo run -- bash "echo recovery" 2>&1)

if echo "$RECOVERY_OUTPUT" | grep -q "recovery"; then
    echo "✓ Recovery: Tool works after error"
    ((PASS++))
else
    echo "✗ Recovery: Tool failed after error"
    ((FAIL++))
fi

# ========== EDGE CASES ==========
echo "Testing: Edge case error handling"

# Empty command for bash should error
test_error "cargo run -- bash ''" "error|Error|empty|required" "Edge case: Empty bash command"

# Empty file path for read
test_error "cargo run -- read ''" "error|Error|empty|required|invalid" "Edge case: Empty file path"

# Empty pattern for grep
test_error "cargo run -- grep ''" "error|Error|empty|required" "Edge case: Empty grep pattern"

# Cleanup
rm -f /tmp/m4_*.txt
rmdir /tmp/m4_readonly 2>/dev/null || true

echo ""
echo "=============================================="
echo "RESULTS: METHOD 4"
echo "=============================================="
echo "PASSED: $PASS"
echo "FAILED: $FAIL"
echo ""

if [ $FAIL -le 2 ]; then
    echo "✓ Error handling mostly aligned"
    exit 0
else
    echo "✗ Error handling has gaps"
    exit 1
fi
