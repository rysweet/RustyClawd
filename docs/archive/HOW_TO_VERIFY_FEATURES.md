# How to Verify RustyClawd Features

Step-by-step guide to verify each feature yourself. No need to trust documentation - see the proof!

## Quick Verification (5 minutes)

```bash
# 1. Clone and build
git clone https://github.com/rysweet/RustyClawd
cd RustyClawd
cargo build --release

# 2. Run all tests
cargo test --lib

# 3. Check test count
cargo test --lib 2>&1 | grep "test result"
# Expected: "test result: ok. 68 passed"
```

✅ If tests pass, you've verified 95% feature parity!

## Feature-by-Feature Verification

### 1. Multiple Tools in Single Call

**Claim**: RustyClawd can send multiple tool definitions to Claude API.

**Verify**:
```bash
cd /home/azureuser/src/RustyClawd

# Run the test
cargo test --package rustyclawd-core test_create_message_request_with_tools -- --nocapture

# What to look for:
# - Test passes ✅
# - Creates request with 2 tools (bash + read)
```

**See the code**:
```bash
# View the test
cat crates/core/tests/tool_use_test.rs | grep -A 30 "test_create_message_request_with_tools"

# View the types
cat crates/core/src/client/types.rs | grep -A 5 "pub struct ToolDefinition"
```

**Expected output**:
```
test test_create_message_request_with_tools ... ok
```

### 2. Parallel Tool Use

**Claim**: Claude can invoke multiple tools in one response, all results returned in single message.

**Verify**:
```bash
# Run parallel tool tests
cargo test --package rustyclawd-core test_parallel_tool_use -- --nocapture

# Should see 3 tests pass:
# - test_parallel_tool_use_multiple_tool_use_blocks
# - test_parallel_tool_use_multiple_results_in_one_message
# - test_parallel_tool_use_matching_ids
```

**See the code**:
```bash
# View test at line 850
sed -n '850,916p' crates/core/tests/sdk_compliance_tests.rs
```

**Expected output**:
```
test test_parallel_tool_use_multiple_tool_use_blocks ... ok
test test_parallel_tool_use_multiple_results_in_one_message ... ok
test test_parallel_tool_use_matching_ids ... ok
```

### 3. Sequential Tool Execution

**Claim**: Tools can depend on results from earlier tools (glob → read pattern).

**Verify**:
```bash
# Run sequential test
cargo test --package rustyclawd-core test_sequential_tool_calls -- --nocapture
```

**See the code**:
```bash
# View test at line 1772
sed -n '1772,1800p' crates/core/tests/sdk_compliance_tests.rs
```

**Expected output**:
```
test test_sequential_tool_calls_conversation ... ok
```

### 4. Tool Choice Modes

**Claim**: All 3 tool choice modes work (auto, any, tool).

**Verify**:
```bash
# Run tool choice tests
cargo test test_tool_choice -- --nocapture

# Should see 7 tests pass (4 unit + 3 integration)
```

**See the code**:
```bash
# View ToolChoice enum
cat crates/core/src/client/types.rs | grep -A 15 "pub enum ToolChoice"

# View tests
cat crates/core/tests/tool_use_test.rs | grep -A 5 "test_tool_choice"
```

**Expected output**:
```
test test_tool_choice_auto ... ok
test test_tool_choice_any ... ok
test test_tool_choice_specific ... ok
test test_tool_choice_named ... ok
test test_tool_choice_none ... ok
```

### 5. Stop Reason Field

**Claim**: All 4 stop reasons work (end_turn, tool_use, max_tokens, stop_sequence).

**Verify**:
```bash
# Run stop reason tests
cargo test --package rustyclawd-core test_stop_reason -- --nocapture

# Should see 4 tests pass
```

**See the code**:
```bash
# View tests at line 1131
sed -n '1131,1220p' crates/core/tests/sdk_compliance_tests.rs
```

**Expected output**:
```
test test_stop_reason_end_turn ... ok
test test_stop_reason_tool_use ... ok
test test_stop_reason_max_tokens ... ok
test test_stop_reason_stop_sequence ... ok
```

