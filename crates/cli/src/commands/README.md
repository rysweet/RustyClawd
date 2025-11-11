# Slash Command Execution System

A production-grade Rust implementation of slash command execution, enabling dynamic command discovery, file-based command definitions, template expansion, and built-in command handling.

## Overview

This module provides a complete system for executing slash commands in the Claude Code CLI. It supports:

- **Command Discovery**: Automatic scanning of `.claude/commands/` directory
- **File Loading**: Loading `.md` files with YAML frontmatter
- **Template Expansion**: Placeholder substitution (`{{args}}`, `{0}`, `{1}`, etc.)
- **Built-in Commands**: `/help`, `/exit`, `/clear`, `/history`, `/stats`
- **Character Budgeting**: 15,000 character limit for expanded prompts
- **Namespace Support**: Commands like `/amplihack:ultrathink`

## Architecture

### Module Structure

```
commands/
├── mod.rs          # Public API and command result types
├── parser.rs       # Command parsing (format: /name [args...])
├── loader.rs       # File loading and template expansion
├── registry.rs     # Command discovery and registration
├── executor.rs     # Command execution pipeline
├── builtins.rs     # Built-in command implementations
└── README.md       # This file
```

### Data Flow

```
User Input (/command-name arg1 arg2)
    ↓
CommandParser (parse command and arguments)
    ↓
Registry (lookup command definition)
    ↓
Executor (execute and expand)
    ↓
CommandResult (expanded prompt + metadata)
```

## Module Reference

### parser.rs - Command Parsing

Parses slash command input into structured commands.

**Key Types:**
- `Command` - Parsed command with name and arguments
- `CommandParser` - Parser implementation

**Supported Formats:**
```
/help                          # No arguments
/review-pr 123                 # Single argument
/analyze PR-456 high alice     # Multiple arguments
/amplihack:ultrathink test     # Namespace support
```

**Validation:**
- Command name: alphanumeric, hyphens, underscores, colons
- Argument extraction: split by whitespace
- 13 tests covering edge cases

### loader.rs - File Loading & Template Expansion

Loads command files and expands templates with arguments.

**YAML Frontmatter Format:**
```yaml
---
description: Review a pull request
model: claude-sonnet
allowed-tools:
  - Bash
  - Grep
---
Review PR #{0} with priority {1}
```

**Template Placeholders:**
- `{{args}}` - Full argument string
- `{0}`, `{1}`, `{2}` - Individual positional arguments

**Example:**
```
Template: "Review PR #{0} with priority {1} assigned to {2}"
Arguments: "123 high alice"
Result: "Review PR #123 with priority high assigned to alice"
```

**Key Types:**
- `FrontMatter` - YAML metadata (description, model, allowed_tools)
- `LoadedCommand` - Command with content and metadata
- `CommandLoader` - Loader implementation

**Features:**
- Graceful YAML parsing (falls back to defaults on error)
- Multiple template formats supported
- Empty argument handling
- 20 tests covering frontmatter parsing and expansion

### registry.rs - Command Discovery & Registration

Discovers commands from `.claude/commands/` directory and manages registry.

**Discovery Process:**
1. Scan `.claude/commands/` directory
2. Load all `.md` files
3. Register in-memory registry
4. Support dynamic registration for testing

**Key Types:**
- `Registry` - Command storage and lookup
- `RegistryError` - Error types

**Features:**
- Async directory scanning
- Command listing and searching
- Help information generation
- Sorted command names
- 10 tests covering registration and lookup

### executor.rs - Command Execution

Executes commands with error handling and budget checking.

**Execution Pipeline:**
1. Check if built-in command
2. Lookup in registry
3. Expand template with arguments
4. Check character limit (15,000 chars)
5. Return CommandResult

**Key Types:**
- `Executor` - Execution engine
- `ExecutorError` - Execution errors
- `CommandResult` - Result with expanded prompt

**Features:**
- Built-in command handling
- Character budget enforcement
- Async/await support
- Comprehensive error types
- 8 tests covering execution scenarios

### builtins.rs - Built-in Commands

Implements system commands that don't require file definitions.

**Supported Commands:**
- `/help [search-term]` - Show help
- `/exit` or `/quit` - Exit session
- `/clear` - Clear history
- `/history` - Show history
- `/stats` - Show statistics

**Example Output:**
```
/help                   # Shows all available commands
/help slash-commands    # Searches for commands matching term
/exit                   # Exits with goodbye message
/clear                  # Clears history
```

**Key Types:**
- `BuiltinCommands` - Builtin handler with static methods

**Features:**
- Detection of built-in commands
- Execution with optional arguments
- Consistent output formatting
- 9 tests covering all built-in commands

### mod.rs - Public API

Public interface and command result types.

**Key Types:**
- `SlashCommands` - Main API entry point
- `CommandResult` - Result structure
- Constants: `DEFAULT_COMMANDS_DIR`, `MAX_EXPANDED_CHARS`

**Usage Example:**
```rust
let slash_cmds = SlashCommands::new().await?;
let result = slash_cmds.execute("/review-pr 123").await?;
println!("{}", result.expanded_prompt);
```

