# MCP `list_changed` Notification Support

## Overview

RustyClawd now supports MCP `list_changed` notifications, allowing MCP servers to dynamically update their available tools, prompts, and resources without requiring client reconnection.

This implementation follows the [Model Context Protocol specification (2025-11-25)](https://modelcontextprotocol.io/specification/2025-11-25) for notification handling.

## Supported Notifications

### 1. `notifications/tools/list_changed`
Sent by MCP servers when their list of available tools changes.

**Notification Format:**
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/tools/list_changed",
  "params": {}
}
```

### 2. `notifications/resources/list_changed`
Sent by MCP servers when their list of available resources changes.

**Notification Format:**
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/list_changed",
  "params": {}
}
```

### 3. `notifications/prompts/list_changed`
Sent by MCP servers when their list of available prompts changes.

**Notification Format:**
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/prompts/list_changed",
  "params": {}
}
```

## API

### Notification Types

```rust
use rustyclawd::plugins::{McpNotification, McpNotificationType};

// Parse notification type from method string
let notif_type = McpNotificationType::from_method("notifications/tools/list_changed");
assert_eq!(notif_type, McpNotificationType::ToolsListChanged);

// Convert back to method string
assert_eq!(notif_type.to_method(), "notifications/tools/list_changed");
```

### Parsing Notifications

```rust
use rustyclawd::plugins::McpNotification;

let json = r#"{
    "jsonrpc": "2.0",
    "method": "notifications/tools/list_changed",
    "params": {}
}"#;

let notification: McpNotification = serde_json::from_str(json)?;
```

### Manual Registry Refresh

You can manually refresh all registries (tools, resources, prompts) for a server:

```rust
use rustyclawd::plugins::McpProxy;

let mut proxy = McpProxy::new();

// Start a server
proxy.start_server("my-server").await?;

// Manually refresh all registries
proxy.refresh_server_registries("my-server").await?;

// Or refresh individual registries
proxy.refresh_tools("my-server").await?;
proxy.refresh_resources("my-server").await?;
proxy.refresh_prompts("my-server").await?;
```

## Implementation Details

### Thread-Safe Registry Updates

All registry refresh operations are thread-safe and can be called concurrently from different tasks. The `McpProxy` internally handles synchronization.

### Notification Handling Flow

When a `list_changed` notification is received:

1. **Parse** the notification JSON to determine the type (tools/resources/prompts)
2. **Dispatch** to the appropriate handler based on notification type
3. **Refresh** the affected registry by calling the corresponding `list` method on the MCP server
4. **Update** the local cache with the new list

### Connection Types

- **Stdio Connections**: Notifications are read from the server's stdout stream alongside responses
- **HTTP Connections**: Notifications would use server-sent events or long-polling (future enhancement)

## Testing

Comprehensive tests are provided in `crates/cli/tests/mcp_notification_tests.rs`:

```bash
# Run notification tests
cd crates/cli
cargo test --test mcp_notification_tests

# Run all tests
cargo test
```

Test coverage includes:
- Notification type parsing
- JSON notification deserialization
- Registry refresh operations
- Mock MCP server integration

## Future Enhancements

### Background Notification Listener

Currently, notifications are handled through manual refresh calls. A future enhancement could add:

- Background task that continuously monitors stdout for notifications
- Automatic registry updates when notifications are received
- Event callbacks to notify application code of registry changes

### HTTP Transport Notifications

For HTTP-based MCP servers, notification delivery could be implemented using:
- Server-Sent Events (SSE)
- WebSocket connections
- Long-polling mechanisms

## References

-  [Model Context Protocol Specification (2025-11-25)](https://modelcontextprotocol.io/specification/2025-11-25)
- [MCP Notification Documentation](https://modelcontextprotocol.io/specification/2025-11-25#notifications)
- Issue #249: Support for MCP `list_changed` notifications
