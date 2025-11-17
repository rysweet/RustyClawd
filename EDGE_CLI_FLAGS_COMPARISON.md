# Edge CLI Flags - Visual Comparison

## At a Glance

```
┌─────────────────────────────────────────────────────────────────────┐
│                    THREE MISSING FLAGS ANALYSIS                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  FLAG              TYPE              IN SPEC    IN RUSTYCLAWD      │
│  ──────────────────────────────────────────────────────────────────│
│  --compact         SLASH COMMAND     Yes        Stub (line 69)     │
│  --rewind          SLASH COMMAND     Yes        Stub (line 42)     │
│  --sandbox         CLI FLAG          Yes        MISSING            │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Feature Comparison

### --compact Flag

```
OFFICIAL CLAUDE CODE:
  Type:     /compact (slash command, interactive)
  Input:    /compact [focus: keyword]
  Use case: Compress conversation history
  Triggers: PreCompact, PostCompact hooks
  Scope:    Conversation messages only

CURRENT RUSTYCLAWD:
  Status:   Stub implementation
  Output:   "Compacting conversation history..."
  Missing:  Actual summarization, token counting

RECOMMENDATION:
  Action:   ENHANCE /compact slash command
  Effort:   2-3 hours
  Status:   NOT a CLI startup flag
```

---

### --rewind Flag

```
OFFICIAL CLAUDE CODE:
  Type:     /rewind (slash command, interactive)
  Input:    /rewind OR Esc+Esc keyboard shortcut
  Use case: Rollback to previous checkpoint
  Triggers: PreRewind, PostRewind hooks
  Scope:    Conversation and/or code state

CLI EQUIVALENT:
  Flag:     --resume-from-checkpoint N (ALREADY EXISTS!)
  Input:    claude --resume-from-checkpoint 2
  Use case: Restore checkpoint on startup
  Triggers: Checkpoint hooks
  Scope:    Conversation and/or code state

CURRENT RUSTYCLAWD:
  /rewind:  Stub implementation
  --resume-from-checkpoint: FULLY IMPLEMENTED (lines 135-137, main.rs)

RECOMMENDATION:
  Action:   ENHANCE /rewind slash command
  Don't add: separate --rewind CLI flag (redundant)
  Effort:   1-2 hours
  Note:     --resume-from-checkpoint already provides CLI equivalent
```

---

### --sandbox Flag

```
OFFICIAL CLAUDE CODE:
  Type:     --sandbox (CLI flag) + /sandbox (slash command)
  CLI:      claude --sandbox "prompt"
  Command:  /sandbox (shows status, can toggle)
  Use case: Isolate execution from host system
  Platforms: macOS (sandbox-exec), Linux (firejail), Windows (process)
  Scope:    Bash tool execution, file system, network

CURRENT RUSTYCLAWD:
  Status:   MISSING (not in Cli struct)
  /sandbox: Stub implementation only
  Missing:  Actual sandboxing backend

RECOMMENDATION:
  Action:   IMPLEMENT --sandbox CLI flag
  Effort:   3-4 hours
  Status:   PROPER CLI startup flag (unlike --compact and --rewind)
  Backends: macOS sandbox, Linux firejail, Windows process containment
```

---

## Decision Tree

```
                    THREE FLAGS
                         │
              ┌──────────┴───────────┐
              │                      │
          --compact               --rewind              --sandbox
              │                      │                      │
          FIND SPEC             FIND SPEC              FIND SPEC
              │                      │                      │
           SLASH CMD              SLASH CMD            CLI FLAG
              │                      │                    │
         BUT WAIT...            BUT WAIT...          GOOD! ADD IT
              │                      │                    │
      Already has a          Already has an         Add to Cli
       CLI equivalent?        equivalent flag?       struct as:
              │                      │                    │
             NO                    YES!                 YES
              │                      │                    │
         Implement:            Don't add CLI!        Implement:
         /compact              --resume-from-        --sandbox/
         slash cmd             checkpoint            --no-sandbox
              │                exists (lines 135)        │
         2-3 hours                  │               3-4 hours
                              Just enhance
                              /rewind cmd
                              1-2 hours
