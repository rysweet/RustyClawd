# allowedTools Permission System

This document describes the `allowedTools` feature for explicitly allowing specific tools in custom agent definitions.

## Overview

The `allowedTools` field allows agent configurations to explicitly allow only certain tools to be used by that agent. This provides fine-grained control over agent capabilities and enables security-conscious deployments by restricting access to only the necessary tools.

## Usage

### CLI Flag

Allow specific tools globally for a session using the `--allowedTools` CLI flag:

```bash
# Allow only specific tools
rustyclawd --allowedTools Read Grep "Analyze this code"

# Allow multiple tools
rustyclawd --allowedTools Read Write Grep Glob "Review this file"
```

### Runtime Agent Definition

When defining agents via the `--agents` JSON flag, include the `allowedTools` field:

```bash
rustyclawd --agents '{
  "code-reader": {
    "description": "Read-only code analyzer",
    "prompt": "You analyze code. You can only read files and search.",
    "allowedTools": ["Read", "Grep", "Glob"]
  }
}'
```

### Plugin Manifest

In `plugin.json`, agents can specify allowed tools:

```json
{
  "agents": [
    {
      "id": "safe-analyzer",
      "name": "Safe Analyzer",
      "description": "Read-only analysis agent",
      "path": "agents/safe-analyzer.md",
      "allowedTools": ["Read", "Grep", "Glob"]
    }
  ]
}
```

## Behavior

### Priority Rules

1. **disallowedTools takes precedence over allowedTools**
   - If a tool is in `allowedTools` but also in `disallowedTools`, it is BLOCKED
   - disallowedTools always wins for explicit blocking

2. **Empty allowedTools means all tools allowed (unless disallowed)**
   - If `allowedTools` is empty or not specified, ALL tools are available
   - This maintains backward compatibility

3. **Non-empty allowedTools creates explicit allowlist**
   - Only tools in the `allowedTools` list are available
   - All other tools are implicitly blocked

4. **Session-level restrictions are additive**
   - CLI `--allowedTools` restricts available tools for the session
   - Agent-level `allowedTools` further restricts within that set

5. **Inheritance**
   - Sub-agents inherit parent session's allowed tools

### Error Messages

When a tool is invoked that is not in the allowed list, the agent receives a clear error:

```
Tool execution blocked: Tool 'Bash' is not in the allowed tools list for this agent.
```

## Examples

### Read-Only Agent

```json
{
  "description": "Documentation agent that only reads",
  "prompt": "You analyze documentation. You can only read and search files.",
  "allowedTools": ["Read", "Grep", "Glob"]
}
```

### Limited Write Agent

```json
{
  "description": "Agent that can read and write but not execute",
  "prompt": "You can modify files but not run commands.",
  "allowedTools": ["Read", "Write", "Edit", "Grep", "Glob"]
}
```

### Analysis-Only Agent with Delegation

```json
{
  "description": "Code analyzer that can delegate subtasks",
  "prompt": "Analyze code patterns and delegate subtasks to other agents.",
  "allowedTools": ["Read", "Grep", "Glob", "Task"]
}
```

## Implementation Details

### Data Structures

```rust
/// Runtime agent definition with tool restrictions
pub struct RuntimeAgentDefinition {
    pub description: String,
    pub prompt: String,
    pub tools: Vec<String>,            // Allowed tools (legacy field)
    pub allowed_tools: Vec<String>,    // Explicitly allowed tools
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
    pub allowed_tools: Option<Vec<String>>,     // Explicitly allowed tools
    pub disallowed_tools: Option<Vec<String>>,  // Blocked tools
}
```

### Filtering Logic

Tool filtering happens at execution time in the tool executor:

```rust
// Check if tool is in allowed list (if allowlist is non-empty)
if let Some(allowed) = &agent_context.allowed_tools {
    if !allowed.is_empty() && !allowed.contains(&tool_name) {
        return Err(format!(
            "Tool '{}' is not in the allowed tools list for this agent",
            tool_name
        ));
    }
}

// Check if tool is explicitly disallowed (takes precedence)
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
fn test_allowed_tools_permits_execution() {
    let agent = RuntimeAgentDefinition {
        description: "Test agent".to_string(),
        prompt: "Test".to_string(),
        tools: vec![],
        allowed_tools: vec!["Read".to_string()],
        disallowed_tools: vec![],
        model: None,
    };

    assert!(is_tool_allowed("Read", &agent));
    assert!(!is_tool_allowed("Bash", &agent));
}

#[test]
fn test_empty_allowed_tools_permits_all() {
    let agent = RuntimeAgentDefinition {
        description: "Test agent".to_string(),
        prompt: "Test".to_string(),
        tools: vec![],
        allowed_tools: vec![],  // Empty means all allowed
        disallowed_tools: vec![],
        model: None,
    };

    assert!(is_tool_allowed("Read", &agent));
    assert!(is_tool_allowed("Bash", &agent));
}

#[test]
fn test_disallowed_takes_precedence_over_allowed() {
    let agent = RuntimeAgentDefinition {
        description: "Test agent".to_string(),
        prompt: "Test".to_string(),
        tools: vec![],
        allowed_tools: vec!["Bash".to_string()],  // Allowed
        disallowed_tools: vec!["Bash".to_string()],  // But also blocked
        model: None,
    };

    // disallowedTools wins
    assert!(!is_tool_allowed("Bash", &agent));
}
```

## Security Considerations

1. **Defense in Depth**: Use `allowedTools` alongside other security measures
2. **Principle of Least Privilege**: Only allow tools needed for the agent's purpose
3. **Audit Trail**: All tool blocking decisions are logged
4. **No Bypass**: Agents cannot circumvent allowedTools restrictions
5. **Explicit is Better**: Using `allowedTools` makes permissions explicit and reviewable

## Comparison with disallowedTools

| Feature | allowedTools | disallowedTools |
|---------|-------------|-----------------|
| **Approach** | Allowlist (explicit allow) | Blocklist (explicit deny) |
| **Empty List** | All tools allowed | No tools blocked |
| **Priority** | Lower (blocked by disallowedTools) | Higher (always blocks) |
| **Use Case** | Restrict to minimal set | Block specific dangerous tools |
| **Security** | More secure (explicit allow) | Less secure (implicit allow) |

## Best Practices

1. **Use allowedTools for restricted agents**: When you want tight control, use `allowedTools`
2. **Use disallowedTools for exceptions**: When most tools are fine, block specific ones
3. **Combine both for layered security**: Use `allowedTools` as base, `disallowedTools` for overrides
4. **Document why**: Clearly document why specific tools are allowed/disallowed
5. **Review regularly**: Periodically review tool permissions as agent capabilities evolve

## Related Documentation

- [Disallowed Tools](./DISALLOWED_TOOLS.md)
- [Tool Use Examples](./TOOL_USE_EXAMPLES.md)
- [Agent SDK Tests](../../crates/cli/tests/agent_sdk_tests.rs)
- [Permission Mode](../../crates/cli/src/permission_mode.rs)
