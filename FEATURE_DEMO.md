# MCP Server Wildcard - Feature Demonstration

## Problem Solved

**Before Issue #250:**
```json
{
  "deny": [
    "mcp__filesystem__read_file",
    "mcp__filesystem__write_file",
    "mcp__filesystem__list_dir",
    "mcp__filesystem__delete_file",
    "mcp__filesystem__create_dir",
    "mcp__filesystem__move_file"
  ]
}
```
❌ Need to list every single tool from the server
❌ Easy to miss tools
❌ Verbose and error-prone

**After Issue #250:**
```json
{
  "deny": [
    "mcp__filesystem__*"
  ]
}
```
✅ Single pattern matches all tools from server
✅ Automatically includes new tools
✅ Clean and maintainable

## Usage Examples

### Example 1: Block Entire Server

**Scenario:** Block all filesystem operations

```json
{
  "PermissionRequest": [{
    "matcher": "mcp__filesystem__*",
    "hooks": [{
      "type": "command",
      "command": "scripts/deny-filesystem.sh",
      "timeout": 5000
    }]
  }]
}
```

**Result:** All filesystem tools automatically denied
- `mcp__filesystem__read_file` → DENIED
- `mcp__filesystem__write_file` → DENIED
- `mcp__filesystem__delete` → DENIED

### Example 2: Allow Server, Block Specific Tool

**Scenario:** Allow all memory operations except delete

```json
{
  "PermissionRequest": [
    {
      "matcher": "mcp__memory__*",
      "hooks": [{
        "type": "command",
        "command": "scripts/allow-memory.sh"
      }]
    },
    {
      "matcher": "mcp__memory__delete",
      "hooks": [{
        "type": "command",
        "command": "scripts/deny-delete.sh"
      }]
    }
  ]
}
```

**Result:** Memory tools allowed except delete (specific takes precedence)
- `mcp__memory__store` → ALLOWED (matches wildcard)
- `mcp__memory__read` → ALLOWED (matches wildcard)
- `mcp__memory__delete` → DENIED (exact match takes priority)

### Example 3: Multi-Server Configuration

**Scenario:** Different rules for different servers

```json
{
  "PermissionRequest": [
    {
      "matcher": "mcp__filesystem__*",
      "hooks": [{
        "type": "command",
        "command": "scripts/review-filesystem.sh"
      }]
    },
    {
      "matcher": "mcp__web__*",
      "hooks": [{
        "type": "command",
        "command": "scripts/allow-web.sh"
      }]
    },
    {
      "matcher": "mcp__.*",
      "hooks": [{
        "type": "command",
        "command": "scripts/deny-unknown-mcp.sh"
      }]
    }
  ]
}
```

**Result:** Layered security with fallback
- `mcp__filesystem__read` → Reviewed (server wildcard matches)
- `mcp__web__fetch` → Auto-allowed (server wildcard matches)
- `mcp__unknown_server__tool` → Denied (general MCP pattern matches)

## Priority System

The matcher uses specificity-based priority:

```
HIGHEST PRIORITY
    ↓
1. Exact Match: "mcp__filesystem__read_file"
    ↓
2. Server Wildcard: "mcp__filesystem__*"
    ↓
3. All MCP Tools: "mcp__.*"
    ↓
LOWEST PRIORITY
```

### Priority Example

Configuration:
```json
{
  "PermissionRequest": [
    {
      "matcher": "mcp__.*",
      "hooks": [{"type": "command", "command": "deny-all-mcp.sh"}]
    },
    {
      "matcher": "mcp__filesystem__*",
      "hooks": [{"type": "command", "command": "allow-filesystem.sh"}]
    },
    {
      "matcher": "mcp__filesystem__delete",
      "hooks": [{"type": "command", "command": "deny-delete.sh"}]
    }
  ]
}
```

Results:
- `mcp__web__fetch` → Uses `mcp__.*` (general)
- `mcp__filesystem__read` → Uses `mcp__filesystem__*` (server wildcard)
- `mcp__filesystem__delete` → Uses exact match (highest priority)

## Pattern Format

### Valid Server Wildcard Patterns

✅ `mcp__filesystem__*` - Standard format
✅ `mcp__my_server__*` - Underscores in server name
✅ `mcp__my-server__*` - Hyphens in server name
✅ `mcp__custom123__*` - Numbers in server name

### Invalid Patterns (Not Server Wildcards)

❌ `mcp__*` - Only 1 "__", not a server wildcard
❌ `mcp__filesystem_*` - Ends with "_*" not "__*"
❌ `mcp__server__tool__*` - Too many "__" separators

### Other Valid Patterns (Not Server Wildcards)

✅ `mcp__.*` - Matches ALL MCP tools (different pattern)
✅ `mcp__.*__.*` - Matches all properly formatted MCP tools
✅ `*` - Matches everything

## Testing

All tests pass with comprehensive coverage:

```bash
$ cargo test --test hooks_doc_tests
test result: ok. 124 passed; 0 failed; 0 ignored
```

**Test Coverage:**
- Pattern recognition ✓
- Matching behavior ✓
- Deserialization ✓
- Priority/specificity ✓
- Edge cases (underscores, hyphens, empty names) ✓
- Case sensitivity ✓
- Invalid patterns ✓
- Real-world configurations ✓

## Migration Guide

### Upgrading Existing Configurations

**Old Configuration:**
```json
{
  "deny": [
    "mcp__filesystem__read_file",
    "mcp__filesystem__write_file",
    "mcp__filesystem__list_dir"
  ]
}
```

**New Configuration (Recommended):**
```json
{
  "deny": [
    "mcp__filesystem__*"
  ]
}
```

**Benefits:**
- Shorter configuration
- Automatically covers new tools added to the server
- Easier to maintain
- Less error-prone

### Backward Compatibility

✅ All existing configurations continue to work unchanged
✅ Can mix old and new syntax in same configuration
✅ No breaking changes

---

**Feature Status**: ✅ Implemented and Tested
**Ready for**: Production Use
