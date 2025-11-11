# Plugin System

Complete plugin management system for Claude Code CLI with discovery, loading, validation, and execution.

## Overview

The plugin system provides:

- **Discovery**: Scan plugin directories for `plugin.json` manifests
- **Validation**: Enforce plugin API contract and manifest schema
- **Loading**: Load plugin files and metadata with status tracking
- **Execution**: Execute plugin commands and skills with argument validation
- **Lifecycle**: Full plugin lifecycle from discovery to initialization

## Architecture

### Modules

1. **manifest.rs** - Plugin manifest parsing and validation
   - `PluginManifest` - plugin.json structure
   - `parse_manifest()` - Load manifest from file
   - `validate_manifest()` - Schema validation
   - `validate_references()` - File existence checking

2. **discovery.rs** - Plugin scanning and location
   - `PluginDiscovery` - Directory scanner
   - `discover_all()` - Find all plugins
   - `validate_structure()` - Full validation
   - `PluginMetadata` - Plugin info + status
   - `PluginLoadStatus` - Enum for status tracking

3. **loader.rs** - Plugin file loading and state management
   - `PluginLoader` - Lifecycle manager
   - `load()` - Validate and load plugin
   - `initialize()` - Post-load initialization
   - `enable()/disable()` - Runtime control
   - `loaded_plugins()` - Query by status

4. **executor.rs** - Plugin command/skill execution
   - `PluginExecutor` - Execution engine
   - `execute_command()` - Run command with args
   - `execute_skill()` - Run skill
   - `PluginExecutionResult` - Result struct
   - `PluginValidator` - Contract validation

5. **mod.rs** - Public API
   - Re-exports public types
   - Error types and result alias
   - Integration tests

## API Contract

### Plugin Manifest (plugin.json)

```json
{
  "id": "com.example.plugin",
  "name": "Example Plugin",
  "version": "1.0.0",
  "description": "A sample plugin",
  "author": "Author Name",
  "license": "MIT",
  "main": "index.js",
  "commands": [
    {
      "name": "my-command",
      "description": "Does something",
      "path": "commands/my-command.js",
      "argsSchema": {}
    }
  ],
  "skills": [
    {
      "id": "my-skill",
      "name": "My Skill",
      "description": "A skill",
      "path": "skills/my-skill.md"
    }
  ],
  "hooks": [
    {
      "event": "onLoad",
      "handler": "handlers/onLoad.js"
    }
  ],
  "dependencies": {
    "dependency": "1.0.0"
  },
  "configSchema": {}
}
```

### Required Fields

- `id`: Dotted identifier (e.g., `com.example.plugin`)
- `name`: Human-readable name
- `version`: Semantic version (X.Y.Z)
- `description`: Plugin description
- `author`: Plugin author
- `license`: License identifier
- `main`: Entry point file path

### Optional Fields

- `commands`: Command definitions (default: empty)
- `skills`: Skill definitions (default: empty)
- `hooks`: Hook definitions (default: empty)
- `dependencies`: Runtime dependencies (default: empty)
- `configSchema`: JSON Schema for configuration (default: empty)

## Usage

### 1. Discovery

Scan a plugin directory:

```rust
use claude_code_cli::plugins::*;

let discovery = PluginDiscovery::new("./plugins");
let plugins = discovery.discover_all()?;

for plugin in plugins {
    println!("Found: {} v{}", plugin.manifest.name, plugin.manifest.version);
}
```

### 2. Loading

Register and load plugins:

```rust
let mut loader = PluginLoader::new();

for plugin in plugins {
    loader.register(plugin);
}

// Load a plugin
loader.load("com.example.plugin")?;

// Initialize after loading
loader.initialize("com.example.plugin")?;

// Check status
assert!(loader.is_loaded("com.example.plugin"));
```

### 3. Execution

Execute plugin commands and skills:

```rust
let mut executor = PluginExecutor::new();
executor.register(plugin);

// Execute command with arguments
let result = executor.execute_command(
    "com.example.plugin",
    "my-command",
    serde_json::json!({ "arg": "value" })
)?;

if result.success {
    println!("Output: {}", result.output);
} else {
    println!("Errors: {:?}", result.errors);
}

// Execute skill
let result = executor.execute_skill("com.example.plugin", "my-skill")?;
```

