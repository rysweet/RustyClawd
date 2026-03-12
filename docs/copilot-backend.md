# Using RustyClawd with GitHub Copilot LM API

RustyClawd supports two API backends: the **Anthropic Messages API** (default) and the **GitHub Copilot LM API**. The Copilot backend lets you use any model available through your GitHub Copilot subscription — including Claude, GPT, and Gemini models — using your existing GitHub authentication.

## Prerequisites

1. A GitHub account with [GitHub Copilot](https://github.com/features/copilot) access
2. The [GitHub CLI](https://cli.github.com/) (`gh`) installed and authenticated with the `copilot` scope:
   ```bash
   gh auth login
   gh auth refresh --hostname github.com --scopes copilot
   ```
3. Verify your auth has the `copilot` scope:
   ```bash
   gh auth status
   # Should show: Token scopes: '...copilot...'
   ```

The `copilot` scope is required — `gh auth login` alone does not include it.

## Quick Start

```bash
# List available models
rusty --provider copilot --list-models

# Run a prompt with Claude Sonnet 4.6 via Copilot
rusty --provider copilot --model claude-sonnet-4.6 -p "Hello world"

# Interactive mode
rusty --provider copilot --model claude-sonnet-4.6
```

## CLI Flags

| Flag | Description |
|------|-------------|
| `--provider <PROVIDER>` | API backend: `anthropic` (default) or `copilot` |
| `--list-models` | List available models for the selected provider and exit |
| `--model <MODEL_ID>` | Model to use (see `--list-models` for valid IDs) |

The `--provider` flag accepts these aliases (case-insensitive):
- Anthropic: `anthropic`, `claude`
- Copilot: `copilot`, `github`, `gh`

## Authentication

The Copilot backend obtains a GitHub token through this priority chain:

1. `GITHUB_TOKEN` environment variable
2. `gh auth token` CLI command
3. `~/.config/github-copilot/hosts.json` config file

The token is validated eagerly at startup by hitting the Copilot models endpoint. If validation fails, RustyClawd errors immediately — it does not fall back to GitHub Models or any other hidden backend.

## Using with the Claude Agent SDK

The Copilot backend works with the Claude Agent SDK's subprocess protocol (`stream-json`). Create a wrapper script that passes `--provider copilot`:

```bash
#!/bin/bash
# Save as ~/bin/rusty-copilot and chmod +x
exec /path/to/rusty --provider copilot "$@"
```

Then point the SDK at the wrapper:

```python
import asyncio
from claude_code_sdk import query, ClaudeCodeOptions

async def main():
    async for event in query(
        prompt="List the files in the current directory",
        options=ClaudeCodeOptions(
            cli_path="~/bin/rusty-copilot",
            model="claude-sonnet-4.6",
        ),
    ):
        print(event)

asyncio.run(main())
```

### SDK Integration Details

- **Protocol**: RustyClawd supports `--input-format stream-json` and `--output-format stream-json` for the SDK's bidirectional protocol
- **Tool use**: The full tool execution loop works through Copilot — tools are called, results sent back, and the model continues the conversation
- **Streaming**: SSE streaming is translated from OpenAI format to Anthropic format transparently
- **Hooks**: SDK hook callbacks (bidirectional protocol) work identically across both backends

## Available Models

Run `--provider copilot --list-models` to see your available models. Common ones include:

| Model ID | Vendor | Type |
|----------|--------|------|
| `claude-opus-4.6` | Anthropic | Powerful |
| `claude-sonnet-4.6` | Anthropic | Versatile |
| `claude-sonnet-4.5` | Anthropic | Versatile |
| `claude-haiku-4.5` | Anthropic | Lightweight |
| `gpt-4o` | OpenAI | Versatile |
| `gpt-5.1` | OpenAI | Versatile |
| `gemini-2.5-pro` | Google | Powerful |

Model availability depends on your GitHub Copilot subscription and organization policies. Some models may require explicit opt-in at [github.com/settings/copilot/features](https://github.com/settings/copilot/features).

## Architecture

The Copilot backend translates between RustyClawd's internal Anthropic-native types and the OpenAI-compatible Copilot Chat API:

```
CreateMessageRequest (Anthropic format)
        │
        ▼
    to_oai_request()  ──→  POST api.githubcopilot.com/chat/completions
        │                           │
        │                    OAI ChatResponse
        │                           │
        ▼                           ▼
    from_oai_response()  ←──  JSON parsing
        │
        ▼
MessageResponse (Anthropic format)
```

The tool execution loop (`execute_with_tools`) is unchanged — it works identically on both backends because the translation happens at the HTTP layer.

## Differences from Anthropic Backend

| Feature | Anthropic | Copilot |
|---------|-----------|---------|
| Auth | `ANTHROPIC_API_KEY` | `gh auth` token (with `copilot` scope) |
| API format | Anthropic Messages | OpenAI Chat Completions |
| Fast mode (`--speed fast`) | Supported (Opus 4.6) | Not applicable |
| Extended thinking | Supported | Not available via OpenAI format |
| Model aliases (`sonnet`, `opus`) | Supported | Use full model IDs from `--list-models` |
| Retry-After headers | Supported | Supported |

## Troubleshooting

**"GitHub token not found"**
- Run `gh auth login` then `gh auth refresh --hostname github.com --scopes copilot`

**"Copilot credential validation failed (HTTP 401)"**
- Your token may have expired. Run `gh auth refresh --hostname github.com --scopes copilot`

**"Copilot credential validation failed (HTTP 404)"**
- Your token is missing the `copilot` scope. Run `gh auth refresh --hostname github.com --scopes copilot`

**"The requested model is not supported"**
- Check available models with `--provider copilot --list-models`
- Use the exact model ID from the list

**"Access to this endpoint is forbidden"**
- The model may not be enabled for your account. Check your [Copilot model settings](https://github.com/settings/copilot/features) to enable specific models.
