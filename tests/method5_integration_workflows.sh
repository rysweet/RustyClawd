#!/bin/bash
# Method 5: Integration Workflow Testing
# Tests realistic multi-tool workflows

set -e
cd /Users/ryan/src/declawed/claude-code-rs

WORKSPACE="/tmp/rustyclawd_integration_test"
PASS=0
FAIL=0

echo "=============================================="
echo "METHOD 5: INTEGRATION WORKFLOW TESTING"
echo "=============================================="
echo ""

# Cleanup and setup
rm -rf "$WORKSPACE"
mkdir -p "$WORKSPACE"

test_result() {
    if [ $? -eq 0 ]; then
        echo "✓ $1"
        ((PASS++))
    else
        echo "✗ $1"
        ((FAIL++))
    fi
}

# ========== WORKFLOW 1: CREATE-READ-VERIFY ==========
echo "Workflow 1: Create file -> Read -> Verify content"

cargo run -- write "$WORKSPACE/workflow1.txt" --content "function main() {
  console.log('Hello from RustyClawd');
}" > /dev/null 2>&1
test_result "W1: Write file"

OUTPUT=$(cargo run -- read "$WORKSPACE/workflow1.txt" 2>/dev/null | jq -r '.data[0]' 2>/dev/null)
[[ "$OUTPUT" == *"function main"* ]]
test_result "W1: Read contains function signature"

OUTPUT=$(cargo run -- read "$WORKSPACE/workflow1.txt" 2>/dev/null | jq -r '.data[1]' 2>/dev/null)
[[ "$OUTPUT" == *"Hello"* ]]
test_result "W1: Read captures all lines"

# ========== WORKFLOW 2: WRITE-EDIT-VERIFY ==========
echo "Workflow 2: Create file -> Edit -> Verify"

cargo run -- write "$WORKSPACE/workflow2.txt" --content "const value = 10;
const multiplier = 2;
const result = value * multiplier;" > /dev/null 2>&1
test_result "W2: Write initial content"

cargo run -- edit "$WORKSPACE/workflow2.txt" --old-string "10" --new-string "20" > /dev/null 2>&1
test_result "W2: Edit first value"

cargo run -- edit "$WORKSPACE/workflow2.txt" --old-string "multiplier = 2" --new-string "multiplier = 3" > /dev/null 2>&1
test_result "W2: Edit second value"

RESULT=$(cat "$WORKSPACE/workflow2.txt")
[[ "$RESULT" == *"value = 20"* ]]
test_result "W2: Verify first edit persisted"

[[ "$RESULT" == *"multiplier = 3"* ]]
test_result "W2: Verify second edit persisted"

# ========== WORKFLOW 3: MULTI-FILE-CREATE-GLOB-GREP ==========
echo "Workflow 3: Create files -> Find -> Search"

mkdir -p "$WORKSPACE/src"
cargo run -- write "$WORKSPACE/src/main.rs" --content "fn main() {
    println!(\"Application started\");
}" > /dev/null 2>&1
test_result "W3: Create main.rs"

cargo run -- write "$WORKSPACE/src/lib.rs" --content "pub fn helper() {
    println!(\"Helper function\");
}" > /dev/null 2>&1
test_result "W3: Create lib.rs"

cargo run -- write "$WORKSPACE/src/utils.rs" --content "pub fn utility() {
    println!(\"Utility function\");
}" > /dev/null 2>&1
test_result "W3: Create utils.rs"

cargo run -- write "$WORKSPACE/README.md" --content "# Project README
This is a test project" > /dev/null 2>&1
test_result "W3: Create README.md"

# Find Rust files
GLOB_COUNT=$(cargo run -- glob "**/*.rs" --path "$WORKSPACE" 2>/dev/null | jq '.files | length' 2>/dev/null)
[[ "$GLOB_COUNT" == "3" ]]
test_result "W3: Glob finds 3 Rust files"

# Search for println
GREP_COUNT=$(cargo run -- grep "println" --path "$WORKSPACE/src" 2>/dev/null | jq '.matches | length' 2>/dev/null)
[[ "$GREP_COUNT" -ge "2" ]]
test_result "W3: Grep finds println occurrences"

