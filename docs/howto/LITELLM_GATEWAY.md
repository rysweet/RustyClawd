# Route Anthropic Requests Through LiteLLM

RustyClawd can route print and interactive Anthropic requests through a LiteLLM
proxy that exposes an Anthropic-compatible Messages API.

**Last updated:** 2026-09-02

## Prerequisites

- RustyClawd installed as `rusty`
- A running LiteLLM proxy with an Anthropic-compatible `/v1/messages` route
- A model alias configured by the proxy
- A gateway token, if the proxy requires authentication

## Configure the gateway

Set the Anthropic-compatible base URL, gateway credential, and model alias:

```bash
export ANTHROPIC_BASE_URL="http://127.0.0.1:4000"
export ANTHROPIC_AUTH_TOKEN="synthetic-example-gateway-token"
export ANTHROPIC_MODEL="litellm-anthropic-route"
```

`ANTHROPIC_AUTH_TOKEN` accepts the opaque credential issued by the
gateway. It does not require an Anthropic API key format. The example value is
deliberately synthetic; replace it only in your local environment and never
commit the real value.

Do not append `/v1/messages` to `ANTHROPIC_BASE_URL`. RustyClawd constructs:

```text
http://127.0.0.1:4000/v1/messages
```

Trailing slashes are safe, so `http://127.0.0.1:4000/` produces the
same request URL.

## Send a one-shot request

Select the Anthropic backend explicitly and run print mode:

```bash
rusty --provider anthropic -p "Return only the word connected."
```

RustyClawd sends the gateway token through its existing Anthropic
authentication header and sends `litellm-anthropic-route` as the
request model.

## Start an interactive session

The same environment configuration applies to interactive mode:

```bash
rusty --provider anthropic
```

Print and interactive modes resolve credentials, endpoints, and models
through one shared Anthropic configuration resolver. Both modes pass the
resolver's selected credential, normalized base URL, and model into their
request path.

## Override the model for one invocation

Use `--model` without changing the environment default:

```bash
rusty --provider anthropic \
  --model litellm-analysis-route \
  -p "Review the files changed in this branch."
```

The CLI model overrides `ANTHROPIC_MODEL`. A model in a RustyClawd
settings file also overrides `ANTHROPIC_MODEL`, while `--model` has
the highest precedence.

## Use an Anthropic API key instead

Existing `ANTHROPIC_API_KEY` configuration remains supported. Remove the
auth token before testing API-key fallback:

```bash
unset ANTHROPIC_AUTH_TOKEN
export ANTHROPIC_API_KEY="sk-ant-synthetic-example-api-key"
rusty --provider anthropic -p "Return only the word connected."
```

When both variables contain values, `ANTHROPIC_AUTH_TOKEN` takes
precedence.

## Keep another provider selected

Anthropic environment variables do not force provider selection. An
explicit non-Anthropic provider continues to use its own endpoint,
credential, and model:

```bash
export ANTHROPIC_MODEL="litellm-anthropic-route"
rusty --provider copilot --model gpt-4o -p "Return only the word connected."
```

This command uses Copilot and ignores `ANTHROPIC_MODEL`.

For all environment variables, precedence rules, and the gateway request
contract, see the
[Anthropic backend configuration reference](../reference/ANTHROPIC_CONFIGURATION.md).
