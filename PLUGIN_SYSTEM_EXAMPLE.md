# Plugin System Usage Example

## Quick Start

### 1. Create a Plugin Directory

```bash
mkdir -p plugins/my-plugin
cd plugins/my-plugin
```

### 2. Create plugin.json

```json
{
  "id": "com.example.myplugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "A simple example plugin",
  "author": "John Doe",
  "license": "MIT",
  "main": "index.js",
  "commands": [
    {
      "name": "greet",
      "description": "Greet someone",
      "path": "commands/greet.js",
      "argsSchema": {
        "type": "object",
        "properties": {
          "name": {
            "type": "string",
            "description": "Name to greet"
          }
        },
        "required": ["name"]
      }
    },
    {
      "name": "calculate",
      "description": "Simple math",
      "path": "commands/calculate.js",
      "argsSchema": {
        "type": "object",
        "properties": {
          "operation": {"type": "string"},
          "a": {"type": "number"},
          "b": {"type": "number"}
        }
      }
    }
  ],
  "skills": [
    {
      "id": "time-management",
      "name": "Time Management",
      "description": "Helps with time management tasks",
      "path": "skills/time.md"
    }
  ],
  "hooks": [
    {
      "event": "onLoad",
      "handler": "handlers/onLoad.js"
    }
  ],
  "dependencies": {},
  "configSchema": {
    "type": "object",
    "properties": {
      "debug": {
        "type": "boolean",
        "default": false
      }
    }
  }
}
```

### 3. Create Entry Point (index.js)

```javascript
// index.js - Plugin entry point
console.log("My Plugin loaded successfully!");
module.exports = {
  version: "1.0.0"
};
```

### 4. Create Commands

```javascript
// commands/greet.js
module.exports = async (args) => {
  return `Hello, ${args.name}! Welcome!`;
};
```

```javascript
// commands/calculate.js
module.exports = async (args) => {
  let result;
  switch(args.operation) {
    case 'add':
      result = args.a + args.b;
      break;
    case 'subtract':
      result = args.a - args.b;
      break;
    case 'multiply':
      result = args.a * args.b;
      break;
    case 'divide':
      result = args.a / args.b;
      break;
    default:
      throw new Error(`Unknown operation: ${args.operation}`);
  }
  return `Result: ${result}`;
};
```

### 5. Create Skills

```markdown
# Time Management Skill

This skill helps with managing time effectively.

## Features
- Time tracking
- Schedule management
- Deadline reminders

## Usage
Use this skill when you need help organizing your time.
```

## Rust Code Integration

### Basic Usage

```rust
use claude_code_cli::plugins::*;

// 1. Discover plugins
let discovery = PluginDiscovery::new("./plugins");
let plugins = discovery.discover_all()?;

for plugin in &plugins {
    println!("Found plugin: {} v{}",
             plugin.manifest.name,
             plugin.manifest.version);
}

// 2. Load plugins
let mut loader = PluginLoader::new();

for plugin in plugins {
    loader.register(plugin);
}

// Load specific plugin
loader.load("com.example.myplugin")?;

// Initialize after loading
loader.initialize("com.example.myplugin")?;

// 3. Execute commands
let mut executor = PluginExecutor::new();

// Get loaded plugin
if let Some(plugin) = loader.get("com.example.myplugin") {
    executor.register(plugin);
}

// Execute command with arguments
let result = executor.execute_command(
    "com.example.myplugin",
    "greet",
    serde_json::json!({
        "name": "Alice"
    })
)?;

println!("Success: {}", result.success);
println!("Output: {}", result.output);
println!("Duration: {}ms", result.duration_ms);

// Execute another command
let calc_result = executor.execute_command(
    "com.example.myplugin",
    "calculate",
    serde_json::json!({
        "operation": "add",
        "a": 5,
        "b": 3
    })
)?;

println!("Calculation: {}", calc_result.output);

// 4. Execute skills
let skill_result = executor.execute_skill(
    "com.example.myplugin",
    "time-management"
)?;

println!("Skill result: {}", skill_result.output);
```

