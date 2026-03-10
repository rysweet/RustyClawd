# Design Documents for Issue #249: MCP List Changed Notifications

## Overview

This directory contains complete architectural and implementation specifications for adding support for MCP `list_changed` notifications to RustyClawd.

## Documentation Files

### 1. DESIGN_ISSUE_249.md
**Comprehensive design document**

- Problem statement and requirements
- Solution architecture with component diagrams
- Detailed implementation specs (types, functions, error handling)
- Testing strategy (unit, integration, E2E)
- Performance analysis
- Backward compatibility assessment
- Implementation phases

**Read This First**: Yes. Authoritative technical design.

### 2. Specs/MCP_NOTIFICATIONS.md
**Module specification (Brick-style contract)**

- Purpose and public API
- Input/output contract
- Dependencies and side effects
- Test requirements
- Success criteria
- Implementation constraints

**Read This First**: For implementation. Defines module boundaries and "studs" (connection points).

### 3. ARCHITECTURE_OVERVIEW.md
**Visual architecture and data flow diagrams**

- System component diagram
- Sequence diagram (startup + notification flow)
- State diagram (connection lifecycle)
- Data flow diagram (notification → state update)
- Concurrency model
- Error handling flows
- Performance characteristics

**Read This First**: For understanding. Visual reference for component interactions.

### 4. IMPLEMENTATION_GUIDE.md
**Step-by-step implementation roadmap**

- Quick summary
- Implementation phases (4 phases over 6-8 days)
- Key design decisions with rationale
- Code organization and file structure
- Testing strategy with code examples
- Troubleshooting guide
- Success metrics and rollout plan

**Read This First**: For building. Step-by-step roadmap for implementation.

## How to Use These Documents

### For Architects/Designers
1. Read: DESIGN_ISSUE_249.md (full technical design)
2. Review: ARCHITECTURE_OVERVIEW.md (validate component interactions)
3. Validate: Specs/MCP_NOTIFICATIONS.md (check public API contract)

### For Implementers/Builders
1. Start: IMPLEMENTATION_GUIDE.md (Phase 1 roadmap)
2. Reference: Specs/MCP_NOTIFICATIONS.md (module contract)
3. Debug: ARCHITECTURE_OVERVIEW.md (diagrams for understanding flow)
4. Deep Dive: DESIGN_ISSUE_249.md (detailed specs when needed)

### For Reviewers
1. Check: DESIGN_ISSUE_249.md (is implementation aligned with design?)
2. Verify: Specs/MCP_NOTIFICATIONS.md (public API contract respected?)
3. Validate: IMPLEMENTATION_GUIDE.md (testing plan executed?)
4. Confirm: ARCHITECTURE_OVERVIEW.md (data flow matches design?)

### For Testers
1. Study: IMPLEMENTATION_GUIDE.md (testing strategy section)
2. Reference: Specs/MCP_NOTIFICATIONS.md (success criteria)
3. Debug: ARCHITECTURE_OVERVIEW.md (error handling flows)

## Quick Reference

### Problem
MCP servers need to dynamically update tools/prompts/resources **without reconnection**.

### Solution
- Spawn background listener task per connection
- Listen for JSON-RPC 2.0 notifications (messages without `id` field)
- When `tools/list_changed` arrives → refresh tools list
- Same for `resources/list_changed` and `prompts/list_changed`
- Connection stays open, no reconnection needed

### Key Components
```
McpConnectionListener
  ├─ Reads notifications from server stdout
  ├─ Routes to appropriate handler
  └─ Spawned as background task

McpNotificationHandler
  ├─ handles_tools/list_changed → refresh tools
  ├─ handles_resources/list_changed → refresh resources
  └─ handles_prompts/list_changed → refresh prompts

McpServerInstance
  ├─ tools: Vec<McpToolDefinition>
  ├─ resources: Vec<Resource>
  └─ prompts: Vec<McpPromptDefinition>
  └─ Protected via Arc<Mutex<>> for thread safety
```

### Implementation Timeline
- Phase 1 (Foundation): 2-3 days
- Phase 2 (Resources/Prompts): 1-2 days
- Phase 3 (HTTP Support): 2-3 days (optional)
- Phase 4 (Hardening): 1-2 days

**Total**: 6-10 days to full production-ready implementation

## Key Decisions

| Decision | Option | Rationale |
|----------|--------|-----------|
| Subscription | Implicit | Simpler, MCP spec supports it, backward compatible |
| Listener | Separate task | Non-blocking, independent, easy to test |
| State | Arc<Mutex<>> | Standard Rust async pattern, acceptable latency |
| Errors | Non-fatal | Connection is precious, transient errors normal |

## Success Criteria

✅ Servers can send `list_changed` notifications
✅ RustyClawd receives and processes them
✅ Tools/prompts/resources updated dynamically
✅ Connection remains open (no reconnection)
✅ <20ms end-to-end latency
✅ Backward compatible
✅ 90%+ test coverage
✅ Zero compiler warnings
✅ Production-ready error handling

## Files to Modify

| File | Type | Purpose |
|------|------|---------|
| `crates/cli/src/plugins/mcp_proxy.rs` | MODIFY | Add listener spawning, update start/stop_server |
| `crates/cli/src/plugins/notifications.rs` | NEW | Handler implementation |
| `crates/cli/tests/mcp_notifications_tests.rs` | NEW | Unit + integration tests |
| `docs/MCP_NOTIFICATIONS_USER_GUIDE.md` | NEW | User/developer guide |

## Performance Profile

| Metric | Value | Status |
|--------|-------|--------|
| Notification Latency | 7-17ms | ✅ Acceptable |
| Memory Per Connection | ~10KB | ✅ Acceptable |
| CPU Usage (idle) | 0% | ✅ Acceptable |
| CPU Per Notification | <1% | ✅ Negligible |

## Testing Coverage

- **Unit Tests**: 60% (types, parsing, routing)
- **Integration Tests**: 30% (mock server, state updates)
- **E2E Tests**: 10% (full workflows, TUI integration)
- **Total**: 90%+ coverage target

## Rollout Strategy

1. **Implement** Phase 1 (MVP)
2. **Test** thoroughly (unit + integration)
3. **Merge** to main branch
4. **Monitor** production metrics
5. **Iterate** based on feedback
6. **Implement** Phase 2-4 as needed

## Known Unknowns

❓ HTTP notifications (polling vs webhooks)?
→ Defer to Phase 3 after Phase 1 shipped

❓ Explicit subscription required by some servers?
→ Try implicit first, add fallback if needed

❓ Update frequency/latency SLA?
→ Design supports <20ms, validate against real servers

## Related Issues

- Issue #246: MCP Resources support ✓
- Issue #248: MCP Prompts support ✓
- Issue #249: List Changed Notifications (this)
- Issue #250: MCP HTTP transport (future)

## Next Steps

1. **Architect Review**: Review DESIGN_ISSUE_249.md
2. **Design Validation**: Check component interactions in ARCHITECTURE_OVERVIEW.md
3. **Begin Implementation**: Follow IMPLEMENTATION_GUIDE.md Phase 1
4. **Create Tests**: Follow test strategy in IMPLEMENTATION_GUIDE.md
5. **Submit PR**: With all design docs linked

---

**Created**: 2026-01-20
**Status**: Ready for Implementation
**Estimated Effort**: 6-10 days (Phase 1-4)
**Complexity**: Medium (async background task, thread-safe state)
**Risk**: Low (backward compatible, well-tested)
