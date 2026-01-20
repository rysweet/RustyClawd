# MCP Tool Search

This document describes the MCPSearch tool and the `auto:N` syntax for configuring MCP (Model Context Protocol) tool search thresholds in RustyClawd.

## Overview

MCPSearch is a built-in tool that dynamically loads MCP tools on-demand rather than preloading all tools upfront. This preserves context window space when you have many MCP servers configured.

By default, MCPSearch activates automatically when MCP tool descriptions exceed 10% of the context window.

## MCPSearch Tool

### What It Does

MCPSearch queries available MCP tools by semantic search, returning only the tools relevant to your current task. Instead of loading hundreds of tool definitions upfront, Claude searches for what it needs when it needs it.

### Tool Definition

```json
{
  "name": "MCPSearch",
  "description": "Search available MCP tools by query. Returns matching tools that can then be invoked.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Search query to find relevant MCP tools"
      }
    },
    "required": ["query"]
  }
}
```

### Example Usage

When MCPSearch is active, Claude uses it to find relevant tools:

```
User: "Create a new GitHub issue for this bug"

Claude invokes: MCPSearch(query: "github issue create")

Returns: github_create_issue, github_list_issues, github_get_issue

Claude then invokes: github_create_issue(...)
```

## Default Behavior

MCPSearch is enabled by default with the `auto:10` threshold:

- **Threshold**: 10% of context window
- **Behavior**: If MCP tool definitions exceed 10% of context, MCPSearch activates automatically
- **Fallback**: If below threshold, all tools load normally

This default ensures optimal context usage without requiring manual configuration.

## Configuration

### Environment Variable

Set the `ENABLE_TOOL_SEARCH` environment variable:

```bash
# Use default 10% threshold (same as auto:10)
ENABLE_TOOL_SEARCH=auto rustyclawd

# Use custom 5% threshold
ENABLE_TOOL_SEARCH=auto:5 rustyclawd

# Always enable tool search
ENABLE_TOOL_SEARCH=true rustyclawd

# Disable tool search (load all tools upfront)
ENABLE_TOOL_SEARCH=false rustyclawd
```

### Settings File

Configure in your settings file (`.claude/settings.json` or `config.toml`):

**JSON format:**
```json
{
  "env_vars": {
    "ENABLE_TOOL_SEARCH": "auto:5"
  }
}
```

**TOML format:**
```toml
[env_vars]
ENABLE_TOOL_SEARCH = "auto:5"
```

## Disabling MCPSearch

To completely disable the MCPSearch tool, add it to your `disallowedTools` list:

**In settings.json:**
```json
{
  "disallowedTools": ["MCPSearch"]
}
```

This prevents MCPSearch from being offered to Claude, forcing all MCP tools to load upfront regardless of context usage.

## Syntax Reference

| Value | Description |
|-------|-------------|
| `auto` | Activates when MCP tools exceed 10% of context (default) |
| `auto:<N>` | Activates at custom threshold, where N is a percentage (0-100) |
| `true` | Always enabled |
| `false` | Disabled - all MCP tools loaded upfront |

### Examples

- `auto` - Default behavior, 10% threshold
- `auto:5` - Tool search activates at 5% context usage
- `auto:15` - Tool search activates at 15% context usage
- `auto:0` - Always use tool search (same as `true`)
- `auto:100` - Never auto-enable (effectively same as manual)

## How It Works

1. **Context Analysis**: RustyClawd calculates the total token cost of all MCP tool definitions
2. **Threshold Check**: If the cost exceeds the configured percentage of the context window, MCPSearch activates
3. **Tool Injection**: The MCPSearch tool is added to Claude's available tools
4. **On-Demand Loading**: Claude searches for relevant tools when needed instead of having all tools preloaded
5. **Token Savings**: Typically reduces MCP-related context usage by 85%

## Benefits

- **Reduced Context Usage**: Only load tools that are actually needed
- **Preserved Context Window**: More space for your actual conversation and code
- **Improved Accuracy**: Better results with large tool libraries
- **Automatic Optimization**: No manual configuration needed for most use cases

## Implementation Details

The `ToolSearchConfig` type handles parsing and validation:

```rust
pub enum ToolSearchConfig {
    /// Disabled - load all tools upfront
    Disabled,
    /// Always enabled
    Enabled,
    /// Auto-enable at threshold (percentage 0-100)
    Auto { threshold_percent: u8 },
}
```

Default threshold is 10% when using `auto` without a specific value.

## See Also

- [MCP Server Configuration](./MCP_SERVE.md)
- [HTTP MCP Transport](./HTTP_MCP_TRANSPORT.md)
