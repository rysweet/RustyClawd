# MCP Tool Search Auto-Configuration

This document describes the `auto:N` syntax for configuring MCP (Model Context Protocol) tool search thresholds in RustyClawd.

## Overview

MCP Tool Search dynamically loads MCP tools on-demand rather than preloading all tools upfront. This preserves context window space when you have many MCP servers configured.

The `auto:N` syntax allows you to configure when tool search automatically activates based on context usage.

## Configuration

### Environment Variable

Set the `ENABLE_TOOL_SEARCH` environment variable:

```bash
# Use default 10% threshold
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
2. **Threshold Check**: If the cost exceeds the configured percentage of the context window, tool search is enabled
3. **On-Demand Loading**: Instead of preloading all tools, Claude searches for relevant tools when needed
4. **Token Savings**: Typically reduces MCP-related context usage by 85%

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
