# Main CLI Integration - Complete Specification Index

## Overview

This document serves as the master index for the complete architectural specification of the Claude Code CLI main entry point orchestration layer.

**Total Documentation:** 3,982 lines across 6 comprehensive specification documents

## Document Hierarchy

```
MASTER INDEX (this file)
│
├─ ARCHITECTURE_DESIGN_COMPLETE.md     (Project status, summary, guidance)
│  └─ START HERE to understand what's being built
│
├─ BUILDER_HANDOFF.md                  (Everything builder needs to know)
│  └─ START HERE if you're implementing
│
├─ MAIN_INTEGRATION_ARCHITECTURE.md    (High-level design)
│  └─ START HERE to understand system integration
│
├─ MAIN_MODULE_SPECIFICATION.md        (Module contracts and details)
│  └─ START HERE for implementation reference
│
├─ MAIN_IMPLEMENTATION_ROADMAP.md      (Step-by-step phased approach)
│  └─ START HERE for daily implementation guidance
│
└─ MAIN_QUICK_REFERENCE.md             (Lookup and patterns)
   └─ START HERE during development for quick answers
```

## Document Purposes and Use Cases

### 1. ARCHITECTURE_DESIGN_COMPLETE.md (12 KB)

**What It Contains:**
- Project status and completion summary
- Overview of all 5 deliverables
- Architecture summary (4 phases, 8 systems, 5 modes)
- Key innovation (ExecutionContext)
- Critical design principles
- Quality assurance notes
- File changes required
- Key metrics
- How to use these documents
- Risk assessment with mitigation
- Success criteria
- Next steps

**Best For:**
- Project managers understanding scope
- Architects reviewing the design
- Anyone new to the project
- Status updates and summaries

**Read Time:** 15-20 minutes

---

### 2. BUILDER_HANDOFF.md (15 KB)