```

---

## Implementation Matrix

| Task | Type | Priority | Effort | Sprint | Status |
|------|------|----------|--------|--------|--------|
| Review this analysis | Review | Critical | 30 min | NOW | Pending |
| Implement `--sandbox` flag | Feature | HIGH | 3-4h | 1 | NOT STARTED |
| Add sandbox backends (macOS) | Feature | HIGH | 1-2h | 1 | NOT STARTED |
| Add sandbox backends (Linux) | Feature | HIGH | 1-2h | 1 | NOT STARTED |
| Sandbox integration tests | Testing | HIGH | 1-2h | 1 | NOT STARTED |
| Enhance `/compact` command | Enhancement | MEDIUM | 2-3h | 2 | NOT STARTED |
| Enhance `/rewind` command | Enhancement | MEDIUM | 1-2h | 2 | NOT STARTED |
| Add checkpoint UI | Enhancement | MEDIUM | 1h | 2 | NOT STARTED |
| Documentation updates | Docs | MEDIUM | 1-2h | 1-2 | NOT STARTED |

**Total Recommended:** 3-4 days (Sprint 1: 2 days, Sprint 2: 1-2 days)

---

## Risk Summary

### LOW RISK
- Enhancing `/compact` (already isolated slash command)
- Enhancing `/rewind` (uses existing checkpoint system)

### MEDIUM RISK
- Implementing `--sandbox`:
  - Security-critical component
  - Platform-specific code (3 backends)
  - File system isolation must be correct
  - Mitigation: Extensive testing, security review

---

## What NOT to Do

```
❌ DON'T: claude --compact "prompt"
   Why: Not in official spec, use /compact instead

❌ DON'T: claude --rewind "prompt"
   Why: Use --resume-from-checkpoint N or /rewind instead

❌ DO: claude --resume-from-checkpoint 2
   Why: This already exists and works!

✓ DO: claude --sandbox "prompt"
   Why: This is missing and needed

✓ DO: claude && /rewind
   Why: Interactive rewind in session

✓ DO: /compact focus: auth
   Why: Compress with focus context
```

---

## Quick Reference

### For Product/Design
- `--compact` and `--rewind` are NOT CLI flags in official Claude Code
- Only `--sandbox` needs to be added as a CLI flag
- This clarification aligns RustyClawd with official spec

### For Engineering
- Phase 1: Implement `--sandbox` flag with backends (2-3 days)
- Phase 2: Enhance `/compact` and `/rewind` commands (1-2 days)
- Total: 2-3 days for full implementation

### For Testing
- Unit tests: Flag parsing, policy engine
- Integration tests: Full sandbox execution on macOS/Linux
- Security tests: Path traversal, network isolation
- All 537 existing tests must still pass

### For Docs
- Add `--sandbox` usage examples
- Clarify `/compact` and `/rewind` are slash commands
- Document `--resume-from-checkpoint` as `/rewind` CLI equivalent
- Add security documentation for sandbox mode

---

## Approval Checklist

- [ ] Confirm: `--compact` NOT added as CLI flag
- [ ] Confirm: `--rewind` NOT added as CLI flag
- [ ] Confirm: `--sandbox` SHOULD be implemented
- [ ] Confirm: Phase 1 (2 days) acceptable effort
- [ ] Confirm: Phase 2 (1-2 days) can wait if needed
- [ ] Assign: Sandbox implementation lead
- [ ] Schedule: Sprint 1 for `--sandbox`
- [ ] Schedule: Sprint 2 for `/compact` and `/rewind` enhancements

---

## See Also

- Full specification: `EDGE_CLI_FLAGS_REQUIREMENTS.md` (comprehensive)
- Executive summary: `EDGE_CLI_FLAGS_SUMMARY.md` (overview)
- Official docs: https://code.claude.com/docs/en/cli-reference
- Issue tracker: https://github.com/rysweet/RustyClawd/issues/51

