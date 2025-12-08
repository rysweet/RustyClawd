# Architecture: --agents JSON Flag Integration (Issue #110)

**Status**: Specification
**Date**: 2025-12-08
**Scope**: CLI parsing, JSON validation, agent merging

---

## Problem

Users need to define custom agents at runtime via CLI without modifying files. The `--agents` JSON flag provides this capability with three integration layers:

1. **CLI Parsing**: Accept JSON string and validate format
2. **Schema Validation**: Enforce required fields and types
3. **Agent Merging**: Integrate runtime agents with file-discovered agents

---

## Solution Overview

### Ruthless Simplicity Principle

Three independent components, zero interdependence:

- **Parse**: String → HashMap (via serde_json)
- **Validate**: HashMap → Error Report (field-level checks)
- **Merge**: Combine file agents + runtime agents in AgentDiscovery

No validation logic in parsing. No merging logic in validation. Each component has ONE responsibility.

---

## Component 1: Agent Discovery (Core Brick)

**File**: `crates/cli/src/plugins/agent_discovery.rs`

**Public API**:

```rust
// Types
pub struct RuntimeAgentDefinition {
    pub description: String,
    pub prompt: String,
    #[serde(default)]
    pub tools: Vec<String>,
    pub model: Option<String>,
}

pub struct AgentDiscovery {
    agents_dir: PathBuf,
    runtime_agents: HashMap<String, RuntimeAgentDefinition>,
}

// Functions
pub fn parse_runtime_agents(json_str: &str) -> Result<HashMap<String, RuntimeAgentDefinition>, String>
pub fn validate_runtime_agents(agents: &HashMap<String, RuntimeAgentDefinition>) -> Result<(), Vec<String>>

// Methods
impl AgentDiscovery {
    pub fn with_runtime_agents(self, agents: HashMap<String, RuntimeAgentDefinition>) -> Self
    pub fn get_runtime_agent(&self, agent_id: &str) -> Option<&RuntimeAgentDefinition>
    pub fn is_runtime_agent(&self, agent_id: &str) -> bool
    pub fn all_agents(&self) -> Result<Vec<AgentDefinition>, String>  // MERGED: file + runtime
}
```

**Status**: COMPLETE - Already implemented

---

## Component 2: CLI Integration

**File**: `crates/cli/src/main.rs`

**Current State**:

```rust
struct Cli {
    #[arg(long, value_name = "JSON")]
    agents: Option<String>,
    // ... other fields
}
```

**Integration Points**:

### Point A: Parse Runtime Agents (Early)

```rust
// After parsing CLI args
let mut agent_discovery = AgentDiscovery::new(&project_root);

// If --agents flag provided, parse and merge
if let Some(agents_json) = &args.agents {
    match parse_runtime_agents(agents_json) {
        Ok(runtime_agents) => {
            // Validate before merging
            if let Err(validation_errors) = validate_runtime_agents(&runtime_agents) {
                eprintln!("Agent validation failed:");
                for error in validation_errors {
                    eprintln!("  - {}", error);
                }
                std::process::exit(1);
            }
            agent_discovery = agent_discovery.with_runtime_agents(runtime_agents);
        }
        Err(parse_error) => {
            eprintln!("Failed to parse --agents JSON: {}", parse_error);
            std::process::exit(1);
        }
    }
}

// agent_discovery now contains both file-discovered and runtime agents
// Can pass to plugin system, task handlers, etc.
```

**Key Decision**: Parse early (before session starts) so all downstream systems have access to merged agent list.

### Point B: Error Handling

**Parse Errors** (malformed JSON):
```
Error: Invalid agents JSON: expected value at line 1 column 5
Usage: --agents '{"name":{"description":"...","prompt":"..."}}'
```

**Validation Errors** (missing required fields):
```
Agent validation failed:
  - Agent 'my-agent' has empty description
  - Agent 'my-agent' has empty prompt
```

---

## Component 3: Schema Definition

**Format**: `--agents '{"agent_name": {...}}'`

**Schema** (implicit in code):

