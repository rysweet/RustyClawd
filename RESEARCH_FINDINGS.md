# Edge CLI Flags (Issue #51) - Research Findings

**Completion Date:** 2025-11-17
**Status:** RESEARCH COMPLETE - READY FOR IMPLEMENTATION PLANNING
**Key Finding:** Three flags are MISCLASSIFIED - only ONE is a CLI flag

---

## Quick Answer

### What Should Be Implemented?

```
IMPLEMENT:   --sandbox (CLI flag) - MISSING, needed for security
ENHANCE:     /compact (slash command) - Already partially implemented
ENHANCE:     /rewind (slash command) - Already partially implemented
DO NOT ADD:  --compact (NOT a CLI flag in official Claude Code)
DO NOT ADD:  --rewind (NOT a CLI flag; --resume-from-checkpoint already exists)
```

---

## The Three Flags Explained

### Flag 1: --compact

| Aspect | Detail |
|--------|--------|
| **Official Type** | Slash command: `/compact` (interactive) |
| **Purpose** | Compress conversation history to reduce tokens |
| **Current State** | Stub implementation in builtins.rs (line 69) |
| **Action** | ENHANCE slash command, NOT add as CLI flag |
| **Effort** | 2-3 hours |
| **Implementation** | Add summarization logic, token counting, focus instruction support |

**Key Quote from Research:**
"In official Claude Code, `/compact` is a slash command used in interactive sessions to compress conversation history."

---

### Flag 2: --rewind

| Aspect | Detail |
|--------|--------|
| **Official Type** | Slash command: `/rewind` (interactive) |
| **Purpose** | Rollback conversation to previous checkpoint |
| **CLI Equivalent** | `--resume-from-checkpoint N` (ALREADY IMPLEMENTED) |
| **Current State** | Stub in builtins.rs (line 42); flag doesn't exist in main.rs |
| **Action** | ENHANCE slash command; checkpoint system fully functional |
| **Effort** | 1-2 hours |
| **Implementation** | Add checkpoint listing, interactive UI, scope selection |

**Key Finding:**
The functionality already exists via `--resume-from-checkpoint` which was added in main.rs (lines 135-137). This is the CLI equivalent of `/rewind`.

```rust
// Already in main.rs:
#[arg(long)]
resume_from_checkpoint: Option<usize>,  // <-- This IS the CLI "rewind" equivalent
```

---

### Flag 3: --sandbox

| Aspect | Detail |
|--------|--------|
| **Official Type** | CLI flag: `--sandbox` and `--no-sandbox` |
| **Purpose** | Enable execution isolation for untrusted code |
| **Current State** | COMPLETELY MISSING (not in Cli struct) |
| **Slash Command** | `/sandbox` also exists to show/toggle status |
| **Action** | IMPLEMENT as CLI flag + enhance slash command |
| **Effort** | 3-4 hours |
| **Platforms** | macOS (sandbox-exec), Linux (firejail), Windows (process containment) |

**Official Spec Confirmation:**
"The `--sandbox` flag enables sandbox mode for security on macOS/Linux. Additionally, the `/sandbox` slash command enables sandboxed bash tool execution with filesystem and network isolation."

---

## Research Evidence

### Source 1: Official Claude Code CLI Reference
- URL: https://code.claude.com/docs/en/cli-reference
- Finding: `--sandbox` is documented as a CLI flag
- Finding: `--compact` and `--rewind` NOT listed as CLI flags

### Source 2: GitHub Issue #9119 (Claude Code)
- Title: "Implement all CLI commands in Claude Code VS Code Extension + /rewind priority"
- Finding: `/rewind` is a slash command with optional Esc+Esc keyboard shortcut
- Finding: Similar to Cursor's message revert functionality

### Source 3: CLI Command Gist (dai/github)
- Lists all official Claude Code CLI flags
- Includes: `--add-dir`, `--allowedTools`, `--print`, `--verbose`, `--model`, etc.
- Excludes: `--compact`, `--rewind` (not CLI flags)
- Includes: `--sandbox` (proper CLI flag)

