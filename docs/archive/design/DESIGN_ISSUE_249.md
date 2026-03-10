# Design: MCP `list_changed` Notifications (Issue #249)

## Problem Statement

MCP servers need the ability to dynamically update their available tools, prompts, and resources **without requiring reconnection**. The MCP protocol supports notifications to communicate these changes to clients.

Currently, RustyClawd:
- Loads tool/prompt/resource lists once at server startup via `initialize`
- Has no mechanism to receive or handle `list_changed` notifications
- Requires reconnection for servers that update capabilities dynamically

This design specifies how to implement support for MCP `list_changed` notifications.

---

## Solution Architecture

### High-Level Components

```
MCP Server (stdio/HTTP)
    ↓
    ├─→ Sends JSON-RPC 2.0 notifications
    │   (no `id` field)
    │
    ↓
McpConnectionListener
    ├─→ Spawns background listener task per connection
    ├─→ Detects notification type (method name)
    ├─→ Routes to appropriate handler
    │
    ↓
NotificationHandlers
    ├─→ list_changed: Refresh tool/prompt/resource lists
    ├─→ Unknown: Log and ignore gracefully
    │
    ↓
McpServerInstance (updated state)
    └─→ Registries updated in real-time
```

### Key Insight: Separation of Concerns

- **Request/Response Loop**: Existing synchronous request-response for queries (tools/list, etc.)
- **Notification Listener**: Separate background task reading notifications
- **State Updates**: Thread-safe updates to McpServerInstance via Arc<Mutex<>>
- **No Reconnection**: Connection stays open; only internal state refreshes

---

## Detailed Design

### 1. Notification Data Types

```rust
/// JSON-RPC 2.0 notification (has no `id` field)
#[derive(Debug, Deserialize)]
pub struct McpNotification {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

/// Supported notification types
#[derive(Debug, Clone)]
pub enum NotificationType {
    /// tools/list_changed: Tools capability updated
    ToolsListChanged,
    /// resources/list_changed: Resources capability updated
    ResourcesListChanged,
    /// prompts/list_changed: Prompts capability updated
    PromptsListChanged,
    /// Unknown notification (log and ignore)
    Unknown(String),
}

impl NotificationType {
    fn from_method(method: &str) -> Self {
        match method {
            "tools/list_changed" => Self::ToolsListChanged,
            "resources/list_changed" => Self::ResourcesListChanged,
            "prompts/list_changed" => Self::PromptsListChanged,
            unknown => Self::Unknown(unknown.to_string()),
        }
    }
}

/// Notification handler result
pub enum NotificationHandlerResult {
    /// Successfully handled
    Handled,
    /// Error handling notification (non-fatal)
    Error(String),
    /// Connection closed
    ConnectionClosed,
}
```

### 2. Connection Wrapper (stdio support)

```rust
/// Wraps stdio connection with separate read and write streams
pub struct StdioConnectionStreams {
    stdin: tokio::io::WriteHalf<tokio::process::ChildStdin>,
    stdout: BufReader<tokio::io::ReadHalf<tokio::process::ChildStdout>>,
}

impl StdioConnectionStreams {
    /// Split child process stdin/stdout for concurrent read/write
    pub fn from_child(
        stdin: Option<Stdio>,
        stdout: Option<Stdio>,
    ) -> Result<Self, String> {
        // Implementation: Split tokio::process::Child streams
        // Allows simultaneous:
        // - Sending requests (write to stdin)
        // - Receiving responses (read from stdout)
        // - Listening for notifications (also read from stdout)
    }
}
```

### 3. Connection Listener Task

