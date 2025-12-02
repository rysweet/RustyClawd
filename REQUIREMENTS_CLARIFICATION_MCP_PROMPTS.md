# MCP Prompts Capability Implementation - Clarified Requirements

## Feature Request: MCP Prompts Support (GAP-MCP-2)

### Objective

Implement Model Context Protocol (MCP) prompts capability in RustyClawd, allowing MCP servers to expose prompt templates to clients with dynamic argument substitution following the same patterns as the existing tools capability.

### Requirements

**Functional Requirements:**

1. **prompts/list RPC Method**
   - Discover all available prompts from an MCP server
   - Return list of prompt definitions with metadata
   - Support optional pagination via cursor parameter
   - Each prompt includes: name, description, optional list of arguments

2. **prompts/get RPC Method**
   - Retrieve specific prompt by name
   - Accept arguments for template variable substitution
   - Return structured prompt messages ready for LLM consumption
   - Support multiple message types (text, image, etc.)

3. **Data Structures**
   - `McpPromptDefinition`: name, description, arguments list
   - `McpPromptArgument`: name, description, required flag
   - `McpPromptMessage`: role, content (text/image/resource)
   - Integrate with existing `McpCapabilities` structure

4. **Integration with Existing System**
   - Add prompts field to `McpCapabilities` struct (already exists)
   - Add prompts storage to `McpServerInstance` struct
   - Implement `list_prompts_internal()` similar to `list_tools_internal()`
   - Implement `get_prompt()` similar to `call_tool()`
   - Add to MCP command handler for CLI/TUI access

**Non-Functional Requirements:**

- Maintain existing error handling patterns (Result<T, String>)
- Follow existing JSON-RPC infrastructure
- Zero-BS implementation (no stubs, no TODOs)
- Use only standard library + existing dependencies (serde, tokio)
- Philosophy compliant: ruthless simplicity, clear module boundaries

### User Story

As a RustyClawd user
I want to discover and use prompt templates from MCP servers
So that I can leverage pre-defined, parameterized prompts for common LLM interactions

### Acceptance Criteria

- [ ] `prompts/list` request sent to MCP server returns structured prompt definitions
- [ ] `prompts/get` request with arguments returns formatted prompt messages
- [ ] Prompts are discovered during MCP server initialization
- [ ] Prompts are accessible via CLI commands (`claude mcp prompts <server-id>`)
- [ ] Prompts are accessible via TUI commands (`/mcp-prompts <server-id>`)
- [ ] Integration tests validate prompts/list and prompts/get with mock server
- [ ] Error messages match existing pattern (user-friendly, actionable)
- [ ] Test coverage > 80%
- [ ] All unit tests pass
- [ ] Philosophy compliance validated

### Technical Considerations

**Architecture Impacts:**
- Extends `McpProxy` with prompts methods (2 new methods)
- Extends `McpCommandHandler` with prompts command (1 new command)
- Extends `McpServerInstance` with prompts storage (1 new field)
- No breaking changes to existing tool

s functionality

**Dependencies:**
- No new external dependencies
- Uses existing: serde, tokio, serde_json
- Leverages existing JSON-RPC request/response infrastructure

**Integration Points:**
- `mcp_proxy.rs`: Add prompts methods to McpProxy impl
- `mcp_commands.rs`: Add prompts command to McpCommandHandler
- Must follow same init sequence as tools (call during start_server)

**Code Locations:**
- Data structures: `crates/cli/src/plugins/mcp_proxy.rs` (lines 25-50 area)
- RPC methods: `crates/cli/src/plugins/mcp_proxy.rs` (add after line 280)
- CLI commands: `crates/cli/src/mcp_commands.rs` (add after line 175)
- Tests: `crates/cli/src/plugins/mcp_proxy.rs` (extend after line 408)

### Implementation Pattern (Based on Existing Tools Code)