### Source 4: RustyClawd Codebase
- Checkpoint system: 58 passing tests, fully functional
- `--resume-from-checkpoint` flag: Already implemented in main.rs (lines 135-137)
- Builtin commands: `/compact`, `/rewind`, `/sandbox` are listed but stubbed

---

## Architecture Implications

### Current State (RustyClawd)

```
CLI Flags (startup):          /Slash Commands (interactive):
--model                       /help
--add-dir                     /clear
--system-prompt               /exit
--append-system-prompt        /compact (STUB)
--resume-from-checkpoint      /rewind (STUB)
--continue                    /sandbox (STUB)
--resume                      + 30+ more commands
(+ 18 more)

MISSING:
--sandbox                     (CLI flag)
```

### After Implementation

```
CLI Flags (startup):          /Slash Commands (interactive):
--model                       /help
--add-dir                     /clear
--system-prompt               /exit
--append-system-prompt        /compact (ENHANCED)
--resume-from-checkpoint      /rewind (ENHANCED)
--continue                    /sandbox (ENHANCED)
--sandbox <-- NEW             + 30+ more commands
--no-sandbox <-- NEW
(+ 19 total)
```

---

## Impact Assessment

### Low Impact Changes (Slash Commands)
- Enhancing `/compact` and `/rewind` is LOW RISK
- Isolated to interactive mode
- Uses existing checkpoint system
- No security implications

### Medium Impact Change (--sandbox)
- Adding `--sandbox` CLI flag is MEDIUM-HIGH RISK
- Security-critical component
- Platform-specific (macOS/Linux/Windows)
- Requires careful testing

### Zero Impact Decision
- NOT adding `--compact` or `--rewind` as CLI flags
- Maintains alignment with official Claude Code architecture
- Prevents API confusion

---

## Recommended Implementation Timeline

### Phase 1: High Priority (2 days)
**Implement: `--sandbox` CLI flag**

1. Day 1 (3-4 hours):
   - Add `--sandbox` and `--no-sandbox` to Cli struct
   - Create sandbox module with trait-based backends
   - Implement macOS backend (sandbox-exec)
   - Implement Linux backend (firejail)

2. Day 2 (1-2 hours):
   - Tool executor integration
   - Hook integration
   - Integration tests
   - Documentation

**Success Criteria:**
- CLI flag parsing works
- macOS execution works
- Linux execution works (with firejail fallback)
- 20+ tests pass
- Security audit complete

### Phase 2: Medium Priority (1-2 days, optional)
**Enhance: `/compact` and `/rewind` slash commands**

1. `/compact` enhancement (2-3 hours):
   - Implement summarization logic
   - Add token counting
   - Support focus instructions
   - Trigger hooks

2. `/rewind` enhancement (1-2 hours):
   - Add checkpoint listing
   - Interactive selection UI
   - Scope selection (conversation/code/both)
   - Clear messaging

---

## What's Already Done

No need to implement from scratch:

1. **Checkpoint System** (58 tests passing)
   - Full session persistence
   - Automatic checkpoint creation
   - Restoration with scoping
   - File integrity verification

2. **--resume-from-checkpoint Flag** (already in main.rs)
   - Allows restoring from checkpoint N at startup
   - This IS the CLI equivalent of `/rewind`

3. **Builtin Commands Framework**
   - Slash command routing
   - Hook system integration
   - Help system

---

## Files to Review

### For Technical Details
1. **EDGE_CLI_FLAGS_REQUIREMENTS.md** (14 KB)
   - Complete specification for all three flags
   - Technical considerations
   - Testing strategy
   - Risk assessment
   - Success criteria

2. **SANDBOX_IMPLEMENTATION_GUIDE.md** (21 KB)
   - Step-by-step implementation for `--sandbox`
   - Complete code examples
   - Architecture diagrams
   - Platform-specific implementation details
   - Testing checklist

### For Decision Making
3. **EDGE_CLI_FLAGS_SUMMARY.md** (5.6 KB)
   - Executive summary
   - Decision matrix
   - Priority timeline
   - Key findings

