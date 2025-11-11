# Interactive Mode (Chat REPL)

## Overview

RustyClawd's interactive mode provides a full-featured REPL for chatting with Claude. It implements real-time streaming responses, conversation history management, and robust input handling.

## Usage

Start an interactive session:

```bash
cargo run -- chat
```

Or after building:

```bash
./target/release/claude-code chat
```

## Features

### 1. **Real-time Streaming**
- Responses stream character-by-character as Claude generates them
- No waiting for complete responses
- Natural conversation flow

### 2. **Conversation Context**
- Multi-turn conversations maintained automatically
- Context window management prevents unbounded memory growth
- Up to 1000 messages with automatic pruning

### 3. **Input Handling with Rustyline**
- Command history (use arrow keys)
- Line editing capabilities
- Persistent history across sessions
- Ctrl+C to cancel current input
- Ctrl+D to exit gracefully

### 4. **Special Commands**

| Command | Description |
|---------|-------------|
| `/exit` or `/quit` | Exit the chat session |
| `/clear` | Clear conversation history |
| `/stats` | Show session statistics |
| `/help` | Display help message |

### 5. **Graceful Exit**
- Ctrl+D: Clean exit
- Ctrl+C: Cancel current input (continue session)
- `/exit` or `/quit`: Exit via command

## Implementation Details

### Architecture

```
InteractiveSession
├── Client (Anthropic API)
├── Context (Conversation history)
└── DefaultEditor (Rustyline)
```

### Message Flow

1. **User Input** → Rustyline reads input with history
2. **Validation** → Check for special commands
3. **Context Update** → Add user message to history
4. **API Call** → Create streaming request with full context
5. **Stream Response** → Display chunks in real-time
6. **History Update** → Add assistant response to context

### Model Configuration

- Default model: `claude-sonnet-4-5-20250929`
- Max tokens: 4096
- Temperature: Not set (uses API default)
- Top-p: Not set (uses API default)

## Examples

### Basic Conversation

```
You> Hello! What can you help me with?
Claude> Hello! I'm Claude, an AI assistant created by Anthropic...

You> Can you explain Rust ownership?
Claude> Certainly! Rust's ownership system is a unique feature...
```

### Using Commands

```
You> /stats
📊 Session Statistics:
  Messages: 4
  Memory usage: 2847 bytes
  Model: claude-sonnet-4-5-20250929

You> /clear
✓ Conversation history cleared

You> /exit
Goodbye!
```

### Error Handling

```
You> Tell me about yourself
❌ Error: API error: Rate limit exceeded
Please try again or type /exit to quit.

You> /help
📖 Available Commands:
  /exit, /quit  - Exit the chat session
  /clear        - Clear conversation history
  ...
```

## Configuration

### API Key

The API key is loaded from `~/.claude-msec-k` by default. Ensure this file:
- Exists and contains your Anthropic API key (starting with `sk-ant-`)
- Has restricted permissions (600 recommended)

### Environment Variables

None required. All configuration is code-based.

## Technical Notes

### Memory Management

- Context automatically prunes old messages when exceeding 1000 messages
- 100 oldest messages removed when threshold reached
- Prevents unbounded memory growth (improvement over JS version)

### Error Recovery

- API errors don't crash the session
- Network issues are reported gracefully
- User can retry after errors

### Streaming Implementation

Uses Anthropic's Server-Sent Events (SSE) streaming:
- Real-time token delivery
- Minimal latency
- Efficient network usage

### Thread Safety

- Session runs on single tokio runtime
- No concurrent access issues
- Clean async/await throughout

## Limitations

1. **No Multi-line Input**: Currently supports single-line input only
2. **No Tool Use**: Interactive mode doesn't execute tools yet
3. **No System Prompt Customization**: Uses default system behavior
4. **No History Persistence**: History cleared on exit

## Future Enhancements

Potential improvements:

- [ ] Multi-line input support (Ctrl+Enter to send)
- [ ] Persistent history to disk
- [ ] Custom system prompts
- [ ] Tool execution in chat context
- [ ] Model selection command (`/model`)
- [ ] Export conversation command (`/export`)
- [ ] Token usage display
- [ ] Configurable max_tokens
- [ ] Syntax highlighting for code blocks

## Troubleshooting

### "API key not found"

```bash
# Verify API key file exists
test -f ~/.claude-msec-k && echo "Found" || echo "Missing"

# Check file permissions
ls -la ~/.claude-msec-k

# Set correct permissions
chmod 600 ~/.claude-msec-k
```

### "Rate limit exceeded"

Wait a moment and try again. The session remains active.

### "Connection timeout"

Check internet connection. The session handles retries gracefully.

### Rustyline errors

If you see rustyline initialization errors, ensure your terminal is compatible with line editing libraries.

## Comparison with Claude Code (TypeScript)

### Improvements

1. **Memory Windowing**: Automatic context pruning prevents unbounded growth
2. **Type Safety**: Strong typing throughout prevents runtime errors
3. **Secure Key Handling**: Zeroization and secret wrappers
4. **Better Error Messages**: Detailed error reporting
5. **Cleaner Architecture**: Separation of concerns

### Maintained Features

1. **Streaming Responses**: Identical UX to original
2. **Command Support**: Same command set
3. **History Management**: Full conversation context
4. **Graceful Exit**: Ctrl+D support

## Code Quality

### Testing

Currently, integration testing requires manual verification with actual API. Unit tests for message conversion and context management are in place.

### Documentation

All public APIs documented with rustdoc comments. Run:

```bash
cargo doc --open --package claude-code-cli
```

### Security

- API keys never logged or displayed
- Secrets zeroized on drop
- File permissions validated
- Error messages sanitized

## Performance

- Minimal memory overhead
- Efficient streaming (no buffering delays)
- Fast startup time
- Low CPU usage during idle

## Dependencies

- `rustyline`: 13.0 - Line editing and history
- `claude-code-core`: Workspace - API client
- `anyhow`: Workspace - Error handling
- `futures`: Workspace - Stream utilities
- `tokio`: Workspace - Async runtime
