# Anthropic Backend Configuration Reference

RustyClawd supports Anthropic API keys and Anthropic-compatible gateways
through one configuration contract shared by print and interactive modes.

**Last updated:** 2026-09-02

## Environment variables

| Variable | Purpose | Scope |
|----------|---------|-------|
| `ANTHROPIC_AUTH_TOKEN` | Preferred opaque credential for Anthropic-compatible services | Anthropic backend only |
| `ANTHROPIC_API_KEY` | Anthropic API key and backward-compatible credential | Anthropic backend only |
| `ANTHROPIC_BASE_URL` | Base URL of an Anthropic-compatible API | Anthropic backend only |
| `ANTHROPIC_MODEL` | Default model when no CLI or settings model is configured | Anthropic backend only |

Missing, empty, and whitespace-only values are treated as unset.
RustyClawd does not print credential values in logs or errors.

These variables configure the Anthropic backend. They do not select it. Use
`--provider anthropic` when provider selection must be explicit:

```bash
rusty --provider anthropic -p "Summarize the current directory."
```

## Credential precedence

RustyClawd selects the first non-empty credential in this order:

1. `ANTHROPIC_AUTH_TOKEN`
2. `ANTHROPIC_API_KEY`
3. `ANTHROPIC_API_KEY` in the current directory's `.env` file
4. The legacy `~/.claude-msec-k` credential file

`ANTHROPIC_AUTH_TOKEN` is opaque. It can contain a gateway-issued token
and does not need an `sk-ant-` prefix. `ANTHROPIC_API_KEY` and credentials
loaded from existing files retain their existing Anthropic API key
validation.

The selected credential is sent through the standard Anthropic
authentication path in the `x-api-key` request header. RustyClawd does not
introduce a gateway-specific authentication scheme.

When both environment variables are set, `ANTHROPIC_AUTH_TOKEN` wins:

```bash
export ANTHROPIC_AUTH_TOKEN="synthetic-example-gateway-token"
export ANTHROPIC_API_KEY="sk-ant-synthetic-example-api-key"
rusty --provider anthropic -p "Report the active model name."
```

The values above are synthetic examples. Replace them in your environment; do
not store credentials in source control.

## Endpoint resolution

For the Anthropic backend, the API base URL is resolved in this order:

1. An explicit API URL from RustyClawd's settings hierarchy
2. A non-empty `ANTHROPIC_BASE_URL`
3. `https://api.anthropic.com`

`ANTHROPIC_BASE_URL` is a base URL, not a complete messages endpoint.
RustyClawd removes trailing slashes and sends message requests to exactly:

```text
<base-url>/v1/messages
```

For example, both of these values produce
`http://127.0.0.1:4000/v1/messages`:

```bash
export ANTHROPIC_BASE_URL="http://127.0.0.1:4000"
export ANTHROPIC_BASE_URL="http://127.0.0.1:4000/"
```

The setting name for a file-based API URL is `api_url`. A JSON settings file
can specify it as follows:

```json
{
  "api_url": "http://127.0.0.1:4000"
}
```

Pass a specific settings file with `--settings`:

```bash
rusty --provider anthropic \
  --settings ./rustyclawd-settings.json \
  -p "Summarize README.md."
```

The endpoint applies only to Anthropic requests. It does not change
Copilot, Azure, or another explicitly selected provider.

## Model precedence

For the Anthropic backend, RustyClawd selects the model in this order:

1. `--model <MODEL_ID>`
2. The `model` value resolved from configuration files and settings
3. A non-empty `ANTHROPIC_MODEL`
4. The mode's existing built-in Anthropic default:
   `claude-sonnet-4-6` in print mode and `claude-opus-4-6` interactively

Print mode and interactive mode use the same precedence while preserving their
distinct defaults.

Example configuration file:

```json
{
  "model": "team-anthropic-route"
}
```

The CLI always overrides the settings file and environment:

```bash
export ANTHROPIC_MODEL="environment-route"
rusty --provider anthropic \
  --settings ./rustyclawd-settings.json \
  --model one-request-route \
  -p "Explain the model selection order."
```

This request uses `one-request-route`.

`ANTHROPIC_MODEL` is ignored when the selected provider is Copilot,
Azure, or another non-Anthropic backend:

```bash
export ANTHROPIC_MODEL="anthropic-only-route"
rusty --provider copilot --model gpt-4o -p "Hello"
```

This request uses the Copilot model `gpt-4o`.

## Shared resolver implementation contract

The implementation uses one shared Anthropic configuration resolver for
both print and interactive modes. The resolver produces the complete
Anthropic request configuration:

- selected credential
- normalized API base URL
- selected model

Both request paths consume that resolved configuration rather than
independently reading Anthropic environment variables or applying separate
defaults.

## Provider selection and fallback

Environment configuration never overrides `--provider`.

- `--provider anthropic` requires a usable Anthropic credential and does not
  fall back to another provider.
- `--provider copilot` uses Copilot configuration even when any
  `ANTHROPIC_*` variables are set.
- `--provider azure` uses Azure configuration even when any `ANTHROPIC_*`
  variables are set.
- Without `--provider`, RustyClawd preserves its existing default-provider
  behavior: it tries Anthropic first and permits the existing Copilot fallback when
  no Anthropic credential is available.
- `ANTHROPIC_BASE_URL` or `ANTHROPIC_MODEL` alone does not count as an
  Anthropic credential and does not suppress that fallback.

## Request contract

An Anthropic-compatible gateway receives the same request contract as the
Anthropic backend:

| Property | Value |
|----------|-------|
| Method | `POST` |
| Path | `/v1/messages` |
| Authentication | Selected credential in `x-api-key` |
| API version | `anthropic-version` header |
| Content type | `application/json` |
| Request body | Anthropic Messages API format |
| Streaming | Anthropic-compatible server-sent events |

The gateway must accept Anthropic Messages API requests and return
compatible message or streaming responses.

See [Route Anthropic requests through LiteLLM](../howto/LITELLM_GATEWAY.md) for
a complete gateway setup.
