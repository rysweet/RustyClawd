# Builder Checklist: MCP List Changed Notifications

Use this checklist while implementing Phase 1-4. Check off items as you complete them.

## Pre-Implementation

### Design Review
- [ ] Read DESIGN_ISSUE_249.md completely
- [ ] Review ARCHITECTURE_OVERVIEW.md system diagram
- [ ] Understand Specs/MCP_NOTIFICATIONS.md contract
- [ ] Clarify any design questions with architect
- [ ] Understand key design decisions (implicit subscription, separate task, Arc<Mutex<>>)

### Environment Setup
- [ ] Clone issue-249 worktree
- [ ] Verify Rust toolchain (1.70+)
- [ ] Run `cargo test` to verify baseline
- [ ] Understand existing mcp_proxy.rs structure
- [ ] Locate and read McpServerInstance definition

---

## Phase 1: Foundation (Estimated 2-3 days)

### Step 1: Define Types
- [ ] Create `crates/cli/src/plugins/notifications.rs`
- [ ] Define `McpNotification` struct
  - [ ] `jsonrpc: String`
  - [ ] `method: String`
  - [ ] `params: serde_json::Value`
  - [ ] Add Deserialize derive
  - [ ] Add test: parse valid notification
- [ ] Define `NotificationType` enum
  - [ ] `ToolsListChanged` variant
  - [ ] `ResourcesListChanged` variant
  - [ ] `PromptsListChanged` variant
  - [ ] `Unknown(String)` variant
  - [ ] Implement `from_method(&str) -> Self`
  - [ ] Add tests for each variant
- [ ] Define `NotificationHandlerResult` enum
  - [ ] `Handled` variant
  - [ ] `Error(String)` variant
  - [ ] `ConnectionClosed` variant

**Verify**: Compiles without warnings, all tests pass

### Step 2: Implement Listener Task
- [ ] Create `McpConnectionListener` struct
  - [ ] `server_id: String`
  - [ ] `handler: Arc<dyn Fn(...) + Send>`
- [ ] Implement `spawn()` async function
  - [ ] Create tokio task
  - [ ] Read line loop structure
  - [ ] Handle EOF (return 0 bytes)
  - [ ] Parse JSON into McpNotification
  - [ ] Extract method
  - [ ] Call handler(notification_type)
  - [ ] Continue on loop
  - [ ] Return JoinHandle
- [ ] Add error handling
  - [ ] Malformed JSON → log warning, continue
  - [ ] Connection error → log error, break
  - [ ] Handler error → log error, continue

**Verify**: Compiles, no panics, listener spawns and reads

### Step 3: Implement Handler Structure
- [ ] Create `McpNotificationHandler` struct
  - [ ] `server_id: String`
  - [ ] `proxy: Arc<Mutex<McpProxy>>`
- [ ] Implement `handle()` async method
  - [ ] Match on NotificationType
  - [ ] Route to refresh_* methods
  - [ ] Return NotificationHandlerResult
- [ ] Implement `refresh_tools()` method
  - [ ] Call `self.proxy.list_tools(server_id)`
  - [ ] Lock server state
  - [ ] Update server.tools
  - [ ] Log metrics ("tools refreshed: X → Y")
  - [ ] Return Handled or Error
- [ ] Add stubs for resources/prompts (implement in Phase 2)

**Verify**: Compiles, handler routes correctly, logs work

### Step 4: Integrate with McpProxy
- [ ] Modify `McpServerInstance` to use Arc<Mutex<>>
  - [ ] Change `tools: Vec<>` → Arc<Mutex<Vec<>>>
  - [ ] Change `resources: Vec<>` → Arc<Mutex<Vec<>>>
  - [ ] Change `prompts: Vec<>` → Arc<Mutex<Vec<>>>
  - [ ] Update all references to use `.lock().await`
- [ ] Add to `McpProxy`:
  - [ ] `listeners: HashMap<String, JoinHandle<()>>`
- [ ] Modify `start_server()`:
  - [ ] After initialize succeeds
  - [ ] Call `self.spawn_notification_listener(server_id)?`
  - [ ] Store listener handle
- [ ] Modify `stop_server()`:
  - [ ] Remove listener from HashMap
  - [ ] Call `listener.abort()`
  - [ ] Continue with existing cleanup
- [ ] Implement `spawn_notification_listener()`:
  - [ ] Get stdout reader from connection
  - [ ] Create notification handler
  - [ ] Spawn McpConnectionListener
  - [ ] Return JoinHandle

**Verify**: start/stop_server work, listener spawns on start, aborts on stop

### Step 5: Thread Safety
- [ ] Verify all lock accesses use `.await`
- [ ] Check for deadlock patterns (circular deps)
- [ ] Use `Arc<tokio::sync::Mutex<>>` not `std::sync::Mutex`
- [ ] Test concurrent access patterns
  - [ ] Handler updating while request loop reads
  - [ ] Multiple queries while notification processing

**Verify**: No panics under concurrent load, no deadlocks

### Step 6: Unit Tests
Create `crates/cli/tests/mcp_notifications_tests.rs`

