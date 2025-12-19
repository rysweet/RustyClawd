# MCP Serve Command

## Overview

The `rusty mcp serve` command starts RustyClawd as an MCP (Model Context Protocol) server, exposing all its tools to external MCP clients like Claude Desktop, Cursor, or any other MCP-compatible application.

## Usage

```bash
rusty mcp serve
```

The server runs on stdin/stdout using the JSON-RPC 2.0 protocol, making it compatible with the MCP specification.

## What It Does

When you run `rusty mcp serve`:

1. **Starts MCP Server**: Listens for JSON-RPC requests on stdin
2. **Exposes Tools**: Makes all RustyClawd tools available via MCP protocol
3. **Handles Requests**: Processes MCP requests and returns results on stdout

## Supported MCP Requests

### initialize
Establishes connection and exchanges capabilities.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "1.0",
    "capabilities": {},
    "clientInfo": {
      "name": "client-name",
      "version": "1.0.0"
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "1.0",
    "capabilities": {
      "tools": true
    },
    "serverInfo": {
      "name": "rustyclawd",
      "version": "0.1.0"
    }
  }
}
```

### tools/list
Lists all available tools.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "Bash",
        "description": "Execute bash commands and return output",
        "inputSchema": {
          "type": "object",
          "properties": {
            "command": {
              "type": "string",
              "description": "The bash command to execute"
            }
          },
          "required": ["command"]
        }
      },
      {
        "name": "Read",
        "description": "Read file contents",
        "inputSchema": {
          "type": "object",
          "properties": {
            "file_path": {
              "type": "string",
              "description": "Path to the file to read"
            }
          },
          "required": ["file_path"]
        }
      }
      // ... more tools
    ]
  }
}
```

### tools/call
Executes a tool with given parameters.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "Bash",
    "arguments": {
      "command": "echo 'Hello from MCP!'"
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Hello from MCP!\n"
      }
    ]
  }
}
```

## Available Tools

All RustyClawd tools are exposed via MCP:

- **Bash** - Execute shell commands
- **BashOutput** - Retrieve background shell output
- **KillShell** - Terminate background shells
- **Read** - Read file contents
- **Write** - Write file contents
- **Edit** - Edit files with replacements
- **Glob** - Find files by pattern
- **Grep** - Search file contents
- **AskUserQuestion** - Prompt user for input
- **Skill** - Invoke Claude Code skills
- **SlashCommand** - Execute slash commands
- **Task** - Invoke subagents
- **AgentOutput** - Retrieve agent output
- **TodoWrite** - Manage todo lists

## Integration with External Clients

### Claude Desktop

Add to Claude Desktop's MCP configuration (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "rustyclawd": {
      "command": "rusty",
      "args": ["mcp", "serve"]
    }
  }
}
```

### Cursor

Add to Cursor's MCP configuration:

```json
{
  "mcp": {
    "servers": {
      "rustyclawd": {
        "command": "rusty",
        "args": ["mcp", "serve"]
      }
    }
  }
}
```

### Custom MCP Clients

Any MCP-compatible client can connect by launching:

```bash
rusty mcp serve
```

And communicating via JSON-RPC 2.0 on stdin/stdout.

## Schema Compatibility

All tool input schemas follow MCP requirements:

- Root level has `"type": "object"`
- Properties are properly typed
- Required fields are specified

This ensures compatibility with all MCP clients.

## Error Handling

Errors are returned in JSON-RPC format:

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": {
      "details": "Missing required parameter: command"
    }
  }
}
```

## Philosophy

The `mcp serve` implementation follows ruthless simplicity:

- **Minimal** - Only implements required MCP methods
- **Direct** - Reuses existing tool infrastructure
- **No abstraction** - Straightforward request → tool → response flow
- **Standard** - Follows JSON-RPC 2.0 and MCP specifications exactly

## Exit

The server runs until:
- EOF on stdin (client disconnects)
- Ctrl+C (SIGINT)
- Error condition

It automatically cleans up resources on exit.
