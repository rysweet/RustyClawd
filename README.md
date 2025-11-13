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

# New spec-compliant features
rusty --verbose -p "debug this"                    # Verbose logging
rusty --system-prompt-file ./prompt.txt "query"    # Custom system prompt
rusty --add-dir ./src --add-dir ./tests "analyze"  # Multiple directories
rusty --allowedTools Read --allowedTools Grep "search only"  # Tool control
rusty update                                       # Update CLI
rusty mcp                                          # Configure MCP servers
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

See `RUST_PATTERNS_LEARNED.md` and `JS_VS_RUST_COMPARISON.md` for technical deep-dive.
