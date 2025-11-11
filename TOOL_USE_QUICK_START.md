# Tool Use Test Suite - Quick Start

## Run Tests

```bash
cd /Users/ryan/src/declawed/claude-code-rs/crates/tools
cargo test --test tool_use_tests
```

**Result**: 53 tests, 100% passing

## Test File Location

```
/Users/ryan/src/declawed/claude-code-rs/crates/tools/tests/tool_use_tests.rs
```

## Test Structure (1400+ lines)

### Module Organization

```
├── Type Definitions (Tool Use Models)
├── Unit Tests - Schema Validation (11 tests)
├── Unit Tests - Tool Use Blocks (5 tests)
├── Unit Tests - Content Array (8 tests)
├── Unit Tests - Tool Choice (4 tests)
├── Unit Tests - Edge Cases (4 tests)
├── Integration Tests - Tool Registry (6 tests)
├── Integration Tests - Execution Flow (4 tests)
├── Integration Tests - Error Handling (5 tests)
├── E2E Tests - Tool Lifecycle (6 tests)
```

## Critical Concepts Tested

### 1. Tool Name Validation

```rust
// Valid names: weather-api, get_temperature, HTTP2Client
// Invalid: "weather api", "weather.api", "weather@api"
// Range: 1-64 characters
// Pattern: ^[a-zA-Z0-9_-]{1,64}$
```

### 2. Content Array Ordering (CRITICAL)

```
Correct:
  1. tool_use block
  2. tool_result block (matching ID)
  3. text content (optional)

Wrong:
  - text before results (400 error)
  - mismatched tool_use/result IDs
  - multiple results for same tool_use
```

### 3. Tool Result Structure

```json
{
  "tool_use_id": "call_123",
  "content": "result data",
  "is_error": false
}
```

### 4. Error Handling

- Invalid inputs → Error result
- Timeouts → Retry 2-3 times
- Tool not found → Error
- Malformed results → 400 error

## Test Categories

| Category | Tests | Coverage | Status |
|----------|-------|----------|--------|
| Schema Validation | 11 | Names, descriptions, schemas | ✓ |
| Tool Blocks | 5 | Use/result structure | ✓ |
| Content Formatting | 8 | Ordering, sequencing | ✓ |
| Tool Registry | 6 | Registration, execution | ✓ |
| Execution Flow | 4 | Processing pipeline | ✓ |
| Error Handling | 5 | Failures, timeouts, retries | ✓ |
| E2E Lifecycle | 6 | Full workflows | ✓ |
| Edge Cases | 4 | Boundaries, nulls, large data | ✓ |
| **Tool Choice** | 4 | auto, any, named, none | ✓ |

## Key Requirements Validated

### Requirement 1: Tool Name Format
```
- Pattern: ^[a-zA-Z0-9_-]{1,64}$
- Test: test_tool_name_valid ✓
```

### Requirement 2: Description Quality
```
- Minimum: 20 characters (3-4 sentences recommended)
- Test: test_tool_definition_adequate_description ✓
```

### Requirement 3: Content Block Ordering
```
- tool_result MUST follow tool_use immediately
- text MUST come AFTER all tool_results
- Test: test_content_array_text_after_results ✓
```

### Requirement 4: Error Handling
```
- Set is_error: true on failures
- Support retry (2-3 times)
- Test: test_tool_retry_on_error ✓
```

### Requirement 5: Parallel Execution
```
- Multiple tools in single message
- Test: test_parallel_tool_execution ✓
```

## Example Test

```rust
#[test]
fn test_tool_name_valid() {
    // Valid names accepted
    assert!(ToolName::new("weather-api").is_ok());
    assert!(ToolName::new("get_temperature").is_ok());
    assert!(ToolName::new("HTTP2Client").is_ok());

    // Invalid rejected
    assert!(ToolName::new("weather api").is_err()); // space
    assert!(ToolName::new("weather@api").is_err()); // @
}
```

## Testing Pyramid

```
        E2E (10%)
       /    \
      /      \
    /        \
  Integration (30%)
   /          \
  /            \
Unit (60%)
```

- **Unit (32 tests)**: Fast, isolated validation logic
- **Integration (15 tests)**: Component interactions
- **E2E (6 tests)**: Full workflows

## Coverage By Concern

### Happy Path
- Valid tool creation ✓
- Successful execution ✓
- Proper result formatting ✓
- Multiple tools ✓

### Error Path
- Invalid names ✓
- Invalid inputs ✓
- Tool not found ✓
- Timeouts ✓
- Retry logic ✓

### Edge Cases
- Empty input ✓
- Large data (1000+ items) ✓
- Deep nesting (4+ levels) ✓
- Null values ✓

### Compliance
- Tool name regex ✓
- Content ordering ✓
- ID matching ✓
- Error flags ✓

## Implementation Checklist

For implementing tool use in production:

- [ ] Copy type definitions from test file
- [ ] Implement `ToolRegistry` for tool management
- [ ] Add `ContentArray` validator for message safety
- [ ] Implement `ToolName` validation
- [ ] Add JSON Schema validation for inputs
- [ ] Support `tool_choice` parameter
- [ ] Implement retry logic
- [ ] Add streaming support

## Common Mistakes Prevented By Tests

1. ✓ Invalid tool names (spaces, dots, special chars)
2. ✓ Text before tool results (causes 400 error)
3. ✓ Mismatched tool_use/result IDs
4. ✓ Duplicate tool registration
5. ✓ Missing error flags on failures
6. ✓ Insufficient tool descriptions
7. ✓ Unhandled timeouts
8. ✓ Large data processing

## Performance

- All tests complete: < 1 second
- No async overhead in tests
- Streaming modeled but not async (placeholder)

## Next Steps

### Read These Files
1. `/Users/ryan/src/declawed/claude-code-rs/TOOL_USE_TEST_SUITE.md` - Full documentation
2. `/Users/ryan/src/declawed/claude-code-rs/crates/tools/tests/tool_use_tests.rs` - Test code

### Implement These
1. Production `ToolRegistry`
2. `ContentFormatter` for safe messages
3. JSON Schema validator
4. Streaming pipeline

### Test These
```bash
# Run full suite
cargo test --test tool_use_tests

# Run specific module
cargo test tool_use_tests::schema_validation

# Run with output
cargo test --test tool_use_tests -- --nocapture
```

## Troubleshooting

**Tests not found?**
```bash
cd /Users/ryan/src/declawed/claude-code-rs/crates/tools
cargo test --test tool_use_tests -- --list
```

**Need to add a test?**
1. Add `#[test]` function to appropriate module
2. Follow pattern: arrange-act-assert
3. Test ONE behavior
4. Run: `cargo test --test tool_use_tests`

**Test failing?**
1. Check test expectations
2. Verify requirement alignment
3. Add missing implementation
4. Re-run to verify

---

**Status**: All 53 tests passing ✓
**Location**: `/Users/ryan/src/declawed/claude-code-rs/crates/tools/tests/tool_use_tests.rs`
**Command**: `cd /Users/ryan/src/declawed/claude-code-rs/crates/tools && cargo test --test tool_use_tests`
