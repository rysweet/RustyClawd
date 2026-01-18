# disallowedTools Permission System

This document describes the `disallowedTools` feature for restricting tool access in custom agent definitions.

## Overview

The `disallowedTools` field allows agent configurations to explicitly block certain tools from being used by that agent. This provides fine-grained control over agent capabilities and enables security-conscious deployments.

## Usage

### CLI Flag

Block tools globally for a session using the `--disallowedTools` CLI flag:

```bash
# Block a single tool
rustyclawd --disallowedTools Bash "Analyze this code"

# Block multiple tools
rustyclawd --disallowedTools Bash Write Edit "Review this file"

# Block with pattern matching
rustyclawd --disallowedTools "Bash(rm:*)" "Clean up files"
```

### Runtime Agent Definition

When defining agents via the `--agents` JSON flag, include the `disallowedTools` field:

```bash
rustyclawd --agents '{
  "code-reviewer": {
    "description": "Reviews code for quality",
    "prompt": "You are a code reviewer. Only read files, never modify them.",
    "tools": ["Read", "Grep", "Glob"],
    "disallowedTools": ["Write", "Edit", "Bash"]
  }
}'
```

### Plugin Manifest

In `plugin.json`, agents can specify disallowed tools:

```json
{
  "agents": [
    {
      "id": "safe-analyzer",
      "name": "Safe Analyzer",
      "description": "Read-only analysis agent",
      "path": "agents/safe-analyzer.md",
      "disallowedTools": ["Write", "Edit", "Bash"]
    }
  ]
}
```

## Behavior

### Priority Rules

1. **disallowedTools takes precedence over tools (allowed)**
   - If a tool is in both `tools` and `disallowedTools`, it is BLOCKED

2. **Session-level disallowedTools are additive**
   - CLI `--disallowedTools` combines with agent-level restrictions

3. **Inheritance**
   - Sub-agents inherit parent session's disallowed tools

### Error Messages

When a disallowed tool is invoked, the agent receives a clear error:

```
Tool execution blocked: Tool 'Bash' is disallowed for this agent.
```

## Examples

### Read-Only Agent

```json
{
  "description": "Documentation agent that only reads",
  "prompt": "You analyze documentation. Never modify files.",
  "tools": ["Read", "Grep", "Glob"],
  "disallowedTools": ["Write", "Edit", "Bash", "Task"]
}
```

### Limited Bash Agent

```json
{
  "description": "Agent with restricted shell access",
  "prompt": "You can run safe commands only.",
  "tools": ["Bash", "Read"],
  "disallowedTools": ["Bash(rm:*)", "Bash(sudo:*)"]
}
```

### Analysis-Only Agent

```json
{
  "description": "Code analysis without execution",
  "prompt": "Analyze code patterns without running anything.",
  "tools": ["Read", "Grep", "Glob"],
  "disallowedTools": ["Bash", "Write", "Edit", "Task", "Skill", "SlashCommand"]
}
```

## Implementation Details

### Data Structures

```rust
/// Runtime agent definition with tool restrictions
pub struct RuntimeAgentDefinition {
    pub description: String,
    pub prompt: String,
    pub tools: Vec<String>,           // Allowed tools
    pub disallowed_tools: Vec<String>, // Blocked tools
    pub model: Option<String>,
}

/// Plugin agent definition with tool restrictions
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub model: Option<String>,
    pub disallowed_tools: Option<Vec<String>>, // Blocked tools
}
```

### Filtering Logic

Tool filtering happens at execution time in the tool executor:

```rust
// Check if tool is disallowed
if let Some(disallowed) = &agent_context.disallowed_tools {
    if disallowed.contains(&tool_name) {
        return Err(format!("Tool '{}' is disallowed for this agent", tool_name));
    }
}
```

## Testing

### Unit Tests

```rust
#[test]
fn test_disallowed_tools_blocks_execution() {
    let agent = RuntimeAgentDefinition {
        description: "Test agent".to_string(),
        prompt: "Test".to_string(),
        tools: vec!["Read".to_string()],
        disallowed_tools: vec!["Bash".to_string()],
        model: None,
    };

    assert!(is_tool_allowed("Read", &agent));
    assert!(!is_tool_allowed("Bash", &agent));
}

#[test]
fn test_disallowed_takes_precedence() {
    let agent = RuntimeAgentDefinition {
        description: "Test agent".to_string(),
        prompt: "Test".to_string(),
        tools: vec!["Bash".to_string()],  // Allowed
        disallowed_tools: vec!["Bash".to_string()],  // But also blocked
        model: None,
    };

    // disallowedTools wins
    assert!(!is_tool_allowed("Bash", &agent));
}
```

## Security Considerations

1. **Defense in Depth**: Use `disallowedTools` alongside other security measures
2. **Principle of Least Privilege**: Block tools not needed for the agent's purpose
3. **Audit Trail**: All tool blocking decisions are logged
4. **No Bypass**: Agents cannot circumvent disallowedTools restrictions

## Related Documentation

- [Tool Use Examples](./TOOL_USE_EXAMPLES.md)
- [Agent SDK Tests](../../crates/cli/tests/agent_sdk_tests.rs)
- [Permission Mode](../../crates/cli/src/permission_mode.rs)
