# Architecture Overview: MCP List Changed Notifications

## System Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│ External MCP Server (Stdio/HTTP)                               │
│                                                                 │
│ ┌─ initialize ─→ (capabilities)                                │
│ │                                                               │
│ └─ [streaming notifications to stdout]                         │
│    {"jsonrpc":"2.0","method":"tools/list_changed","params":{}} │
│    {"jsonrpc":"2.0","method":"resources/list_changed",...}     │
└─────────────────────────────────────────────────────────────────┘
                          │
                          │ Binary I/O (stdio)
                          │ or HTTP
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│ RustyClawd Client (Issue #249 Implementation)                  │
│                                                                 │
│ ┌──────────────────────────────────────────────────────────┐   │
│ │ McpProxy (crates/cli/src/plugins/mcp_proxy.rs)          │   │
│ │                                                          │   │
│ │ servers: HashMap<String, Arc<Mutex<McpServerInstance>>> │   │
│ │ listeners: HashMap<String, JoinHandle<()>>              │   │
│ │                                                          │   │
│ │ ┌──────────────────┐      ┌──────────────────┐          │   │
│ │ │ Request Loop     │      │ Listener Task    │          │   │
│ │ │ (existing)       │      │ (NEW - Phase 1)  │          │   │
│ │ │                  │      │                  │          │   │
│ │ │ Request:         │      │ Spawn on:        │          │   │
│ │ │ - tools/list     │      │ start_server()   │          │   │
│ │ │ - tools/call     │      │                  │          │   │
│ │ │ - resources/list │      │ Listen for:      │          │   │
│ │ │ - prompts/get    │      │ - Notifications  │          │   │
│ │ │                  │      │ - Connection EOF │          │   │
│ │ │ ┌─── stdin ────→ │      │                  │          │   │
│ │ │ │                │      │ ┌─── stdout ─→  │          │   │
│ │ │ └─← stdout ──────┼──────┼─→ BufReader     │          │   │
│ │ │                  │      │                  │          │   │
│ │ └──────────────────┘      └──────────────────┘          │   │
│ │         ↑                                                │   │
│ │         │ (synchronous)                                 │   │
│ │         │                                                │   │
│ │ ┌─────────────────────────────────────────────────────┐ │   │
│ │ │ TUI / CLI / External Code                           │ │   │
│ │ │ (queries tools/prompts/resources)                   │ │   │
│ │ └─────────────────────────────────────────────────────┘ │   │
│ │                      ↑                                   │   │
│ │                      │ (async/await)                     │   │
│ │                      │                                   │   │
│ │ ┌─────────────────────────────────────────────────────┐ │   │
│ │ │ Notification Handler (NEW - Phase 1)                │ │   │
│ │ │ (crates/cli/src/plugins/notifications.rs)          │ │   │
│ │ │                                                     │ │   │
│ │ │ On tools/list_changed:                             │ │   │
│ │ │   → Call tools/list request                        │ │   │
│ │ │   → Update server.tools                            │ │   │
│ │ │   → Log "tools refreshed: 3 → 5"                  │ │   │
│ │ │                                                     │ │   │
│ │ │ On resources/list_changed:                         │ │   │
│ │ │   → Call resources/list request                    │ │   │
│ │ │   → Update server.resources                        │ │   │
│ │ │                                                     │ │   │
│ │ │ On prompts/list_changed:                           │ │   │
│ │ │   → Call prompts/list request                      │ │   │
│ │ │   → Update server.prompts                          │ │   │
│ │ │                                                     │ │   │
│ │ │ Triggered by: McpConnectionListener               │ │   │
│ │ └─────────────────────────────────────────────────────┘ │   │
│ │         ↑                                                │   │
│ │         │ (parses notifications from stdout)            │   │
│ │         │                                                │   │
│ │ ┌─────────────────────────────────────────────────────┐ │   │
│ │ │ McpConnectionListener (NEW - Phase 1)               │ │   │
│ │ │ (Background task reading notifications)             │ │   │
│ │ │                                                     │ │   │
│ │ │ pub struct McpConnectionListener {                 │ │   │
│ │ │   server_id: String,                              │ │   │
│ │ │   handler: Box<dyn Fn(NotificationType)>,        │ │   │
│ │ │ }                                                  │ │   │
│ │ │                                                     │ │   │
│ │ │ Loop:                                              │ │   │
│ │ │  1. Read line from stdout_reader                  │ │   │
│ │ │  2. If EOF → exit listener                        │ │   │
│ │ │  3. Parse JSON → McpNotification                  │ │   │
│ │ │  4. If no `id` → it's a notification              │ │   │
│ │ │  5. Extract method → NotificationType             │ │   │
│ │ │  6. Call handler(notification_type)               │ │   │
│ │ │  7. Continue reading                              │ │   │
│ │ │                                                     │ │   │
│ │ │ Errors:                                            │ │   │
│ │ │  - Invalid JSON → Log warning, continue           │ │   │
│ │ │  - Connection error → Log error, exit             │ │   │
│ │ │  - Handler error → Log error, continue            │ │   │
│ │ │                                                     │ │   │
│ │ └─────────────────────────────────────────────────────┘ │   │
│ │         ↑                                                │   │
│ │         │ (tokio::spawn async task)                     │   │
│ │         │ (spawned from McpProxy::start_server)         │   │
│ │         │                                                │   │
│ │ ┌─────────────────────────────────────────────────────┐ │   │
│ │ │ types.rs (NEW - Phase 1)                            │ │   │
│ │ │                                                     │ │   │
│ │ │ pub enum NotificationType {                        │ │   │
│ │ │   ToolsListChanged,                               │ │   │
│ │ │   ResourcesListChanged,                           │ │   │
│ │ │   PromptsListChanged,                             │ │   │
│ │ │   Unknown(String),                                │ │   │
│ │ │ }                                                  │ │   │
│ │ │                                                     │ │   │
│ │ │ pub enum NotificationHandlerResult {               │ │   │
│ │ │   Handled,                                         │ │   │
│ │ │   Error(String),                                   │ │   │
│ │ │   ConnectionClosed,                                │ │   │
│ │ │ }                                                  │ │   │
│ │ │                                                     │ │   │
│ │ │ pub struct McpNotification {                       │ │   │
│ │ │   jsonrpc: String,                                │ │   │
│ │ │   method: String,        // NO `id` field!       │ │   │
│ │ │   params: serde_json::Value,                      │ │   │
│ │ │ }                                                  │ │   │
│ │ │                                                     │ │   │
│ │ └─────────────────────────────────────────────────────┘ │   │
│ │                                                          │   │
│ └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Sequence Diagram: Startup and Notification Flow

```
Timeline:

[T1] User starts MCP server
     ├─ CLI: claude mcp start my-server
     │
     └─ McpProxy::start_server("my-server")
        ├─ Create Stdio connection
        │
        ├─ Send initialize request → (server responds)
        │  (discovers tools: [tool_a, tool_b, tool_c])
        │
        ├─ Store tools in server.tools
        │
        ├─ Spawn notification listener task
        │  └─ tokio::spawn(McpConnectionListener::listen(...))
        │     └─ Continuously reads from stdout_reader
        │
        └─ Return Ok(())

[T2] Server's tool set changes
     └─ MCP Server sends notification:
        {
          "jsonrpc": "2.0",
          "method": "tools/list_changed",
          "params": {}
        }
        (NO `id` field - this is a notification, not a response)

[T3] Notification listener reads line
     ├─ McpConnectionListener loop receives bytes
     │
     ├─ Parse JSON:
     │  ├─ Find no `id` field → It's a notification (not a response)
     │  ├─ Extract method: "tools/list_changed"
     │  └─ Create NotificationType::ToolsListChanged
     │
     ├─ Call handler(ToolsListChanged)
     │  └─ McpNotificationHandler::handle()
     │
     └─ Route to refresh_tools()
        ├─ Send tools/list request via REQUEST loop
        │  (note: different from listener loop)
        │  └─ Server responds: [tool_a, tool_b, tool_c, tool_d, tool_e]
        │
        ├─ Lock server.tools (Arc<Mutex<>>)
        │
        ├─ Update: server.tools = [a, b, c, d, e]
        │
        ├─ Log: "Updated tools: 3 → 5"
        │
        └─ Continue listening for next notification

[T4] User queries available tools
     ├─ TUI or CLI calls proxy.list_tools("my-server")
     │
     ├─ McpProxy acquires lock: server.lock().await
     │
     ├─ Returns: [tool_a, tool_b, tool_c, tool_d, tool_e]
     │  (includes tools added via list_changed!)
     │
     └─ User sees all 5 tools

[T5] User stops server
     └─ McpProxy::stop_server("my-server")
        ├─ Abort listener task: listeners.remove("my-server").abort()
        │
        ├─ Close connection
        │
        ├─ Terminate process
        │
        └─ Clean up resources
```

## State Diagram: Connection Lifecycle

```
              ┌─────────────────┐
              │ STOPPED         │
              │ (No server)     │
              └────────┬────────┘
                       │
                       │ start_server()
                       ↓
              ┌─────────────────────┐
              │ INITIALIZING        │
              │ - Creating conn     │
              │ - Calling init RPC  │
              │ - Spawning listener │
              └────────┬────────────┘
                       │
                       │ (success)
                       ↓
    ┌──────────────────────────────────────┐
    │ RUNNING - LISTENING FOR CHANGES      │
    │                                      │
    │ STATE:                               │
    │ - Connection open                    │
    │ - Listener task active               │
    │ - tools/prompts/resources cached     │
    │                                      │
    │ ACTIVITY:                            │
    │ - Request loop handles queries       │
    │ - Listener loop reads notifications  │
    │ - Handler updates registries         │
    │                                      │
    │ EVENTS:                              │
    │ ├─ tools/list_changed                │
    │ │  └─ refresh tools, stay RUNNING    │
    │ ├─ resources/list_changed            │
    │ │  └─ refresh resources, stay RUNNING│
    │ ├─ prompts/list_changed              │
    │ │  └─ refresh prompts, stay RUNNING  │
    │ ├─ unknown notification              │
    │ │  └─ log, ignore, stay RUNNING      │
    │ └─ user query (e.g., tools/list)    │
    │    └─ request loop handles it        │
    │                                      │
    └──────────┬────────────────┬──────────┘
               │                │
         stop_server()      connection_closes
         (user action)      (error/disconnect)
               │                │
               ↓                ↓
         ┌──────────────────────────┐
         │ STOPPED                  │
         │ - Listener aborted       │
         │ - Connection closed      │
         │ - Process terminated     │
         └──────────────────────────┘
```

## Data Flow: List Changed Notification Processing

```
INPUT: MCP Notification (from server stdout)
       {
         "jsonrpc": "2.0",
         "method": "tools/list_changed",
         "params": {}
       }

       │
       ↓

PARSE: McpConnectionListener::listen()
       └─ Read line from stdout
       └─ serde_json::from_str::<McpNotification>()
       └─ Check: no `id` field? → Yes, it's a notification
       └─ Extract: method = "tools/list_changed"

       │
       ↓

CLASSIFY: NotificationType::from_method()
          └─ Match method
          └─ Return: NotificationType::ToolsListChanged

       │
       ↓

ROUTE: McpNotificationHandler::handle()
       └─ Match notification_type
       └─ Call: self.refresh_tools()

       │
       ↓

FETCH: Send tools/list request
       ├─ McpProxy::list_tools(server_id)
       ├─ Send JSON-RPC request (WITH id)
       ├─ Receive JSON-RPC response (WITH id)
       └─ Parse: Vec<McpToolDefinition>

       │
       ↓

UPDATE: Arc<Mutex<McpServerInstance>>
        ├─ server.lock().await
        ├─ server.tools = new_tools
        ├─ metrics.tools_refreshed += 1
        ├─ unlock

       │
       ↓

NOTIFY: Tracing
        └─ info!("Updated tools: {} → {}", old_count, new_count)

       │
       ↓

OUTPUT: McpServerInstance::tools
        └─ Available to subsequent queries
```

## Concurrency Model

```
Request/Response Loop (Main)
├─ Read request from TUI/CLI
├─ Send JSON-RPC request (with id) to server
├─ Block reading response (with matching id)
├─ Process response, send to TUI/CLI
└─ Loop

                    ║ (Independent async tasks)
                    ║

Notification Listener (Background Task)
├─ Continuously read from stdout
├─ Filter by: is_notification (no `id` field)
├─ Parse notification
├─ Trigger handler
├─ Handler sends tools/list request
│  └─ (This blocks on request/response loop briefly)
│     └─ But listener is async, so not blocking other listeners
└─ Continue reading

Result:
- Both loops can run concurrently
- Share same connection streams (buffered, safe)
- Handler requests block request loop briefly (acceptable)
- Multiple notification handlers (if many notifications) are async
```

## Thread Safety

```
SHARED STATE: McpServerInstance
              (Arc<Mutex<>>)

WRITE ACCESS (Handler):
  Arc<McpServerInstance>::lock().await
  ├─ Acquire lock
  ├─ Update tools
  ├─ Unlock
  └─ Continue

READ ACCESS (Request Loop):
  Arc<McpServerInstance>::lock().await
  ├─ Acquire lock
  ├─ Read tools
  ├─ Unlock
  └─ Continue

GUARANTEE: Never concurrent access to same data
COST: Small latency (microseconds) for lock contention
```

## Error Handling Flow

```
Scenario 1: Malformed JSON notification
  ├─ Listener read line: `{"invalid json`
  ├─ serde_json::from_str() → Err
  └─ Log warning, continue reading

Result: Connection stable, listener active ✓

Scenario 2: Connection close (EOF)
  ├─ Listener read_line() → Ok(0)  (0 bytes = EOF)
  └─ Break listener loop

Result: Listener exits gracefully ✓

Scenario 3: Handler fails (tools/list returns error)
  ├─ refresh_tools() → Err("Server error")
  ├─ Log error
  └─ Continue listening

Result: Connection stable, listener active ✓

Scenario 4: Unknown notification type
  ├─ method = "custom/event"
  ├─ NotificationType::Unknown("custom/event")
  ├─ Handler: log debug, do nothing
  └─ Continue listening

Result: Connection stable, listener active ✓

Scenario 5: Listener task panic
  ├─ Listener catches panic
  ├─ Log error
  └─ (Restart listener on next start_server call)

Result: User must restart server (acceptable) ✓
```

## Performance Characteristics

```
Notification Latency (end-to-end):
  Server sends notification
  ↓ 1-5ms OS scheduling
  Listener reads line
  ↓ 1-2ms JSON parsing
  Handler called
  ↓ 5-10ms tools/list request/response
  Tools updated
  ├─ Total: 7-17ms typical
  └─ Acceptable for tool discovery UX

Memory Per Connection:
  ├─ BufReader: ~8KB
  ├─ JoinHandle: ~200 bytes
  ├─ Arc<Mutex<>>: ~100 bytes (overhead)
  └─ Total: ~10KB per server
  └─ Acceptable for 20 concurrent servers

CPU Usage:
  ├─ Listening (idle): 0% CPU
  ├─ Notification processing: <1% CPU per notification
  └─ Total: Negligible
```