```rust
/// Listens for incoming notifications on MCP connection
pub struct McpConnectionListener {
    server_id: String,
    notification_handler: Box<dyn Fn(NotificationType) -> NotificationHandlerResult + Send>,
}

impl McpConnectionListener {
    /// Spawn background task to listen for notifications
    pub async fn spawn(
        server_id: String,
        stdout_reader: Arc<Mutex<BufReader<>>>,
        handler: Box<dyn Fn(NotificationType) -> NotificationHandlerResult + Send>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                // Read line from stdout
                let mut line = String::new();
                match stdout_reader.lock().await.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        // Parse JSON
                        match serde_json::from_str::<McpNotification>(&line) {
                            Ok(notification) => {
                                let notif_type = NotificationType::from_method(&notification.method);
                                match handler(notif_type) {
                                    NotificationHandlerResult::ConnectionClosed => break,
                                    NotificationHandlerResult::Error(e) => {
                                        tracing::warn!("Notification handler error: {}", e);
                                    }
                                    NotificationHandlerResult::Handled => {}
                                }
                            }
                            Err(e) => {
                                // If not a valid notification, might be a response (has `id`)
                                // These are handled by the request/response loop
                                tracing::trace!("Ignoring line (likely response or malformed): {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Error reading from MCP server: {}", e);
                        break;
                    }
                }
            }
        })
    }
}
```

### 4. Handler Implementation

```rust
/// Notification handler for updating registries
pub struct McpNotificationHandler {
    server_id: String,
    proxy: Arc<Mutex<McpProxy>>,
}

impl McpNotificationHandler {
    pub async fn handle(&self, notification_type: NotificationType)
        -> NotificationHandlerResult
    {
        match notification_type {
            NotificationType::ToolsListChanged => {
                self.refresh_tools().await
            }
            NotificationType::ResourcesListChanged => {
                self.refresh_resources().await
            }
            NotificationType::PromptsListChanged => {
                self.refresh_prompts().await
            }
            NotificationType::Unknown(method) => {
                tracing::debug!("Ignoring unknown notification: {}", method);
                NotificationHandlerResult::Handled
            }
        }
    }

    async fn refresh_tools(&self) -> NotificationHandlerResult {
        match self.proxy.lock().await.list_tools(&self.server_id).await {
            Ok(tools) => {
                if let Some(server) = self.proxy.lock().await.get_server_mut(&self.server_id) {
                    let old_count = server.tools.len();
                    server.tools = tools;
                    tracing::info!(
                        "Updated tools for server {}: {} → {}",
                        self.server_id,
                        old_count,
                        server.tools.len()
                    );
                }
                NotificationHandlerResult::Handled
            }
            Err(e) => NotificationHandlerResult::Error(
                format!("Failed to refresh tools: {}", e)
            ),
        }
    }

    // Similar for refresh_resources and refresh_prompts
}
```

### 5. Integration with McpProxy

```rust
pub struct McpProxy {
    servers: HashMap<String, Arc<Mutex<McpServerInstance>>>,
    listeners: HashMap<String, tokio::task::JoinHandle<()>>,
    next_request_id: Arc<Mutex<u64>>,
}

impl McpProxy {
    /// Start server AND listener task
    pub async fn start_server(&mut self, server_id: &str) -> Result<(), String> {
        // 1. Initialize connection (existing code)
        self.initialize_connection(...).await?;

        // 2. Spawn notification listener (NEW)
        let listener_handle = self.spawn_notification_listener(server_id)?;
        self.listeners.insert(server_id.to_string(), listener_handle);

        Ok(())
    }

    fn spawn_notification_listener(&self, server_id: &str)
        -> Result<tokio::task::JoinHandle<()>, String>
    {
        let server = self.servers.get(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        let stdout_reader = Arc::new(Mutex::new(
            server.get_stdout_reader()? // Extract reader from connection
        ));

        let server_id_clone = server_id.to_string();
        let proxy = Arc::new(Mutex::new(self.clone()));

        let listener = McpConnectionListener::spawn(
            server_id_clone,
            stdout_reader,
            Box::new(move |notif| {
                let proxy = proxy.clone();
                let server_id = server_id_clone.clone();

                // Call handler inline (blocking would be bad)
                // Instead, send to channel for async processing
                todo!("Use channel for async handler")
            }),
        );

        Ok(listener)
    }

    /// Stop server and listener
    pub async fn stop_server(&mut self, server_id: &str) -> Result<(), String> {
        // Cancel listener task
        if let Some(listener) = self.listeners.remove(server_id) {
            listener.abort();
        }

        // Close connection and terminate process (existing)
        self.close_connection(server_id).await?;

        Ok(())
    }
}
```

