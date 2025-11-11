# Interactive Mode - Quick Start

## TL;DR

```bash
cargo run --bin claude-code -- chat
```

## What You Get

A fully functional REPL for chatting with Claude:
- Real-time streaming responses
- Command history (arrow keys)
- Multi-turn conversations
- Graceful exit (Ctrl+D)

## Commands

- `/help` - Show help
- `/stats` - Session statistics
- `/clear` - Clear history
- `/exit` - Quit

## Implementation

**File**: `/Users/ryan/src/declawed/claude-code-rs/crates/cli/src/interactive.rs` (262 lines)

**Key Features**:
- ✅ Rustyline for input handling
- ✅ Anthropic API streaming integration
- ✅ Context management (up to 1000 messages)
- ✅ Error recovery
- ✅ Ctrl+D / Ctrl+C handling

## Requirements

1. API key in `~/.claude-msec-k`
2. Internet connection
3. Rust toolchain

## Usage Example

```
You> Hello!
Claude> Hello! I'm Claude, an AI assistant...

You> What's 2+2?
Claude> 2+2 equals 4.

You> /stats
📊 Session Statistics:
  Messages: 4
  Memory usage: 782 bytes
  Model: claude-sonnet-4-5-20250929

You> /exit
Goodbye!
```

## Testing

```bash
# Build
cargo build --bin claude-code

# Run
./target/debug/claude-code chat

# Or combined
cargo run --bin claude-code -- chat
```

## Success Criteria: ✅ ALL MET

✅ REPL loop with rustyline
✅ Streaming responses
✅ History management
✅ Multi-turn context
✅ Graceful exit

## Documentation

- Full guide: `crates/cli/INTERACTIVE_MODE.md`
- Examples: `examples/interactive_demo.md`
- Implementation: `INTERACTIVE_MODE_IMPLEMENTATION.md`

---

**Status**: Production ready 🚀
