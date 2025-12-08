# --agents JSON Flag: Integration Guide

**Quick Reference**: How to integrate --agents parsing into CLI

---

## Status Overview

- **Done**: Agent discovery (parse, validate, merge)
- **TODO**: CLI integration (main.rs), error handling, tests

---

## Integration Checklist

### 1. Verify Agent Discovery (DONE)

Location: `/crates/cli/src/plugins/agent_discovery.rs`

What's implemented:
- `RuntimeAgentDefinition` struct
- `parse_runtime_agents()` function
- `validate_runtime_agents()` function
- `AgentDiscovery::with_runtime_agents()` method
- `AgentDiscovery::all_agents()` merges both types
- All unit tests passing

### 2. CLI Parsing (TODO)

**Location**: `crates/cli/src/main.rs`

**Current State**:
```rust
struct Cli {
    #[arg(long, value_name = "JSON")]
    agents: Option<String>,  // ← Already present
}
```

**What to Add** (in main function):

```rust
use crate::plugins::agent_discovery::{parse_runtime_agents, validate_runtime_agents};

fn main() -> Result<()> {
    let args = Cli::parse();
    let project_root = determine_project_root()?;

    // Step 1: Initialize agent discovery
    let mut agent_discovery = AgentDiscovery::new(&project_root);

    // Step 2: Parse and integrate runtime agents
    if let Some(agents_json) = args.agents {
        // Parse JSON
        let runtime_agents = parse_runtime_agents(&agents_json)
            .map_err(|e| anyhow!("Failed to parse --agents JSON: {}", e))?;

        // Validate schema
        if let Err(validation_errors) = validate_runtime_agents(&runtime_agents) {
            eprintln!("Agent validation failed:");
            for error in validation_errors {
                eprintln!("  - {}", error);
            }
            anyhow::bail!("Invalid agent definitions");
        }

        // Merge into discovery
        agent_discovery = agent_discovery.with_runtime_agents(runtime_agents);
    }

    // Step 3: Use merged discovery
    // Pass to plugin system, session initialization, etc.
    let all_agents = agent_discovery.all_agents()?;

    // Continue normal initialization...
}
```

### 3. Error Handling (TODO)

**Parse Errors** (JSON syntax):
```rust
if let Err(e) = parse_runtime_agents(&agents_json) {
    eprintln!("Error: Invalid agents JSON: {}", e);
    eprintln!("Expected format: --agents '{{\"agent_name\": {{\"description\": \"...\", \"prompt\": \"...\"}}}}");
    std::process::exit(1);
}
```

**Validation Errors** (field constraints):
```rust
if let Err(errors) = validate_runtime_agents(&runtime_agents) {
    eprintln!("Agent validation failed:");
    for error in errors {
        eprintln!("  - {}", error);
    }
    std::process::exit(1);
}
```

### 4. Export from Module (TODO)

**File**: `crates/cli/src/plugins/mod.rs`

Add:
```rust
pub use agent_discovery::{
    parse_runtime_agents,
    validate_runtime_agents,
    RuntimeAgentDefinition,
    AgentDiscovery,
};
```

### 5. Testing (TODO)

**Unit Tests** (already in agent_discovery.rs):
- ✓ Parse valid JSON
- ✓ Parse invalid JSON
- ✓ Validate required fields
- ✓ Validate empty fields
- ✓ Multiple agents
- ✓ Merge with file agents

**Integration Tests** (TODO - in main.rs test module):
```rust
#[test]
fn test_cli_agents_flag_parsing() {
    let json = r#"{"test": {"description": "Test", "prompt": "Test prompt"}}"#;
    // Parse via CLI arg → verify in agent_discovery
}
```

**E2E Tests** (TODO - verify actual usage):
```rust
#[test]
fn test_runtime_agent_execution() {
    // Create runtime agent via CLI
    // Execute task using that agent
    // Verify it was actually used
}
```

---

## Minimal JSON Examples

### Single Agent
```json
{
  "my-agent": {
    "description": "Does something useful",
    "prompt": "You are my helpful agent"
  }
}
```

### Multiple Agents
```json
{
  "reviewer": {
    "description": "Code reviewer",
    "prompt": "Review code",
    "tools": ["Read", "Grep"],
    "model": "sonnet"
  },
  "writer": {
    "description": "Doc writer",
    "prompt": "Write docs",
    "tools": ["Write"]
  }
}
```

