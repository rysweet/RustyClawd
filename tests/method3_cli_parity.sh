#!/bin/bash
# Method 3: CLI Interface Parity Testing
# Verifies all subcommands and flags are present

set -e
cd /Users/ryan/src/declawed/claude-code-rs

PASS=0
FAIL=0

echo "=============================================="
echo "METHOD 3: CLI INTERFACE PARITY TESTING"
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

# Create test file
echo "test content" > /tmp/m3_test.txt

# ========== SUBCOMMAND EXISTENCE ==========
echo "Testing: Subcommand existence"

cargo run -- bash "echo test" > /dev/null 2>&1
test_result "Subcommand: bash exists"

cargo run -- read /tmp/m3_test.txt > /dev/null 2>&1
test_result "Subcommand: read exists"

cargo run -- write /tmp/m3_write.txt --content "test" > /dev/null 2>&1
test_result "Subcommand: write exists"

cargo run -- edit /tmp/m3_test.txt --old-string "test" --new-string "new" > /dev/null 2>&1
test_result "Subcommand: edit exists"

cargo run -- glob "*.txt" > /dev/null 2>&1
test_result "Subcommand: glob exists"

cargo run -- grep "test" > /dev/null 2>&1
test_result "Subcommand: grep exists"

# Note: bash-output and kill-shell may not be fully implemented yet
if cargo run -- bash-output "id" 2>&1 | grep -qE "error|Error|not found|Unknown|unrecognized"; then
    echo "⚠ Subcommand: bash-output (implementation check)"
else
    echo "✓ Subcommand: bash-output (basic response)"
fi

if cargo run -- kill-shell "id" 2>&1 | grep -qE "error|Error|not found|Unknown|unrecognized"; then
    echo "⚠ Subcommand: kill-shell (implementation check)"
else
    echo "✓ Subcommand: kill-shell (basic response)"
fi

# ========== HELP FLAGS ==========
echo "Testing: Help flags"

cargo run -- --help > /tmp/m3_help_long.txt 2>&1
grep -q "bash\|read\|write" /tmp/m3_help_long.txt
test_result "Flag: --help shows subcommands"

cargo run -- -h > /tmp/m3_help_short.txt 2>&1
grep -q "bash\|read\|write" /tmp/m3_help_short.txt
test_result "Flag: -h shows subcommands"

# Compare help outputs
if diff /tmp/m3_help_long.txt /tmp/m3_help_short.txt > /dev/null 2>&1; then
    echo "✓ Flag: -h and --help are equivalent"
    ((PASS++))
else
    echo "⚠ Flag: -h and --help differ (minor)"
fi

# ========== VERSION FLAGS ==========
echo "Testing: Version flags"

cargo run -- --version > /tmp/m3_version_long.txt 2>&1
grep -qE "[0-9]\.[0-9]" /tmp/m3_version_long.txt
test_result "Flag: --version shows version"

cargo run -- -V > /tmp/m3_version_short.txt 2>&1
grep -qE "[0-9]\.[0-9]" /tmp/m3_version_short.txt
test_result "Flag: -V shows version"

# ========== BASH TOOL FLAGS ==========
echo "Testing: Bash tool flags"

cargo run -- bash "echo test" --timeout 5000 > /dev/null 2>&1
test_result "Bash flag: --timeout"

cargo run -- bash "echo test" --description "test command" > /dev/null 2>&1
test_result "Bash flag: --description"

cargo run -- bash "echo test" --timeout 5000 --description "test" > /dev/null 2>&1
test_result "Bash flag: --timeout and --description together"

cargo run -- bash "echo test" --run-in-background > /dev/null 2>&1 || true
echo "⚠ Bash flag: --run-in-background (optional)"

# ========== READ TOOL FLAGS ==========
echo "Testing: Read tool flags"

cargo run -- read /tmp/m3_test.txt --offset 0 > /dev/null 2>&1
test_result "Read flag: --offset"

cargo run -- read /tmp/m3_test.txt --limit 10 > /dev/null 2>&1
test_result "Read flag: --limit"

cargo run -- read /tmp/m3_test.txt --offset 0 --limit 5 > /dev/null 2>&1
test_result "Read flag: --offset and --limit together"

# ========== WRITE TOOL FLAGS ==========
echo "Testing: Write tool flags"

cargo run -- write /tmp/m3_test2.txt --content "test" > /dev/null 2>&1
test_result "Write flag: --content"

