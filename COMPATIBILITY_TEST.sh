#!/bin/bash
# Comprehensive Claude Code compatibility test

echo "=== RUSTYCLAWD COMPATIBILITY VERIFICATION ==="
echo ""

# Test 1: Direct prompt
echo "Test 1: Direct prompt execution"
./target/release/claude "Say 'test 1 passed'" | grep -q "test 1" && echo "✅ PASS" || echo "❌ FAIL"

# Test 2: Print flag
echo "Test 2: Print flag (-p)"
./target/release/claude -p "Say 'test 2 passed'" | grep -q "test 2" && echo "✅ PASS" || echo "❌ FAIL"

# Test 3: Model selection
echo "Test 3: Model selection (--model)"
./target/release/claude --model haiku "Count to 2" | grep -q "2" && echo "✅ PASS" || echo "❌ FAIL"

# Test 4: Tool execution (Bash)
echo "Test 4: Bash tool execution"
./target/release/claude -p "Run command: echo TOOL_TEST_PASSED" | grep -q "TOOL_TEST_PASSED" && echo "✅ PASS" || echo "❌ FAIL"

# Test 5: Tool execution (Write)
echo "Test 5: Write tool execution"
rm -f compat_test.txt
./target/release/claude -p "Create file compat_test.txt with content WRITE_WORKS" > /dev/null 2>&1
[ -f compat_test.txt ] && grep -q "WRITE_WORKS" compat_test.txt && echo "✅ PASS" || echo "❌ FAIL"
rm -f compat_test.txt

# Test 6: Tool execution (Read)
echo "Test 6: Read tool execution"
echo "READ_TEST_CONTENT" > read_test.txt
./target/release/claude -p "Read file read_test.txt" | grep -q "READ_TEST_CONTENT" && echo "✅ PASS" || echo "❌ FAIL"
rm -f read_test.txt

echo ""
echo "=== VERIFICATION COMPLETE ==="
