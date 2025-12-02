# MCP Prompts - User Guide

## Overview

RustyClawd supports the Model Context Protocol (MCP) prompts capability, allowing you to discover and use prompt templates from MCP servers. Prompts provide pre-defined, parameterized templates for common LLM interactions.

## What are MCP Prompts?

MCP prompts are reusable prompt templates exposed by MCP servers. Each prompt:
- Has a unique name and description
- May accept arguments for customization
- Returns formatted messages ready for LLM consumption
- Enables consistent, tested prompts across your workflow

## Available Commands

### List Prompts from a Server

**CLI:**
```bash
claude mcp prompts <server-id>
```

**TUI:**
```
/mcp-prompts <server-id>
```

**Example Output:**
```
Prompts from server 'my-server':
------------------------------------------------------------

  code_review
    Asks the LLM to analyze code quality and suggest improvements
    Arguments:
      - code (required): The code to review
      - language (optional): Programming language

  summarize_text
    Generates a concise summary of provided text
    Arguments:
      - text (required): Text to summarize
      - max_length (optional): Maximum summary length

------------------------------------------------------------

Total: 2 prompt(s)
```

### Get Status of All Servers

Use the `mcp list` command to see which servers are running and how many prompts they provide:

```bash
claude mcp list
```

```
MCP Servers:
------------------------------------------------------------
  my-server - RUNNING [5 tool(s), 2 prompt(s)]
  other-server - STOPPED [-]
------------------------------------------------------------
```

## Usage Workflow

### 1. Start an MCP Server

```bash
claude mcp start my-server
```

### 2. List Available Prompts

```bash
claude mcp prompts my-server
```

### 3. Use a Prompt

Once you see the available prompts and their required arguments, you can reference them in your conversations with Claude. The MCP server will handle argument substitution and return formatted prompt messages.

## Prompt Arguments

Prompts can accept two types of arguments:

**Required Arguments:**
- Must be provided when using the prompt
- Marked as `(required)` in the prompts list
- Example: `code` in the `code_review` prompt

**Optional Arguments:**
- Can be provided for customization
- Have default values if not specified
- Marked as `(optional)` in the prompts list
- Example: `language` or `max_length`

## Error Messages

### Server Not Found
```
Error: Server 'unknown-server' not found. Use 'mcp list' to see available servers.
```
**Solution:** Check available servers with `claude mcp list`

### Server Not Running
```
Error: Server 'my-server' is not running. Start it with: mcp start my-server
```
**Solution:** Start the server before listing prompts

### No Prompts Available
```
Server 'my-server' has no prompts available
```
**Meaning:** The server doesn't expose any prompt templates (not an error)

## Under the Hood

When you list prompts, RustyClawd:
1. Sends a `prompts/list` JSON-RPC request to the MCP server
2. Receives the list of available prompts with metadata
3. Formats and displays the results

Prompts are discovered automatically when you start an MCP server and are cached for performance.

## Comparison with Tools

| Feature | Tools | Prompts |
|---------|-------|---------|
| **Purpose** | Execute actions | Provide prompt templates |
| **Arguments** | JSON parameters | Template variables |
| **Returns** | Action result | Formatted LLM messages |
| **Discovery** | `mcp tools <server-id>` | `mcp prompts <server-id>` |
| **Usage** | Claude calls tools | User selects prompts |

## Example: Code Review Workflow

1. **Start server with prompts:**
   ```bash
   claude mcp start code-helper
   ```

2. **Discover available prompts:**
   ```bash
   claude mcp prompts code-helper
   ```

   Output shows `code_review` prompt requiring `code` argument.

3. **Use the prompt in conversation:**
   Reference the `code_review` prompt when talking to Claude, providing the required code snippet.

## Technical Details

### Prompt Structure

Each prompt returned by `prompts/list` includes:
- `name`: Unique identifier
- `description`: Human-readable explanation
- `arguments`: List of parameters (name, description, required flag)

### Prompt Messages

When retrieving a prompt via `prompts/get`, the response includes:
- `description`: Optional detailed description
- `messages`: Array of message objects with:
  - `role`: Message role (user, assistant, system)
  - `content`: Message content (text, images, etc.)

### Server Capabilities

Not all MCP servers support prompts. Check the server's capabilities:
- During initialization, RustyClawd discovers which capabilities each server provides
- Servers without prompts capability will return an empty list

## Troubleshooting

### Prompts Not Appearing

**Check server capabilities:**
```bash
claude mcp status my-server
```

If the server is running but shows no prompts, the server may not implement the prompts capability.

### Outdated Prompt List

Restart the server to refresh:
```bash
claude mcp stop my-server
claude mcp start my-server
claude mcp prompts my-server
```

## For MCP Server Developers

To expose prompts from your MCP server:

1. **Implement `prompts/list` endpoint:**
   Return an array of prompt definitions with names, descriptions, and arguments.

2. **Implement `prompts/get` endpoint:**
   Accept prompt name and arguments, return formatted messages.

3. **Declare prompts capability:**
   Include `"prompts": true` in your initialization response capabilities.

See the [MCP Specification](https://spec.modelcontextprotocol.io/specification/2024-11-05/server/prompts/) for detailed implementation guidance.

## See Also

- [MCP Tools Documentation](./MCP_TOOLS.md)
- [MCP Server Management](./MCP_SERVERS.md)
- [Model Context Protocol Specification](https://modelcontextprotocol.io/)