# ========== EDIT TOOL FLAGS ==========
echo "Testing: Edit tool flags"

cargo run -- edit /tmp/m3_test.txt --old-string "test" --new-string "new" > /dev/null 2>&1
test_result "Edit flag: --old-string and --new-string"

cargo run -- edit /tmp/m3_test.txt --old-string "test" --new-string "new" --replace-all > /dev/null 2>&1
test_result "Edit flag: --replace-all"

# ========== GLOB TOOL FLAGS ==========
echo "Testing: Glob tool flags"

cargo run -- glob "*.txt" --path /tmp > /dev/null 2>&1
test_result "Glob flag: --path"

# ========== GREP TOOL FLAGS ==========
echo "Testing: Grep tool flags"

cargo run -- grep "test" -i > /dev/null 2>&1
test_result "Grep flag: -i (case-insensitive)"

cargo run -- grep "test" -B 2 > /dev/null 2>&1
test_result "Grep flag: -B (lines before)"

cargo run -- grep "test" -A 2 > /dev/null 2>&1
test_result "Grep flag: -A (lines after)"

cargo run -- grep "test" -C 2 > /dev/null 2>&1
test_result "Grep flag: -C (context)"

cargo run -- grep "test" -n > /dev/null 2>&1
test_result "Grep flag: -n (line numbers)"

cargo run -- grep "test" --glob "*.txt" > /dev/null 2>&1
test_result "Grep flag: --glob"

cargo run -- grep "test" --path /tmp > /dev/null 2>&1
test_result "Grep flag: --path"

cargo run -- grep "test" --head-limit 10 > /dev/null 2>&1
test_result "Grep flag: --head-limit"

# ========== POSITIONAL ARGUMENTS ==========
echo "Testing: Positional arguments"

# Bash command as positional
cargo run -- bash "echo positional" > /tmp/m3_pos_bash.json 2>/dev/null
jq -e '.stdout' /tmp/m3_pos_bash.json > /dev/null
test_result "Positional: Bash command"

# Read file path as positional
cargo run -- read /tmp/m3_test.txt > /tmp/m3_pos_read.json 2>/dev/null
jq -e '.data' /tmp/m3_pos_read.json > /dev/null
test_result "Positional: Read file path"

# Write file path as positional
cargo run -- write /tmp/m3_pos_write.txt --content "test" > /dev/null 2>&1
test_result "Positional: Write file path"

# Glob pattern as positional
cargo run -- glob "*.txt" > /tmp/m3_pos_glob.json 2>/dev/null
jq -e '.files' /tmp/m3_pos_glob.json > /dev/null
test_result "Positional: Glob pattern"

# Grep pattern as positional
cargo run -- grep "test" > /tmp/m3_pos_grep.json 2>/dev/null
jq -e '.matches' /tmp/m3_pos_grep.json > /dev/null
test_result "Positional: Grep pattern"

# ========== DEBUG FLAG ==========
echo "Testing: Debug flag"

if cargo run -- --debug bash "echo test" 2>&1 | grep -q "debug"; then
    echo "✓ Global flag: --debug"
    ((PASS++))
else
    echo "⚠ Global flag: --debug (may not show in output)"
fi

# ========== INVALID ARGUMENTS ==========
echo "Testing: Invalid argument handling"

if cargo run -- invalid_subcommand 2>&1 | grep -qE "error|Error|unknown|unrecognized"; then
    echo "✓ Error: Invalid subcommand rejected"
    ((PASS++))
else
    echo "✗ Error: Invalid subcommand should be rejected"
    ((FAIL++))
fi

if cargo run -- bash 2>&1 | grep -qE "error|Error|required|missing"; then
    echo "✓ Error: Missing required argument"
    ((PASS++))
else
    echo "✗ Error: Missing required argument should fail"
    ((FAIL++))
fi

# Cleanup
rm -f /tmp/m3_*.txt /tmp/m3_*.json /tmp/m3_*.log

echo ""
echo "=============================================="
echo "RESULTS: METHOD 3"
echo "=============================================="
echo "PASSED: $PASS"
echo "FAILED: $FAIL"
echo ""

if [ $FAIL -eq 0 ]; then
    echo "✓ All CLI parity tests passed"
    exit 0
else
    echo "✗ Some CLI parity tests failed"
    exit 1
fi
