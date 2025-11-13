# Plugin System Quick Start

Fast guide to using the complete Claude Code plugin system.

## Create a Plugin

### 1. Create Directory Structure

```bash
mkdir -p my-plugin/{commands,skills,agents,hooks}
```

### 2. Create plugin.json

```json
{
  "id": "com.myname.myplugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "My awesome plugin",
  "author": "Your Name",
  "license": "MIT",
  "main": "index.js",
  "commands": [],
  "skills": [],
  "agents": [],
  "hooks": [],
  "mcpServers": []
}
```

### 3. Create Entry Point (index.js)

```javascript
#!/usr/bin/env node
console.log('My plugin loaded!');
module.exports = { name: 'my-plugin', version: '1.0.0' };
```

## Add Features

### Commands

**plugin.json:**
```json
{
  "commands": [{
    "name": "greet",
    "description": "Greet someone",
    "path": "commands/greet.js",
    "argsSchema": {
      "type": "object",
      "properties": {
        "name": { "type": "string" }
      },
      "required": ["name"]
    }
  }]
}
```

**commands/greet.js:**
```javascript
#!/usr/bin/env node
const args = JSON.parse(process.argv[2] || '{}');
console.log(`Hello, ${args.name}!`);
process.exit(0);
```

### Skills

**plugin.json:**
```json
{
  "skills": [{
    "id": "my-skill",
    "name": "My Skill",
    "description": "Does something",
    "path": "skills/my-skill.md"
  }]
}
```

**skills/my-skill.md:**
```markdown
# My Skill

You are an expert at doing X.

## Instructions
1. Do this
2. Then that

## Output Format
- Format like this
```

### Agents

**plugin.json:**
```json
{
  "agents": [{
    "id": "my-agent",
    "name": "My Agent",
    "description": "Specialized agent",
    "path": "agents/my-agent.md",
    "model": "sonnet"
  }]
}
```

**agents/my-agent.md:**
```markdown
# My Agent

You are a specialized agent for X.

[Agent instructions here]
```

### Hooks

**plugin.json:**
```json
{
  "hooks": [{
    "event": "PreToolUse",
    "handler": "hooks/pre-tool.js"
  }]
}
```

**hooks/pre-tool.js:**
```javascript
#!/usr/bin/env node
const context = JSON.parse(process.env.HOOK_CONTEXT || '{}');

// Allow by default
console.log(JSON.stringify({
  continue: true,
  permissionDecision: 'allow'
}));
process.exit(0);
```

### MCP Servers

**plugin.json:**
```json
{
  "mcpServers": [{
    "id": "my-mcp",
    "name": "My MCP Server",
    "command": "node",
    "args": ["mcp-server.js"],
    "env": {},
    "description": "My MCP server"
  }]
}
```

## Use the Plugin System

### Basic Usage

```rust
use claude_code_cli::plugins::*;
use claude_code_cli::hooks::registry::HookRegistry;

#[tokio::main]
async fn main() {
    // Setup
    let mut manager = PluginManager::new("./plugins")
        .with_project_root(".");

    // Load all plugins
    let loaded = manager.discover_and_load_all().await.unwrap();
    println!("Loaded: {:?}", loaded);

    // Execute command
    let result = manager.execute_command(
        "com.myname.myplugin",
        "greet",
        serde_json::json!({"name": "World"})
    ).await.unwrap();
    println!("{}", result.output);

    // Load skill
    let skill = manager.execute_skill(
        "com.myname.myplugin",
        "my-skill"
    ).await.unwrap();
    println!("Skill: {}", skill.output);

    // Register hooks
    let mut registry = HookRegistry::new();
    manager.register_all_hooks(&mut registry).unwrap();

    // Start MCP servers
    manager.start_all_mcp_servers().await.unwrap();

    // Get summary
    println!("{}", manager.summary());
}
```

### Discover Agents

```rust
// From .claude/agents/ + plugins
let agents = manager.discover_agents().unwrap();
for id in agents {
    println!("Agent: {}", id);
}
```

