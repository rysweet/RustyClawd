# --agents JSON Flag: Quick Reference Card

**Print this.** One page. Taped to monitor. Done.

---

## TL;DR

Three independent layers, parse → validate → merge.

```
--agents '{"name":{"description":"X","prompt":"Y"}}'
  ↓
parse_runtime_agents()          → HashMap
  ↓
validate_runtime_agents()       → Ok/Err
  ↓
with_runtime_agents()           → Store
  ↓
all_agents()                    → MERGED!
```

---

## What's Done

✅ Agent discovery system (complete)
✅ Parse function (complete)
✅ Validate function (complete)
✅ Merge logic (complete)
✅ Unit tests (26 tests)
✅ CLI flag definition

## What's TODO

❌ CLI integration in main.rs (~20 lines)
❌ Error handling (~10 lines)
❌ Module exports (~5 lines)
❌ Integration tests (~30 lines)
❌ E2E tests (~50 lines)

---

## JSON Format

```json
{
  "agent_name": {
    "description": "What it does",
    "prompt": "System prompt",
    "tools": ["Read", "Write"],        // Optional
    "model": "sonnet"                  // Optional
  }
}
```

**Required**: description, prompt (both non-empty)
**Optional**: tools, model

---

## Code Location Reference

| Component | File | Lines |
|-----------|------|-------|
| Types | `plugins/agent_discovery.rs` | 17-29 |
| Parse | `plugins/agent_discovery.rs` | 31-41 |
| Validate | `plugins/agent_discovery.rs` | 43-71 |
| Merge | `plugins/agent_discovery.rs` | 80-137 |
| CLI Flag | `main.rs` | 78 |
| Tests | `plugins/agent_discovery.rs` | 250-630 |

---

## Integration Pseudocode

```rust
// In main.rs
let mut discovery = AgentDiscovery::new(&project_root);

if let Some(json) = args.agents {
    let agents = parse_runtime_agents(&json)?;
    validate_runtime_agents(&agents)?;
    discovery = discovery.with_runtime_agents(agents);
}

let all = discovery.all_agents()?;  // File + runtime merged
```

---

## API at a Glance

```rust
// Parse JSON string
fn parse_runtime_agents(json_str: &str)
  -> Result<HashMap<String, RuntimeAgentDefinition>, String>

// Validate field constraints
fn validate_runtime_agents(agents: &HashMap<...>)
  -> Result<(), Vec<String>>

// Store runtime agents in discovery
fn with_runtime_agents(self, agents: HashMap<...>)
  -> Self

// Get merged list (file + runtime)
fn all_agents(&self)
  -> Result<Vec<AgentDefinition>, String>
```

---

## Error Messages

**Parse Error**:
```
Error: Invalid agents JSON: expected value at line 1 column 5
```

**Validation Error**:
```
Agent validation failed:
  - Agent 'x' has empty description
  - Agent 'y' has empty prompt
```

---

## Precedence Rules

File agent wins on ID collision.

```
File: ["a", "b"]
Runtime: ["b", "c"]
Result: ["a", "b", "c"]  ← "b" from file, not runtime
```

---

## Test Locations

**Unit Tests** (DONE):
- `plugins/agent_discovery.rs:250-630`
- 26 tests, all passing

**Integration Tests** (TODO):
- CLI arg → discovery merge
- ~30 lines

**E2E Tests** (TODO):
- Runtime agent execution
- ~50 lines

---

## Key Decisions (Why?)

| What | Why |
|------|-----|
| Separate parse/validate | Clean separation, easier testing |
| File wins on collision | Preserves explicit project agents |
| Collect all errors | Complete feedback to user |
| path: "runtime:id" | Easy detection downstream |

---

## Examples

### Minimal Agent
```json
{
  "my-agent": {
    "description": "Does something",
    "prompt": "Do something useful"
  }
}
```

### Full Agent
```json
{
  "reviewer": {
    "description": "Security code reviewer",
    "prompt": "You are a security expert. Review this code.",
    "tools": ["Read", "Grep", "Bash"],
    "model": "sonnet"
  }
}
```

### Multiple Agents
```json
{
  "reviewer": { ... },
  "writer": { ... },
  "tester": { ... }
}
```

---

## Validation Checklist

Before runtime agent is used:

- [ ] JSON parses without error
- [ ] description field exists and non-empty
- [ ] prompt field exists and non-empty
- [ ] tools/model optional (but must be valid types if present)
- [ ] Agent ID not empty
- [ ] File agents checked for collisions

---

## Flow Diagram (Inline)

```
Input JSON
    ↓
serde_json::from_str()
    ↓
HashMap or Error
    ↓
validate (empty check)
    ↓
Ok or Errors
    ↓
with_runtime_agents()
    ↓
all_agents()
    ↓
Vec<AgentDefinition>
    ↓
Use in plugin system
```

---

## Common Pitfalls

❌ **Don't**: Validate tool names (plugins define validity)
✅ **Do**: Let downstream code handle tool validation

❌ **Don't**: Validate model IDs (LLM API defines validity)
✅ **Do**: Let LLM API reject invalid models

❌ **Don't**: Allow empty description/prompt
✅ **Do**: Reject them with clear error

❌ **Don't**: Override file agents
✅ **Do**: Preserve file agents, add new runtime agents

---

## Performance Notes

- Parse: O(n) where n = JSON size
- Validate: O(m) where m = number of agents
- Merge: O(f + r) where f = file agents, r = runtime agents
- Total: Linear, no exponential operations

---

## Security Notes

- JSON parsing: Safe (serde_json)
- No code execution: Pure data structures
- No file access in parse/validate: Safe
- Validation before use: Prevents invalid state

---

## Regeneration Test

Can rebuild entire system from this spec? YES

1. Define RuntimeAgentDefinition
2. Implement parse_runtime_agents()
3. Implement validate_runtime_agents()
4. Add with_runtime_agents() to AgentDiscovery
5. Implement merge in all_agents()
6. Add tests

All unambiguous. All regeneratable.

---

## Status Badges

```
[x] Design Complete
[x] Core Implementation Done
[x] Unit Tests Passing
[ ] CLI Integration
[ ] Error Handling
[ ] Integration Tests
[ ] E2E Tests
[ ] Documentation
```

---

## Support Links

- **Full Architecture**: AGENTS_JSON_FLAG_ARCHITECTURE.md
- **Implementation Guide**: AGENTS_JSON_INTEGRATION_GUIDE.md
- **Visual Flows**: AGENTS_JSON_FLOW.md
- **Summary**: ARCHITECTURE_SUMMARY.md

---

**Printed at**: 2025-12-08
**Confidence**: High (all core code done, isolated integration remaining)
**Est. Time to Complete**: 1.5-2 hours
