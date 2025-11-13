# Educational Error Messages Implementation

## Problem
When Claude makes tool calls with missing required fields, the error messages were not helpful:
```
Error: Failed to parse Write tool parameters: missing field `content`
```

This didn't provide enough information for Claude to understand what went wrong and how to fix it.

## Solution
Created a `create_schema_error()` function that returns structured, educational error messages with:

1. **Clear error description** - What went wrong
2. **Required fields list** - All fields that MUST be provided
3. **Optional fields list** - Fields that can be included but aren't required
4. **Concrete example** - A valid JSON example showing correct usage
5. **Helpful guidance** - Plain text explanation

## Implementation

### File Modified
- `/Users/ryan/src/declawed/claude-code-rs/crates/cli/src/tool_executor.rs`

### Changes Made

1. Added `json` macro to imports
2. Created `create_schema_error()` helper function with schemas for all 6 tools:
   - Write (file_path, content)
   - Read (file_path, offset?, limit?)
   - Edit (file_path, old_string, new_string, replace_all?)
   - Bash (command, timeout?, description?, run_in_background?, dangerouslyDisableSandbox?)
   - Glob (pattern, path?)
   - Grep (pattern, path?, output_mode?, ...)

3. Updated all 6 tool execution functions to use the helper:
   - `execute_bash_tool()`
   - `execute_read_tool()`
   - `execute_write_tool()`
   - `execute_edit_tool()`
   - `execute_glob_tool()`
   - `execute_grep_tool()`

## Example Error Message

When Claude calls Write without 'content':

```json
{
  "error": "Failed to parse Write tool parameters",
  "details": "missing field `content` at line 1 column 36",
  "required_fields": [
    "file_path",
    "content"
  ],
  "optional_fields": [],
  "example": {
    "file_path": "/absolute/path/to/file.txt",
    "content": "The content to write to the file"
  },
  "help": "The Write tool requires these fields: file_path, content. Please ensure all required fields are provided with the correct types."
}
```

## Testing

Created comprehensive test suite in `/Users/ryan/src/declawed/claude-code-rs/crates/cli/tests/tool_executor_tests.rs`:

- ✅ Write tool missing content field
- ✅ Write tool missing file_path field
- ✅ Bash tool missing command field
- ✅ Read tool missing file_path
- ✅ Edit tool missing fields
- ✅ Glob tool missing pattern
- ✅ Grep tool missing pattern
- ✅ Error includes help text
- ✅ Error includes example JSON

All 9 tests pass!

## Benefits

1. **Self-correcting** - Claude can learn from errors and retry correctly
2. **Reduces iteration cycles** - No more back-and-forth asking for clarification
3. **Educational** - Each error teaches the correct schema
4. **Consistent** - All tools provide the same level of detail
5. **Actionable** - Clear examples show exactly what to do

## Verification

```bash
# Run tool executor tests
cargo test --package rustyclawd-cli --test tool_executor_tests

# Results: ok. 9 passed; 0 failed; 0 ignored
```

## Impact

This fix will:
- Reduce repeated tool call failures
- Help Claude self-correct when making mistakes
- Make debugging easier for developers
- Provide clear documentation through error messages
- Improve overall system reliability
