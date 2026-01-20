# Architecture Summary: Issue #245 - Variable Substitution

## Executive Summary

**Objective**: Enable `${CLAUDE_PLUGIN_ROOT}` variable substitution in plugin frontmatter.

**Scope**: Single-purpose module to substitute environment-like variables in frontmatter string values.

**Complexity**: Low (simple string substitution, ~200 lines of implementation code).

**Risk**: Minimal (non-breaking, opt-in, safe degradation).

---

## The Problem

Plugin developers want to reference tool paths relative to their plugin root:

```yaml
---
allowed-tools:
  - "${CLAUDE_PLUGIN_ROOT}/tools/verify"
  - "Read"
---
```

Currently, the literal string `"${CLAUDE_PLUGIN_ROOT}/tools/verify"` is passed to the tool system, which fails because it's not a valid tool path.

**Root Cause**: Frontmatter YAML parsing stops at deserialization. No variable substitution pass occurs before downstream usage.

---

## The Solution

### Single Responsibility Module

**Module**: `frontmatter_substitution`

**Responsibility**: Transform `${VAR_NAME}` patterns in strings to actual values.

**Nothing Else**: No parsing, no file I/O, no caching.

### Architecture

```
┌─────────────────────────────────────────────┐
│ Plugin Frontmatter Loading                  │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│ 1. Read file from disk                      │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│ 2. Parse YAML frontmatter (existing)        │
│    Result: FrontMatter struct               │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│ [NEW] 3. Substitute variables               │
│    Input: FrontMatter with "${PLUGIN_ROOT}" │
│    Output: FrontMatter with "/actual/path"  │
└──────────────┬──────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────┐
│ 4. Use frontmatter downstream (unchanged)   │
│    Tool system receives resolved paths      │
└──────────────┬──────────────────────────────┘
               │
               ▼
          ✓ Successful plugin load
```

### Key Components

```
┌─────────────────────────────────────────────┐
│         frontmatter_substitution.rs          │
├─────────────────────────────────────────────┤
│                                             │
│  Variable enum                              │
│  ├─ PluginRoot                              │
│  ├─ ProjectRoot                             │
│  ├─ Home                                    │
│  ├─ User                                    │
│  └─ Pwd                                     │
│                                             │
│  SubstitutionContext struct                 │
│  ├─ plugin_root: PathBuf                    │
│  ├─ project_root: Option<PathBuf>           │
│  └─ resolve_variable() → Option<String>     │
│                                             │
│  Substituter struct                         │
│  ├─ new(ctx) → Self                         │
│  ├─ substitute(&str) → String               │
│  └─ substitute_frontmatter(&mut FrontMatter)│
│                                             │
└─────────────────────────────────────────────┘
```

---

## Integration Points

### CommandLoader (commands/loader.rs)

**Current Flow**:
```rust
let (frontmatter, body) = self.parse_frontmatter(&content)?;
// Use frontmatter directly
```

**New Flow**:
```rust
let (mut frontmatter, body) = self.parse_frontmatter(&content)?;

// NEW: Substitute variables
if let Some(plugin_root) = plugin_context.root {
    let ctx = SubstitutionContext::new(plugin_root, project_root);
    let substituter = Substituter::new(ctx);
    substituter.substitute_frontmatter(&mut frontmatter);
}

// Use frontmatter with resolved paths
```

### AgentDiscovery (plugins/agent_discovery.rs)

Optional integration for file-based agents (if they use frontmatter with allowed_tools in future).

### No Changes Required

