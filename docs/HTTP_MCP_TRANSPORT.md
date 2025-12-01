# HTTP Transport for MCP Servers

This document describes the HTTP transport support for MCP (Model Context Protocol) servers in RustyClawd.

## Overview

RustyClawd now supports two transport mechanisms for MCP servers:

1. **Stdio Transport** (default) - Communicates with MCP servers via standard input/output
2. **HTTP Transport** (new) - Communicates with MCP servers via HTTP/HTTPS endpoints

## Configuration

### Stdio Transport (Backward Compatible)

Old format (still supported):
```json
{
  "mcpServers": [
    {
      "id": "my-server",
      "name": "My Server",
      "command": "node",
      "args": ["server.js"],
      "env": {
        "API_KEY": "secret"
      },
      "description": "A stdio MCP server"
    }
  ]
}
```

New format:
```json
{
  "mcpServers": [
    {
      "id": "my-server",
      "name": "My Server",
      "type": "stdio",
      "command": "node",
      "args": ["server.js"],
      "env": {
        "API_KEY": "secret"
      },
      "description": "A stdio MCP server"
    }
  ]
}
```

### HTTP Transport

Basic HTTP configuration:
```json
{
  "mcpServers": [
    {
      "id": "http-server",
      "name": "HTTP MCP Server",
      "type": "http",
      "url": "http://localhost:3000/mcp",
      "description": "An HTTP MCP server"
    }
  ]
}
```

HTTP with authentication headers:
```json
{
  "mcpServers": [
    {
      "id": "secure-http-server",
      "name": "Secure HTTP MCP Server",
      "type": "http",
      "url": "https://api.example.com/mcp",
      "headers": {
        "Authorization": "Bearer your-token-here",
        "X-Custom-Header": "custom-value"
      },
      "description": "An authenticated HTTP MCP server"
    }
  ]
}
```

## Protocol

HTTP MCP servers must:

1. Accept POST requests with JSON-RPC 2.0 payloads
2. Return JSON-RPC 2.0 responses
3. Support these methods:
   - `initialize` - Server initialization
   - `tools/list` - List available tools
   - `tools/call` - Execute a tool

### Request Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "tool_name",
    "arguments": {
      "param1": "value1"
    }
  }
}
```

### Response Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Result from tool"
      }
    ]
  }
}
```

## Examples

### Example 1: Local HTTP Server

```json
{
  "mcpServers": [
    {
      "id": "local-api",
      "name": "Local API Server",
      "type": "http",
      "url": "http://localhost:8080/mcp"
    }
  ]
}
```

### Example 2: Remote HTTPS Server with Auth

```json
{
  "mcpServers": [
    {
      "id": "cloud-api",
      "name": "Cloud API Server",
      "type": "http",
      "url": "https://api.company.com/mcp/v1",
      "headers": {
        "Authorization": "Bearer ${MCP_API_TOKEN}",
        "X-API-Version": "1.0"
      },
      "description": "Production MCP server in the cloud"
    }
  ]
}
```

### Example 3: Mixed Transport Configuration

```json
{
  "mcpServers": [
    {
      "id": "local-stdio",
      "name": "Local Stdio Server",
      "command": "python",
      "args": ["-m", "local_mcp"],
      "description": "Local development server"
    },
    {
      "id": "remote-http",
      "name": "Remote HTTP Server",
      "type": "http",
      "url": "https://mcp.example.com/api",
      "headers": {
        "Authorization": "Bearer production-key"
      },
      "description": "Production HTTP server"
    }
  ]
}
```

## Error Handling

HTTP transport includes comprehensive error handling:

- **Connection Failures**: Graceful degradation when server is unreachable
- **HTTP Errors**: Clear error messages for non-2xx status codes
- **Timeout Support**: Configurable request timeouts
- **Invalid Responses**: Proper parsing error messages

## Security Considerations

1. **HTTPS Recommended**: Always use HTTPS for production deployments
2. **Header Security**: Never commit API keys to version control
3. **Token Management**: Use environment variables for sensitive credentials
4. **Network Isolation**: Consider VPN or private networks for sensitive data

## Migration Guide

### Migrating from Stdio to HTTP

1. Update your MCP server to expose an HTTP endpoint
2. Update your `plugin.json` configuration:
   ```diff
   - "command": "node",
   - "args": ["server.js"]
   + "type": "http",
   + "url": "http://localhost:3000/mcp"
   ```
3. Test the connection with RustyClawd
4. Update any authentication mechanisms

## Troubleshooting

### Server Not Responding

```
Error: HTTP request failed: connection refused
```

**Solution**: Ensure the MCP server is running and accessible at the specified URL.

### Authentication Errors

```
Error: HTTP error: 401 Unauthorized
```

**Solution**: Check that your Authorization header is correct and the token is valid.

### Invalid Response Format

```
Error: Failed to parse response: invalid JSON
```

**Solution**: Ensure your HTTP server returns valid JSON-RPC 2.0 responses.

## Testing

The HTTP transport includes comprehensive test coverage:

- Unit tests for transport configuration
- Integration tests with mock HTTP servers
- Backward compatibility tests
- Error handling tests

Run tests with:
```bash
cargo test --package rustyclawd-cli --lib plugins::mcp_proxy
```

## Future Enhancements

Potential future additions:

- Server-Sent Events (SSE) support for streaming responses
- WebSocket transport for bidirectional communication
- Connection pooling for improved performance
- Automatic retry with exponential backoff
- Request/response compression

## See Also

- [MCP Specification](https://github.com/modelcontextprotocol/specification)
- [Plugin Development Guide](../PLUGIN_DEVELOPMENT.md)
- [Stdio Transport Documentation](../STDIO_MCP_TRANSPORT.md)