**Step 1: Add Data Structures**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptDefinition {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub arguments: Vec<McpPromptArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptMessage {
    pub role: String,
    pub content: serde_json::Value,
}
```

**Step 2: Extend McpServerInstance**
```rust
pub struct McpServerInstance {
    // ... existing fields ...
    pub prompts: Vec<McpPromptDefinition>,  // Add this
}
```

**Step 3: Add McpProxy Methods** (mirror tools pattern)
- `list_prompts_internal()` - called during start_server
- `list_prompts()` - public method for clients
- `get_prompt()` - retrieve prompt with arguments

**Step 4: Add CLI Command**
- Add `prompts` subcommand to McpCommandHandler
- Format output similar to tools command
- Add TUI slash command `/mcp-prompts`

**Step 5: Integration Tests**
- Mock MCP server responding to prompts/list
- Mock MCP server responding to prompts/get with arguments
- Validate argument substitution works correctly
- Test error cases (server not running, prompt not found, missing required args)

### Complexity: Medium

**Reasoning:**
- 2-3 files affected (mcp_proxy.rs, mcp_commands.rs, tests)
- Clear existing pattern to follow (tools capability)
- Well-defined MCP specification
- Standard testing required
- Low risk (additive feature, no breaking changes)
- No data migration needed

**Estimated Effort:** 4-6 hours

**Breakdown:**
- Data structures: 30 min
- McpProxy methods: 1.5 hours
- CLI commands: 1 hour
- Integration tests: 1.5 hours
- Manual testing: 30 min
- Documentation: 30 min

### Testing Requirements

**Unit Tests:**
- [ ] McpPromptDefinition serialization/deserialization
- [ ] prompts field initialization in McpServerInstance
- [ ] list_prompts returns empty vec for non-running server
- [ ] CLI command parsing for prompts subcommand

**Integration Tests:**
- [ ] prompts/list request-response cycle with mock server
- [ ] prompts/get request-response with argument substitution
- [ ] Error handling for missing required arguments
- [ ] Error handling for prompt not found
- [ ] Prompts discovered during server start
- [ ] Prompts cleared during server stop

**Manual Tests:**
- [ ] Start MCP server with prompts capability
- [ ] Run `claude mcp prompts <server-id>` - see prompts list
- [ ] Use `/mcp-prompts <server-id>` in TUI
- [ ] Verify error messages are user-friendly

### Risk Assessment

**Low Risk Feature:**
- Additive only (no modifications to existing code)
- Clear specification to follow
- Existing tools pattern provides blueprint
- No external service dependencies
- Isolated to MCP subsystem

**Mitigation Strategies:**
- Follow existing patterns exactly
- Comprehensive tests before commit
- Manual testing with real MCP server
- Philosophy compliance review

### Success Criteria

- [ ] All existing tests pass
- [ ] New tests added and passing (8+ test cases)
- [ ] No performance degradation
- [ ] Code coverage maintained (>80%)
- [ ] Documentation updated (MCP command help text)
- [ ] Philosophy compliance validated (zero-BS, ruthless simplicity)
- [ ] PR approved and merged

### Explicit User Requirements (MUST PRESERVE)

These requirements CANNOT be optimized away or simplified:

1. Implement `prompts/list` RPC method
2. Implement `prompts/get` RPC method with arguments
3. Support prompt templates with variable substitution
4. Return structured prompt data (name, description, arguments)
5. Add integration tests with mock MCP server

### Quality Validation

**Completeness Check:**
- [x] Objective clearly stated
- [x] All required sections filled
- [x] Acceptance criteria measurable
- [x] Technical context provided
- [x] Complexity assessed
- [x] Risks identified
- [x] Testing approach defined

**Clarity Check:**
- [x] No ambiguous terms
- [x] Concrete examples provided (JSON structures)
- [x] Technical terms defined (MCP, RPC, prompts)
- [x] Success is measurable (test coverage, passing tests)

**Consistency Check:**
- [x] No contradictory requirements
- [x] Scope clearly bounded (prompts only, no other MCP features)
- [x] Dependencies identified (existing infrastructure)
- [x] Timeline realistic for complexity (4-6 hours for Medium)

**Quality Score: 95%**

### Recommendations

**Next Steps:**
1. Architect review (design validation)
2. Builder agent implementation
3. Tester agent for comprehensive test suite
4. Manual testing with real MCP server
5. Reviewer agent for philosophy compliance

**Review Needed:** No (straightforward feature, clear pattern exists)

**Break Down Suggested:** No (Medium complexity, manageable scope)

### Sources

- [Model Context Protocol Prompts Specification](https://spec.modelcontextprotocol.io/specification/2024-11-05/server/prompts/)
- [MCP Prompts Documentation](https://modelcontextprotocol.io/docs/concepts/prompts)
- Existing RustyClawd tools implementation (reference pattern)
