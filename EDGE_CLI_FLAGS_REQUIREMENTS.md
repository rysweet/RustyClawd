# Edge CLI Flags Requirements - Issue #51

**Date:** 2025-11-17
**Priority:** P2 (Medium)
**Effort Estimate:** 2-3 days (5-7 hours implementation + testing)
**Complexity:** Medium

---

## Executive Summary

Three missing CLI flags require clarification before implementation:
- `--compact`: Token/context optimization for conversations
- `--rewind`: Conversation rollback functionality
- `--sandbox`: Execution isolation with security controls

**Key Finding:** `--compact` and `--rewind` are primarily **slash commands** (`/compact` and `/rewind`) in official Claude Code, not CLI startup flags. `--sandbox` IS a proper CLI flag.

---

## Flag Analysis

### 1. `--compact` Flag

#### Current State
- **In RustyClawd:** Listed as built-in command (line 69, builtins.rs)
- **Placeholder impl:** `"Compacting conversation history..."`
- **Hook exists:** `PreCompact` hook in hooks system

#### Official Claude Code Behavior
- **Slash command:** `/compact` (interactive mode)
- **Purpose:** Reduces conversation context by summarizing history
- **Use case:** Trim token usage while preserving conversation understanding
- **Scope:** Works on conversation history, not code changes
- **Type:** Session optimization, not a CLI startup flag

#### Classification: NOT a CLI startup flag

**Recommendation:** Do NOT implement as `--compact` CLI flag. Instead:
1. Keep `/compact` slash command (already partially implemented)
2. Full implementation should support:
   - Summarization of old messages
   - Token count reduction display
   - Optional focus instructions (e.g., `/compact focus: keep bug context`)
   - Hook triggering (`PreCompact` hook)

---

### 2. `--rewind` Flag

#### Current State
- **In RustyClawd:** Listed as built-in command (line 15, 42, builtins.rs)
- **Placeholder impl:** `"Rewinding conversation..."`
- **Checkpoint system:** Exists and fully functional (58 tests passing)
- **--resume-from-checkpoint flag:** Already implemented in main.rs (line 135-137)

#### Official Claude Code Behavior
- **Slash command:** `/rewind` (interactive mode)
- **Activation:** Double Escape key (Esc+Esc) or `/rewind` command
- **Purpose:** Rollback conversation to previous points
- **Scope:** Both conversation context AND code state
- **Alternative:** `--resume-from-checkpoint` (CLI flag) already exists for this

#### Classification: NOT a new CLI flag (already covered)

**Recommendation:** Do NOT implement as separate `--rewind` CLI flag. Instead:
1. Keep `/rewind` slash command (interactive mode)
2. **The existing `--resume-from-checkpoint` flag IS the CLI equivalent**
3. Full implementation should support:
   - List available checkpoints
   - Interactive selection of checkpoint to restore
   - Scope selection (conversation only, code only, or both)
   - Clear messaging about what will be restored

#### Clarification
`--rewind` vs `--resume-from-checkpoint`:
```
--resume-from-checkpoint N    # Explicit: restore checkpoint N on startup
/rewind                       # Interactive: choose where to rewind during session
```

Both serve similar purposes but in different contexts.

---

### 3. `--sandbox` Flag

#### Current State
- **In RustyClawd:** Listed as built-in command (line 19, 51, builtins.rs)
- **Placeholder impl:** `"Sandbox mode: Commands will be executed in isolated environment"`
- **No CLI flag implementation:** Not present in main.rs Cli struct
- **No execution isolation:** No actual sandboxing mechanism

#### Official Claude Code Behavior
- **CLI flag:** `--sandbox` and `--no-sandbox`
- **Slash command:** `/sandbox` (enables sandboxed bash execution)
- **Purpose:** Isolate tool execution from host system
- **Scope:**
  - File system containment (restricted file access)
  - Network isolation options
  - Process resource limits
- **Platforms:** macOS (via sandbox), Linux (via firejail/bubblewrap), Windows (via WSL containment)

#### Classification: PROPER CLI flag + slash command

**Recommendation:** IMPLEMENT both:
1. CLI flag: `--sandbox` / `--no-sandbox`
2. Slash command: `/sandbox` for enabling during session
3. Full implementation requires:
   - OS-specific sandboxing backend
   - File access restrictions (.claude/ paths allowed, others configurable)
   - Network policy enforcement
   - Tool execution redirection

---

## Implementation Recommendations

### Priority Matrix

| Flag | Type | Effort | Impact | Recommendation |
|------|------|--------|--------|-----------------|
| `--compact` | Slash command | 2-3 hours | Medium | Enhance `/compact` slash command, NOT CLI flag |
| `--rewind` | Slash command | 1-2 hours | Low | Enhance `/rewind` slash command; `--resume-from-checkpoint` replaces CLI equivalent |
| `--sandbox` | CLI flag + command | 4-6 hours | High | IMPLEMENT as CLI flag + `/sandbox` command |