**What It Contains:**
- Quick summary (what you're building)
- Visual representation of the system
- Files to create/modify
- Key concepts explained
- ExecutionContext deep dive
- Hook lifecycle
- Permission model
- Checkpoint strategy
- Signal handling
- Implementation strategy (Phase A-E)
- Testing strategy
- Common pitfalls with code examples
- Integration points with existing modules
- FAQ section
- Quick reference for code structure
- Deliverables checklist
- Success criteria

**Best For:**
- Builder agents starting implementation
- Context setting for new team members
- Understanding why decisions were made
- Troubleshooting during implementation

**Read Time:** 30-40 minutes

---

### 3. MAIN_INTEGRATION_ARCHITECTURE.md (17 KB)

**What It Contains:**
- System overview and layers
- Startup sequence (7 steps) with code flow
- Mode detection and dispatch for all 5 modes
- Lifecycle management (hooks, error recovery, checkpoints)
- Shutdown sequence (4 steps)
- Type system definitions
- Error handling patterns (3 patterns with code)
- Implementation phases (A-E)
- Testing strategy (unit + integration)
- Constraints and assumptions
- Future extensions

**Best For:**
- Architects reviewing design decisions
- Understanding how all pieces fit together
- Code reviewers validating design
- Planning future extensions
- Learning the overall architecture

**Read Time:** 40-50 minutes

---

### 4. MAIN_MODULE_SPECIFICATION.md (17 KB)

**What It Contains:**
- Module organization and file structure
- main.rs module (600-900 lines)
  - Purpose, contract, dependencies
  - Implementation notes and structure
  - Key functions with signatures
  - Signal handling
  - Error handling strategy
- context.rs module (200 lines)
  - ExecutionContext definition
  - PermissionMatrix definition
- error.rs module (150 lines)
  - CliError enum definition
  - Exit code mapping
- Integration points with all 8 systems
- Testing strategy (unit and integration)
- Build and deployment
- Key design decisions

**Best For:**
- Implementation reference during coding
- Code review against specification
- Understanding module contracts
- Testing requirements
- Type definitions

**Read Time:** 45-60 minutes

---

### 5. MAIN_IMPLEMENTATION_ROADMAP.md (28 KB)

**What It Contains:**
- 5 implementation phases (A through E)
- Phase A: Foundation (1-2 days)
  - Module skeleton
  - Type definitions
  - Error handling
  - Stub main.rs
  - Test infrastructure
- Phase B: Initialization Systems (2-3 days)
  - Settings loading
  - Hooks system
  - Plugin loading
  - Checkpoints
  - Permission matrix
  - SessionStart hooks
  - Signal handling
- Phase C: Mode Dispatch (2-3 days)
  - CLI enum update
  - Mode routing
  - All 5 mode handlers
  - Error handling
- Phase D: Lifecycle Management (1-2 days)
  - Pre/post hooks
  - Permission checking
  - Checkpoint scheduling
  - Error recovery
- Phase E: Shutdown Sequence (1-2 days)
  - Shutdown guard
  - SessionEnd hooks
  - Final checkpoint
  - Resource cleanup
  - Exit codes
- Acceptance criteria per phase
- Risk mitigation
- Timeline estimate
- Success metrics

**Best For:**
- Daily implementation guidance
- Phase-by-phase progress tracking
- Code sketches and templates
- Risk identification
- Timeline planning

**Read Time:** 60-90 minutes (reference as you build)

---

### 6. MAIN_QUICK_REFERENCE.md (17 KB)

**What It Contains:**
- File map with organization
- Execution flow diagram (ASCII art)
- State machines (execution phases, hook sequence)
- Key data structures
- Command syntax reference
- Hook execution patterns
- Permission checking patterns
- Checkpoint strategy
- Testing quick checks
- Common code patterns
- Exit code reference
- Dependency graph
- Performance considerations
- Debug logging guide

**Best For:**
- Quick lookup during development
- Pattern reference for implementation
- Troubleshooting
- Understanding data flow
- Command syntax reminders
- Performance tuning

**Read Time:** 5-10 minutes (lookup as needed)

---

## Reading Paths

### Path 1: New Project Manager
1. ARCHITECTURE_DESIGN_COMPLETE.md (15 min)
2. BUILDER_HANDOFF.md - "Quick Summary" section (5 min)
3. MAIN_QUICK_REFERENCE.md - "State Machines" section (5 min)

**Total:** 25 minutes

---

### Path 2: New Architect
1. ARCHITECTURE_DESIGN_COMPLETE.md (15 min)
2. MAIN_INTEGRATION_ARCHITECTURE.md (40 min)
3. MAIN_MODULE_SPECIFICATION.md (45 min)
4. MAIN_QUICK_REFERENCE.md (10 min)

**Total:** 110 minutes

---

### Path 3: New Builder (Implementation)
1. BUILDER_HANDOFF.md (40 min)
2. MAIN_IMPLEMENTATION_ROADMAP.md - Phase A (30 min)
3. MAIN_MODULE_SPECIFICATION.md (45 min)
4. MAIN_QUICK_REFERENCE.md (reference as needed)

**Total:** 115 minutes before starting

---

### Path 4: Code Reviewer
1. MAIN_MODULE_SPECIFICATION.md - Module contracts (30 min)
2. MAIN_IMPLEMENTATION_ROADMAP.md - Specific phase being reviewed (20 min)
3. MAIN_QUICK_REFERENCE.md - Patterns and exit codes (10 min)
4. ARCHITECTURE_DESIGN_COMPLETE.md - Design decisions (15 min)

**Total:** 75 minutes

---

### Path 5: Troubleshooter (During Implementation)
1. MAIN_QUICK_REFERENCE.md - Quick lookup (2 min)
2. BUILDER_HANDOFF.md - FAQ section (5 min)
3. MAIN_IMPLEMENTATION_ROADMAP.md - Current phase (10 min)
4. MAIN_MODULE_SPECIFICATION.md - Specific module (10 min)

**Total:** As needed during implementation

---

## Document Statistics

| Document | Lines | Size | Purpose |
|----------|-------|------|---------|
| ARCHITECTURE_DESIGN_COMPLETE | 340 | 12 KB | Project status & overview |
| BUILDER_HANDOFF | 520 | 15 KB | Builder context & guidance |
| MAIN_INTEGRATION_ARCHITECTURE | 680 | 17 KB | System design & integration |
| MAIN_MODULE_SPECIFICATION | 620 | 17 KB | Module contracts & details |
| MAIN_IMPLEMENTATION_ROADMAP | 1,080 | 28 KB | Step-by-step roadmap |
| MAIN_QUICK_REFERENCE | 500 | 17 KB | Lookup & patterns |
| **TOTAL** | **3,740** | **106 KB** | **Complete specification** |

---

## Key Concepts Explained Across Documents

### ExecutionContext
- **Introduced in:** BUILDER_HANDOFF.md
- **Defined in:** MAIN_MODULE_SPECIFICATION.md
- **Detailed in:** MAIN_QUICK_REFERENCE.md
- **Used throughout:** All documents

Core struct carrying all state through execution. Everything flows through it.

### Hook Lifecycle
- **Overview in:** ARCHITECTURE_DESIGN_COMPLETE.md
- **Details in:** MAIN_INTEGRATION_ARCHITECTURE.md
- **Pattern in:** BUILDER_HANDOFF.md
- **Reference in:** MAIN_QUICK_REFERENCE.md

8 hook points (SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop, PreCompact, SessionEnd, + 1 future).

### 4-Phase Execution
- **Overview in:** ARCHITECTURE_DESIGN_COMPLETE.md
- **Detailed in:** MAIN_INTEGRATION_ARCHITECTURE.md
- **Diagram in:** MAIN_QUICK_REFERENCE.md

Startup → Dispatch → Lifecycle → Shutdown

### 5 Command Modes
- **Introduced in:** BUILDER_HANDOFF.md
- **Routed in:** MAIN_MODULE_SPECIFICATION.md
- **Implemented in:** MAIN_IMPLEMENTATION_ROADMAP.md - Phase C

Chat, Tool, Command, Plugin, Agent

### Error Handling
- **Strategy in:** MAIN_INTEGRATION_ARCHITECTURE.md
- **Implementation in:** MAIN_MODULE_SPECIFICATION.md - error.rs
- **Patterns in:** BUILDER_HANDOFF.md - Common Pitfalls

Exit codes 0, 1, 2, 10-22, 130, 143

### Signal Handling
- **Setup in:** MAIN_IMPLEMENTATION_ROADMAP.md - Phase B
- **Pattern in:** BUILDER_HANDOFF.md
- **Reference in:** MAIN_QUICK_REFERENCE.md

SIGINT (Ctrl+C), SIGTERM, graceful shutdown

### Checkpoint Strategy
- **Overview in:** ARCHITECTURE_DESIGN_COMPLETE.md
- **Integration in:** MAIN_INTEGRATION_ARCHITECTURE.md
- **Implementation in:** MAIN_IMPLEMENTATION_ROADMAP.md - Phase B & D
- **Reference in:** MAIN_QUICK_REFERENCE.md

Save on write/edit, periodically, on error, on shutdown

### Permission Model
- **Defined in:** BUILDER_HANDOFF.md
- **Implementation in:** MAIN_MODULE_SPECIFICATION.md - PermissionMatrix
- **Pattern in:** MAIN_QUICK_REFERENCE.md

Three modes: Allow, Deny, Ask

---

## Implementation Phases Quick Reference

| Phase | Duration | Focus | Documents |
|-------|----------|-------|-----------|
| A: Foundation | 1-2 days | Types, skeleton, errors | Roadmap A.1-A.5 |
| B: Initialization | 2-3 days | Settings, hooks, plugins, checkpoints | Roadmap B.1-B.8 |
| C: Mode Dispatch | 2-3 days | Routing, all 5 modes, error handling | Roadmap C.1-C.8 |
| D: Lifecycle | 1-2 days | Hooks, permissions, checkpoints, errors | Roadmap D.1-D.4 |
| E: Shutdown | 1-2 days | SessionEnd, final checkpoint, cleanup | Roadmap E.1-E.6 |
| **TOTAL** | **7-12 days** | **Complete orchestration** | **All documents** |

---

## Files to Create/Modify

### New Files
1. `/crates/cli/src/context.rs` (200 lines) - [Specified in](MAIN_MODULE_SPECIFICATION.md#module-contextrs)
2. `/crates/cli/src/error.rs` (150 lines) - [Specified in](MAIN_MODULE_SPECIFICATION.md#module-errorsrs)
3. `/tests/integration_main.rs` (500+ lines) - [Specified in](MAIN_IMPLEMENTATION_ROADMAP.md#a5-add-test-infrastructure)

### Modified Files
1. `/crates/cli/src/main.rs` (700-900 lines) - [Specified in](MAIN_MODULE_SPECIFICATION.md#module-mainrs)
2. `/crates/cli/src/lib.rs` (2 additions) - [Specified in](MAIN_IMPLEMENTATION_ROADMAP.md#a3-update-librs-exports)
3. `/Cargo.toml` (1 addition) - Add `signal-hook-tokio = "0.3"`

### Unchanged Files
All existing modules work as-is with new main orchestrating them.

---

## Quick Navigation

### Looking for...

**How to build this?**
→ Start with [MAIN_IMPLEMENTATION_ROADMAP.md](MAIN_IMPLEMENTATION_ROADMAP.md)

**Why was this designed this way?**
→ Check [MAIN_INTEGRATION_ARCHITECTURE.md](MAIN_INTEGRATION_ARCHITECTURE.md) - "Key Design Decisions"

**What's the contract for module X?**
→ See [MAIN_MODULE_SPECIFICATION.md](MAIN_MODULE_SPECIFICATION.md) - "Module: X"

**Quick code pattern for Y?**
→ Find in [MAIN_QUICK_REFERENCE.md](MAIN_QUICK_REFERENCE.md) - "Common Code Patterns"

**Exit code for error Z?**
→ Reference [MAIN_QUICK_REFERENCE.md](MAIN_QUICK_REFERENCE.md) - "Exit Code Reference"

**How do hooks work?**
→ Read [BUILDER_HANDOFF.md](BUILDER_HANDOFF.md) - "Hook Lifecycle"

**What's ExecutionContext?**
→ Learn in [BUILDER_HANDOFF.md](BUILDER_HANDOFF.md) - "Key Concepts"

**Phase timeline?**
→ Check [MAIN_IMPLEMENTATION_ROADMAP.md](MAIN_IMPLEMENTATION_ROADMAP.md) - "Timeline Estimate"

**How to test?**
→ See [MAIN_MODULE_SPECIFICATION.md](MAIN_MODULE_SPECIFICATION.md) - "Testing Strategy"

**Common mistakes?**
→ Review [BUILDER_HANDOFF.md](BUILDER_HANDOFF.md) - "Common Pitfalls to Avoid"

---

## Success Criteria

### By End of Phase A
- [ ] All modules compile
- [ ] Types are defined
- [ ] Main.rs skeleton in place
- [ ] No runtime errors

### By End of Phase B
- [ ] Settings load from hierarchy
- [ ] All systems initialize
- [ ] Session recovery works
- [ ] Signals handled

### By End of Phase C
- [ ] All 5 modes work
- [ ] Proper error codes
- [ ] Dispatch routing works

### By End of Phase D
- [ ] Hooks execute at all points
- [ ] Permissions enforced
- [ ] Checkpoints save
- [ ] Errors escalate properly

### By End of Phase E
- [ ] Graceful shutdown
- [ ] Resources cleaned up
- [ ] Exit codes correct
- [ ] Integration tests pass

---

## Maintenance and Updates

### If Requirements Change
1. Update relevant specification document(s)
2. Flag the change across all documents
3. Update implementation roadmap
4. Notify builder of implications

### If Implementation Diverges
1. Document why in code comments
2. Update specification to match
3. File architecture review if needed
4. Ensure documentation stays current

### Future Extensions
See [MAIN_INTEGRATION_ARCHITECTURE.md](MAIN_INTEGRATION_ARCHITECTURE.md) - "Future Extensions" for planned enhancements.

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-11-11 | Initial complete specification |

---

## Contact & Questions

For questions about:
- **Architecture decisions:** See MAIN_INTEGRATION_ARCHITECTURE.md
- **Implementation details:** See MAIN_MODULE_SPECIFICATION.md
- **Build process:** See MAIN_IMPLEMENTATION_ROADMAP.md
- **Day-to-day guidance:** See BUILDER_HANDOFF.md
- **Quick reference:** See MAIN_QUICK_REFERENCE.md

---

**The complete specification is ready for implementation.**

All decisions have been documented. All integration points have been mapped. All error cases have recovery strategies.

Begin with your appropriate reading path above, then proceed to implementation with confidence.

**Status: READY FOR BUILDER**
