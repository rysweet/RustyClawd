# Issue #250: Wildcard Syntax for MCP Tool Permissions

## Deliverables

This directory contains the complete architectural design for implementing wildcard syntax `mcp__<server>__*` for MCP tool permissions.

### Documents

1. **DESIGN.md** (Primary Design Specification)
   - Complete problem statement and solution overview
   - Detailed pattern format specification
   - Matcher architecture and implementation strategy
   - Priority rules for specificity matching
   - Comprehensive testing strategy with 5 test cases
   - Integration points and backwards compatibility analysis
   - Implementation checklist and success criteria

2. **IMPLEMENTATION_SUMMARY.md** (Executive Summary)
   - High-level overview of changes
   - Before/after examples
   - Key implementation locations
   - Pattern matching order fixes
   - Test coverage summary
   - Priority rules explanation
   - Configuration examples
   - Files modified with line counts

3. **QUICK_REFERENCE.md** (Quick Start Guide)
   - Pattern syntax examples
   - Matching examples with test cases
   - Specificity priority chart
   - Common use cases
   - Implementation details
   - Files changed summary

## Key Design Decisions

### Problem
Users cannot efficiently allow or deny all tools from a specific MCP server without creating individual entries for each tool.

### Solution
Introduce pattern `mcp__<server>__*` to match all tools from a specific server.

### Implementation Approach
- **Location**: `crates/cli/src/hooks/types.rs`
- **Method**: Enhance `HookMatcher::matches()` with server wildcard pattern detection
- **Strategy**: Fix pattern matching order to handle specific cases before generic ones
- **Bonus**: Fixes existing ignored test `test_matcher_mcp_full_pattern`

### Priority Rules
1. **Exact match** - `mcp__server__tool` (HIGHEST)
2. **Server wildcard** - `mcp__server__*`
3. **All MCP tools** - `mcp__.*` (LOWEST)

## Implementation Plan

### Phase 1: Core Logic
1. Add `is_mcp_server_wildcard()` helper function
2. Update `HookMatcher::matches()` with new pattern check
3. Reorder pattern checks for correct priority

### Phase 2: Deserialization
1. Update `HookMatcher::deserialize()` to recognize wildcards

### Phase 3: Testing
1. Add 8-10 comprehensive unit tests
2. Fix ignored test `test_matcher_mcp_full_pattern`
3. Add edge case tests

### Phase 4: Validation
1. Run full test suite
2. Verify backwards compatibility
3. Document configuration examples

## Pattern Format

```rust
// Matches all tools from specific server
"mcp__filesystem__*"    // Matches: mcp__filesystem__read, mcp__filesystem__write, etc.
"mcp__memory__*"        // Matches: mcp__memory__store, mcp__memory__read, etc.

// Still supported (backwards compatible)
"mcp__filesystem__read"         // Exact match
"mcp__.*"                       // All MCP tools
"*"                            // Match everything
"Edit|Write"                   // Alternation
```

## Example Configuration

```json
{
  "PermissionRequest": [
    {
      "matcher": "mcp__filesystem__*",
      "hooks": [{
        "type": "command",
        "command": "scripts/auto-deny-filesystem.sh"
      }]
    },
    {
      "matcher": "mcp__memory__*",
      "hooks": [{
        "type": "command",
        "command": "scripts/auto-allow-memory.sh"
      }]
    }
  ]
}
```

## Testing Strategy

### Unit Tests (8-10 cases)
- Pattern recognition validation
- Matching behavior verification
- Deserialization handling
- Priority/specificity rules
- Edge cases (underscores, hyphens, etc.)

### Integration Tests
- Full hooks configuration scenarios
- Multiple pattern interactions
- Backwards compatibility verification

## Complexity Assessment

- **Complexity Level**: LOW
- **Risk Level**: LOW
- **Performance Impact**: NONE
- **Backwards Compatibility**: 100%

## Success Criteria

✓ Single pattern `mcp__filesystem__*` matches all tools from that server
✓ Specific patterns take precedence over wildcards
✓ All existing tests pass
✓ Ignored test becomes passing test
✓ New test cases cover all scenarios
✓ Fully backwards compatible
✓ No performance degradation
✓ Handles edge cases correctly

## Files to Modify

1. **crates/cli/src/hooks/types.rs** (~50 lines)
   - Core implementation

2. **crates/cli/tests/hooks_doc_tests.rs** (~150 lines)
   - New test cases

## Related Code

- Hook registry: `crates/cli/src/hooks/registry.rs`
- Hook executor: `crates/cli/src/hooks/executor.rs`
- Permission checking: Uses `HookMatcher::matches()` for permission decisions

## Next Steps

1. Read DESIGN.md for complete specification
2. Review IMPLEMENTATION_SUMMARY.md for executive overview
3. Check QUICK_REFERENCE.md for examples and quick lookup
4. Begin implementation following the implementation checklist in DESIGN.md