### Recommended Implementation Order

#### Phase 1 (This Sprint) - 2 days
1. **Document reality** (this file)
2. **Implement `--sandbox` CLI flag** (4-5 hours)
   - Add to Cli struct in main.rs
   - Create sandbox module with trait-based backends
   - macOS: Native sandbox (sandbox-exec)
   - Linux: Firejail integration
   - Windows: Process containment or skip
3. **Enhance `/sandbox` slash command** (1-2 hours)
   - Hook into sandbox mode
   - Display sandbox status
4. **Update documentation** (1 hour)

#### Phase 2 (Next Sprint) - 1-2 days
1. **Enhance `/compact` slash command** (2-3 hours)
   - Implement actual summarization logic
   - Add focus instruction support
   - Trigger PreCompact hooks
2. **Enhance `/rewind` slash command** (1-2 hours)
   - Interactive checkpoint selection
   - Scope selection UI
   - Restore verification

#### Not Recommended
- ~~`--compact` as CLI flag~~ - This should stay as slash command
- ~~`--rewind` as new CLI flag~~ - Already covered by `--resume-from-checkpoint`

---

## Detailed Specifications

### `--sandbox` Implementation

#### CLI Interface
```bash
# Enable sandbox mode on startup
claude --sandbox "your prompt"

# Disable sandbox mode (for unsandboxed operations)
claude --no-sandbox "your prompt"

# Sandbox mode is optional at runtime
claude --sandbox  # Interactive mode with sandbox enabled
```

#### Behavior
1. **Startup:** Apply sandbox restrictions before tool execution
2. **Bash tool:** All commands run in isolated environment
3. **File access:**
   - Allow: `.claude/` directories, specified `--add-dir` paths
   - Block: Home directory, system directories (configurable policy)
4. **Network:** Optionally block outbound connections
5. **Status:** Report sandbox status in `/status` command

#### Configuration
```toml
# .claude/settings.toml
[sandbox]
enabled = true
backend = "firejail"  # or "sandbox" (macOS), "process" (Windows)
policy = "strict"     # or "permissive"
allowed_paths = [".claude", "/tmp"]
block_network = true
resource_limits = { max_memory = "512MB", max_cpu = 1 }
```

#### Hook Integration
```toml
[[hooks.PreSandbox]]
matcher = "*"
command = "echo 'Enabling sandbox mode'"

[[hooks.PostSandbox]]
matcher = "*"
command = "echo 'Sandbox disabled'"
```

#### Architecture
```
sandbox/
├── mod.rs              # Public API
├── backend.rs          # Trait-based abstraction
├── macos.rs            # macOS sandbox-exec implementation
├── linux.rs            # Linux firejail implementation
├── windows.rs          # Windows process containment
└── policy.rs           # File/network policy engine
```

---

### `/compact` Enhancement

#### Slash Command Behavior
```
/compact                          # Default: compress entire history
/compact focus: auth logic        # Compress with retained focus
/compact --token-count            # Show token savings estimate
/compact --keep-recent 10         # Keep last 10 messages
```

#### Output
```
Compacting conversation...
  Original: 4,250 tokens (8 messages)
  After:    2,100 tokens (3 messages + summary)
  Saved:    2,150 tokens (50.6%)
  Focus:    Authentication logic retained

Restart: /compact --apply to save changes
```

#### Implementation
1. Analyze message history for redundancy
2. Identify key concepts from older messages
3. Generate summary while preserving critical context
4. Display before/after token counts
5. Option to apply or cancel
6. Trigger `PreCompact` and `PostCompact` hooks

---

### `/rewind` Enhancement

#### Slash Command Behavior
```
/rewind                           # Interactive: show checkpoints
/rewind --list                    # List available checkpoints
/rewind -1                        # Go back 1 checkpoint
/rewind checkpoint-123            # Restore specific checkpoint
/rewind --scope conversation      # Only restore conversation
/rewind --scope code              # Only restore code changes
```

#### Interactive Selection
```
Available checkpoints (most recent first):
  0. checkpoint-123  [2:30 PM]  "Fixed auth bug"
  1. checkpoint-122  [2:15 PM]  "Added login form"
  2. checkpoint-121  [1:45 PM]  "Initial setup"

Choose checkpoint to restore (0-2):
```

#### Implementation
1. List checkpoints with metadata
2. Interactive selection or explicit ID
3. Preview what will be restored
4. Confirm scope (conversation/code/both)
5. Execute restore via checkpoint system
6. Trigger `PreRewind` and `PostRewind` hooks

---

## Testing Strategy

