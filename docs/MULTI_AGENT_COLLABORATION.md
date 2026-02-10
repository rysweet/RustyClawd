# Multi-Agent Collaboration System

This document describes the Multi-Agent Collaboration (Agent Teams) system in RustyClawd, which enables multiple agents to work together on complex tasks through shared memory, coordination hooks, and team orchestration.

## Overview

The multi-agent collaboration system provides:

- **Shared Memory**: Agents can communicate via scoped memory (user/project/local)
- **Coordination Hooks**: New hook events (`TeammateIdle`, `TaskCompleted`) enable agent coordination
- **Agent Registry**: Track agent status and manage background execution
- **Team Patterns**: Common collaboration patterns for agent teams

## Architecture

### Philosophy Alignment

This design follows RustyClawd's core philosophy:

- **Ruthless Simplicity**: Just key-value storage with scopes, no complex query engines
- **Modular Design (Bricks & Studs)**: AgentMemory is self-contained with clear public API
- **Zero-BS Implementation**: Every function works - no stubs or placeholders
- **Reuses Existing Infrastructure**: Builds on top of AgentRegistry

### Components

```
┌─────────────────────┐
│   Team Coordinator  │ (Spawns and coordinates agents)
└──────────┬──────────┘
           │
           ├─────────────────────┐
           │                     │
    ┌──────▼──────┐      ┌──────▼──────┐
    │   Agent 1   │      │   Agent 2   │
    └──────┬──────┘      └──────┬──────┘
           │                     │
           └─────────┬───────────┘
                     │
           ┌─────────▼──────────┐
           │   Agent Memory     │
           │  ┌───────────────┐ │
           │  │ User Scope    │ │ (Shared across all agents)
           │  ├───────────────┤ │
           │  │ Project Scope │ │ (Shared within project)
           │  ├───────────────┤ │
           │  │ Local Scope   │ │ (Private to agent)
           │  └───────────────┘ │
           └────────────────────┘
```

## Agent Memory

### Memory Scopes

Three scopes provide different sharing boundaries:

1. **User Scope** - Shared across all agents for a user
   - Use for: User preferences, global settings
   - Example: Theme preference, default model

2. **Project Scope** - Shared across agents within a project
   - Use for: Build status, test results, project state
   - Example: Compilation artifacts, dependencies

3. **Local Scope** - Private to a single agent
   - Use for: Agent-specific state, progress tracking
   - Example: Current step, temporary data

### API

```rust
use rustyclawd_tools::{AgentMemory, MemoryScope, global_agent_memory};

// Get global memory instance
let memory = global_agent_memory();

// Store a value in project scope
memory.set(
    MemoryScope::Project,
    "build_status".to_string(),
    serde_json::json!({"status": "compiled"}),
    "agent_builder".to_string(),
    Some("project123".to_string()),
).await?;

// Read a value from project scope
let entry = memory.get(
    MemoryScope::Project,
    "build_status",
    "agent_tester",
    Some("project123"),
).await?;

// List all keys in a scope
let keys = memory.list_keys(
    MemoryScope::Project,
    "agent_tester",
    Some("project123"),
).await?;

// Delete a value
memory.delete(
    MemoryScope::Project,
    "build_status",
    "agent_tester",
    Some("project123"),
).await?;

// Clear all memory in a scope
memory.clear(
    MemoryScope::Project,
    "agent_tester",
    Some("project123"),
).await?;
```

### Memory Entry Metadata

Each memory entry includes metadata:

```rust
pub struct MemoryEntry {
    pub value: serde_json::Value,  // The actual data
    pub created_at: u64,            // Unix timestamp
    pub updated_at: u64,            // Unix timestamp
    pub created_by: String,         // Agent ID that created this
}
```

## Hook Events

Two new hook events enable multi-agent coordination:

### TeammateIdle

Fires when an agent becomes idle and available for new tasks.

**Use cases:**
- Load balancing work across available agents
- Detecting when agents can pick up new tasks
- Coordinating parallel work distribution

**Hook configuration:**

```json
{
  "TeammateIdle": [
    {
      "matcher": "*",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/dispatch-next-task.sh",
          "timeout": 5000
        }
      ]
    }
  ]
}
```

### TaskCompleted

Fires when an agent completes its assigned task.

**Use cases:**
- Triggering next agent in a pipeline
- Aggregating results from parallel agents
- Recording task completion metrics

**Hook configuration:**

```json
{
  "TaskCompleted": [
    {
      "matcher": "Agent",
      "hooks": [
        {
          "type": "command",
          "command": "scripts/task-complete-handler.sh",
          "timeout": 5000
        }
      ]
    }
  ]
}
```

## Common Collaboration Patterns

### Pattern 1: Sequential Pipeline

Agents work in sequence, passing results via shared memory.

```
Architect → Builder → Tester → Reviewer
    ↓           ↓        ↓         ↓
    [Memory]────────────────────────
```

**Example:**

```rust
// Architect stores design
memory.set(
    MemoryScope::Project,
    "design",
    json!({"modules": ["auth", "api"]}),
    "agent_architect",
    Some("project123"),
).await?;

// Builder reads design and implements
let design = memory.get(
    MemoryScope::Project,
    "design",
    "agent_builder",
    Some("project123"),
).await?.unwrap();

// Builder stores implementation
memory.set(
    MemoryScope::Project,
    "implementation",
    json!({"status": "complete"}),
    "agent_builder",
    Some("project123"),
).await?;
```

### Pattern 2: Parallel Processing with Aggregation