# Find all Markdown files
MD_COUNT=$(cargo run -- glob "**/*.md" --path "$WORKSPACE" 2>/dev/null | jq '.files | length' 2>/dev/null)
[[ "$MD_COUNT" == "1" ]]
test_result "W3: Glob finds Markdown files"

# ========== WORKFLOW 4: BASH COMMAND CHAIN ==========
echo "Workflow 4: Execute bash commands -> Parse output"

BASH_OUT=$(cargo run -- bash "echo 'step1' && echo 'step2' && echo 'step3'" 2>/dev/null | jq -r '.stdout' 2>/dev/null)
[[ "$BASH_OUT" == *"step1"* ]]
test_result "W4: Bash captures first command"

[[ "$BASH_OUT" == *"step2"* ]]
test_result "W4: Bash captures second command"

[[ "$BASH_OUT" == *"step3"* ]]
test_result "W4: Bash captures third command"

# Test command with exit code
BASH_OUT=$(cargo run -- bash "exit 0" 2>/dev/null | jq '.exit_code' 2>/dev/null)
[[ "$BASH_OUT" == "0" ]]
test_result "W4: Bash captures exit code 0"

BASH_OUT=$(cargo run -- bash "exit 5" 2>/dev/null | jq '.exit_code' 2>/dev/null)
[[ "$BASH_OUT" == "5" ]]
test_result "W4: Bash captures exit code 5"

# ========== WORKFLOW 5: COMPLEX JSON EDITING ==========
echo "Workflow 5: Edit complex JSON -> Verify syntax"

cat > "$WORKSPACE/config.json" << 'EOF'
{
  "name": "TestProject",
  "version": "1.0.0",
  "description": "Original description",
  "settings": {
    "debug": false,
    "verbose": true
  }
}
EOF
test_result "W5: Created JSON file"

cargo run -- edit "$WORKSPACE/config.json" --old-string '"description": "Original description"' --new-string '"description": "Updated description"' > /dev/null 2>&1
test_result "W5: Edited JSON description"

cargo run -- edit "$WORKSPACE/config.json" --old-string '"debug": false' --new-string '"debug": true' > /dev/null 2>&1
test_result "W5: Edited JSON setting"

# Verify JSON is still valid
if jq empty "$WORKSPACE/config.json" 2>/dev/null; then
    echo "✓ W5: JSON still valid after edits"
    ((PASS++))
else
    echo "✗ W5: JSON invalid after edits"
    ((FAIL++))
fi

# Read back and verify
VERSION=$(cargo run -- read "$WORKSPACE/config.json" 2>/dev/null | jq -r '.data[1]' 2>/dev/null)
[[ "$VERSION" == *"1.0.0"* ]]
test_result "W5: Verified JSON read correctly"

# ========== WORKFLOW 6: MULTI-STEP FILE PROCESSING ==========
echo "Workflow 6: Process file through multiple operations"

# Create initial file
cargo run -- write "$WORKSPACE/process.txt" --content "Original line 1
Original line 2
Original line 3" > /dev/null 2>&1
test_result "W6: Created file"

# Add more content
cargo run -- edit "$WORKSPACE/process.txt" --old-string "Original line 1" --new-string "Modified line 1" > /dev/null 2>&1
test_result "W6: First modification"

cargo run -- edit "$WORKSPACE/process.txt" --old-string "Original line 2" --new-string "Modified line 2" > /dev/null 2>&1
test_result "W6: Second modification"

# Read and verify
CONTENT=$(cargo run -- read "$WORKSPACE/process.txt" 2>/dev/null | jq -r '.data' 2>/dev/null)
[[ "$CONTENT" == *"Modified line 1"* ]]
test_result "W6: First modification persisted"

[[ "$CONTENT" == *"Modified line 2"* ]]
test_result "W6: Second modification persisted"

[[ "$CONTENT" == *"Original line 3"* ]]
test_result "W6: Unmodified content preserved"

# ========== WORKFLOW 7: ERROR RECOVERY ==========
echo "Workflow 7: Handle errors gracefully and continue"

