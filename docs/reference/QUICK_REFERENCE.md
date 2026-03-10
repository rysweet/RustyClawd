# Issue #250: Quick Reference Guide

## Pattern Syntax

### Current (Already Works)
- Exact match: `mcp__filesystem__read_file`
- All MCP tools: `mcp__.*`
- All in context: `*`
- Alternation: `Edit|Write`

### New (This Issue)
- Server wildcard: `mcp__filesystem__*`

## Matching Examples - Filesystem Server

Pattern: `mcp__filesystem__*`

Matches:
- `mcp__filesystem__read_file` ✓
- `mcp__filesystem__write_file` ✓
- `mcp__filesystem__delete` ✓

Does NOT match:
- `mcp__memory__store` ✗
- `mcp__memory__read` ✗
- `Bash` ✗

## Specificity Priority

Most specific pattern wins:
1. `mcp__filesystem__read` (EXACT - highest priority)
2. `mcp__filesystem__*` (WILDCARD)
3. `mcp__.*` (GENERAL - lowest priority)

## Common Use Cases

### Block entire server
```json
{
  "deny": ["mcp__filesystem__*"]
}
```

### Allow one server, block all others
```json
{
  "allow": ["mcp__memory__*"],
  "deny": ["mcp__.*"]
}
```

### Block everything except one tool
```json
{
  "deny": ["mcp__filesystem__*"],
  "allow": ["mcp__filesystem__read_file"]
}
```

## Implementation Details

File: `crates/cli/src/hooks/types.rs`

Method: `HookMatcher::matches()`

Pattern Recognition:
- Starts with "mcp__"
- Ends with "__*"
- Has exactly 2 "__" separators

## Files Changed

1. `crates/cli/src/hooks/types.rs`
   - Update matches() method
   - Update deserialize() method
   - Add is_mcp_server_wildcard() helper

2. `crates/cli/tests/hooks_doc_tests.rs`
   - Add 5+ new test cases
   - Un-ignore test_matcher_mcp_full_pattern

## Success Criteria

✓ Pattern `mcp__filesystem__*` matches all filesystem tools
✓ Specific patterns take precedence
✓ All tests pass
✓ Backwards compatible
✓ No performance impact