### 6. Subscription Management (Explicit vs. Implicit)

The design uses **implicit subscription** via connection establishment:

- When client starts MCP server → Server may immediately send notifications
- No explicit "subscribe" needed (MCP protocol doesn't require it)
- Listener starts immediately after `initialize` completes
- Handler automatically refreshes lists when notifications arrive

**Alternative: Explicit Subscription** (Not implemented, for future)
```rust
// If servers require explicit subscription (rare):
pub async fn subscribe_to_notifications(&mut self, server_id: &str) -> Result<(), String> {
    self.send_request(server_id, "notifications/subscribe", json!({
        "subscriptions": ["tools/list_changed", "resources/list_changed"]
    })).await
}
```

---

## Testing Strategy

### Unit Tests: Notification Parsing

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_tools_list_changed_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"tools/list_changed","params":{}}"#;
        let notif: McpNotification = serde_json::from_str(json).unwrap();
        assert_eq!(notif.method, "tools/list_changed");
    }

    #[test]
    fn test_notification_type_detection() {
        let notif_type = NotificationType::from_method("tools/list_changed");
        assert!(matches!(notif_type, NotificationType::ToolsListChanged));
    }

    #[test]
    fn test_unknown_notification_ignored() {
        let notif_type = NotificationType::from_method("custom/event");
        assert!(matches!(notif_type, NotificationType::Unknown(_)));
    }
}
```

### Integration Tests: End-to-End

```rust
#[cfg(test)]
mod integration_tests {
    #[tokio::test]
    async fn test_list_changed_updates_tools() {
        // 1. Start MCP server that sends list_changed
        let mut proxy = McpProxy::new();
        proxy.register_server(test_server_definition());
        proxy.start_server("test-server").await.unwrap();

        // 2. Get initial tool count
        let initial_tools = proxy.list_tools("test-server").await.unwrap();
        assert_eq!(initial_tools.len(), 2);

        // 3. Trigger server to send list_changed notification
        send_notification_to_server("tools/list_changed");

        // 4. Wait for async handler to process
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 5. Verify tools updated
        let updated_tools = proxy.list_tools("test-server").await.unwrap();
        assert_eq!(updated_tools.len(), 3);
    }

    #[tokio::test]
    async fn test_connection_stays_open_after_notification() {
        // Verify connection not closed after handling list_changed
        // Subsequent requests should still work
    }
}
```

### Mock MCP Server for Testing

```rust
/// Test MCP server that sends notifications
struct MockMcpServer {
    tools: Arc<Mutex<Vec<McpToolDefinition>>>,
}

impl MockMcpServer {
    async fn send_list_changed_notification(&self) {
        // Write JSON-RPC notification to stdout
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "tools/list_changed",
            "params": {}
        });
        println!("{}", serde_json::to_string(&notification).unwrap());
    }
}
```

---

## Error Handling

### Connection Issues

**Problem**: Listener task crashes if connection drops
**Solution**: Detect EOF (read returns 0 bytes), exit listener gracefully

```rust
match stdout_reader.read_line(&mut line).await {
    Ok(0) => {
        tracing::info!("MCP server closed connection");
        break; // Exit listener
    }
    Ok(_) => { /* process line */ }
    Err(e) => {
        tracing::error!("Connection error: {}", e);
        break; // Exit listener
    }
}
```

### Malformed Notifications

**Problem**: Server sends invalid JSON
**Solution**: Log warning, continue reading

```rust
match serde_json::from_str::<McpNotification>(&line) {
    Ok(notification) => { /* process */ }
    Err(e) => {
        tracing::warn!("Failed to parse notification: {}", e);
        continue; // Keep listening
    }
}
```

### Race Conditions

**Problem**: Listener tries to update McpServerInstance while request loop is reading it
**Solution**: Use Arc<Mutex<>> for shared state

```rust
// Old: Direct reference to server
pub fn list_tools(&mut self, server_id: &str) -> Vec<McpToolDefinition> {
    self.servers[server_id].tools.clone()
}