### Advanced Usage

```rust
use claude_code_cli::plugins::*;

// Discover all plugins
let discovery = PluginDiscovery::new("./plugins");
let discovered = discovery.discover_all()?;

// Filter and validate
let valid_plugins: Vec<_> = discovered
    .iter()
    .filter(|p| {
        discovery.validate_structure(&p.path).is_ok()
    })
    .cloned()
    .collect();

println!("Valid plugins: {}", valid_plugins.len());

// Load with error handling
let mut loader = PluginLoader::new();

for plugin in valid_plugins {
    loader.register(plugin);

    match loader.load(&plugin.id) {
        Ok(_) => println!("Loaded: {}", plugin.id),
        Err(e) => println!("Failed to load {}: {}", plugin.id, e),
    }
}

// Get all loaded plugins
let loaded = loader.loaded_plugins();
println!("Loaded {} plugins", loaded.len());

// Get specific plugin info
if let Some(plugin) = loader.get("com.example.myplugin") {
    println!("Plugin: {}", plugin.manifest.name);
    println!("Commands: {}", plugin.manifest.commands.len());
    println!("Skills: {}", plugin.manifest.skills.len());
    println!("Enabled: {}", plugin.enabled);
}

// Disable a plugin
loader.disable("com.example.myplugin")?;

// Enable it again
loader.enable("com.example.myplugin")?;

// Validate execution
let mut executor = PluginExecutor::new();

for plugin in loader.all_plugins() {
    executor.register(plugin);
}

// Get available commands
let commands = executor.get_commands("com.example.myplugin")?;
println!("Available commands: {:?}", commands);

// Get available skills
let skills = executor.get_skills("com.example.myplugin")?;
println!("Available skills: {:?}", skills);

// Execute with validation
let result = executor.execute_command(
    "com.example.myplugin",
    "greet",
    serde_json::json!({"name": "Bob"})
)?;

// Validate result contract
if let Err(errors) = PluginValidator::validate_result(&result) {
    println!("Result validation failed:");
    for err in errors {
        println!("  - {}", err);
    }
} else {
    println!("Result is valid");
    println!("Success: {}", result.success);
    println!("Errors: {:?}", result.errors);
}

// Unload plugin
loader.unload("com.example.myplugin")?;
```

### Error Handling

```rust
use claude_code_cli::plugins::*;

// Try to discover plugins
match PluginDiscovery::new("./plugins").discover_all() {
    Ok(plugins) => {
        println!("Discovered {} plugins", plugins.len());
    }
    Err(e) => {
        eprintln!("Discovery failed: {}", e);
    }
}

// Try to load a plugin
let mut loader = PluginLoader::new();

match loader.load("non.existent.plugin") {
    Ok(_) => println!("Plugin loaded"),
    Err(e) => match e.as_str() {
        "Plugin not found" => println!("Plugin not registered"),
        "Plugin directory not found" => println!("Directory missing"),
        "Missing plugin.json manifest" => println!("Manifest missing"),
        "Missing entry point" => println!("Entry point missing"),
        other => println!("Loading failed: {}", other),
    }
}

// Try to execute a command
let executor = PluginExecutor::new();

match executor.execute_command(
    "com.example.plugin",
    "unknown-command",
    serde_json::json!({})
) {
    Ok(result) => {
        if result.success {
            println!("Command succeeded");
        } else {
            println!("Command failed: {:?}", result.errors);
        }
    }
    Err(e) => match e.as_str() {
        "Plugin not found: ..." => println!("Plugin not found"),
        "Plugin is disabled" => println!("Plugin is disabled"),
        "Command not found: ..." => println!("Command not found"),
        other => println!("Execution failed: {}", other),
    }
}
```

### Full Lifecycle Example