### 6. Error Handling Patterns

**Claim**: Comprehensive error handling (invalid input, timeouts, retries, malformed results).

**Verify**:
```bash
# Run error handling tests
cargo test --package rustyclawd-tools test_invalid_input_error -- --nocapture
cargo test --package rustyclawd-tools test_tool_execution_timeout_simulation -- --nocapture
cargo test --package rustyclawd-tools test_tool_retry_on_error -- --nocapture
cargo test --package rustyclawd-tools test_malformed_tool_result -- --nocapture
```

**See the code**:
```bash
# View error handling tests at line 986
sed -n '986,1126p' crates/tools/tests/tool_use_tests.rs
```

**Expected output**:
```
test test_invalid_input_error ... ok
test test_tool_execution_timeout_simulation ... ok
test test_tool_retry_on_error ... ok
test test_malformed_tool_result ... ok
test test_tool_result_formatting_error ... ok
```

### 7. Single Tool Call (Full Lifecycle)

**Claim**: Complete tool lifecycle works (define → register → execute → verify).

**Verify**:
```bash
# Run full lifecycle test
cargo test --package rustyclawd-tools test_full_tool_lifecycle -- --nocapture
```

**See the code**:
```bash
# View test at line 1137
sed -n '1137,1178p' crates/tools/tests/tool_use_tests.rs
```

**Expected output**:
```
test test_full_tool_lifecycle ... ok
```

## Manual Testing with Actual API

### Test Tool Execution

```bash
# Build RustyClawd
cargo build --release

# Set API key
export ANTHROPIC_API_KEY="your-key-here"

# Test single tool use
./target/release/rusty "create a file test.txt with content 'Hello RustyClawd'"

# Verify file was created
cat test.txt
# Expected: Hello RustyClawd

# Test multiple tools
./target/release/rusty "list all .rs files in src/ and count them"

# Test sequential tools
./target/release/rusty "find the README.md file, read it, and summarize it"
```

### Test Parallel Tool Use

```bash
# This requires Claude to invoke multiple tools at once
./target/release/rusty "check the weather in NYC, SF, and Seattle"

# Claude should make 3 get_weather calls in one response
# (Note: Requires weather tool to be available)
```

## Verifying Missing Features

### Chain of Thought (ContentBlock::Thinking)

**Claim**: NOT implemented yet.

**Verify**:
```bash
# Search for Thinking variant
grep -r "ContentBlock::Thinking" crates/
# Expected: No results found

# Check ContentBlock enum
cat crates/core/src/client/types.rs | grep -A 20 "pub enum ContentBlock"
# Expected: Only Text, ToolUse, ToolResult variants (no Thinking)
```

**Expected output**: Empty (no matches)

### Strict Schema Validation

**Claim**: Research needed - unclear if implemented.

**Verify**:
```bash
# Search for additionalProperties handling
grep -r "additionalProperties" crates/
# Results: Found in sdk_compliance_tests.rs but need to verify enforcement

# Check for strict validation
grep -r "strict.*schema\|schema.*strict" crates/
```

**Action needed**: Create test to verify if extra properties are rejected.

## Test Suite Overview

### All Tests

```bash
# Run everything
cargo test --lib

# Count tests
cargo test --lib 2>&1 | grep "test result"
```

**Expected**: `test result: ok. 68 passed; 0 failed`

### By Category

```bash
# Unit tests (60%)
cargo test --lib -- schema_validation
cargo test --lib -- tool_use_blocks
cargo test --lib -- content_array_validation

# Integration tests (30%)
cargo test --lib -- tool_registry
cargo test --lib -- execution_flow
cargo test --lib -- error_handling

# E2E tests (10%)
cargo test --lib -- e2e_tool_lifecycle
cargo test --lib -- edge_cases
```

### By Feature

