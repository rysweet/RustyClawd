# Module Specification: Frontmatter Variable Substitution

## Purpose

Substitute environment-like variables (${VARIABLE_NAME}) in plugin frontmatter, enabling plugin developers to write plugin-relative paths like `${CLAUDE_PLUGIN_ROOT}/tools/script`.

## Single Responsibility

Perform variable substitution on frontmatter string values. Nothing more.

## Public Contract

### Input
- Plugin root directory path
- Optional project root path
- Frontmatter struct or HashMap with string values
- String containing ${VAR} patterns

### Output
- Substituted string with `${PLUGIN_ROOT}` → `/actual/path`
- Updated frontmatter with all tool path variables resolved
- Unmatched patterns left unchanged (safe degradation)

### Side Effects
None. Pure transformation.

## Dependencies

**Required**:
- `std::path::PathBuf` - for path handling
- `std::collections::HashMap` - for variable lookup
- `regex` crate - for pattern matching (optional: manual parsing also viable)

**Optional**:
- `tracing` - for logging warnings when variables can't resolve

**None of**:
- File I/O (paths don't need to exist)
- Async operations
- External services

## Supported Variables

| Variable | Value |
|----------|-------|
| `${CLAUDE_PLUGIN_ROOT}` | Plugin root directory path |
| `${CLAUDE_PROJECT_ROOT}` | Project root directory path |
| `${HOME}` | User home directory |
| `${USER}` | Current username |
| `${PWD}` | Current working directory |

## Implementation Notes

### Key Design Decisions

1. **Simple Regex Pattern**: `\$\{([A-Z_]+)\}`
   - Only matches uppercase names (reduces false positives)
   - Captured group extracts variable name
   - No nested variables or complex expressions

2. **Safe Degradation**: Unknown/unresolvable variables left as-is
   - Never fails the plugin load
   - Logs warning for debugging
   - Enables partial substitution if some variables unavailable

3. **String-Based Substitution**: Works on any string value
   - Can substitute in any frontmatter field
   - Works with allowed_tools, disallowed_tools, descriptions, etc.

4. **No Caching**: Fresh resolution on each substitution
   - Plugin loads infrequently
   - Env vars can change
   - Better predictability

### Error Handling Strategy

| Scenario | Behavior |
|----------|----------|
| Variable resolves | Replace with value |
| Variable unknown | Leave pattern as-is |
| Variable unavailable (e.g., HOME) | Leave pattern as-is, log warning |
| Malformed pattern | Leave as-is |
| Empty string | Return empty string |

## Test Requirements

### Unit Tests (Core Functionality)
- ✓ Each variable type resolves correctly
- ✓ Multiple variables in one string
- ✓ Unknown variables left unchanged
- ✓ Malformed patterns left unchanged
- ✓ Empty inputs handled

### Integration Tests (Plugin Scenarios)
- ✓ Load agent with frontmatter containing variables
- ✓ Load command with frontmatter containing variables
- ✓ Verify downstream tool system receives resolved paths
- ✓ Mixed absolute and relative paths

### Edge Cases
- ✓ Nested ${} patterns (not supported, left as-is)
- ✓ Partial matches like $VARIABLE (not matched, left as-is)
- ✓ Case sensitivity (CLAUDE_PLUGIN_ROOT not Clayde_Plugin_Root)

## Integration Points

### 1. CommandLoader (crates/cli/src/commands/loader.rs)

After YAML frontmatter parsing:

```rust
let (mut frontmatter, body) = self.parse_frontmatter(&content)?;

// NEW: Substitute variables
if let Some(plugin_root) = plugin_context.root {
    let ctx = SubstitutionContext::new(plugin_root, project_root);
    let substituter = Substituter::new(ctx);
    substituter.substitute_frontmatter(&mut frontmatter);
}
```

### 2. AgentDiscovery (crates/cli/src/plugins/agent_discovery.rs)

When creating AgentDefinition from file:

```rust
// After creating AgentDefinition
let ctx = SubstitutionContext::new(plugin_root, project_root);
let substituter = Substituter::new(ctx);
// Apply to disallowed_tools if supported
```

### 3. No Changes Needed
- Plugin manifest (plugin.json) - doesn't use frontmatter
- Runtime agents - handled at invocation level

## File Location

```
crates/cli/src/plugins/frontmatter_substitution.rs
```

## Exposed Types

```rust
pub enum Variable { ... }
pub struct SubstitutionContext { ... }
pub struct Substituter { ... }
```

## Not In Scope

- Variable nesting (${VAR1}${VAR2})
- Conditional substitution
- Custom plugins for variable resolution
- Env var interpolation in body content
- Cache management
- Hot-reload updates

## Regeneration Notes

This module is self-contained and can be rebuilt from specification:
- Variable enum derived from supported list
- Pattern matching is simple regex
- Resolution logic is straightforward mapping
- Error handling is documented
- All tests can be regenerated from scenarios

## Success Metrics

1. Agent with `allowed-tools: ["${CLAUDE_PLUGIN_ROOT}/safe"]` works
2. Tool system receives resolved paths (not literal strings)
3. Unknown variables degrade gracefully (not fatal)
4. 100% test coverage of public API
5. Zero performance regression (single regex pass per string)

