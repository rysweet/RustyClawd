# Architecture Design: --agents JSON Flag (Issue #110)

**Status**: Architecture Specification Complete
**Date**: 2025-12-08
**Scope**: CLI flag integration for runtime agent definitions

---

## Executive Summary

The `--agents` JSON flag architecture implements runtime agent definition through three independent layers:

1. **Parse Layer** (`parse_runtime_agents()`): JSON string → HashMap
2. **Validate Layer** (`validate_runtime_agents()`): Check field constraints
3. **Merge Layer** (`AgentDiscovery::with_runtime_agents()`): Combine file + runtime agents

Zero dependencies between layers. Zero magic. Ruthlessly simple.

---

## What's Already Done

**Location**: `crates/cli/src/plugins/agent_discovery.rs`

- ✅ `RuntimeAgentDefinition` struct with serde support
- ✅ `parse_runtime_agents()` function (JSON → HashMap)
- ✅ `validate_runtime_agents()` function (constraint checking)
- ✅ `AgentDiscovery::with_runtime_agents()` method (storage)
- ✅ `AgentDiscovery::all_agents()` merge logic
- ✅ Complete unit test suite (26 tests)
- ✅ CLI flag definition in `main.rs:78`

---

## What Needs to Be Done

**Location**: `crates/cli/src/main.rs`

- [ ] Integration in main function (~20 lines)
  - Parse --agents flag
  - Call parse_runtime_agents()
  - Call validate_runtime_agents()
  - Merge via with_runtime_agents()

- [ ] Error handling (~10 lines)
  - Print errors to stderr
  - Exit on failure

**Location**: `crates/cli/src/plugins/mod.rs`

- [ ] Export functions (~5 lines)
  - parse_runtime_agents
  - validate_runtime_agents
  - RuntimeAgentDefinition

**Testing**:

- [ ] Integration test (~30 lines)
  - CLI arg → discovery merge

- [ ] E2E test (~50 lines)
  - Runtime agent actually used

---

## Architecture Principles

### 1. Single Responsibility
Each component does ONE thing:
- Parse: Handle JSON syntax only
- Validate: Check field constraints only
- Merge: Combine two agent sources only

### 2. Fail Fast, Report All
- Parse errors: Single error for malformed JSON
- Validation errors: Collect all errors, report together
- Never silent failures

### 3. File Agents Win on Collision
If runtime agent has same ID as file agent:
- File agent is kept
- Runtime agent is ignored
- Preserves explicit project agents

### 4. Clear Precedence
Runtime agents converted to `AgentDefinition` format:
```
path: "runtime:agent_id"  ← Special marker for runtime origin
```

Downstream code can detect runtime agents via path prefix.

---

## Data Flow Overview

```
User Input
  ↓
--agents '{"agent": {...}}'
  ↓
parse_runtime_agents()
  ↓
HashMap<String, RuntimeAgentDefinition>
  ↓
validate_runtime_agents()
  ↓
Ok(()) or Err(Vec<String>)
  ↓
with_runtime_agents()
  ↓
AgentDiscovery with runtime agents stored
  ↓
all_agents()
  ↓
Vec<AgentDefinition> (merged!)
  ↓
Downstream systems use merged agents
```

---

## Specification Documents

### 1. AGENTS_JSON_FLAG_ARCHITECTURE.md
**What**: Complete architectural specification
**Contains**:
- Problem statement
- Solution overview
- Component definitions
- Data flow
- Integration points
- Validation logic
- Error messages
- Testing strategy
- Trade-offs and decisions

**Read this for**: Understanding the full design

