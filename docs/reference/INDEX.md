# Issue #250 Design Documentation Index

## Overview
Complete architectural design for wildcard syntax `mcp__<server>__*` in MCP tool permission system.

## Documentation Map

### Start Here
- **README.md** - Project overview, key decisions, files to modify, success criteria

### For Implementation
- **DESIGN.md** - Complete specification (451 lines)
  - Problem statement
  - Solution architecture
  - Pattern format specification
  - Matcher implementation details
  - Priority rules
  - Testing strategy (5 test case scenarios)
  - Integration points
  - Implementation checklist

### For Quick Reference
- **QUICK_REFERENCE.md** - Quick lookup guide
  - Pattern examples
  - Matching test cases
  - Common use cases
  - Files changed summary

### For Project Overview
- **IMPLEMENTATION_SUMMARY.md** - Executive summary
  - High-level changes
  - Before/after examples
  - Test coverage summary
  - Configuration examples

## Quick Facts

| Aspect | Detail |
|--------|--------|
| Issue | #250 |
| Feature | Wildcard syntax for MCP tool permissions |
| Pattern | `mcp__<server>__*` |
| Complexity | LOW |
| Risk | LOW |
| Performance Impact | NONE |
| Backwards Compatible | 100% YES |
| Files Modified | 2 |
| Lines of Code | ~50 (implementation) + ~150 (tests) |

## Key Pattern

```
Before:  Multiple entries per server
deny: ["mcp__filesystem__read_file", "mcp__filesystem__write_file", ...]

After:   Single entry per server
deny: ["mcp__filesystem__*"]
```

## Implementation Phases

1. **Phase 1**: Core wildcard matching logic
2. **Phase 2**: Deserialization updates
3. **Phase 3**: Comprehensive testing
4. **Phase 4**: Validation and compatibility

## Priority Rules

```
Exact:    mcp__filesystem__read_file  (HIGHEST)
Wildcard: mcp__filesystem__*
General:  mcp__.*                     (LOWEST)
```

## Test Coverage

8-10 unit tests covering:
- Pattern recognition
- Matching behavior
- Deserialization
- Priority handling
- Edge cases

Plus fix for existing ignored test: `test_matcher_mcp_full_pattern`

## Files to Modify

1. `/home/azureuser/src/RustyClawd/crates/cli/src/hooks/types.rs`
   - Core implementation
   - ~50 lines

2. `/home/azureuser/src/RustyClawd/crates/cli/tests/hooks_doc_tests.rs`
   - Test cases
   - ~150 lines

## Reading Guide

**For Understanding**:
1. README.md (overview)
2. QUICK_REFERENCE.md (examples)
3. IMPLEMENTATION_SUMMARY.md (executive summary)

**For Implementation**:
1. DESIGN.md (complete spec)
2. Follow implementation checklist

**For Quick Lookup**:
- QUICK_REFERENCE.md (pattern examples, common cases)

## Success Criteria Checklist

- [ ] Single pattern `mcp__filesystem__*` matches all tools from server
- [ ] Specific patterns take precedence over wildcards
- [ ] All existing tests pass
- [ ] Ignored test becomes passing test
- [ ] New tests cover all scenarios
- [ ] 100% backwards compatible
- [ ] No performance degradation
- [ ] Edge cases handled (underscores, hyphens, empty names)

## Document Statistics

| Document | Lines | Purpose |
|----------|-------|---------|
| DESIGN.md | 451 | Complete specification |
| IMPLEMENTATION_SUMMARY.md | 172 | Executive overview |
| QUICK_REFERENCE.md | 88 | Quick lookup |
| README.md | 164 | Project overview |
| **Total** | **875** | **Complete design** |

## Architecture Summary

### Current State (Limitation)
- Can match: exact tools, all MCP tools, alternation patterns
- Cannot match: all tools from specific server efficiently

### Proposed Solution
- Add server-level wildcard pattern: `mcp__<server>__*`
- Fix pattern matching order for correct priority
- Bonus: Fix existing ignored test

### Key Components
- **Pattern Detection**: `is_mcp_server_wildcard()`
- **Matching Logic**: Updated `HookMatcher::matches()`
- **Deserialization**: Updated pattern heuristic in deserialize()

## Integration Points

- Hook registry uses matcher for permission decisions
- Hook executor calls matcher when checking permissions
- Configuration loaded from JSON with pattern matching

## Backwards Compatibility

- Exact patterns still work: `"mcp__server__tool"`
- Existing regex patterns still work: `"mcp__.*"`
- Only adds new pattern type, no breaking changes
- All existing configs continue to work

## Performance Characteristics

- No new loops or inefficient algorithms
- Simple string prefix/suffix checks
- Same time complexity as existing patterns
- Negligible performance impact

---

**Created**: 2026-01-20
**Status**: Design Complete, Ready for Implementation
**Next Step**: Begin implementation following DESIGN.md checklist
