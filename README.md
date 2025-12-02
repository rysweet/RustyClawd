# RustyClawd 🦀

Production Rust implementation of Claude Code with complete SDK and CLI spec compliance.

✅ **100% CLI Spec Compliant** - Matches [official Claude Code CLI](https://code.claude.com/docs/en/cli-reference) exactly

## Installation

### Option 1: Build from Source (Recommended)

```bash
git clone https://github.com/rysweet/RustyClawd
cd RustyClawd
cargo build --release

# Add to PATH or create alias
alias claude="$PWD/target/release/claude"

# Then use:
claude "your prompt"
```

### Option 2: Cargo Install

```bash
cargo install --git https://github.com/rysweet/RustyClawd --bin claude
claude "your prompt"
```

## Usage

```bash
# Interactive chat
claude

# Direct prompt
claude "what is rust?"

# Print mode
claude -p "calculate 2+2"

# With model
claude --model haiku "count to 5"

# Tool execution (automatic)
claude -p "create file test.txt with 'hello'"
claude -p "run: ls -la"

# New spec-compliant features
claude --verbose -p "debug this"                    # Verbose logging
claude --system-prompt-file ./prompt.txt "query"    # Custom system prompt
claude --add-dir ./src --add-dir ./tests "analyze"  # Multiple directories
claude --allowedTools Read --allowedTools Grep "search only"  # Tool control
claude update                                       # Update CLI
claude mcp                                          # Configure MCP servers
```

See `MIGRATION_GUIDE.md` for breaking changes and `CLI_SPEC_COMPLIANCE.md` for full details.

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

## Features

- **537 tests passing** (110 SDK compliance + 427 core)
- **17 tools** working via Anthropic tool use protocol
- **Real API** with SSE streaming
- **5-10x faster** than JavaScript version
- **Production security** (zeroized keys, sanitized errors)

## What Was Built

Reverse-engineered Claude Code (421K lines JS) and rebuilt in Rust (14K lines).

Complete implementation:
- All tools (Bash, Read, Write, Edit, Glob, Grep, etc.)
- Interactive mode with REPL
- Hooks system (9 lifecycle events)
- Session checkpointing
- Plugin system
- Settings hierarchy

## Documentation

- **[Contributing Guide](CONTRIBUTING.md)** - How to contribute to RustyClawd
- **[Architecture Guide](docs/ARCHITECTURE.md)** - System design, module structure, and key decisions
- **[Hook Lifecycle Integration](docs/HOOK_LIFECYCLE_INTEGRATION.md)** - Complete hook system documentation
- **[Rust Patterns Learned](RUST_PATTERNS_LEARNED.md)** - Technical patterns and best practices
- **[JS vs Rust Comparison](JS_VS_RUST_COMPARISON.md)** - Performance and design comparisons