Multiple agents process data in parallel, results are aggregated.

```
         ┌─ Agent 1 ─┐
Task ──┤  ├─ Agent 2 ─┼── Aggregator
         └─ Agent 3 ─┘
```

**Example:**

```rust
// Spawn 3 agents in parallel
for i in 1..=3 {
    let agent_id = format!("agent_worker_{}", i);
    registry.register(
        agent_id.clone(),
        "worker".to_string(),
        "sonnet".to_string(),
    ).await?;

    // Each agent processes a chunk
    memory.set(
        MemoryScope::Project,
        format!("chunk_{}", i),
        json!({"processed": true}),
        agent_id,
        Some("project123".to_string()),
    ).await?;
}

// Aggregator collects results
let keys = memory.list_keys(
    MemoryScope::Project,
    "agent_aggregator",
    Some("project123"),
).await?;

for key in keys.iter().filter(|k| k.starts_with("chunk_")) {
    let result = memory.get(
        MemoryScope::Project,
        key,
        "agent_aggregator",
        Some("project123"),
    ).await?;
    // Process result
}
```

### Pattern 3: Coordinated Debate

Multiple agents with different perspectives debate a decision.

```
     ┌─ Security Agent  ─┐
     ├─ Performance Agent┤─── Moderator → Decision
     └─ Simplicity Agent ─┘
```

**Example:**

```rust
let perspectives = ["security", "performance", "simplicity"];

for perspective in &perspectives {
    let agent_id = format!("agent_{}", perspective);

    // Each agent stores their analysis
    memory.set(
        MemoryScope::Project,
        format!("{}_analysis", perspective),
        json!({"recommendation": "...", "reasoning": "..."}),
        agent_id,
        Some("debate_session_1".to_string()),
    ).await?;
}

// Moderator synthesizes decision
let analyses = memory.list_keys(
    MemoryScope::Project,
    "agent_moderator",
    Some("debate_session_1"),
).await?;

// Read all perspectives and make decision
for key in analyses {
    let analysis = memory.get(
        MemoryScope::Project,
        &key,
        "agent_moderator",
        Some("debate_session_1"),
    ).await?;
    // Consider this perspective
}
```

## Agent Registry Integration

The agent registry tracks agent status and enables coordination:

```rust
use rustyclawd_tools::{global_agent_registry, AgentStatus};

let registry = global_agent_registry();

// Register an agent
registry.register(
    "agent_builder".to_string(),
    "builder".to_string(),
    "sonnet".to_string(),
).await?;

// Check agent status
let status = registry.get_status("agent_builder").await?;
match status {
    AgentStatus::Running => println!("Agent is working"),
    AgentStatus::Completed => println!("Agent finished"),
    AgentStatus::Failed(msg) => println!("Agent failed: {}", msg),
}

// Mark agent as completed
registry.mark_completed("agent_builder").await?;

// List all active agents
let active = registry.list_ids().await;
```

## Best Practices

### 1. Choose the Right Memory Scope

- **User Scope**: Global preferences, cross-project settings
- **Project Scope**: Task results, coordination data, handoffs
- **Local Scope**: Agent state, temporary data, progress

### 2. Clean Up Memory

```rust
// Clear local memory when agent completes
memory.clear(
    MemoryScope::Local,
    agent_id,
    None,
).await?;

// Clear project memory when project completes
memory.clear(
    MemoryScope::Project,
    agent_id,
    Some(project_id),
).await?;
```

### 3. Use Metadata for Tracking

```rust
// Track who created data
let entry = memory.get(...).await?.unwrap();
println!("Created by: {}", entry.created_by);
println!("Created at: {}", entry.created_at);
```

### 4. Handle Missing Data Gracefully

```rust
// Always check if data exists
match memory.get(...).await? {
    Some(entry) => {
        // Use the data
    }
    None => {
        // Handle missing data case
    }
}
```

### 5. Use Hook Events for Coordination

- Set up `TaskCompleted` hooks to trigger next agent
- Use `TeammateIdle` hooks for dynamic work distribution
- Keep hooks simple - complex logic belongs in agents

## Testing

The system includes comprehensive tests:

```bash
# Test agent memory
cargo test --lib agent_memory

# Test hook events
cargo test --lib hooks::types

# Test integration scenarios
cargo test --test test_agent_collaboration
```

## Examples

See `crates/tools/tests/test_agent_collaboration.rs` for complete working examples of:

- Basic agent collaboration
- Memory scope usage
- Team coordination via registry
- Agent handoff patterns
- Memory cleanup

## Future Extensions

Potential enhancements (not yet implemented):

- **Agent Communication Tool**: Explicit tool for sending messages between agents
- **Team Coordinator Tool**: Tool for spawning and managing agent teams
- **Memory Query DSL**: More sophisticated memory queries
- **Persistent Memory**: Option to save memory to disk

## Philosophy Compliance

This implementation aligns with RustyClawd's philosophy:

✅ **Ruthless Simplicity**: Key-value storage with scopes, no query engine
✅ **Modular Design**: Self-contained modules with clear public API
✅ **Zero-BS**: Every function works, no stubs or placeholders
✅ **Reuses Infrastructure**: Built on existing AgentRegistry
✅ **Test-Driven**: Comprehensive unit and integration tests

## References

- Agent Memory: `crates/tools/src/agent_memory.rs`
- Hook Types: `crates/cli/src/hooks/types.rs`
- Agent Registry: `crates/tools/src/agent_registry.rs`
- Integration Tests: `crates/tools/tests/test_agent_collaboration.rs`