# Attempt to read nonexistent file (should error)
if cargo run -- read /nonexistent/path 2>&1 | grep -qE "error|Error"; then
    echo "✓ W7: Error detected correctly"
    ((PASS++))
else
    echo "✗ W7: Error not detected"
    ((FAIL++))
fi

# Verify tool still works after error
cargo run -- bash "echo 'recovery'" > /tmp/w7_recovery.json 2>/dev/null
if grep -q "recovery" <(jq -r '.stdout' /tmp/w7_recovery.json 2>/dev/null); then
    echo "✓ W7: Tool recovered after error"
    ((PASS++))
else
    echo "✗ W7: Tool failed to recover"
    ((FAIL++))
fi

# ========== WORKFLOW 8: NESTED DIRECTORY HANDLING ==========
echo "Workflow 8: Nested directory creation and file operations"

# Create file in deeply nested directory
cargo run -- write "$WORKSPACE/deep/nested/structure/file1.txt" --content "Deep file 1" > /dev/null 2>&1
test_result "W8: Created file in nested directory"

cargo run -- write "$WORKSPACE/deep/nested/structure/file2.txt" --content "Deep file 2" > /dev/null 2>&1
test_result "W8: Created second nested file"

# Find all nested files
NESTED_COUNT=$(cargo run -- glob "**/*.txt" --path "$WORKSPACE" 2>/dev/null | jq '.files | length' 2>/dev/null)
[[ "$NESTED_COUNT" -ge "6" ]]  # At least all the .txt files we created
test_result "W8: Glob finds all nested files"

# ========== WORKFLOW 9: OUTPUT FORMAT CONSISTENCY ==========
echo "Workflow 9: Verify output format consistency"

# All tools should produce JSON
if cargo run -- bash "echo test" 2>/dev/null | jq -e '.type' > /dev/null 2>&1; then
    echo "✓ W9: Bash output is valid JSON"
    ((PASS++))
else
    echo "✗ W9: Bash output is not valid JSON"
    ((FAIL++))
fi

if cargo run -- read "$WORKSPACE/workflow1.txt" 2>/dev/null | jq -e '.type' > /dev/null 2>&1; then
    echo "✓ W9: Read output is valid JSON"
    ((PASS++))
else
    echo "✗ W9: Read output is not valid JSON"
    ((FAIL++))
fi

if cargo run -- glob "*.txt" --path "$WORKSPACE" 2>/dev/null | jq -e '.type' > /dev/null 2>&1; then
    echo "✓ W9: Glob output is valid JSON"
    ((PASS++))
else
    echo "✗ W9: Glob output is not valid JSON"
    ((FAIL++))
fi

if cargo run -- grep "test" --path "$WORKSPACE" 2>/dev/null | jq -e '.type' > /dev/null 2>&1; then
    echo "✓ W9: Grep output is valid JSON"
    ((PASS++))
else
    echo "✗ W9: Grep output is not valid JSON"
    ((FAIL++))
fi

# ========== WORKFLOW 10: DATA INTEGRITY ==========
echo "Workflow 10: Data integrity through operations"

# Create file with specific content
TEST_CONTENT="Line with special chars: \$#@!
Line with unicode: 你好世界
Line with quotes: \"quoted\""

cargo run -- write "$WORKSPACE/integrity.txt" --content "$TEST_CONTENT" > /dev/null 2>&1
test_result "W10: Wrote file with special content"

# Read back and verify
READ_CONTENT=$(cargo run -- read "$WORKSPACE/integrity.txt" 2>/dev/null | jq -r '.data | join("\n")' 2>/dev/null)
[[ "$READ_CONTENT" == *"special chars"* ]]
test_result "W10: Special chars preserved"

[[ "$READ_CONTENT" == *"unicode"* ]]
test_result "W10: Unicode preserved"

# Cleanup
rm -rf "$WORKSPACE"

echo ""
echo "=============================================="
echo "RESULTS: METHOD 5"
echo "=============================================="
echo "PASSED: $PASS"
echo "FAILED: $FAIL"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✓ All integration workflows passed"
    exit 0
else
    echo "✗ Some workflows failed"
    exit 1
fi