### List All Commands

```rust
for (plugin_id, commands) in manager.list_all_commands() {
    println!("{}: {:?}", plugin_id, commands);
}
```

### List All Skills

```rust
for (plugin_id, skills) in manager.list_all_skills() {
    println!("{}: {:?}", plugin_id, skills);
}
```

## Agent Auto-Discovery

Put agent markdown files in `.claude/agents/`:

```
.claude/
└── agents/
    ├── builder.md
    ├── tester.md
    └── reviewer.md
```

They'll be automatically discovered:

```rust
let discovery = AgentDiscovery::new(".");
let agents = discovery.discover_all().unwrap();
```

## Hook Events

Available hook events:
- `SessionStart` / `onLoad`
- `SessionEnd` / `onUnload`
- `PreToolUse`
- `PostToolUse`
- `UserPromptSubmit`
- `Stop`
- `SubagentStop`
- `Notification`
- `PreCompact`

## MCP Protocol

MCP servers must implement:
- `initialize` - Handshake
- `tools/list` - List available tools
- `tools/call` - Execute a tool

Communication via JSON-RPC 2.0 over stdin/stdout.

## Example Plugin

See `/examples/plugins/example-plugin/` for a complete working example with:
- 2 commands
- 2 skills
- 1 agent
- 2 hooks
- 1 MCP server config

## Testing

```bash
# Test plugin system
cargo test --package rustyclawd-cli --test plugin_integration_tests

# Test specific module
cargo test --package rustyclawd-cli manifest
```

## File Permissions

Make scripts executable:

```bash
chmod +x commands/*.js
chmod +x hooks/*.js
```

## Common Patterns

### Command with File I/O

```javascript
#!/usr/bin/env node
const fs = require('fs');
const args = JSON.parse(process.argv[2] || '{}');

const content = fs.readFileSync(args.file, 'utf8');
console.log(`File has ${content.length} characters`);
process.exit(0);
```

### Hook that Blocks

```javascript
#!/usr/bin/env node
const context = JSON.parse(process.env.HOOK_CONTEXT || '{}');

if (context.tool_name === 'Write') {
  console.log(JSON.stringify({
    continue: false,
    permissionDecision: 'deny',
    systemMessage: 'Write blocked by plugin'
  }));
  process.exit(2); // Blocking error
}

console.log(JSON.stringify({ continue: true }));
process.exit(0);
```

### Skill with Sections

```markdown
# My Skill

Brief description.

## Role
What you are

## Process
1. Step one
2. Step two

## Output Format
Expected output structure

## Examples
Example inputs/outputs
```

## Plugin Manager Summary

```rust
let summary = manager.summary();
println!("Plugins: {}/{} loaded",
    summary.loaded_plugins,
    summary.total_plugins);
println!("Commands: {}", summary.total_commands);
println!("Skills: {}", summary.total_skills);
println!("Agents: {}", summary.total_agents);
println!("MCP: {}/{} running",
    summary.running_mcp_servers,
    summary.total_mcp_servers);
```

## Troubleshooting

### Plugin Won't Load

Check:
1. Is `plugin.json` valid JSON?
2. Do all required fields exist?
3. Does `main` file exist?
4. Are command/skill paths correct?

### Command Fails

Check:
1. Is script executable? (`chmod +x`)
2. Does it parse args correctly?
3. Does it exit with code 0?
4. Does it output to stdout (not stderr)?

### Hook Not Firing

Check:
1. Is event name correct?
2. Is handler path relative to plugin root?
3. Is hook script executable?
4. Is it registered with HookRegistry?

## Next Steps

1. Copy example plugin as template
2. Modify for your use case
3. Test locally
4. Deploy to plugins directory
5. Load with PluginManager

## Resources

- Full docs: `PLUGIN_SYSTEM_COMPLETE.md`
- Example: `/examples/plugins/example-plugin/`
- Tests: `crates/cli/tests/plugin_integration_tests.rs`
- Module: `crates/cli/src/plugins/README.md`
