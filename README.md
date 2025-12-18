# RustyClawd 🦀

Claude Code Compatible agentic coding cli+sdk implemented in Rust. 

Intended to be fully compatible with Claude Code CLI and SDK, including support for commands, subagents, hooks, etc. 

## Installation

### Option 1: Build from Source (Recommended)

```bash
git clone https://github.com/rysweet/RustyClawd
cd RustyClawd
cargo build --release

# Add to PATH or create alias
alias rusty="$PWD/target/release/rusty"

# Then use:
rusty "your prompt"
```

### Option 2: Cargo Install

```bash
cargo install --git https://github.com/rysweet/RustyClawd --bin rusty
rusty "your prompt"
```

## Usage

```bash
# Interactive chat
rusty

# Direct prompt
rusty "what is rust?"

# Print mode
rusty -p "calculate 2+2"

# With model
rusty --model haiku "count to 5"

# Tool execution (automatic)
rusty -p "create file test.txt with 'hello'"
rusty -p "run: ls -la"

# Other features
rusty --verbose -p "debug this"                    # Verbose logging
rusty --system-prompt-file ./prompt.txt "query"    # Custom system prompt
rusty --add-dir ./src --add-dir ./tests "analyze"  # Multiple directories
rusty --allowedTools Read --allowedTools Grep "search only"  # Tool control
rusty update                                       # Update CLI
rusty mcp                                          # Configure MCP servers
```

## Amplihack Integration

Works with amplihack framework:

```bash
# From PR branch
uvx --from git+https://github.com/rysweet/MicrosoftHackathon2025-AgenticCoding.git@feat/rustyclawd-integration amplihack RustyClawd -- -p "test"

# Or set environment
export AMPLIHACK_USE_RUSTYCLAWD=1
amplihack -- -p "your prompt"
```

PR: https://github.com/rysweet/MicrosoftHackathon2025-AgenticCoding/pull/1310

## Documentation

- **[Contributing Guide](CONTRIBUTING.md)** - How to contribute to RustyClawd
- **[Architecture Guide](docs/ARCHITECTURE.md)** - System design, module structure, and key decisions
- **[Hook Lifecycle Integration](docs/HOOK_LIFECYCLE_INTEGRATION.md)** - Complete hook system documentation
- **[Rust Patterns Learned](RUST_PATTERNS_LEARNED.md)** - Technical patterns and best practices
