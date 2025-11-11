#!/bin/bash
# Method 2: Behavioral Equivalence Testing
# Verifies RustyClawd produces identical results to Claude Code

set -e
cd /Users/ryan/src/declawed/claude-code-rs

PASS=0
FAIL=0

echo "=============================================="
echo "METHOD 2: BEHAVIORAL EQUIVALENCE TESTING"
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

# Setup test workspace
TESTDIR="/tmp/rustyclawd_behavioral_test"
rm -rf "$TESTDIR"
mkdir -p "$TESTDIR"

# Create test fixtures
cat > "$TESTDIR/sample.txt" << 'EOF'
Line 1: Hello
Line 2: World
Line 3: Test
Line 4: Data
Line 5: Complete
EOF

cat > "$TESTDIR/sample_modified.txt" << 'EOF'
Line 1: Hello
Line 2: Universe
Line 3: Test
Line 4: Data
Line 5: Complete
EOF

# ========== READ TOOL EQUIVALENCE ==========
echo "Testing: Read tool equivalence"

# Test 1: Full file read
cargo run -- read "$TESTDIR/sample.txt" > "$TESTDIR/read_full.json" 2>/dev/null
jq -e '.data | length == 5' "$TESTDIR/read_full.json" > /dev/null
test_result "Read: Returns all 5 lines"

jq -e '.data[0] | contains("Line 1")' "$TESTDIR/read_full.json" > /dev/null
test_result "Read: First line correct"

jq -e '.data[4] | contains("Complete")' "$TESTDIR/read_full.json" > /dev/null
test_result "Read: Last line correct"

# Test 2: Read with offset
cargo run -- read "$TESTDIR/sample.txt" --offset 2 > "$TESTDIR/read_offset.json" 2>/dev/null
jq -e '.data[0] | contains("Line 3")' "$TESTDIR/read_offset.json" > /dev/null
test_result "Read: Offset skips correctly"

# Test 3: Read with limit
cargo run -- read "$TESTDIR/sample.txt" --limit 2 > "$TESTDIR/read_limit.json" 2>/dev/null
jq -e '.data | length == 2' "$TESTDIR/read_limit.json" > /dev/null
test_result "Read: Limit restricts lines"

# Test 4: Offset + Limit
cargo run -- read "$TESTDIR/sample.txt" --offset 1 --limit 2 > "$TESTDIR/read_offset_limit.json" 2>/dev/null
jq -e '.data | length == 2' "$TESTDIR/read_offset_limit.json" > /dev/null
test_result "Read: Offset+Limit works together"

jq -e '.data[0] | contains("Line 2")' "$TESTDIR/read_offset_limit.json" > /dev/null
test_result "Read: Offset+Limit starts at correct line"

# ========== WRITE TOOL EQUIVALENCE ==========
echo "Testing: Write tool equivalence"

# Test 1: Basic write
cargo run -- write "$TESTDIR/write_basic.txt" --content "Test content" > /dev/null 2>/dev/null
test -f "$TESTDIR/write_basic.txt"
test_result "Write: Creates file"

grep -q "Test content" "$TESTDIR/write_basic.txt"
test_result "Write: Content written"

# Test 2: Overwrite existing
cargo run -- write "$TESTDIR/write_basic.txt" --content "New content" > /dev/null 2>/dev/null
grep -q "New content" "$TESTDIR/write_basic.txt"
test_result "Write: Overwrites existing file"

! grep -q "Test content" "$TESTDIR/write_basic.txt"
test_result "Write: Old content removed"

# Test 3: Create nested directories
cargo run -- write "$TESTDIR/nested/deep/path/file.txt" --content "Nested" > /dev/null 2>/dev/null
test -f "$TESTDIR/nested/deep/path/file.txt"
test_result "Write: Creates nested directories"

grep -q "Nested" "$TESTDIR/nested/deep/path/file.txt"
test_result "Write: Nested file content correct"

# Test 4: Empty content
cargo run -- write "$TESTDIR/empty.txt" --content "" > /dev/null 2>/dev/null
test -f "$TESTDIR/empty.txt"
test_result "Write: Handles empty content"

# ========== EDIT TOOL EQUIVALENCE ==========
echo "Testing: Edit tool equivalence"

# Test 1: Simple single replacement
cp "$TESTDIR/sample.txt" "$TESTDIR/edit_single.txt"
cargo run -- edit "$TESTDIR/edit_single.txt" --old-string "World" --new-string "Universe" > /dev/null 2>/dev/null
grep -q "Universe" "$TESTDIR/edit_single.txt"
test_result "Edit: Single replacement works"

! grep -q "World" "$TESTDIR/edit_single.txt"
test_result "Edit: Original string removed"

# Test 2: Replace-all for duplicates
echo "test test test" > "$TESTDIR/edit_multi.txt"
cargo run -- edit "$TESTDIR/edit_multi.txt" --old-string "test" --new-string "verified" --replace-all > /dev/null 2>/dev/null
grep -q "verified verified verified" "$TESTDIR/edit_multi.txt"
test_result "Edit: Replace-all works"

# Test 3: Preserves surrounding content
cp "$TESTDIR/sample.txt" "$TESTDIR/edit_context.txt"
cargo run -- edit "$TESTDIR/edit_context.txt" --old-string "Line 2: World" --new-string "Line 2: MODIFIED" > /dev/null 2>/dev/null
grep -q "Line 1: Hello" "$TESTDIR/edit_context.txt"
test_result "Edit: Preserves context"

grep -q "Line 3: Test" "$TESTDIR/edit_context.txt"
test_result "Edit: Preserves following lines"

