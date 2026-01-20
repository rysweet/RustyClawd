# Issue #245: ${CLAUDE_PLUGIN_ROOT} Variable Substitution - Design Documentation

## Quick Navigation

This directory contains complete architectural and implementation guidance for Issue #245.

### Documents

| Document | Purpose | Audience |
|----------|---------|----------|
| **ARCHITECTURE_SUMMARY.md** | Executive overview, component diagrams, integration points | Architecture Review, Quick Reference |
| **DESIGN_VARIABLE_SUBSTITUTION.md** | Comprehensive architectural design with rationale | Architects, Design Review |
| **MODULE_SPEC_FRONTMATTER_SUBSTITUTION.md** | Formal module specification with contracts | Implementation, Verification |
| **IMPLEMENTATION_GUIDE.md** | Step-by-step implementation instructions with code | Builders, Developers |

**Start here**: ARCHITECTURE_SUMMARY.md (5-minute read)

---

## The Issue (One-Minute Summary)

Plugin developers want to write tool paths relative to their plugin:

```yaml
---
allowed-tools:
  - "${CLAUDE_PLUGIN_ROOT}/tools/verify"
---
```

Currently, this literal string is passed to the tool system, which fails because it's not a valid path. The variable should be substituted with the actual plugin root directory.

---

## The Solution (One-Minute Summary)

Create a single-purpose module `frontmatter_substitution` that:
1. Identifies `${VARIABLE_NAME}` patterns in frontmatter strings
2. Resolves them to actual values (plugin root, home dir, etc.)
3. Substitutes them in place
4. Leaves unknown patterns as-is (safe degradation)

**Impact**: Low-risk, non-breaking, opt-in enhancement. ~250 lines of code.

---

## Architecture at a Glance

```
Frontmatter Loading
    ↓
[1] Read file
    ↓
[2] Parse YAML
    ↓
[3] NEW: Substitute variables
    ├─ ${CLAUDE_PLUGIN_ROOT} → /actual/root
    ├─ ${HOME} → /home/user
    └─ ${UNKNOWN} → ${UNKNOWN} (preserved)
    ↓
[4] Use in tool system
    ↓
✓ Success (resolved paths)
```

---

## Module Design

### Single Responsibility
Transform `${VAR_NAME}` patterns in strings to actual values. Nothing else.

### Public API
```rust
// Create context with plugin information
let ctx = SubstitutionContext::new(plugin_root, project_root);

// Create substituter
let substituter = Substituter::new(ctx);

// Substitute single string
let result = substituter.substitute("${CLAUDE_PLUGIN_ROOT}/tools");

// Substitute entire frontmatter
substituter.substitute_frontmatter(&mut frontmatter);
```

### Supported Variables
- `${CLAUDE_PLUGIN_ROOT}` - Plugin root directory
- `${CLAUDE_PROJECT_ROOT}` - Project root
- `${HOME}` - User home
- `${USER}` - Current username
- `${PWD}` - Current directory

### Error Handling
Unknown or unavailable variables are left as-is. Plugin load never fails due to substitution.

---

## Integration Points

### CommandLoader (commands/loader.rs)
After YAML frontmatter parsing, add substitution call:
```rust
let (mut frontmatter, body) = self.parse_frontmatter(&content)?;
if let Some(plugin_root) = plugin_context.root {
    let ctx = SubstitutionContext::new(plugin_root, project_root);
    let substituter = Substituter::new(ctx);
    substituter.substitute_frontmatter(&mut frontmatter);
}
```

### AgentDiscovery (plugins/agent_discovery.rs)
Optional integration for future enhancements where agents load frontmatter with variables.

---

## Testing Strategy

### Unit Tests
- Variable matching and resolution
- Multiple variables in one string
- Unknown variables preserved
- FrontMatter struct integration
- 100% public API coverage

### Integration Tests
- Load plugin with variables → verify resolved
- Tool system receives correct paths
- Mixed absolute and relative paths

---

## Implementation Roadmap

### Phase 1: Core Module (1-2 hours)
- Create frontmatter_substitution.rs
- Implement Variable enum, SubstitutionContext, Substituter
- Add comprehensive tests

### Phase 2: Integration (30 min)
- Update CommandLoader to use substitution
- Update module exports

### Phase 3: Testing & Verification (1-2 hours)
- Run integration tests
- Verify no regressions
- Validate with real plugin scenarios

### Phase 4: Documentation (1 hour)
- Create developer guide
- Add examples to documentation

---

## File Locations

**Implementation**:
- `crates/cli/src/plugins/frontmatter_substitution.rs` (NEW)
- `crates/cli/src/plugins/mod.rs` (update exports)
- `crates/cli/src/commands/loader.rs` (integrate)

**Documentation** (in this worktree):
- ARCHITECTURE_SUMMARY.md
- DESIGN_VARIABLE_SUBSTITUTION.md
- MODULE_SPEC_FRONTMATTER_SUBSTITUTION.md
- IMPLEMENTATION_GUIDE.md