### 4. Validation

Validate plugin contracts:

```rust
use claude_code_cli::plugins::executor::PluginValidator;

// Validate manifest
PluginValidator::validate_manifest(&manifest)?;

// Validate execution result
PluginValidator::validate_result(&result)?;
```

## Plugin Load States

```
Discovered -> Loaded -> Initialized
   |           |          |
   +-----------|----------+
              |
           Failed(reason)
```

- **Discovered**: Plugin found and manifest parsed
- **Loaded**: Files validated, ready for execution
- **Initialized**: Plugin initialized and running
- **Failed**: Error during loading (with reason)

## File Structure Example

```
plugins/
├── my-plugin/
│   ├── plugin.json           # Manifest
│   ├── index.js              # Entry point
│   ├── commands/
│   │   ├── cmd1.js
│   │   └── cmd2.js
│   ├── skills/
│   │   └── skill1.md
│   └── handlers/
│       └── onLoad.js
└── another-plugin/
    └── plugin.json
```

## Error Handling

### PluginError Types

```rust
pub enum PluginError {
    Discovery(String),       // Discovery error
    Manifest(String),        // Manifest parsing error
    Load(String),            // Loading error
    Execution(String),       // Execution error
    Validation(Vec<String>), // Multiple validation errors
}
```

### Result Type

```rust
pub type PluginResult<T> = Result<T, PluginError>;
```

## Tests

18 comprehensive tests covering:

- Discovery (6 tests)
  - Empty directory
  - Single/multiple plugins
  - Validation success/failure

- Loading (5 tests)
  - Load valid plugin
  - Load with commands
  - Initialize plugin
  - Error cases

- Execution (2 tests)
  - Command execution
  - Skill execution

- API Contract (5 tests)
  - Manifest validation
  - Result validation
  - Consistency checks

### Running Tests

```bash
# Run all plugin tests
cargo test --test plugin_tests

# Run specific test module
cargo test --test plugin_tests discovery::

# Run with output
cargo test --test plugin_tests -- --nocapture
```

## Plugin Development Guide

### Creating a Plugin

1. Create plugin directory:
```bash
mkdir my-plugin
cd my-plugin
```

2. Create `plugin.json`:
```json
{
  "id": "com.example.myplugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "My first plugin",
  "author": "Me",
  "license": "MIT",
  "main": "index.js",
  "commands": [
    {
      "name": "hello",
      "description": "Say hello",
      "path": "commands/hello.js",
      "argsSchema": {
        "type": "object",
        "properties": {
          "name": { "type": "string" }
        }
      }
    }
  ]
}
```

3. Create entry point `index.js`:
```javascript
// Plugin initialization
console.log("Plugin loaded!");
```

4. Create command `commands/hello.js`:
```javascript
// Command implementation
module.exports = async (args) => {
  return `Hello, ${args.name}!`;
};
```

## Best Practices

1. **Plugin ID**: Use reverse domain notation (com.company.plugin)
2. **Versioning**: Follow semantic versioning (MAJOR.MINOR.PATCH)
3. **Manifests**: Keep plugin.json minimal and valid
4. **Errors**: Always handle errors in execution
5. **Dependencies**: List all runtime dependencies
6. **Documentation**: Include README in plugin directory

## Integration Points

The plugin system integrates with:

- CLI command handling
- Skill system
- Hook system
- Configuration management
- Error handling

## Performance

- Discovery: O(n) directory scan
- Loading: O(n) file validation
- Execution: O(1) plugin lookup
- Memory: Plugins loaded once, cached by ID

## Future Enhancements

- Plugin auto-reload on manifest change
- Plugin dependencies resolution
- Plugin marketplace
- Plugin version constraints
- Hot-reloading
- Plugin sandboxing
- Plugin permissions system
- Plugin metrics and monitoring

## Security Considerations

1. Validate all plugin manifests
2. Check plugin directory permissions
3. Validate file paths (prevent traversal)
4. Isolate plugin execution contexts
5. Limit plugin resource access
6. Log all plugin operations