### Unit Tests
- `--sandbox` flag parsing and validation
- Sandbox policy application
- Path allowlist/blocklist logic
- Token counting for compact operation

### Integration Tests
- Full `--sandbox` mode execution
- Tool execution in sandboxed environment
- `/compact` summarization accuracy
- `/rewind` checkpoint restoration correctness
- Hook triggering for all three commands

### Security Tests (for sandbox)
- Verify file system restrictions enforced
- Test path traversal attempts blocked
- Network isolation verification
- Resource limit enforcement

### Manual Tests
- Interactive `/compact` user flow
- Interactive `/rewind` user flow
- `/sandbox` status reporting
- Cross-platform sandbox behavior (macOS/Linux/Windows)

---

## Files to Modify/Create

### Existing Files
- `crates/cli/src/main.rs` - Add `--sandbox` and `--no-sandbox` flags
- `crates/cli/src/commands/builtins.rs` - Enhance command implementations
- `crates/cli/src/hooks/types.rs` - Add PreSandbox, PostSandbox, PreCompact, PostCompact, PreRewind, PostRewind hooks
- `crates/cli/src/interactive.rs` - Hook into slash command system

### New Files (for sandbox implementation)
- `crates/cli/src/sandbox/mod.rs` - Public sandbox API
- `crates/cli/src/sandbox/backend.rs` - Trait-based backend abstraction
- `crates/cli/src/sandbox/macos.rs` - macOS implementation (sandbox-exec)
- `crates/cli/src/sandbox/linux.rs` - Linux implementation (firejail)
- `crates/cli/src/sandbox/windows.rs` - Windows implementation
- `crates/cli/src/sandbox/policy.rs` - Policy engine for file/network restrictions
- `crates/cli/tests/sandbox_integration_tests.rs` - Integration test suite

---

## Documentation Updates

### Files to Update
1. `README.md` - Add sandbox example usage
2. `CLI_SPEC_COMPLIANCE.md` - Clarify which flags are CLI vs slash commands
3. `MIGRATION_GUIDE.md` - Note: `--rewind` is not added; use `--resume-from-checkpoint` or `/rewind`
4. New: `SANDBOX_SECURITY.md` - Sandbox architecture and security model

### Examples
```bash
# Show that --sandbox is recommended for untrusted prompts
claude --sandbox --verbose "analyze this code"

# Clarify --rewind is NOT added (use existing alternatives)
claude --resume-from-checkpoint 2 "continue from checkpoint 2"

# Interactive rewind inside session
/rewind  # List checkpoints and choose
```

---

## Risk Assessment

### Low Risk
- `/compact` enhancement (modifies only interactive mode)
- `/rewind` enhancement (builds on existing checkpoint system)

### Medium Risk
- `--sandbox` CLI flag (security-critical, OS-dependent)
  - Mitigation: Extensive test coverage, security audit before merge
  - Platform fallback: If sandbox unavailable, warn and proceed unconfined

### Dependencies
- Firejail: Linux systems (optional, graceful degradation if missing)
- sandbox-exec: macOS (built-in)
- No external deps for Windows process containment

---

## Success Criteria

1. `--sandbox` flag works on macOS (via sandbox-exec) and Linux (via firejail or fallback)
2. `/compact` displays accurate token count savings
3. `/rewind` correctly restores checkpoints with selected scope
4. All existing tests continue to pass (537 tests)
5. No new security vulnerabilities introduced
6. Documentation clearly distinguishes CLI flags vs slash commands
7. 80%+ code coverage for new sandbox module

---

## Open Questions for Product/Design

1. Should `--sandbox` be ON by default for security? Or OFF by default for performance?
   - **Recommended:** OFF by default (opt-in for security)

2. Should `/compact` support LLM-powered summarization or rule-based?
   - **Recommended:** Start with rule-based, upgrade to LLM later

3. For sandbox network isolation, should it be:
   - Complete block (safe, restrictive)
   - Allow specific ports (flexible, complex policy)
   - No network isolation (current proposal)

4. Should checkpoint retention limit (currently 50) be configurable per-session?
   - **Recommended:** YES, via CLI flag `--checkpoint-limit N`

---

## Conclusion

### Do NOT Implement
- ~~`--compact` as CLI flag~~ (slash command only)
- ~~`--rewind` as CLI flag~~ (slash command + `--resume-from-checkpoint` exists)

### DO Implement
- **`--sandbox` CLI flag** with execution isolation (2-3 day effort)
- **Enhance `/compact` slash command** with actual summarization (1-2 days)
- **Enhance `/rewind` slash command** with interactive UI (1-2 days)

### Priority
**Phase 1 (Next Sprint):** `--sandbox` implementation only
**Phase 2 (Following Sprint):** `/compact` and `/rewind` enhancements

This aligns with official Claude Code design (slash commands for session control, CLI flags for startup configuration).

