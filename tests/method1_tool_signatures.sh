#!/bin/bash
# Method 1: Tool Signature Validation
# Tests that all tools exist and accept correct parameters

set -e
cd /Users/ryan/src/declawed/claude-code-rs

PASS=0
FAIL=0

echo "=============================================="
echo "METHOD 1: TOOL SIGNATURE VALIDATION"
echo "=============================================="
echo ""

test_result() {
    if [ $? -eq 0 ]; then
        echo "✓ $1"
        ((PASS++))
    else
        echo "✗ $1"
        ((FAIL++))
    fi
}

# Test 1: Bash subcommand exists and accepts command
echo "Testing: Bash tool"
cargo run -- bash "echo 'signature_test'" > /tmp/m1_bash_test.json 2>/dev/null
jq -e '.type' /tmp/m1_bash_test.json > /dev/null
test_result "Bash tool responds with JSON"

jq -e '.stdout' /tmp/m1_bash_test.json | grep -q "signature_test"
test_result "Bash captures stdout correctly"

# Test 2: Read subcommand exists and accepts file path
echo "Testing: Read tool"
echo "test content" > /tmp/m1_read_test.txt
cargo run -- read /tmp/m1_read_test.txt > /tmp/m1_read_test.json 2>/dev/null
jq -e '.data' /tmp/m1_read_test.json > /dev/null
test_result "Read tool returns data field"

jq -e '.data[0]' /tmp/m1_read_test.json | grep -q "test content"
test_result "Read captures file content"

# Test 3: Write subcommand exists and accepts file path + content
echo "Testing: Write tool"
cargo run -- write /tmp/m1_write_test.txt --content "write signature test" > /tmp/m1_write_test.json 2>/dev/null
test -f /tmp/m1_write_test.txt
test_result "Write creates file"

cat /tmp/m1_write_test.txt | grep -q "write signature test"
test_result "Write stores content correctly"

# Test 4: Edit subcommand exists and accepts old/new strings
echo "Testing: Edit tool"
echo "original text" > /tmp/m1_edit_test.txt
cargo run -- edit /tmp/m1_edit_test.txt --old-string "original" --new-string "modified" > /tmp/m1_edit_test.json 2>/dev/null
cat /tmp/m1_edit_test.txt | grep -q "modified text"
test_result "Edit modifies content correctly"

# Test 5: Glob subcommand exists and accepts pattern
echo "Testing: Glob tool"
cargo run -- glob "*.txt" --path /tmp > /tmp/m1_glob_test.json 2>/dev/null
jq -e '.files' /tmp/m1_glob_test.json > /dev/null
test_result "Glob returns files array"

jq '.files | length' /tmp/m1_glob_test.json | grep -q "[0-9]"
test_result "Glob finds matching files"

# Test 6: Grep subcommand exists and accepts pattern
echo "Testing: Grep tool"
cargo run -- grep "test" --path /tmp > /tmp/m1_grep_test.json 2>/dev/null
jq -e '.matches' /tmp/m1_grep_test.json > /dev/null
test_result "Grep returns matches array"

# Test 7: Error handling - empty command should fail
echo "Testing: Error handling"
if cargo run -- bash "" 2>&1 | grep -qE "error|Error|required"; then
    echo "✓ Bash rejects empty command"
    ((PASS++))
else
    echo "✗ Bash should reject empty command"
    ((FAIL++))
fi

# Test 8: Error handling - nonexistent file should fail
if cargo run -- read /nonexistent/file.txt 2>&1 | grep -qE "error|Error|not found|No such"; then
    echo "✓ Read rejects nonexistent file"
    ((PASS++))
else
    echo "✗ Read should reject nonexistent file"
    ((FAIL++))
fi

# Test 9: Parameter types are validated
echo "Testing: Parameter validation"
if cargo run -- read /tmp/m1_read_test.txt --offset abc 2>&1 | grep -qE "error|Error|invalid"; then
    echo "✓ Read validates offset type"
    ((PASS++))
else
    echo "✗ Read should validate offset type"
    ((FAIL++))
fi

# Test 10: Optional parameters work
cargo run -- read /tmp/m1_read_test.txt --offset 0 --limit 1 > /tmp/m1_optional.json 2>/dev/null
jq -e '.data' /tmp/m1_optional.json > /dev/null
test_result "Optional parameters work together"

# Cleanup
rm -f /tmp/m1_*.txt /tmp/m1_*.json

echo ""
echo "=============================================="
echo "RESULTS: METHOD 1"
echo "=============================================="
echo "PASSED: $PASS"
echo "FAILED: $FAIL"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✓ All tool signatures validated successfully"
    exit 0
else
    echo "✗ Some signatures failed validation"
    exit 1
fi
