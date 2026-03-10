# Using RustyClawd with the Claude Agent SDK

RustyClawd can be used as a drop-in replacement for Claude Code when building
applications with the Claude Agent SDK. Point the SDK at the RustyClawd binary
using the `cli_path` option.

## Quick Start

### Python

```python
import asyncio
from claude_agent_sdk import query, ClaudeAgentOptions

async def main():
    async for message in query(
        prompt="Find all TODO comments and create a summary",
        options=ClaudeAgentOptions(
            cli_path="path/to/rusty",  # Path to RustyClawd binary
            allowed_tools=["Read", "Glob", "Grep"],
        ),
    ):
        if hasattr(message, "result") and message.result:
            print(message.result)

asyncio.run(main())
```

### TypeScript

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

for await (const message of query({
  prompt: "Find all TODO comments and create a summary",
  options: {
    cliPath: "path/to/rusty",  // Path to RustyClawd binary
    allowedTools: ["Read", "Glob", "Grep"],
  }
})) {
  if ("result" in message) console.log(message.result);
}
```

## Prerequisites

1. Build RustyClawd: `cargo build --release`
2. Set your API key: `export ANTHROPIC_API_KEY=sk-ant-...`
3. Install the SDK: `pip install claude-agent-sdk` or `npm install @anthropic-ai/claude-agent-sdk`

## Supported Features

| Feature | Status |
|---|---|
| Text prompts | Supported |
| Tool use (Read, Write, Edit, Bash, Glob, Grep, etc.) | Supported |
| Session management (resume, continue) | Supported |
| System prompt override | Supported |
| Tool restrictions (allowed/disallowed) | Supported |
| Permission modes (ask, plan, auto-accept) | Supported |
| Model selection with aliases | Supported |
| Per-turn streaming | Supported |
| Session ID tracking | Supported |
| parent_tool_use_id (subagent correlation) | Supported |
| MCP servers | Supported (via `--mcp-config` flag) |
| Hooks via SDK callbacks | Partial (SDK hook configs accepted, callback protocol pending) |

## Performance

| Metric | RustyClawd | Claude Code | Advantage |
|---|---|---|---|
| Startup time | 6-7ms | 300ms | 45x faster |
| Memory usage | 9 MB | 275 MB | 30x less |
| API latency | Same | Same | Both use Anthropic API |

The startup and memory advantages compound when running multiple parallel agent
sessions, which is common in SDK applications.

## Protocol

RustyClawd implements the same bidirectional JSON protocol that the SDK uses
to communicate with Claude Code:

1. SDK spawns the binary with `--output-format stream-json --input-format stream-json`
2. SDK sends `initialize` control request via stdin
3. Binary responds with `control_response` containing `session_id`
4. SDK sends user message via stdin
5. Binary streams responses: `system/init`, `assistant` (per turn), `result`

## Running the Compatibility Tests

```bash
# Build first
cargo build --release

# Run format comparison test (no SDK needed)
ANTHROPIC_API_KEY=sk-ant-... python3 tests/sdk_compatibility/test_sdk_compat.py --rusty-only

# Run real SDK integration test (requires claude-agent-sdk)
pip install claude-agent-sdk
ANTHROPIC_API_KEY=sk-ant-... python3 tests/sdk_compatibility/test_sdk_real.py
```

## Differences from Claude Code

- RustyClawd uses API key auth (`ANTHROPIC_API_KEY`). Claude Code also supports OAuth.
- RustyClawd binary is named `rusty`, not `claude`. Use `cli_path` to point the SDK at it.
- SDK hook configs are accepted during initialize but the bidirectional callback
  protocol is not yet implemented. Hooks still work via `.claude/hooks.json` config.
- Some newer Claude Code features (Plan mode, auto-memory UI) are not yet implemented.

## Troubleshooting

**SDK times out on initialize**: Ensure RustyClawd is built with the latest code
that includes the bidirectional protocol (`--input-format stream-json` support).

**Tracing output in JSON stream**: RustyClawd redirects tracing to stderr when
`--output-format stream-json` is active. If you see log lines in the JSON stream,
rebuild with the latest code.

**Missing `--setting-sources` flag**: Older builds don't recognize this flag.
The SDK passes it by default. Update to the latest build.