```rust
RuntimeAgentDefinition {
    description: String,      // Required, non-empty
    prompt: String,           // Required, non-empty
    tools: Vec<String>,       // Optional, defaults to empty
    model: Option<String>,    // Optional, e.g., "sonnet", "opus", "claude-3-sonnet-20241022"
}
```

**JSON Examples**:

### Minimal (valid):
```json
{
  "code-reviewer": {
    "description": "Reviews code",
    "prompt": "Review this code"
  }
}
```

### Full (valid):
```json
{
  "code-reviewer": {
    "description": "Reviews code for quality and security",
    "prompt": "You are a senior code reviewer. Analyze the provided code for quality, performance, and security issues.",
    "tools": ["Read", "Grep", "Bash"],
    "model": "sonnet"
  },
  "doc-writer": {
    "description": "Writes documentation",
    "prompt": "You are a technical writer. Create clear, comprehensive documentation.",
    "tools": ["Write", "Read"],
    "model": "haiku"
  }
}
```

### Invalid Examples:
```json
// Missing description
{"agent": {"prompt": "..."}}

// Empty prompt
{"agent": {"description": "...", "prompt": ""}}

// Invalid JSON
{"agent": {"description": "..."  <- missing quote
```

---

## Data Flow Diagram

```
CLI Input
  |
  v
parse_runtime_agents() ─────> HashMap<String, RuntimeAgentDefinition>
  |                            |
  |                            v
  |                      validate_runtime_agents()
  |                            |
  |                            v ✓ or ✗
  |
  v
AgentDiscovery::with_runtime_agents()
  |
  v
merged_agents = all_agents() ─────> Vec<AgentDefinition>
                                      (file + runtime combined)
  |
  v
Pass to plugin system, task handlers, etc.
```

---

## Integration Points with Existing Systems

### 1. Plugin System
The merged agent list feeds into the plugin manifest and is available to:
- Task execution
- Subagent invocation
- Agent selection logic

### 2. Manifest Building
Runtime agents are converted to `AgentDefinition` format:
```rust
AgentDefinition {
    id: agent_id.clone(),
    name: agent_id.clone(),           // Use ID as name for runtime agents
    description: runtime_agent.description,
    path: format!("runtime:{}", id), // Special marker for runtime agents
    model: runtime_agent.model,
}
```

The `path: "runtime:agent_id"` marker allows downstream code to identify runtime agents.

### 3. Precedence
**File-based agents take precedence** (can be overridden by user):
- If runtime agent has same ID as file-based agent, file-based wins
- This preserves explicit project agents while allowing one-off overrides

**Alternative (not chosen)**: Runtime agents override file-based agents
- Chosen to be conservative: explicit project agents always win
- User can delete file agents if they want complete runtime replacement

---

## Validation Logic

### Field Validation
```
├─ Agent name (key in map)
│  └─ Non-empty (handled by JSON structure)
│
├─ description (String)
│  ├─ Required (serde enforces)
│  └─ Non-empty (validate_runtime_agents checks)
│
├─ prompt (String)
│  ├─ Required (serde enforces)
│  └─ Non-empty (validate_runtime_agents checks)
│
├─ tools (Vec<String>)
│  ├─ Optional (defaults to empty)
│  └─ No validation of individual tool names (plugins define validity)
│
└─ model (Option<String>)
   ├─ Optional
   └─ No validation (whatever LLM API supports is valid)
```

**Validation Strategy**: Fail fast, report all errors at once
- Parse: Single error if JSON malformed
- Validate: Collect all validation errors, report all together
- Never silent failures or partial acceptance

---

## Error Messages

### User-Friendly Format

**Parse Error**:
```
Error: Invalid agents JSON: expected `,` at line 2 column 45
Expected format: --agents '{"agent_name": {"description": "...", "prompt": "..."}}'
```

**Validation Errors**:
```
Agent validation failed:
  - Agent 'my-agent' has empty description
  - Agent 'my-agent' has empty prompt

Both description and prompt are required fields.
```

### Guidelines
- Explain WHAT is wrong
- Explain WHY it's wrong
- Suggest HOW to fix it
- Include valid example in first error