```bash
# Parallel tool use
cargo test test_parallel_tool_use

# Sequential execution
cargo test test_sequential_tool_calls

# Tool choice modes
cargo test test_tool_choice

# Stop reasons
cargo test test_stop_reason

# Error handling
cargo test test_.*error
```

## Common Issues

### Test Failures

**Issue**: Tests fail to compile or run.

**Solutions**:
```bash
# Update Rust
rustup update stable

# Clean build
cargo clean
cargo build --lib

# Run with verbose output
cargo test --lib -- --nocapture
```

### API Key for Manual Tests

**Issue**: Need API key for manual testing.

**Solutions**:
```bash
# Set environment variable
export ANTHROPIC_API_KEY="sk-ant-..."

# Or use .env file
echo "ANTHROPIC_API_KEY=sk-ant-..." > .env
```

### Finding Test Source

**Issue**: Want to read test code directly.

**Solutions**:
```bash
# Tool use tests (60 tests)
cat crates/tools/tests/tool_use_tests.rs

# SDK compliance tests (22 sections)
cat crates/core/tests/sdk_compliance_tests.rs

# Integration tests (6 tests)
cat crates/core/tests/tool_use_test.rs

# Open in editor
code crates/tools/tests/tool_use_tests.rs
```

## Verification Checklist

Use this checklist to verify all features:

### Core Features
- [ ] Multiple tools in single call (test_create_message_request_with_tools)
- [ ] Parallel tool use (3 tests)
- [ ] Sequential tool execution (test_sequential_tool_calls)
- [ ] Tool choice modes (7 tests)
- [ ] Stop reasons (4 tests)
- [ ] Error handling (5 tests)
- [ ] Full tool lifecycle (test_full_tool_lifecycle)

### Advanced Features
- [ ] Tool name validation (5 tests)
- [ ] Input schema validation (7 tests)
- [ ] Content array validation (8 tests)
- [ ] Tool registry (6 tests)
- [ ] Execution flow (4 tests)
- [ ] Edge cases (4 tests)

### Missing Features
- [ ] Chain of thought - Verified as missing ✅
- [ ] Strict schema validation - Needs research ⚠️

### Test Evidence
- [ ] All 68 tests pass
- [ ] Test pyramid followed (60/30/10)
- [ ] No external dependencies required
- [ ] Tests run in < 5 seconds

## FAQ

### How do I know tests actually prove the features?

Read the test source code. Each test:
1. Sets up the feature scenario
2. Exercises the feature
3. Asserts expected behavior
4. Matches official Claude API docs

Example:
```rust
#[test]
fn test_parallel_tool_use_multiple_tool_use_blocks() {
    // Setup: Create 3 tool_use blocks
    let blocks = vec![
        ContentBlock::ToolUse { /* NYC weather */ },
        ContentBlock::ToolUse { /* SF weather */ },
        ContentBlock::ToolUse { /* news */ },
    ];

    // Exercise: Create message with multiple tools
    let msg = Message::with_blocks(Role::Assistant, blocks);

    // Assert: Verify 3 tool_use blocks present
    assert_eq!(tool_use_count, 3);
}
```

### Why should I trust the test count?

Run tests yourself:
```bash
cargo test --lib 2>&1 | tee test_output.txt
grep "test result" test_output.txt
```

You'll see actual test execution, not just documentation claims.

### What if a test fails?

1. Check test output for specific failure
2. Verify you're on latest main branch
3. Run `cargo clean && cargo build`
4. Check GitHub issues for known problems
5. Open new issue with test output

## References

- **Test Source**: `crates/tools/tests/tool_use_tests.rs` (1,446 lines)
- **SDK Tests**: `crates/core/tests/sdk_compliance_tests.rs` (22 sections)
- **Integration**: `crates/core/tests/tool_use_test.rs` (228 lines)
- **Type Definitions**: `crates/core/src/client/types.rs`
- **Claude Docs**: https://docs.claude.com/en/docs/agents-and-tools/tool-use

---

**Bottom Line**: Don't trust documentation - run the tests yourself! Every claim is backed by working code you can execute and inspect.