4. **EDGE_CLI_FLAGS_COMPARISON.md** (8.2 KB)
   - Visual comparison
   - Decision tree
   - Implementation matrix
   - Risk summary

---

## Key Decisions Made

### Decision 1: --compact Classification
**Decision:** NOT a CLI flag, enhance slash command only
**Rationale:** Official Claude Code only has `/compact` slash command
**Impact:** Prevents API confusion, maintains spec alignment

### Decision 2: --rewind Classification
**Decision:** NOT a new CLI flag (redundant with existing `--resume-from-checkpoint`)
**Rationale:** CLI equivalent already exists and works
**Impact:** Reduces scope, no redundant features

### Decision 3: --sandbox Priority
**Decision:** IMPLEMENT as highest priority
**Rationale:** Security-critical, missing from official spec compliance
**Impact:** Enables secure code execution for untrusted prompts

---

## Questions for Stakeholders

### For Product Team
1. Should `--sandbox` be ON by default (security-first) or OFF by default (performance-first)?
   - **Recommendation:** OFF by default (opt-in for security)

2. Should sandbox policy be configurable per-session?
   - **Recommendation:** YES via CLI flags

3. Should `/compact` use LLM summarization or rule-based?
   - **Recommendation:** Start with rule-based, upgrade later

### For Engineering Team
1. Approval to implement `--sandbox` in Phase 1?
   - **Recommended:** YES (2-3 days)

2. Should `/compact` and `/rewind` enhancements wait for Phase 2?
   - **Recommended:** YES (not critical)

3. Should we add CI/CD tests for sandbox on Linux/macOS?
   - **Recommended:** YES before merge

---

## Deliverables

This research provides:

1. **Clear Classification** - Which flags are CLI vs slash commands
2. **Complete Specification** - Technical details for implementation
3. **Implementation Guide** - Step-by-step code for `--sandbox`
4. **Testing Strategy** - Test coverage and verification approach
5. **Timeline** - Realistic effort and sprint estimates
6. **Risk Assessment** - Security considerations and mitigations
7. **Success Criteria** - Measurable outcomes

---

## Next Steps

### Immediate (This Week)
- [ ] Review all four documents with technical team
- [ ] Approve Phase 1 (--sandbox) implementation
- [ ] Decision on Phase 2 (/compact and /rewind enhancements)

### Implementation (Next Sprint)
- [ ] Assign sandbox implementation lead
- [ ] Create GitHub issue for Phase 1 work
- [ ] Create GitHub issue for Phase 2 work (if approved)
- [ ] Schedule code review and security audit

### Documentation
- [ ] Update README with sandbox examples
- [ ] Add security documentation
- [ ] Update CLI specification docs
- [ ] Create migration guide (if needed)

---

## Summary

**The three "missing" flags are not actually missing - they're MISCLASSIFIED.**

- **`--compact`**: Exists as `/compact` slash command, needs enhancement
- **`--rewind`**: Exists as `/rewind` slash command + `--resume-from-checkpoint` flag
- **`--sandbox`**: Actually missing, should be implemented as CLI flag

**Recommended approach:**
1. Implement `--sandbox` as CLI flag (HIGH PRIORITY)
2. Enhance `/compact` and `/rewind` slash commands (MEDIUM PRIORITY, can wait)
3. Do NOT add `--compact` or `--rewind` as CLI flags (maintains spec alignment)

**Total effort:** 2-3 days for Phase 1, 1-2 days for Phase 2 (optional)

---

## Documents Provided

1. **EDGE_CLI_FLAGS_REQUIREMENTS.md** - Comprehensive specification
2. **SANDBOX_IMPLEMENTATION_GUIDE.md** - Implementation guide with code examples
3. **EDGE_CLI_FLAGS_SUMMARY.md** - Executive summary
4. **EDGE_CLI_FLAGS_COMPARISON.md** - Visual comparison and decision tree
5. **RESEARCH_FINDINGS.md** - This document

All files are in: `/home/azureuser/src/RustyClawd/`