---

## Testing Strategy

### Unit Tests (60%)
- Parse valid/invalid JSON
- Validate required fields
- Validate empty string handling
- Multiple agents in single JSON

### Integration Tests (30%)
- CLI arg parsing with agents flag
- Merge file + runtime agents
- Precedence when IDs overlap
- Full discovery with runtime agents

### E2E Tests (10%)
- End-to-end: CLI arg → merged agents available to plugins
- Real agent invocation with runtime-defined agent

---

## Minimal Implementation Checklist

- [x] `RuntimeAgentDefinition` struct with serde
- [x] `parse_runtime_agents()` function
- [x] `validate_runtime_agents()` function
- [x] `AgentDiscovery::with_runtime_agents()` method
- [x] `AgentDiscovery::all_agents()` merges file + runtime
- [ ] CLI flag `--agents` parsing (in main.rs)
- [ ] Error handling for parse/validate failures
- [ ] Integration test: CLI → merged agents
- [ ] E2E test: runtime agent actually used

---

## Key Architectural Decisions

### 1. String Keys, Not Agent IDs
Runtime agents are keyed by agent name (String), not a separate AgentId type.

**Why**: Users provide agent names in JSON. Matching type prevents conversion errors.

### 2. Separate Parse & Validate Functions
Not combined into single function.

**Why**:
- Parse only handles JSON syntax
- Validate only checks field values
- Caller can choose to proceed with invalid agents if needed (though we don't)

### 3. File Agents Win on ID Collision
If runtime agent has same ID as file agent, file agent is kept.

**Why**: Preserves explicit project agents. Users can delete file agents if they want runtime-only.

### 4. RuntimeAgentDefinition != AgentDefinition
Two different structures.

**Why**:
- RuntimeAgentDefinition: minimal, what users provide
- AgentDefinition: complete, what plugin system expects
- Conversion is explicit in `all_agents()`

### 5. Special Path Marker for Runtime Agents
Path set to `"runtime:agent_id"` for runtime agents.

**Why**: Downstream code can detect origin without additional metadata. Example:
```rust
if agent.path.starts_with("runtime:") {
    // This is a runtime agent
}
```

---

## Trade-Offs

| Decision | Upside | Downside | Chosen |
|----------|--------|----------|--------|
| File agents override runtime | Preserves project agents | Users can't override project agents | Yes |
| Runtime agents override file | Maximum flexibility | Risky for projects | No |
| Validate at parse | Fail fast | Couples concerns | No |
| Validate separately | Clean separation | Extra function | Yes |
| Fail on first error | Fast feedback | Misses other errors | No |
| Collect all errors | Complete feedback | Slightly slower | Yes |

---

## Zero-BS Guarantees

- No silent failures or partial acceptance
- All invalid JSON rejected with clear error
- All validation errors reported together
- No stub/unimplemented code
- All functions return working results or clear errors

---

## Regenerability

This spec is complete enough to rebuild from scratch:

1. Define `RuntimeAgentDefinition` struct with required/optional fields
2. Implement `parse_runtime_agents()` using serde_json
3. Implement `validate_runtime_agents()` checking field constraints
4. Add CLI flag `--agents: Option<String>`
5. Add early parse/validate in main.rs
6. Merge runtime agents into AgentDiscovery
7. Test all three layers

No ambiguity. No guessing required.

---

## Files Involved

| File | Change | Status |
|------|--------|--------|
| `crates/cli/src/plugins/agent_discovery.rs` | Add RuntimeAgentDefinition, parse/validate, merge logic | Complete |
| `crates/cli/src/main.rs` | Add --agents flag, integrate parsing | TODO |
| `crates/cli/src/plugins/mod.rs` | Export parse/validate functions | TODO |
| Tests | Unit/integration/E2E tests | TODO |

---

## Next Steps

1. **CLI Integration**: Add --agents parsing to main.rs
2. **Error Handling**: Implement error reporting
3. **Testing**: Add unit/integration tests
4. **E2E Testing**: Verify runtime agent actually used in task execution
5. **Documentation**: Update CLI help text, add examples
