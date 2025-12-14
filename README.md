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
alias rustyclawd="$PWD/target/release/rustyclawd"

# Then use:
rustyclawd "your prompt"
```

### Option 2: Cargo Install

```bash
cargo install --git https://github.com/rysweet/RustyClawd --bin rustyclawd
rustyclawd "your prompt"
```

## Usage

```bash
# Interactive chat
rustyclawd

# Direct prompt
rustyclawd "what is rust?"

# Print mode
rustyclawd -p "calculate 2+2"

# With model
rustyclawd --model haiku "count to 5"

# Tool execution (automatic)
rustyclawd -p "create file test.txt with 'hello'"
rustyclawd -p "run: ls -la"

# Other features
rustyclawd --verbose -p "debug this"                    # Verbose logging
rustyclawd --system-prompt-file ./prompt.txt "query"    # Custom system prompt
rustyclawd --add-dir ./src --add-dir ./tests "analyze"  # Multiple directories
rustyclawd --allowedTools Read --allowedTools Grep "search only"  # Tool control
rustyclawd update                                       # Update CLI
rustyclawd mcp                                          # Configure MCP servers
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