**Features:**
- Character budget tracking
- Budget percentage calculation
- Budget validation
- 4 tests for public API

## Test Coverage

**Total: 63 tests passing (required: 50)**

### Test Breakdown by Module:
- **parser.rs**: 14 tests
  - Command parsing with/without arguments
  - Namespace support
  - Invalid input handling
  - Edge cases

- **loader.rs**: 20 tests
  - Frontmatter parsing
  - Template expansion
  - Placeholder substitution
  - Edge cases (empty, malformed)

- **registry.rs**: 10 tests
  - Command registration
  - Discovery and lookup
  - Searching and listing
  - Error handling

- **executor.rs**: 8 tests
  - Built-in execution
  - Custom command execution
  - Character limit enforcement
  - Error cases

- **builtins.rs**: 9 tests
  - All built-in commands
  - Argument handling
  - Command detection

- **mod.rs** (public API): 4 tests
  - Constants
  - Result types
  - Budget tracking

## Usage Examples

### Basic Command Execution
```rust
use claude_code_cli::commands::*;

let parser = CommandParser::new();
let cmd = parser.parse("/help")?;
assert_eq!(cmd.name, "help");
```

### Registry with Custom Command
```rust
let mut registry = Registry::new(PathBuf::from(".commands"));

let cmd = LoadedCommand {
    name: "review".to_string(),
    frontmatter: FrontMatter::default(),
    content: "Review PR #{0}".to_string(),
};

registry.register(cmd)?;
```

### Full Execution Pipeline
```rust
let executor = Executor::new();
let registry = Registry::discover(PathBuf::from(".claude/commands")).await?;

let cmd = Command::new("review-pr".to_string(), Some("456".to_string()));
let result = executor.execute(&cmd, &registry).await?;

println!("Command: {}", result.command_name);
println!("Expanded: {}", result.expanded_prompt);
println!("Budget: {:.1}%", result.budget_percentage());
```

### SlashCommands High-Level API
```rust
let slash_cmds = SlashCommands::new().await?;

// Execute a command
let result = slash_cmds.execute("/review-pr 789").await?;

// List all commands
let commands = slash_cmds.list_commands();

// Get help
let help = slash_cmds.get_help(None);
```

## Character Budget

All expanded prompts are limited to **15,000 characters** to manage API costs and response times.

```rust
let result: CommandResult = ...;
assert!(result.is_within_budget());
let percent = result.budget_percentage();  // 0.0-100.0
```

## File Format Specification

### Command File Structure

Located in: `.claude/commands/command-name.md`

```markdown
---
description: Human-readable description
model: claude-sonnet (optional)
allowed-tools:    (optional list)
  - Bash
  - Read
---
Your command template here with {0} placeholders
```

### Minimum Valid File
```markdown
Command template content
```

### Full Example
```markdown
---
description: Review a GitHub pull request
model: claude-sonnet-4-5
allowed-tools:
  - Bash
  - Grep
  - Web Fetch
---
Review the following pull request #{0}:

Priority: {1}
Assigned to: {2}

Focus on:
- Code quality
- Security implications
- Performance considerations
- Test coverage
```

## Error Handling

### Error Types

**Parser Errors:**
- Command must start with `/`
- Command name cannot be empty
- Invalid command name characters

**Executor Errors:**
- `CommandNotFound` - Command not in registry
- `CharacterLimitExceeded` - Expanded prompt too long
- `ExpansionFailed` - Template expansion error

**Registry Errors:**
- `CommandNotFound` - Command not registered
- `InvalidCommand` - Invalid command structure

## Performance Characteristics

- **Command Parsing**: O(1) - microsecond range
- **Placeholder Replacement**: O(n) where n = argument count
- **Registry Lookup**: O(1) - hash map
- **File Loading**: O(1) - single file read
- **Directory Scan**: O(n) where n = number of files

## Integration Points

### Interactive Mode
```rust
// In interactive.rs
if input.starts_with('/') {
    let slash_cmds = SlashCommands::new().await?;
    let result = slash_cmds.execute(input).await?;
    // Pass result.expanded_prompt to Claude API
}
```

### API Client
The expanded prompt from `CommandResult.expanded_prompt` is passed directly to the Claude API as part of the user's message.

## Future Enhancements

- Permission-based command access
- Command parameter validation
- Multi-stage command chaining
- Command result caching
- Custom command namespaces
- Plugin system integration
- Command versioning

## Dependencies

- `tokio` - Async runtime
- `serde` & `serde_yaml` - Serialization
- `anyhow` & `thiserror` - Error handling
- `tracing` - Logging (debug output)

## Testing

Run all command tests:
```bash
cargo test --lib commands::
```

Run specific module:
```bash
cargo test --lib commands::parser::
cargo test --lib commands::loader::
cargo test --lib commands::registry::
cargo test --lib commands::executor::
cargo test --lib commands::builtins::
```

Run integration tests:
```bash
cargo test commands_integration
```

## Constants

- `DEFAULT_COMMANDS_DIR` = `.claude/commands`
- `MAX_EXPANDED_CHARS` = `15_000`

## License

Part of Claude Code - Educational Rust Translation