- plugin.json manifest (doesn't use frontmatter)
- Runtime agents (passed via CLI JSON, not files)
- Existing plugins without variables (unchanged behavior)

---

## Supported Variables

| Variable | Resolves To | Example |
|----------|-------------|---------|
| `${CLAUDE_PLUGIN_ROOT}` | Plugin root directory | `/home/user/plugins/my-plugin` |
| `${CLAUDE_PROJECT_ROOT}` | Project root | `/home/user/project` |
| `${HOME}` | User home | `/home/user` |
| `${USER}` | Username | `alice` |
| `${PWD}` | Current directory | `/home/user/project` |

## Pattern Matching

**Recognized Pattern**: `${VARIABLE_NAME}`

```
✓ ${CLAUDE_PLUGIN_ROOT}  - matches
✓ ${HOME}                - matches
✓ ${MY_CUSTOM}           - no, unknown variable (preserved)
✗ $VARIABLE              - no, missing braces
✗ ${plugin_root}         - no, lowercase
✗ ${NESTED${VAR}}        - no, nesting not supported (preserved)
```

---

## Error Handling Strategy

**Philosophy**: Fail gracefully. Missing variables don't break plugin load.

| Scenario | Behavior | Example |
|----------|----------|---------|
| Variable found | Substitute | `${HOME}` → `/home/user` |
| Variable unknown | Preserve | `${UNKNOWN}` → `${UNKNOWN}` |
| Variable unavailable | Preserve + warn | `${HOME}` → `${HOME}` (log warning) |
| Malformed pattern | Preserve | `${BAD}` (if not uppercase) → preserved |
| Multiple in one value | All substituted | `${HOME}/path:${PWD}` → `/home/user/path:/current` |

---

## Backwards Compatibility

**Impact**: Zero breaking changes.

✓ Existing frontmatter without variables works unchanged
✓ New variable syntax is opt-in
✓ Unknown patterns are left as-is (safe default)
✓ All downstream APIs unchanged
✓ No new dependencies required

---

## Testing Strategy

### Unit Tests (frontmatter_substitution module)

```
✓ Variable matching and resolution (each type)
✓ Multiple variables in one string
✓ Unknown variables preserved
✓ Malformed patterns preserved
✓ Empty/null handling
✓ FrontMatter integration
```

### Integration Tests

```
✓ Load agent with variables → verify resolved
✓ Load command with variables → verify resolved
✓ Mixed absolute and relative paths work
✓ Tool system receives resolved paths
✓ Downstream permissions work correctly
```

### Coverage Target

100% of public API (`substitute()`, `substitute_frontmatter()`, `Variable::resolve()`)

---

## Implementation Size & Effort

### Code Volume
- **frontmatter_substitution.rs**: ~250 lines (impl + tests)
- **Integration changes**: ~20 lines total (add 1 call in loader)
- **Module exports**: ~3 lines

### Effort Estimate
- **Core Implementation**: 1-2 hours
- **Testing**: 1-2 hours
- **Integration**: 30 minutes
- **Documentation**: 1 hour
- **Total**: 3-6 hours

### Risk Level: **LOW**

- Isolated module (no dependencies on other systems)
- Non-breaking (existing code works unchanged)
- Opt-in (only active when variables present)
- Graceful degradation (failures don't crash)
- Easy to test and verify

---

## Performance Characteristics

**Substitution is lightweight:**

- Single pass through string with simple regex
- HashMap lookups for variable names
- Environment variable resolution (cached by OS)
- No file I/O
- No allocations unless substitution occurs
- Runs once during plugin load (not hot path)

**Expected Performance**:
- Per string: < 100 microseconds
- Per plugin load: < 1 millisecond
- Total impact on startup: negligible

---

## Documentation for Plugin Developers

### Quick Start

```markdown
# Using Variables in Plugin Frontmatter

Add variable references to your agent or command frontmatter:

## Example: Safe Tools

agents/checker.md:
---
allowed-tools:
  - "${CLAUDE_PLUGIN_ROOT}/bin/verify-config"
  - "${CLAUDE_PLUGIN_ROOT}/bin/check-syntax"
  - "Read"
---

The variables are automatically expanded to actual paths when the plugin loads.

## Supported Variables

- ${CLAUDE_PLUGIN_ROOT} - Your plugin's root directory
- ${CLAUDE_PROJECT_ROOT} - The project root
- ${HOME} - User's home directory
- ${USER} - Current username
- ${PWD} - Current working directory
```

---

## Success Criteria

Verify implementation by testing:

1. ✓ Plugin with `allowed-tools: ["${CLAUDE_PLUGIN_ROOT}/safe"]` loads successfully
2. ✓ Tool system receives `/actual/plugin/root/safe`, not literal string
3. ✓ Tool permissions work correctly with resolved paths
4. ✓ Unknown variables degrade gracefully (not fatal)
5. ✓ Existing plugins without variables work unchanged
6. ✓ 100% test coverage of substitution module
7. ✓ Zero performance regression in plugin loading

---

## File Structure

```
crates/cli/src/plugins/
├── mod.rs (updated - export new module)
├── frontmatter_substitution.rs (NEW)
│   ├── Variable enum
│   ├── SubstitutionContext struct
│   ├── Substituter struct
│   └── #[cfg(test)] comprehensive tests
├── agent_discovery.rs (unchanged or minimal)
└── ... (existing files)

crates/cli/src/commands/
└── loader.rs (updated - integrate substitution)
```

---

## Deliverables

1. **frontmatter_substitution.rs** - Core module with full implementation
2. **Integration in CommandLoader** - Substitution call after frontmatter parse
3. **Tests** - 100% coverage of public API
4. **Documentation** - Design, spec, implementation guide
5. **Examples** - Real plugin scenarios showing usage

---

## Next Steps

1. Implement `frontmatter_substitution.rs` with all components
2. Add unit tests with 100% coverage
3. Integrate into `CommandLoader::load_command()`
4. Add integration tests with real plugin scenarios
5. Verify existing tests still pass
6. Document for plugin developers
7. Code review and merge

---

## Related Documentation

- **DESIGN_VARIABLE_SUBSTITUTION.md** - Comprehensive architectural design
- **MODULE_SPEC_FRONTMATTER_SUBSTITUTION.md** - Formal module specification
- **IMPLEMENTATION_GUIDE.md** - Step-by-step implementation instructions