---

## Success Criteria

1. ✓ Plugin with `allowed-tools: ["${CLAUDE_PLUGIN_ROOT}/safe"]` loads
2. ✓ Tool system receives `/actual/root/safe`, not literal string
3. ✓ Unknown variables degrade gracefully
4. ✓ Existing plugins work unchanged
5. ✓ 100% test coverage
6. ✓ Zero performance regression

---

## Risk Assessment

**Level**: LOW

✓ Isolated module (no dependencies on other systems)
✓ Non-breaking (existing code works unchanged)
✓ Opt-in (only active when variables present)
✓ Graceful degradation (failures don't crash)
✓ Easy to test and verify

---

## Decision Framework

### Why This Design?

**Q: Why a separate module?**
A: Single responsibility. Substitution is conceptually separate from parsing. Enables testing in isolation and reuse.

**Q: Why safe degradation?**
A: Unknown variables don't break plugin load. Better UX than hard failures. Plugin works without the variable.

**Q: Why only uppercase variables?**
A: Reduces false positives. `${variable}` is unlikely to be a variable reference. Follows shell convention.

**Q: Why no nesting?**
A: Adds significant complexity for rare use case. `${VAR${NESTED}}` is confusing. Simple patterns work for 99% of cases.

---

## Related Issues

**Dependency**: None. This is standalone.

**Affects**: Tool permission system, agent definitions, command loading.

**Future Enhancements**:
- Support for custom variables via plugins
- Variable caching if performance concerns arise
- Substitution in command body (if desired)

---

## Implementation Checklists

### Before Starting
- [ ] Read ARCHITECTURE_SUMMARY.md
- [ ] Review MODULE_SPEC_FRONTMATTER_SUBSTITUTION.md
- [ ] Understand integration points in CommandLoader

### Implementation
- [ ] Create frontmatter_substitution.rs
- [ ] Implement Variable enum
- [ ] Implement SubstitutionContext
- [ ] Implement Substituter
- [ ] Add comprehensive tests
- [ ] Update mod.rs exports
- [ ] Integrate into CommandLoader
- [ ] Run existing tests (verify no regression)

### Verification
- [ ] Unit tests pass (100% coverage)
- [ ] Integration tests pass
- [ ] Real plugin scenarios work
- [ ] Performance verified (< 1ms per plugin)
- [ ] Documentation complete

---

## Getting Started

1. **Understand the Problem**: Read the first 3 sections of ARCHITECTURE_SUMMARY.md
2. **Review the Design**: Study the "Module Design" and "Integration Points" sections
3. **Plan Implementation**: Use IMPLEMENTATION_GUIDE.md as step-by-step checklist
4. **Implement**: Follow code examples in IMPLEMENTATION_GUIDE.md
5. **Test**: Use provided test structure and add real plugin scenarios
6. **Integrate**: Add substitution call to CommandLoader
7. **Verify**: Run all tests and validate with real plugins

---

## Key Insights

1. **Minimal Scope**: This is NOT a full templating engine. Just `${WORD}` substitution.
2. **Safe Defaults**: Unknown variables cause degradation, not failure.
3. **One Job**: Substitution only. Parsing happens elsewhere.
4. **Testable**: Comprehensive unit tests required. Simple logic.
5. **Non-Breaking**: Zero impact on existing plugins.

---

## Questions & Answers

**Q: Will this slow down plugin loading?**
A: No. Substitution is < 100 microseconds per string, and runs once during load.

**Q: What if a plugin uses undefined variables?**
A: The literal string is preserved (e.g., `"${UNDEFINED}/path"` stays as-is). Plugin still loads.

**Q: Can I use nested variables like `${VAR${INNER}}`?**
A: No. Not supported. Use simple patterns like `${PLUGIN_ROOT}/path`.

**Q: What about performance with many substitutions?**
A: Fast. Single regex pass per string, HashMap lookups for resolution. Negligible overhead.

**Q: Is this backwards compatible?**
A: 100%. Existing frontmatter without variables works unchanged. New syntax is opt-in.

---

## Document Locations in This Worktree

- `/home/azureuser/src/RustyClawd/worktrees/issue-245/ARCHITECTURE_SUMMARY.md`
- `/home/azureuser/src/RustyClawd/worktrees/issue-245/DESIGN_VARIABLE_SUBSTITUTION.md`
- `/home/azureuser/src/RustyClawd/worktrees/issue-245/MODULE_SPEC_FRONTMATTER_SUBSTITUTION.md`
- `/home/azureuser/src/RustyClawd/worktrees/issue-245/IMPLEMENTATION_GUIDE.md`

---

## Summary

Issue #245 requires a simple, focused module to substitute `${VARIABLE_NAME}` patterns in plugin frontmatter. The design is low-risk, non-breaking, and straightforward to implement. Complete documentation guides implementation from architecture through testing.

**Estimated effort**: 4-8 hours total (design + implementation + testing + integration).

**Ready to implement**: Yes. All specifications and implementation guidance complete.

