# Example Plugin

A comprehensive example plugin demonstrating all features of the Claude Code plugin system.

## Features Demonstrated

### 1. Commands

Two example commands showing different use cases:

- **hello** - Simple greeting command with required argument
- **analyze-code** - More complex command with file I/O and optional parameters

Usage:
```bash
# Via plugin system
plugin.execute_command("com.claudecode.example", "hello", {"name": "World"})
plugin.execute_command("com.claudecode.example", "analyze-code", {"file": "test.js", "strict": true})
```

### 2. Skills

Two skills for code-related tasks:

- **code-reviewer** - Reviews code for quality and best practices
- **test-generator** - Generates comprehensive unit tests

Skills are markdown files containing system prompts that can be loaded into Claude.

### 3. Agents

One specialized agent:

- **refactorer** - Refactors code to improve structure and maintainability

Agents are like skills but can be invoked as sub-agents with context isolation.

### 4. Hooks

Lifecycle hooks that integrate with the CLI:

- **pre-tool.js** - Runs before tool execution (can block operations)
- **post-tool.js** - Runs after tool execution (can log and audit)

### 5. MCP Servers

Example MCP server configuration (would require actual MCP server implementation):

- **example-mcp** - Demonstrates how to configure external MCP servers

### 6. Configuration Schema

Defines plugin configuration options:

- `theme` - UI theme preference (light/dark)
- `verbosity` - Logging verbosity level (0-3)

## Installation

1. Copy this directory to your plugins folder:
   ```bash
   cp -r example-plugin ~/.claude/plugins/
   ```

2. Ensure commands are executable:
   ```bash
   chmod +x example-plugin/commands/*.js
   chmod +x example-plugin/hooks/*.js
   ```

3. Install dependencies (if needed):
   ```bash
   cd example-plugin && npm install
   ```

## Plugin Structure

```
example-plugin/
├── plugin.json           # Plugin manifest
├── index.js             # Entry point
├── README.md            # This file
├── commands/            # Command implementations
│   ├── hello.js
│   └── analyze.js
├── skills/              # Skill prompts
│   ├── code-reviewer.md
│   └── test-generator.md
├── agents/              # Agent prompts
│   └── refactorer.md
└── hooks/               # Lifecycle hooks
    ├── pre-tool.js
    └── post-tool.js
```

## Development

### Adding a New Command

1. Create a JavaScript file in `commands/`
2. Add command definition to `plugin.json`
3. Implement command logic (read args from argv[2] as JSON)
4. Exit with appropriate code (0 = success, 1 = error)

### Adding a New Skill

1. Create a markdown file in `skills/`
2. Add skill definition to `plugin.json`
3. Write the skill prompt following the skill template format

### Adding a New Hook

1. Create a JavaScript file in `hooks/`
2. Add hook definition to `plugin.json`
3. Implement hook logic following the hook contract
4. Output JSON for advanced control

## Testing

Test the plugin locally:

```rust
use rustyclawd_cli::plugins::PluginManager;

#[tokio::main]
async fn main() {
    let mut manager = PluginManager::new("./examples/plugins");

    // Discover and load
    let loaded = manager.discover_and_load_all().await.unwrap();
    println!("Loaded plugins: {:?}", loaded);

    // Execute command
    let result = manager.execute_command(
        "com.claudecode.example",
        "hello",
        serde_json::json!({"name": "Test"})
    ).await.unwrap();

    println!("Result: {}", result.output);
}
```

## API Reference

### Commands

Commands receive JSON arguments via `process.argv[2]`:

```javascript
const args = JSON.parse(process.argv[2] || '{}');
console.log(`Hello, ${args.name}!`);
process.exit(0);
```

### Hooks

Hooks receive context via environment variable `HOOK_CONTEXT`:

```javascript
const context = JSON.parse(process.env.HOOK_CONTEXT || '{}');
const toolName = context.tool_name;

// Output control JSON
const output = {
  continue: true,
  permissionDecision: 'allow',
  systemMessage: 'Optional message'
};
console.log(JSON.stringify(output));
process.exit(0);
```

## License

MIT License - See LICENSE file for details.

## Contributing

This is an example plugin. Feel free to use it as a template for your own plugins!

## Support

For issues or questions about the plugin system, please refer to the main Claude Code documentation.