// New: Thread-safe access
pub async fn list_tools(&self, server_id: &str)
    -> Result<Vec<McpToolDefinition>, String>
{
    let server = self.servers.get(server_id)
        .ok_or("Server not found")?;
    let locked = server.lock().await;
    Ok(locked.tools.clone())
}
```

---

## Backward Compatibility

- ✅ Servers without notification support: Listener will see no notifications, keep working
- ✅ Existing request/response loop: Unchanged
- ✅ API surface: No breaking changes to public methods
- ⚠️ Internal: McpProxy now has async requirements (but already async)

---

## Performance Considerations

### Memory

- **Per Connection**: One background task + one reader stream
- **Estimated**: ~10KB per connection (buffer, task overhead)
- **Acceptable** for typical deployments (< 20 MCP servers)

### Latency

- **Notification Latency**: ~1-10ms (depends on OS scheduling)
- **List Operations**: First `tools/list` after notification might be slower (Mutex contention)
- **Typical**: < 50ms total for "list changed" → "tools available"

### Resource Cleanup

- **Listener Cleanup**: Call `listener.abort()` on `stop_server`
- **Stream Cleanup**: Reader dropped when connection closed
- **Verification**: Ensure no task leaks on server restart

---

## Files to Modify

### Core Changes

1. **`crates/cli/src/plugins/mcp_proxy.rs`**
   - Add `McpNotification`, `NotificationType` types
   - Add `McpConnectionListener`
   - Modify `McpProxy::start_server` to spawn listener
   - Modify `McpProxy::stop_server` to abort listener
   - Change internal state to use `Arc<Mutex<>>` for thread-safety

2. **`crates/cli/src/plugins/notifications.rs`** (NEW)
   - `McpNotificationHandler` implementation
   - Handler logic for each notification type
   - Logging and metrics

### Testing

3. **`crates/cli/tests/mcp_notifications_tests.rs`** (NEW)
   - Unit tests for notification parsing
   - Integration tests with mock MCP server
   - End-to-end tests for dynamic capability updates

### Documentation

4. **`docs/MCP_NOTIFICATIONS.md`** (NEW)
   - User guide for MCP server developers
   - How to implement `list_changed` in MCP servers
   - Troubleshooting notification issues

---

## Implementation Phases

### Phase 1: Foundation (MVP)
- [ ] Define notification types
- [ ] Implement listener (stdio only)
- [ ] Implement `tools/list_changed` handler
- [ ] Unit tests

### Phase 2: HTTP Support
- [ ] Extend listener for HTTP connections
- [ ] HTTP webhook or polling mechanism
- [ ] Integration tests

### Phase 3: Production Hardening
- [ ] Metrics/observability
- [ ] Error recovery (reconnect on connection loss)
- [ ] Performance testing
- [ ] Documentation

---

## Open Questions

1. **HTTP Notifications**: MCP spec allows HTTP. Implement webhooks (push) or polling (pull)?
   - **Decision**: Start with polling via background task, upgrade if needed

2. **Subscription**: Some servers might require explicit `notifications/subscribe`. How to detect?
   - **Decision**: Try implicit first, fall back to explicit if server returns error

3. **State Consistency**: If client requests tools while list_changed is processing, what's returned?
   - **Decision**: Use Arc<Mutex<>> to ensure consistent snapshots

4. **Backpressure**: What if server sends notifications faster than we can process?
   - **Decision**: Queue in reader buffer (OS-managed), log if buffer grows

---

## Success Criteria

- ✅ MCP servers can send `list_changed` notifications
- ✅ RustyClawd detects and processes notifications
- ✅ Tools/prompts/resources updated without reconnection
- ✅ No breaking changes to existing API
- ✅ Connection stability maintained under notification load
- ✅ Error cases handled gracefully
- ✅ Full test coverage
- ✅ Documentation complete