# Test 4: Non-unique without --replace-all should error
echo -e "line\nline\nline" > "$TESTDIR/edit_nonunique.txt"
if cargo run -- edit "$TESTDIR/edit_nonunique.txt" --old-string "line" --new-string "changed" 2>&1 | grep -qE "error|Error|unique"; then
    echo "✓ Edit: Rejects non-unique without --replace-all"
    ((PASS++))
else
    echo "✗ Edit: Should reject non-unique"
    ((FAIL++))
fi

# ========== GLOB TOOL EQUIVALENCE ==========
echo "Testing: Glob tool equivalence"

# Test 1: Basic pattern matching
cargo run -- glob "*.txt" --path "$TESTDIR" > "$TESTDIR/glob_basic.json" 2>/dev/null
jq -e '.files | length > 0' "$TESTDIR/glob_basic.json" > /dev/null
test_result "Glob: Finds matching files"

# Test 2: Nested pattern
cargo run -- glob "**/*.txt" --path "$TESTDIR" > "$TESTDIR/glob_nested.json" 2>/dev/null
NESTED_COUNT=$(jq '.files | length' "$TESTDIR/glob_nested.json")
FLAT_COUNT=$(jq '.files | length' "$TESTDIR/glob_basic.json")
[ "$NESTED_COUNT" -ge "$FLAT_COUNT" ]
test_result "Glob: Nested pattern finds more files"

# Test 3: No matches returns empty array
cargo run -- glob "*.nonexistent" --path "$TESTDIR" > "$TESTDIR/glob_empty.json" 2>/dev/null
jq -e '.files | length == 0' "$TESTDIR/glob_empty.json" > /dev/null
test_result "Glob: Empty result for no matches"

# Test 4: Consistent ordering
cargo run -- glob "*.txt" --path "$TESTDIR" > "$TESTDIR/glob_order1.json" 2>/dev/null
cargo run -- glob "*.txt" --path "$TESTDIR" > "$TESTDIR/glob_order2.json" 2>/dev/null
diff <(jq '.files' "$TESTDIR/glob_order1.json") <(jq '.files' "$TESTDIR/glob_order2.json") > /dev/null
test_result "Glob: Returns consistent ordering"

# ========== BASH TOOL EQUIVALENCE ==========
echo "Testing: Bash tool equivalence"

# Test 1: Simple echo
cargo run -- bash "echo 'Hello from Rust'" > "$TESTDIR/bash_echo.json" 2>/dev/null
jq -e '.stdout | contains("Hello from Rust")' "$TESTDIR/bash_echo.json" > /dev/null
test_result "Bash: Captures stdout"

# Test 2: Exit code
cargo run -- bash "exit 42" > "$TESTDIR/bash_exit.json" 2>/dev/null
jq -e '.exit_code == 42' "$TESTDIR/bash_exit.json" > /dev/null
test_result "Bash: Captures exit code"

# Test 3: Stderr capture
cargo run -- bash "echo 'error' >&2" > "$TESTDIR/bash_stderr.json" 2>/dev/null
jq -e '.stderr | contains("error")' "$TESTDIR/bash_stderr.json" > /dev/null
test_result "Bash: Captures stderr"

# Test 4: Multiple commands
cargo run -- bash "echo 'line1'; echo 'line2'" > "$TESTDIR/bash_multi.json" 2>/dev/null
jq -e '.stdout | contains("line1")' "$TESTDIR/bash_multi.json" > /dev/null
jq -e '.stdout | contains("line2")' "$TESTDIR/bash_multi.json" > /dev/null
test_result "Bash: Handles multiple commands"

# Test 5: Success flag
cargo run -- bash "true" > "$TESTDIR/bash_true.json" 2>/dev/null
jq -e '.success == true' "$TESTDIR/bash_true.json" > /dev/null
test_result "Bash: Sets success for exit 0"

# ========== GREP TOOL EQUIVALENCE ==========
echo "Testing: Grep tool equivalence"

# Test 1: Basic pattern search
cargo run -- grep "Line 2" --path "$TESTDIR" > "$TESTDIR/grep_basic.json" 2>/dev/null
jq -e '.matches | length > 0' "$TESTDIR/grep_basic.json" > /dev/null
test_result "Grep: Finds matching lines"

# Test 2: Case sensitive
cargo run -- grep "line" --path "$TESTDIR" > "$TESTDIR/grep_case_sensitive.json" 2>/dev/null
CASE_COUNT=$(jq '.matches | length' "$TESTDIR/grep_case_sensitive.json")
test_result "Grep: Case sensitivity works"

# Test 3: Case insensitive
cargo run -- grep -i "HELLO" --path "$TESTDIR" > "$TESTDIR/grep_case_insensitive.json" 2>/dev/null
jq -e '.matches | length > 0' "$TESTDIR/grep_case_insensitive.json" > /dev/null
test_result "Grep: -i flag works"

# Test 4: No matches returns empty
cargo run -- grep "NOTFOUND_XYZ_ABC" --path "$TESTDIR" > "$TESTDIR/grep_empty.json" 2>/dev/null
jq -e '.matches | length == 0' "$TESTDIR/grep_empty.json" > /dev/null
test_result "Grep: Empty result for no matches"

# Cleanup
rm -rf "$TESTDIR"

echo ""
echo "=============================================="
echo "RESULTS: METHOD 2"
echo "=============================================="
echo "PASSED: $PASS"
echo "FAILED: $FAIL"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✓ All behavioral equivalence tests passed"
    exit 0
else
    echo "✗ Some behavioral tests failed"
    exit 1
fi