### 2. AGENTS_JSON_INTEGRATION_GUIDE.md
**What**: Quick reference for integration
**Contains**:
- Status overview (what's done/TODO)
- Integration checklist
- Code snippets for main.rs
- Error handling examples
- API contract
- Precedence rules
- Testing pyramid

**Read this for**: Implementing the integration

### 3. AGENTS_JSON_FLOW.md
**What**: Visual diagrams and flows
**Contains**:
- End-to-end data flow
- Data structure transformations
- Validation logic tree
- Merging strategy
- Error handling pathways
- Component responsibility matrix
- Class diagram
- State transitions
- Invocation timeline
- Decision points
- Testing map
- Complete example

**Read this for**: Understanding the visual flow

---

## API Contract Summary

### parse_runtime_agents()
```rust
pub fn parse_runtime_agents(json_str: &str) -> Result<HashMap<String, RuntimeAgentDefinition>, String>
```
- Input: JSON string
- Output: HashMap or single error
- Responsibility: JSON syntax only

### validate_runtime_agents()
```rust
pub fn validate_runtime_agents(agents: &HashMap<String, RuntimeAgentDefinition>) -> Result<(), Vec<String>>
```
- Input: HashMap of agents
- Output: Ok or all validation errors
- Responsibility: Field constraints only

### RuntimeAgentDefinition
```rust
pub struct RuntimeAgentDefinition {
    pub description: String,      // Required, non-empty
    pub prompt: String,           // Required, non-empty
    pub tools: Vec<String>,       // Optional, defaults empty
    pub model: Option<String>,    // Optional
}
```

### AgentDiscovery::with_runtime_agents()
```rust
pub fn with_runtime_agents(mut self, agents: HashMap<String, RuntimeAgentDefinition>) -> Self
```
- Stores runtime agents
- Returns Self for method chaining
- Responsibility: Storage only

### AgentDiscovery::all_agents()
```rust
pub fn all_agents(&self) -> Result<Vec<AgentDefinition>, String>
```
- Returns: File agents + runtime agents merged
- Precedence: File agents first (win on collision)
- Responsibility: Merging only

---

## JSON Schema

### Format
```json
{
  "agent_name": {
    "description": "Human readable description",
    "prompt": "System prompt for agent",
    "tools": ["Tool1", "Tool2"],  // Optional
    "model": "sonnet"              // Optional
  }
}
```

### Validation Rules
- **description**: Required, non-empty string
- **prompt**: Required, non-empty string
- **tools**: Optional, array of strings (not validated)
- **model**: Optional, string (not validated)

### Examples

**Minimal**:
```json
{
  "reviewer": {
    "description": "Reviews code",
    "prompt": "Review this code"
  }
}
```

**Full**:
```json
{
  "reviewer": {
    "description": "Reviews code for quality",
    "prompt": "You are a senior reviewer",
    "tools": ["Read", "Grep", "Bash"],
    "model": "sonnet"
  },
  "writer": {
    "description": "Writes documentation",
    "prompt": "Write clear docs",
    "tools": ["Write"],
    "model": "haiku"
  }
}
```

---

## Integration Points

### 1. CLI Parsing (main.rs)
- Read args.agents if provided
- Parse and validate
- Store in agent_discovery
- Pass downstream

### 2. Plugin System
- Query agents via all_agents()
- Use merged list (file + runtime)
- Identify runtime agents via path prefix

### 3. Task Execution
- Runtime agents available for invocation
- Same interface as file agents
- path field indicates origin

---

## Error Handling

### Parse Errors (JSON Syntax)
```
Error: Invalid agents JSON: expected value at line 1 column 5
Expected format: --agents '{"agent": {"description":"...", "prompt":"..."}}'
```

### Validation Errors (Field Constraints)
```
Agent validation failed:
  - Agent 'my-agent' has empty description
  - Agent 'my-agent' has empty prompt
```

### No Errors (Success)
Agents merged and available for use.

---

## Testing Strategy

**60% Unit Tests** (DONE)
- Parse valid/invalid JSON
- Validate field constraints
- Handle edge cases
- Multiple agents

**30% Integration Tests** (TODO)
- CLI arg to discovery
- Merge behavior
- Collision handling

**10% E2E Tests** (TODO)
- Runtime agent execution
- Full workflow

---

## Key Decisions

| Decision | Choice | Why |
|----------|--------|-----|
| Separate parse/validate | Yes | Clean separation, easier testing |
| File agents win on collision | Yes | Preserves explicit project agents |
| Collect all validation errors | Yes | Complete feedback to user |
| Special path marker for runtime | Yes | Easy detection downstream |
| Case sensitive names | Yes | Consistent with file agents |

---

## Minimal Implementation Checklist

To get this working end-to-end:

1. **main.rs** (~20 lines)
   - [ ] Parse --agents flag
   - [ ] Call parse_runtime_agents()
   - [ ] Call validate_runtime_agents()
   - [ ] Call with_runtime_agents()

2. **plugins/mod.rs** (~5 lines)
   - [ ] Export parse_runtime_agents
   - [ ] Export validate_runtime_agents
   - [ ] Export RuntimeAgentDefinition

3. **Tests** (~80 lines)
   - [ ] Integration: CLI → merged agents
   - [ ] E2E: Runtime agent execution

4. **Documentation** (~30 minutes)
   - [ ] Help text with examples
   - [ ] CLI reference update

---

## Estimated Effort

| Task | Lines | Time |
|------|-------|------|
| main.rs integration | 20 | 15 min |
| Module exports | 5 | 5 min |
| Integration test | 30 | 20 min |
| E2E test | 50 | 30 min |
| Documentation | - | 30 min |
| **Total** | **105** | **1.5-2 hours** |

---

## Files Modified

| File | Change | Lines |
|------|--------|-------|
| `crates/cli/src/main.rs` | CLI integration | +20 |
| `crates/cli/src/plugins/mod.rs` | Exports | +5 |
| `crates/cli/src/plugins/agent_discovery.rs` | Tests | +80 |
| Specs (NEW) | Documentation | +3000 |

---

## Design Highlights

### 1. Zero Magic
No implicit conversions, no hidden state, no clever tricks.
Each layer is straightforward and testable.

### 2. Regenerability
Any component can be rebuilt from specification without breaking others.
Contracts are explicit, boundaries are clear.

### 3. Ruthless Simplicity
- No validation in parse
- No merging in validate
- No side effects in any layer
- Each function returns exactly what you'd expect

### 4. User-Friendly Errors
- Clear descriptions of what went wrong
- Actionable guidance on how to fix
- Complete error reporting (all errors at once)

---

## Next Steps

1. Read specification documents (in order listed above)
2. Implement main.rs integration (20 lines)
3. Add module exports (5 lines)
4. Write integration/E2E tests
5. Test with real agent invocation
6. Update CLI help text
7. Commit and push

---

## Related Documentation

- **AGENTS_JSON_FLAG_ARCHITECTURE.md**: Complete specification
- **AGENTS_JSON_INTEGRATION_GUIDE.md**: Implementation guide
- **AGENTS_JSON_FLOW.md**: Visual diagrams and flows
- **Source Code**: `crates/cli/src/plugins/agent_discovery.rs` (complete implementation)
- **Issue**: #110 (GitHub issue for reference)

---

## Philosophy Alignment

This architecture embodies three core principles:

### 1. Ruthless Simplicity
Three independent layers, each with single responsibility.
No complex orchestration or interdependencies.

### 2. Modular Design (Bricks & Studs)
Each component is a self-contained "brick":
- Parse: Takes string, returns HashMap
- Validate: Takes HashMap, returns errors
- Merge: Takes HashMap, returns Self

Clear contracts, regeneratable components.

### 3. Zero-BS Implementation
- No stubs or placeholders
- Every function works
- All tests pass
- Clear error handling
- Complete specification

---

**Status**: Ready for implementation
**Quality**: Architecture complete, design validated
**Risk**: Low - All core logic already implemented and tested
