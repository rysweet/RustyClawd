# RustyClawd CLI Interface

## Overview

RustyClawd now supports the **exact same interface** as the official Claude Code CLI, making it a drop-in compatible alternative for Rust environments.

## Usage Modes

### 1. Interactive Mode (Default)

Start an interactive chat session with Claude:

```bash
claude-code
```

This launches a REPL where you can have multi-turn conversations with Claude.

### 2. Direct Query Execution

Execute a single prompt and exit:

```bash
# Using positional argument
claude-code "What is Rust ownership?"

# Using the -p/--prompt flag
claude-code -p "Explain async/await in Rust"
```

### 3. Piped Input

Pipe content directly to Claude:

```bash
# Pipe file content
cat main.rs | claude-code

# Pipe command output
ls -la | claude-code

# Chain with other commands
echo "Explain this code:" | cat - main.rs | claude-code
```

### 4. Tool Subcommand Mode

Execute specific tools directly from the CLI:

```bash
# Read a file
claude-code read src/main.rs

# Execute bash command
claude-code bash "cargo test" --timeout 30000

# Search for patterns
claude-code grep "async fn" --path src/ --glob "*.rs"

# Edit a file
claude-code edit src/lib.rs \
  --old-string "fn old_name" \
  --new-string "fn new_name"

# Find files
claude-code glob "**/*.toml"

# Manage todos
claude-code todo-write --todos '[{"content":"Fix bug","status":"pending","activeForm":"Fixing bug"}]'
```

## Priority Order

When multiple input methods are provided, RustyClawd follows this priority:

1. **Subcommand** - Explicit tool execution (e.g., `claude-code bash "ls"`)
2. **Prompt flag** - `-p/--prompt` option
3. **Piped stdin** - Input from pipe
4. **Positional query** - Direct query argument
5. **Interactive mode** - Default when no input provided

## Examples

### Example 1: Quick Question

```bash
claude-code "How do I implement a binary search tree in Rust?"
```

### Example 2: Code Review

```bash
cat src/algorithm.rs | claude-code -p "Review this code for performance issues"
```

### Example 3: File Analysis

```bash
claude-code read Cargo.toml | claude-code "What dependencies are outdated?"
```

### Example 4: Batch Processing

```bash
for file in src/*.rs; do
  echo "Analyzing $file..."
  cat "$file" | claude-code -p "Find potential bugs in this Rust code"
done
```

### Example 5: Tool Mode

```bash
# Execute a bash command with the Bash tool
claude-code bash "cargo build --release" --timeout 300000

# Search codebase
claude-code grep "TODO" --path . --glob "*.rs" -i
```

## Available Tools

All Claude Code tools are available as subcommands:

- `bash` - Execute bash commands
- `read` - Read files
- `write` - Write files
- `edit` - Edit files
- `glob` - Find files by pattern
- `grep` - Search for text
- `todo-write` - Manage task lists
- `agent` - Invoke sub-agents
- `skill` - Execute skills
- `slash-command` - Run slash commands
- `web-fetch` - Fetch web content
- `web-search` - Search the web
- `notebook-edit` - Edit Jupyter notebooks
- `ask-user-question` - Interactive questions
- `bash-output` - Get background shell output
- `kill-shell` - Terminate background shell

## Global Options

Available with all modes:

```bash
-d, --debug                      # Enable debug logging
--resume <SESSION_ID>            # Resume a previous session
--checkpoint-limit <N>           # Set checkpoint history limit (default: 50)
-p, --prompt <PROMPT>           # Execute prompt directly
-h, --help                       # Show help
-V, --version                    # Show version
```

## Interactive Mode Commands

When in interactive mode, use these commands:

- `/exit`, `/quit` - Exit the session
- `/clear` - Clear conversation history
- `/help` - Show help message
- `/stats` - Show session statistics
- `Ctrl+D` - Exit gracefully
- `Ctrl+C` - Cancel current input

## Compatibility with Claude Code

RustyClawd is designed to be **100% CLI-compatible** with the official Claude Code. You can:

1. Use it as a drop-in replacement in scripts
2. Keep the same aliases and workflows
3. Switch between implementations seamlessly

The only difference is the binary name: `claude-code` instead of `claude`.

## Environment Variables

RustyClawd respects standard environment variables:

- `ANTHROPIC_API_KEY` - Your Anthropic API key (required)
- `CLAUDE_CODE_CONFIG` - Custom config file path

## Configuration

Configuration follows the same 5-tier hierarchy as Claude Code:

1. CLI arguments (highest priority)
2. Environment variables
3. Project settings (`.claude/settings.json`)
4. User settings (`~/.config/claude-code/settings.json`)
5. Default settings (lowest priority)

## Migration from Claude Code

To migrate from the official Claude Code to RustyClawd:

1. Install RustyClawd: `cargo install claude-code-cli`
2. Create an alias: `alias claude=claude-code`
3. All your existing scripts and workflows will work unchanged

## Performance Benefits

RustyClawd offers several advantages:

- **Faster startup** - Native binary with minimal overhead
- **Lower memory usage** - Rust's efficient memory management
- **Better concurrency** - Tokio async runtime
- **Type safety** - Compile-time guarantees

## Error Handling

RustyClawd provides clear, actionable error messages:

```bash
$ claude-code read /nonexistent
Error: File not found: /nonexistent

$ echo "test" | claude-code bash "invalid command &@#"
Error: Command execution failed: invalid syntax
```

## Shell Integration

Add to your `.bashrc` or `.zshrc`:

```bash
# Quick Claude query
alias ask='claude-code'

# Code review current git diff
alias review='git diff | claude-code -p "Review these changes"'

# Explain command
explain() {
  claude-code "Explain this bash command: $*"
}
```

## Debugging

Enable debug mode for verbose output:

```bash
claude-code --debug "test query"

# Or with environment variable
RUST_LOG=debug claude-code "test query"
```

This shows:

- API request/response details
- Tool execution traces
- Session management
- Hook execution
- Checkpoint creation

## Session Management

RustyClawd automatically manages sessions with checkpoints:

```bash
# Start a new session (automatic)
claude-code

# Resume a previous session
claude-code --resume session-1234567890

# List sessions
ls ~/.cache/claude-code/sessions/
```

Each session includes:

- Full conversation history
- Checkpoint snapshots
- Tool execution logs
- Configuration state

## Contributing

RustyClawd is an educational project demonstrating Rust's capabilities. Contributions welcome!

Repository: https://github.com/yourusername/claude-code-rs

## License

MIT OR Apache-2.0
