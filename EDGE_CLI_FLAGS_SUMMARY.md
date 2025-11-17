# Edge CLI Flags - Executive Summary

**Issue:** #51 - Clarify requirements for `--compact`, `--rewind`, `--sandbox` flags
**Status:** CLARIFIED - Ready for implementation decisions
**Date:** 2025-11-17

---

## Key Finding: Misclassification

The three "missing" flags have been **misclassified as CLI startup flags**. In official Claude Code:

| Flag | Type | Status in RustyClawd | Recommendation |
|------|------|---------------------|-----------------|
| `--compact` | Slash command (`/compact`) | Partial stub | Enhance slash command, NOT add CLI flag |
| `--rewind` | Slash command (`/rewind`) | Partial stub | Enhance slash command, NOT add CLI flag |
| `--sandbox` | CLI flag + slash command | Missing CLI flag | **IMPLEMENT CLI flag** + slash command |

---

## The Reality

### What Official Claude Code Does

1. **`/compact` (slash command)**
   - Interactive: Summarizes conversation history to reduce tokens
   - Not a CLI startup flag
   - Example: `claude -p "code" && /compact`

2. **`/rewind` (slash command)**
   - Interactive: Rollback to previous checkpoint
   - Equivalent CLI flag: `--resume-from-checkpoint N` (already exists!)
   - Example: `claude -c && /rewind` OR `claude --resume-from-checkpoint 2`

3. **`--sandbox` (CLI flag + command)**
   - CLI: `claude --sandbox "prompt"` - enables execution isolation
   - Interactive: `/sandbox` - shows/toggles sandbox status
   - Platform support: macOS (sandbox-exec), Linux (firejail), Windows (process isolation)

---

## What We're NOT Implementing

```bash
# These should NOT be added as CLI flags:
claude --compact "prompt"  # WRONG - /compact is slash command only
claude --rewind "prompt"   # WRONG - use --resume-from-checkpoint or /rewind

# These ALREADY EXIST in RustyClawd:
claude --resume-from-checkpoint 2  # CORRECT - restore checkpoint 2
```

---

## What We SHOULD Implement

### Phase 1 - Priority HIGH (--sandbox)

**Effort:** 2-3 days
**Impact:** Enables secure code execution for untrusted prompts

```rust
// Add to CLI
struct Cli {
    #[arg(long)]
    sandbox: bool,

    #[arg(long)]
    no_sandbox: bool,
}

// Usage
claude --sandbox "review this code"
claude --no-sandbox "trust this code"
```

**Implementation:**
- Trait-based sandbox backend (OS-agnostic)
- macOS: Use `sandbox-exec` (built-in)
- Linux: Use `firejail` or builtin capability isolation
- Windows: Process containment or skip
- Restrict file access to `.claude/` and `--add-dir` paths
- Optional network isolation
- Add `PreSandbox` and `PostSandbox` hooks

---

### Phase 2 - Enhancement MEDIUM (/compact, /rewind)

**Effort:** 2-3 days total
**Impact:** Better UX for session control

#### `/compact` Enhancement
```
/compact                    # Compress conversation
/compact focus: auth logic  # Keep specific context
/compact --token-count      # Show savings
```

**Implement:**
- Message summarization logic
- Token count calculation
- Focus instruction support
- `PreCompact`/`PostCompact` hooks

#### `/rewind` Enhancement
```
/rewind --list             # Show checkpoints
/rewind                    # Interactive selection
/rewind -1                 # Go back 1 checkpoint
```

**Implement:**
- Checkpoint listing with metadata
- Interactive UI for selection
- Scope choice (conversation/code/both)
- `PreRewind`/`PostRewind` hooks

---

## Decision Matrix

### Option A: RECOMMENDED - Implement Only What's Missing
- **Do:** Implement `--sandbox` CLI flag with sandboxing backends
- **Do:** Enhance `/compact` and `/rewind` slash commands
- **Don't:** Add `--compact` and `--rewind` CLI flags (not in spec)
- **Result:** 100% spec compliance, 2-3 days work
- **Risk:** Low

### Option B: Add All as CLI Flags
- **Do:** Add `--compact`, `--rewind`, `--sandbox` as CLI flags
- **Consequence:** Deviates from official Claude Code design
- **Result:** Spec non-compliance, confusing UX
- **Risk:** Medium (maintenance debt)

---

## Recommendation

**Implement Option A** - Phase 1 + Phase 2

This gives you:
1. Correct architectural alignment with Claude Code
2. Clear distinction: CLI flags for startup, slash commands for runtime
3. Actual working `--sandbox` security feature
4. Better UX with enhanced interactive commands
5. All 537 existing tests remain passing

---

## Action Items

### Immediate (This Meeting)
- [ ] Review this summary with team
- [ ] Approve Phase 1 (`--sandbox`) implementation
- [ ] Decision: Keep existing `--resume-from-checkpoint` or add `--rewind` flag anyway?

### Implementation (Next Sprint)
- [ ] Create sandbox module with trait backends
- [ ] Add `--sandbox`/`--no-sandbox` flags
- [ ] Implement macOS sandbox-exec
- [ ] Add firejail support for Linux
- [ ] Integration tests for all platforms
- [ ] Update README and docs

### Following Sprint (if approved)
- [ ] Enhance `/compact` with summarization
- [ ] Enhance `/rewind` with checkpoint UI
- [ ] Add missing hooks

---

## Files Modified

See full details in: `EDGE_CLI_FLAGS_REQUIREMENTS.md` (12 KB comprehensive spec)

**Key Changes:**
- `crates/cli/src/main.rs` - Add CLI flags
- NEW: `crates/cli/src/sandbox/*` - Sandbox implementation
- `crates/cli/src/commands/builtins.rs` - Enhance commands
- `crates/cli/src/hooks/types.rs` - Add new hook types

---

## Reference

- Full specification: `EDGE_CLI_FLAGS_REQUIREMENTS.md`
- Checkpoint system: Already 58 tests passing, fully functional
- Official docs: https://code.claude.com/docs/en/cli-reference
- Current CLI flags: 23 flags implemented, all spec-compliant

---

## Questions?

See comprehensive analysis in `EDGE_CLI_FLAGS_REQUIREMENTS.md` with:
- Detailed flag specifications
- Architecture diagrams
- Security considerations
- Testing strategy
- Risk assessment
- Success criteria