```rust
use claude_code_cli::plugins::*;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plugin_dir = Path::new("./plugins");

    // Phase 1: Discovery
    println!("=== Phase 1: Discovery ===");
    let discovery = PluginDiscovery::new(plugin_dir);
    let plugins = discovery.discover_all()?;

    for plugin in &plugins {
        println!("Found: {} v{}", plugin.manifest.name, plugin.manifest.version);
        println!("  ID: {}", plugin.id);
        println!("  Status: {:?}", plugin.load_status);
    }

    // Phase 2: Validation
    println!("\n=== Phase 2: Validation ===");
    for plugin in &plugins {
        match discovery.validate_structure(&plugin.path) {
            Ok(_) => println!("{}: Valid", plugin.id),
            Err(e) => println!("{}: Invalid - {}", plugin.id, e),
        }
    }

    // Phase 3: Loading
    println!("\n=== Phase 3: Loading ===");
    let mut loader = PluginLoader::new();

    for plugin in plugins {
        loader.register(plugin);
    }

    for id in ["com.example.myplugin", "com.example.other"] {
        match loader.load(id) {
            Ok(_) => {
                println!("{}: Loaded", id);
                if let Ok(_) = loader.initialize(id) {
                    println!("{}: Initialized", id);
                }
            }
            Err(e) => println!("{}: Failed - {}", id, e),
        }
    }

    // Phase 4: Execution
    println!("\n=== Phase 4: Execution ===");
    let mut executor = PluginExecutor::new();

    for plugin in loader.loaded_plugins() {
        executor.register(plugin.clone());
        println!("Registered: {}", plugin.id);
    }

    // Execute commands
    let result = executor.execute_command(
        "com.example.myplugin",
        "greet",
        serde_json::json!({"name": "World"})
    )?;

    println!("\nCommand execution:");
    println!("  Success: {}", result.success);
    println!("  Output: {}", result.output);
    println!("  Duration: {}ms", result.duration_ms);

    Ok(())
}
```

## Plugin Directory Structure

```
plugins/
├── my-plugin/
│   ├── plugin.json              # Manifest
│   ├── index.js                 # Entry point
│   ├── commands/
│   │   ├── greet.js
│   │   └── calculate.js
│   ├── skills/
│   │   └── time.md
│   └── handlers/
│       └── onLoad.js
├── another-plugin/
│   ├── plugin.json
│   ├── main.js
│   └── commands/
│       └── cmd.js
└── broken-plugin/               # This won't be loaded
    └── plugin.json              # (missing entry point)
```

## Best Practices

1. **Always validate**: Check manifest validity before loading
2. **Handle errors**: Plugins can fail to load
3. **Enable/disable**: Use enable/disable instead of unloading
4. **Query first**: Check available commands before executing
5. **Validate results**: Ensure execution results are consistent
6. **Use types**: Leverage strong types for safety
7. **Document**: Add README to your plugins
8. **Test**: Write tests for your plugins

## Running Tests

```bash
# Run all plugin system tests
cargo test --test plugin_tests

# Run specific test module
cargo test --test plugin_tests discovery::

# Run with output
cargo test --test plugin_tests -- --nocapture --test-threads=1
```

## Performance Tips

- Load plugins once and reuse
- Cache plugin metadata
- Disable unused plugins
- Validate manifests early
- Use Status enum for quick checks

## Security Notes

- Always validate plugin.json
- Check file paths (prevent traversal)
- Validate arguments before execution
- Isolate plugin contexts
- Log plugin operations
- Limit plugin permissions

## Troubleshooting

**Plugin not found after discovery:**
- Check if plugin directory exists
- Verify plugin.json is in plugin root
- Check JSON syntax

**Plugin fails to load:**
- Verify entry point file exists
- Check command/skill file paths
- Ensure manifest is valid

**Execution fails:**
- Verify plugin is enabled
- Check command name exists
- Validate argument schema
- Check for plugin-side errors