- [ ] Test notification parsing
  - [ ] `test_parse_valid_tools_notification`
  - [ ] `test_parse_valid_resources_notification`
  - [ ] `test_parse_valid_prompts_notification`
  - [ ] `test_notification_missing_id_field` (confirms it's notification, not response)
- [ ] Test notification type detection
  - [ ] `test_from_method_tools_list_changed`
  - [ ] `test_from_method_resources_list_changed`
  - [ ] `test_from_method_prompts_list_changed`
  - [ ] `test_from_method_unknown_method`
- [ ] Test error handling
  - [ ] `test_parse_invalid_json`
  - [ ] `test_parse_malformed_json`
  - [ ] `test_parse_json_missing_method`

**Verify**: `cargo test mcp_notifications_tests` passes, 90%+ coverage

### Step 7: Integration Tests
- [ ] Create mock MCP server struct
  - [ ] Responds to initialize
  - [ ] Can send notifications on command
- [ ] Test full flow
  - [ ] `test_list_changed_triggers_refresh`
    - Start server with 3 tools
    - Send tools/list_changed
    - Verify tools updated
  - [ ] `test_connection_remains_open_after_notification`
    - Verify subsequent requests work
  - [ ] `test_malformed_notification_ignored`
    - Send invalid JSON
    - Verify listener continues
  - [ ] `test_listener_exits_on_eof`
    - Close server connection
    - Verify listener exits cleanly

**Verify**: All integration tests pass, simulate real MCP server behavior

### Phase 1 Done ✓
- [ ] Merge Phase 1 branch
- [ ] Write CHANGELOG entry
- [ ] Tag with phase-1-complete
- [ ] Document any learnings in DISCOVERIES.md

---

## Phase 2: Extension (Estimated 1-2 days)

### Step 1: Implement Resources Handler
- [ ] In `McpNotificationHandler::handle()`, complete `ResourcesListChanged` arm
- [ ] Implement `refresh_resources()` method
  - [ ] Call resources/list
  - [ ] Update server.resources
  - [ ] Log metrics
- [ ] Add tests for resources notification

### Step 2: Implement Prompts Handler
- [ ] In `McpNotificationHandler::handle()`, complete `PromptsListChanged` arm
- [ ] Implement `refresh_prompts()` method
  - [ ] Call prompts/get (or list, depending on MCP spec)
  - [ ] Update server.prompts
  - [ ] Log metrics
- [ ] Add tests for prompts notification

### Step 3: Test Multiple Notifications
- [ ] Test rapid-fire notifications (5+ in sequence)
- [ ] Test mixed notification types (tools + resources + prompts)
- [ ] Verify no interference between handlers

### Phase 2 Done ✓
- [ ] All three notification types working
- [ ] Full integration test coverage
- [ ] Merge Phase 2 branch
- [ ] Tag with phase-2-complete

---

## Phase 3: HTTP Support (Estimated 2-3 days, Optional)

### Note
HTTP connections have a different I/O model (not streaming stdout). Choose one approach:

**Option A: Polling** (Simpler)
- Background task periodically calls resources/list
- On 404/error → infer list_changed and refresh

**Option B: Webhooks** (Complex)
- HTTP server listens on port for notifications
- MCP server posts to webhook URL

**Recommendation**: Start with polling, upgrade if needed

### Step 1: Detect HTTP Connections
- [ ] In listener spawning, check connection type
- [ ] Branch logic: Stdio → listen for notifications, HTTP → poll

### Step 2: Implement HTTP Listener
- [ ] Create `HttpNotificationListener`
- [ ] Implement polling loop (every 30 seconds)
- [ ] Call resources/list, prompts/get, tools/list
- [ ] Detect changes (compare against cached)
- [ ] Trigger refresh if changed

### Step 3: Tests
- [ ] Mock HTTP server
- [ ] Test polling detection

### Phase 3 Done ✓
- [ ] HTTP connections support notifications
- [ ] Merge Phase 3 branch
- [ ] Tag with phase-3-complete

---

## Phase 4: Hardening (Estimated 1-2 days)

### Step 1: Metrics & Observability
- [ ] Add `notifications_received` counter
- [ ] Add `refreshes_attempted` counter
- [ ] Add `refreshes_succeeded` counter
- [ ] Add `refresh_latency_ms` histogram
- [ ] Log all metrics periodically

### Step 2: Error Recovery
- [ ] If listener exits, attempt reconnect
- [ ] Exponential backoff on reconnect failures
- [ ] Log reconnect attempts

### Step 3: Stress Testing
- [ ] Send 100+ notifications/second
- [ ] Verify no drops, no panics
- [ ] Measure latency under load
- [ ] Check memory growth

### Step 4: Performance Testing
- [ ] Measure end-to-end latency (notification → update)
- [ ] Target: <20ms
- [ ] Memory per connection: ~10KB
- [ ] CPU impact: <1% per notification

### Step 5: Documentation
- [ ] Update CHANGELOG
- [ ] Write docs/MCP_NOTIFICATIONS_USER_GUIDE.md
- [ ] Document for MCP server developers
- [ ] Add code comments for non-obvious sections

### Phase 4 Done ✓
- [ ] Production-ready
- [ ] Metrics working
- [ ] Error recovery implemented
- [ ] Stress tested
- [ ] Documented
- [ ] Merge Phase 4 branch
- [ ] Tag with release version

---

## Code Quality Checklist (Every Phase)

### Compilation
- [ ] `cargo check` passes
- [ ] `cargo build` succeeds
- [ ] No compiler warnings
- [ ] `cargo clippy` passes

### Testing
- [ ] `cargo test` all pass
- [ ] `cargo test mcp_notifications_tests` specific tests pass
- [ ] Test coverage > 90%
- [ ] No flaky tests (run multiple times)

### Code Review
- [ ] No commented-out code
- [ ] No TODOs without context
- [ ] Functions <100 lines
- [ ] Clear variable names
- [ ] Error messages helpful

### Documentation
- [ ] Public functions documented
- [ ] Complex logic commented
- [ ] Examples in doc comments
- [ ] No obvious code smells

---

## Git Workflow

### Before Starting
```bash
git checkout issue-249
git pull origin issue-249
cargo test  # Baseline
```

### During Development
```bash
git checkout -b feature/notifications-phase-1
# Work...
git add crates/cli/src/plugins/notifications.rs
git add crates/cli/tests/mcp_notifications_tests.rs
git add crates/cli/src/plugins/mcp_proxy.rs
git commit -m "feat(mcp): Add notification listener (Phase 1)"
```

### Before PR
```bash
cargo test --all          # All tests
cargo clippy --all        # Linting
cargo fmt --all           # Formatting
git push origin feature/notifications-phase-1
```

---

## Debugging Tips

### Listener Not Reading Notifications
- [ ] Add `tracing::debug!("Listener loop started")` at start
- [ ] Add `tracing::debug!("Read line: {}", line)` in loop
- [ ] Verify server is sending notifications (use separate terminal)
- [ ] Check if stdout is being closed

### Handler Not Called
- [ ] Add `tracing::debug!("Handler called: {:?}", notification_type)`
- [ ] Verify notification JSON is valid
- [ ] Check if `from_method()` matches correctly

### State Not Updating
- [ ] Add `tracing::info!("Acquired lock for server {}", server_id)`
- [ ] Verify refresh request succeeds
- [ ] Check if tools returned from refresh_tools()
- [ ] Verify Arc<Mutex<>> lock isn't deadlocking

### Tests Failing
- [ ] Run with `RUST_LOG=debug cargo test` for more output
- [ ] Use `assert_eq!(actual, expected)` with clear messages
- [ ] Check for async timeout issues
- [ ] Verify mock server behavior matches real server

---

## Success Criteria Per Phase

### Phase 1 Complete When
- [ ] Notification parsing works
- [ ] Listener spawns and reads
- [ ] tools/list_changed refreshes tools
- [ ] Connection stays open
- [ ] All unit + integration tests pass
- [ ] No compiler warnings

### Phase 2 Complete When
- [ ] resources/list_changed works
- [ ] prompts/list_changed works
- [ ] Multiple notifications tested
- [ ] All tests pass
- [ ] No compiler warnings

### Phase 3 Complete When
- [ ] HTTP polling implemented
- [ ] HTTP tests pass
- [ ] Both stdio and HTTP connections work
- [ ] All tests pass
- [ ] No compiler warnings

### Phase 4 Complete When
- [ ] Metrics working
- [ ] Error recovery implemented
- [ ] Stress tested (100+ notifications/second)
- [ ] Performance validated (<20ms latency)
- [ ] Documentation complete
- [ ] All tests pass
- [ ] No compiler warnings

---

## Questions to Ask While Building

1. **"Is this the simplest way?"**
   - If not, simplify or ask architect

2. **"Can I test this in isolation?"**
   - If not, refactor to make it testable

3. **"Will this break existing code?"**
   - If yes, ensure it's backward compatible

4. **"Is there error handling?"**
   - If not, add it (non-fatal by default)

5. **"Does this have logging?"**
   - If not, add debug/info logs

6. **"Would I understand this in 3 months?"**
   - If not, add comments or simplify

---

## Resources

- MCP Protocol: https://spec.modelcontextprotocol.io
- JSON-RPC 2.0: https://www.jsonrpc.org/specification
- Tokio Async: https://tokio.rs/tokio/tutorial
- Serde: https://serde.rs
- Arc<Mutex<>>: https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html

---

## Final Checklist

Before submitting PR:
- [ ] All 4 phases complete (or Phase 1 if stopping early)
- [ ] All tests pass
- [ ] No compiler warnings
- [ ] Code reviewed by self (re-read for quality)
- [ ] CHANGELOG updated
- [ ] Documentation complete
- [ ] Commit messages clear
- [ ] PR description references design docs
- [ ] Ready to merge!

---

**Remember**: Follow the design first. If something doesn't match DESIGN_ISSUE_249.md, stop and clarify with architect before proceeding.

Good luck! Ye be buildin' a fine feature, matey! ⚓

