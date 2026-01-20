# Module Specification: MCP Notification Handler

## Purpose

Handle MCP JSON-RPC 2.0 notifications sent by MCP servers to dynamically update tool, prompt, and resource registries without requiring reconnection.

## Contract

### Inputs

- **MCP Notification (JSON-RPC 2.0)**
  ```json
  {
    "jsonrpc": "2.0",
    "method": "tools/list_changed",
    "params": {}
  }
  ```
  - `jsonrpc`: Always "2.0"
  - `method`: Notification type (no `id` field)
  - `params`: Notification-specific parameters (typically empty)

- **Connection Stream**: Active stdio/HTTP connection to MCP server
- **Capability Flags**: Which capabilities (tools/resources/prompts) server supports

### Outputs

- **Updated Registries**: Tool/prompt/resource lists refreshed in McpServerInstance
- **Tracing Events**: Debug/info logs of notification handling
- **Connection Status**: Indicate if connection remains viable

### Side Effects

- **State Mutation**: Updates `McpServerInstance.tools`, `.resources`, `.prompts`
- **Background Task**: Spawns async listener task on server start
- **Task Cleanup**: Aborts listener task on server stop
- **Network I/O**: Sends `tools/list`, `resources/list`, `prompts/list` requests to refresh data

## Dependencies

### Internal

- `crates/cli/src/plugins/mcp_proxy.rs`: McpProxy, McpServerInstance, connection types
- `crates/cli/src/plugins/manifest.rs`: Server definitions
- Tokio (async runtime)
- Serde (JSON serialization)
- Tracing (logging)

### External

- MCP Protocol Spec v2025-11
- JSON-RPC 2.0 Specification

## Implementation Notes

### Key Design Decisions

1. **Implicit Subscription**: No explicit subscribe needed; listener starts after `initialize`
2. **Background Task**: Separate async task reads notifications while request loop handles queries
3. **Thread-Safe State**: McpServerInstance accessed via `Arc<Mutex<>>` to allow concurrent updates
4. **Non-Fatal Errors**: Notification handler errors don't close connection; logged and ignored
5. **Graceful Degradation**: Unknown notifications simply logged and skipped

### Notification Types

```
tools/list_changed       → Refresh tools registry
resources/list_changed   → Refresh resources registry
prompts/list_changed     → Refresh prompts registry
<unknown>                → Log and ignore
```

### Connection Handling

**Stdio Connections**:
- Reader runs in background task (non-blocking)
- Writer remains in request/response loop
- Both read from/write to same stdio streams (safe with buffering)

**HTTP Connections**:
- Polling mechanism (future phase)
- Or webhook callbacks (future phase)

### Error Recovery

| Error | Action |
|-------|--------|
| EOF on read | Exit listener gracefully (server closed) |
| Connection I/O error | Exit listener, log warning |
| Malformed JSON | Log warning, continue reading |
| Failed refresh (tools/list failed) | Log error, continue listening |
| Server error in notification | Logged, doesn't affect other notifications |

## Test Requirements

### Unit Tests (60%)

- Notification JSON parsing
- NotificationType detection from method name
- Unknown notification handling
- Handler result types

```rust
#[test]
fn test_parse_tools_list_changed_notification() { }
#[test]
fn test_notification_type_from_method() { }
#[test]
fn test_unknown_notification_ignored() { }
#[test]
fn test_parse_invalid_json_fails_gracefully() { }
```

### Integration Tests (30%)

- Mock MCP server sends notification
- Verify tools list refreshed without reconnection
- Multiple notifications in sequence
- Connection remains open after notification

```rust
#[tokio::test]
async fn test_list_changed_updates_tools_registry() { }
#[tokio::test]
async fn test_connection_stable_after_notification() { }
#[tokio::test]
async fn test_concurrent_request_and_notification() { }
```

### End-to-End Tests (10%)

- Real MCP server (if available) sends notifications
- Verify end-to-end behavior in TUI
- Stress test with rapid notifications

## Public API

### Types

```rust
pub enum NotificationType {
    ToolsListChanged,
    ResourcesListChanged,
    PromptsListChanged,
    Unknown(String),
}

pub enum NotificationHandlerResult {
    Handled,
    Error(String),
    ConnectionClosed,
}
```

### Functions

```rust
impl McpProxy {
    /// Spawn notification listener task (called from start_server)
    pub(crate) async fn spawn_notification_listener(
        &self,
        server_id: &str,
    ) -> Result<tokio::task::JoinHandle<()>, String>;
}

impl NotificationType {
    pub fn from_method(method: &str) -> Self;
}
```

## Non-Requirements

- ❌ Explicit subscription protocol (implicit on connection)
- ❌ Persistent notification history
- ❌ Notification filtering by client
- ❌ Metrics/observability (nice-to-have for Phase 3)
- ❌ HTTP webhook support (Phase 2)

## Implementation Constraints

1. **No Breaking Changes**: Existing McpProxy API must remain stable
2. **No New Dependencies**: Use only existing (tokio, serde, tracing)
3. **Backward Compatible**: Servers without notification support must work unchanged
4. **Single Responsibility**: Notification handler only handles notifications, not queries
5. **Async-First**: All I/O async via tokio (no blocking operations)

## Success Criteria

✅ Parses valid MCP JSON-RPC 2.0 notifications
✅ Detects `tools/list_changed`, `resources/list_changed`, `prompts/list_changed`
✅ Refreshes registries by calling tools/list, resources/list, prompts/list
✅ Updates McpServerInstance state atomically
✅ Handles malformed notifications gracefully
✅ Listener exits cleanly when connection closes
✅ No connection interruption from notifications
✅ Full test coverage (unit + integration)

## Files Modified/Created

- `crates/cli/src/plugins/mcp_proxy.rs` - Core changes
- `crates/cli/src/plugins/notifications.rs` - NEW handler implementation
- `crates/cli/tests/mcp_notifications_tests.rs` - NEW tests

## Implementation Order

1. Define types (NotificationType, NotificationHandlerResult)
2. Implement listener (spawn background task)
3. Implement handlers (refresh each registry type)
4. Integrate with McpProxy (spawn on start, cleanup on stop)
5. Write unit tests
6. Write integration tests
7. Document behavior