---

## API Contract

### parse_runtime_agents()
```rust
pub fn parse_runtime_agents(
    json_str: &str,
) -> Result<HashMap<String, RuntimeAgentDefinition>, String>
```

**Input**: JSON string
**Output**: HashMap of agent name → definition
**Errors**: Malformed JSON (single error message)

### validate_runtime_agents()
```rust
pub fn validate_runtime_agents(
    agents: &HashMap<String, RuntimeAgentDefinition>,
) -> Result<(), Vec<String>>
```

**Input**: HashMap of runtime agents
**Output**: Nothing (or all validation errors)
**Errors**: Vec of all validation errors found

### AgentDiscovery::with_runtime_agents()
```rust
pub fn with_runtime_agents(
    mut self,
    agents: HashMap<String, RuntimeAgentDefinition>,
) -> Self
```

**Input**: HashMap of runtime agents
**Output**: Self (builder pattern)
**Effect**: Stores runtime agents, merged in all_agents()

### AgentDiscovery::all_agents()
```rust
pub fn all_agents(&self) -> Result<Vec<AgentDefinition>, String>
```

**Output**: Combined list of file-based + runtime agents
**Merging**: File agents are added first, then runtime agents
**Precedence**: If ID collision, file agent wins (already exists in vec)

---

## Precedence Rules

When a runtime agent has same ID as file-based agent:

```
agent_discovery.all_agents() returns:
  1. All file-based agents (discovered from .claude/agents/)
  2. All runtime agents not in file-based list

If ID collision:
  File-based agent is kept, runtime agent is skipped
```

**Example**:
```
File agents: ["architect", "builder"]
Runtime agents: {"builder": {...}, "tester": {...}}
Result: ["architect", "builder", "tester"]
  - "architect": from file
  - "builder": from file (not overridden)
  - "tester": from runtime
```

---

## Error Flow

```
User provides: --agents '{"name": {...}}'
       ↓
parse_runtime_agents()
  ├─ Success → HashMap
  └─ Malformed JSON → Error("Invalid agents JSON: ...")
       ↓
validate_runtime_agents()
  ├─ Success → Ok(())
  └─ Invalid fields → Err(Vec["field errors"])
       ↓
Show errors and exit(1)
  OR
with_runtime_agents() + all_agents()
  ↓
Continue with merged agents
```

---

## Code Locations Summary

| Component | File | Status |
|-----------|------|--------|
| Types | `plugins/agent_discovery.rs:1-30` | Done |
| Parse | `plugins/agent_discovery.rs:31-41` | Done |
| Validate | `plugins/agent_discovery.rs:43-71` | Done |
| Merge | `plugins/agent_discovery.rs:116-132` | Done |
| CLI Flag | `main.rs:78` | Done (definition only) |
| Integration | `main.rs:main()` | TODO |
| Exports | `plugins/mod.rs` | TODO |
| Tests | `plugins/agent_discovery.rs:250+` | Done (unit only) |

---

## Testing Pyramid

**60% Unit Tests** (DONE):
- Parse valid/invalid JSON
- Validate fields
- Handle empty values
- Multiple agents

**30% Integration Tests** (TODO):
- CLI arg → discovery
- File + runtime merge
- Precedence rules

**10% E2E Tests** (TODO):
- CLI invocation with --agents
- Runtime agent actually used in task

---

## Quick Links

- **Architecture Spec**: `Specs/AGENTS_JSON_FLAG_ARCHITECTURE.md`
- **Agent Discovery Code**: `crates/cli/src/plugins/agent_discovery.rs`
- **Tests**: Lines 250-630 in agent_discovery.rs
- **CLI Struct**: `crates/cli/src/main.rs:78`

---

## Minimal Required Changes

To get this working, only need:

1. **main.rs** (~20 lines): Parse flag, call parse/validate, merge agents
2. **plugins/mod.rs** (~5 lines): Export functions
3. **Integration test** (~30 lines): Verify merge works
4. **E2E test** (~50 lines): Verify agent actually used

Everything else is already done in agent_discovery.rs!

---

## Next Steps (Priority Order)

1. Integrate parse/validate into main.rs
2. Add module exports
3. Run integration tests
4. Test E2E with actual agent execution
5. Update help text with examples
6. Create Spec documentation (DONE)

**Estimated effort**: 2-3 hours for complete integration + testing
