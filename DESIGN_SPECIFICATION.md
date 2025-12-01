# MCP Prompts Capability - Design Specification

## Architecture Overview

This design extends the existing MCP proxy system to support the prompts capability, following the exact same patterns as the tools capability implementation.

## Module Structure

### 1. Data Structures (mcp_proxy.rs)

#### McpPromptDefinition
```rust
/// MCP prompt definition from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptDefinition {
    /// Prompt name (unique identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Optional list of arguments
    #[serde(default)]
    pub arguments: Vec<McpPromptArgument>,
}
```

#### McpPromptArgument
```rust
/// Argument for MCP prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    /// Argument name
    pub name: String,
    /// Argument description
    pub description: String,
    /// Whether this argument is required
    pub required: bool,
}
```

#### McpPromptMessage
```rust
/// Message in a prompt response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptMessage {
    /// Message role (user, assistant, system)
    pub role: String,
    /// Message content (structured JSON)
    pub content: serde_json::Value,
}
```

#### McpPromptResult
```rust
/// Result from prompts/get
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptResult {
    /// Prompt description
    #[serde(default)]
    pub description: Option<String>,
    /// List of messages
    pub messages: Vec<McpPromptMessage>,
}
```

### 2. McpServerInstance Extension

Add prompts storage to track available prompts:

```rust
pub struct McpServerInstance {
    pub definition: McpServerDefinition,
    pub process: Option<TokioChild>,
    pub capabilities: Option<McpCapabilities>,
    pub tools: Vec<McpToolDefinition>,
    pub prompts: Vec<McpPromptDefinition>,  // NEW: prompts storage
}
```

### 3. McpProxy Methods

#### list_prompts_internal
```rust
/// List prompts from a server (internal helper called during start_server)
async fn list_prompts_internal(
    &mut self,
    _server_id: &str,
    child: &mut TokioChild,
) -> Result<Vec<McpPromptDefinition>, String>
```

**Implementation Pattern:**
1. Create RPC request with method "prompts/list"
2. Send to server via stdin
3. Read response from stdout
4. Parse JSON response
5. Extract prompts array from result
6. Return Vec<McpPromptDefinition>

**Error Handling:**
- Serialize errors → "Failed to serialize request"
- Write errors → "Failed to write to MCP server"
- Read errors → "Failed to read prompts list"
- Parse errors → "Failed to parse prompts list response"
- Server errors → "MCP server error listing prompts: {message}"

#### list_prompts
```rust
/// List all available prompts from a server (public API)
pub fn list_prompts(&self, server_id: &str) -> Result<Vec<McpPromptDefinition>, String>
```

**Implementation Pattern:**
1. Get server from HashMap
2. Check if server is running
3. Return clone of prompts vec

**Error Cases:**
- Server not found → "Server not found: {server_id}"
- Server not running → "Server not started: {server_id}"

#### get_prompt
```rust
/// Get a prompt with arguments
pub async fn get_prompt(
    &mut self,
    server_id: &str,
    prompt_name: &str,
    arguments: serde_json::Value,
) -> Result<McpPromptResult, String>
```

**Implementation Pattern:**
1. Get server from HashMap (mut)
2. Get process from server
3. Create RPC request with method "prompts/get"
4. Include prompt name and arguments in params
5. Send to server via stdin
6. Read response from stdout
7. Parse response and extract result
8. Return McpPromptResult

**Error Handling:**
- Server not found → "Server not found: {server_id}"
- Server not running → "Server not started: {server_id}"
- Serialize errors → "Failed to serialize request"
- Write/read errors → "Failed to communicate with MCP server"
- Parse errors → "Failed to parse prompt response"
- Server errors → "MCP prompt error: {message}"

### 4. Server Lifecycle Integration

#### start_server modifications

Add prompts discovery after tools discovery:

```rust
// After existing tools discovery (line ~194)
let prompts = self.list_prompts_internal(server_id, &mut child).await?;

// Store state (line ~197)
let server = self.servers.get_mut(server_id).unwrap();
server.tools = tools;
server.prompts = prompts;  // NEW: store prompts
server.process = Some(child);
```

#### stop_server modifications

Clear prompts when stopping:

```rust
server.capabilities = None;
server.tools.clear();
server.prompts.clear();  // NEW: clear prompts
```

### 5. Command Interface (mcp_commands.rs)

#### prompts command

Add new command handler to McpCommandHandler:

```rust
/// List prompts from a server
pub async fn prompts(&self, server_id: &str) -> McpCommandResult
```

**Output Format:**
```
Prompts from server '{server_id}':
------------------------------------------------------------

  code_review
    Asks the LLM to analyze code quality
    Arguments:
      - code (required)
      - language (optional)

  summarize_text
    Generates a concise summary
    Arguments:
      - text (required)
      - max_length (optional)

------------------------------------------------------------

Total: 2 prompt(s)

Usage: Use 'mcp get-prompt <server-id> <prompt-name>' to retrieve
```

**Error Cases:**
- Server not found → "Server '{server_id}' not found. Use 'mcp list' to see available servers."
- Server not running → "Server '{server_id}' is not running. Start it with: mcp start {server_id}"
- Empty prompts → "Server '{server_id}' has no prompts available"

#### CLI integration (handle_cli_command)

Add case for "prompts" subcommand:

```rust
"prompts" => {
    if args.len() < 2 {
        return Err("Missing server ID. Usage: mcp prompts <server-id>".to_string());
    }
    handler.prompts(&args[1]).await
}
```

#### TUI integration (handle_tui_command)

Add case for "prompts" command:

```rust
"prompts" => {
    if args.is_empty() {
        return Err("Missing server ID. Usage: /mcp-prompts <server-id>".to_string());
    }
    handler.prompts(&args[0]).await
}
```

#### TUI slash command parsing

Add to parse_slash_command test cases:

```rust
#[test]
fn test_parse_slash_command_prompts() {
    let result = parse_slash_command("/mcp-prompts filesystem");
    assert!(result.is_some());
    let (cmd, args) = result.unwrap();
    assert_eq!(cmd, "prompts");
    assert_eq!(args, vec!["filesystem"]);
}
```

## Testing Strategy

### Unit Tests

#### mcp_proxy.rs tests

```rust
#[test]
fn test_prompt_definition_creation() {
    let arg = McpPromptArgument {
        name: "code".to_string(),
        description: "The code to review".to_string(),
        required: true,
    };

    let prompt = McpPromptDefinition {
        name: "code_review".to_string(),
        description: "Review code".to_string(),
        arguments: vec![arg],
    };

    assert_eq!(prompt.name, "code_review");
    assert_eq!(prompt.arguments.len(), 1);
}

#[test]
fn test_server_instance_prompts_initialization() {
    let mut proxy = McpProxy::new();
    let definition = McpServerDefinition {
        id: "test-server".to_string(),
        name: "Test Server".to_string(),
        command: "node".to_string(),
        args: vec![],
        env: HashMap::new(),
        description: None,
    };

    proxy.register_server(definition);
    let server = proxy.servers.get("test-server").unwrap();
    assert_eq!(server.prompts.len(), 0);
}

#[test]
fn test_list_prompts_server_not_running() {
    let mut proxy = McpProxy::new();
    let definition = McpServerDefinition {
        id: "test".to_string(),
        name: "Test".to_string(),
        command: "node".to_string(),
        args: vec![],
        env: HashMap::new(),
        description: None,
    };

    proxy.register_server(definition);
    let result = proxy.list_prompts("test");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not started"));
}
```

#### mcp_commands.rs tests

```rust
#[test]
fn test_parse_slash_command_prompts() {
    let result = parse_slash_command("/mcp-prompts filesystem");
    assert!(result.is_some());
    let (cmd, args) = result.unwrap();
    assert_eq!(cmd, "prompts");
    assert_eq!(args, vec!["filesystem"]);
}

#[test]
fn test_parse_slash_command_prompts_no_args() {
    let result = parse_slash_command("/mcp-prompts");
    assert!(result.is_some());
    let (cmd, args) = result.unwrap();
    assert_eq!(cmd, "prompts");
    assert!(args.is_empty());
}
```

### Integration Tests

Create new test module in mcp_proxy.rs:

```rust
#[cfg(test)]
mod prompts_integration_tests {
    use super::*;
    use tokio::test;

    // Mock MCP server that responds to prompts/list
    async fn create_mock_prompts_server() -> (TokioChild, McpProxy) {
        // Implementation similar to mock tools server
        // Responds to:
        // 1. initialize request
        // 2. prompts/list request
        // 3. prompts/get request
    }

    #[test]
    async fn test_prompts_list_integration() {
        // Start mock server
        // Send prompts/list
        // Verify response contains expected prompts
        // Verify prompts have correct structure
    }

    #[test]
    async fn test_prompts_get_integration() {
        // Start mock server
        // Send prompts/get with arguments
        // Verify response contains messages
        // Verify arguments were substituted
    }

    #[test]
    async fn test_prompts_get_missing_required_arg() {
        // Start mock server
        // Send prompts/get without required argument
        // Verify error response
    }
}
```

## Implementation Order

1. **Phase 1: Data Structures** (30 min)
   - Add all structs to mcp_proxy.rs
   - Add prompts field to McpServerInstance
   - Update initialization in register_server

2. **Phase 2: Core RPC Methods** (1.5 hours)
   - Implement list_prompts_internal
   - Implement list_prompts
   - Implement get_prompt
   - Add to start_server/stop_server

3. **Phase 3: Command Interface** (1 hour)
   - Add prompts method to McpCommandHandler
   - Add CLI case to handle_cli_command
   - Add TUI case to handle_tui_command
   - Add slash command parsing test

4. **Phase 4: Testing** (1.5 hours)
   - Write unit tests for data structures
   - Write unit tests for command parsing
   - Write integration tests with mock server
   - Manual testing with real MCP server

5. **Phase 5: Documentation** (30 min)
   - Update command help text
   - Add usage examples
   - Update README if needed

## Error Handling Strategy

Follow existing patterns exactly:

1. **User-Facing Errors**: Clear, actionable messages
   - "Server 'X' not found. Use 'mcp list' to see available servers."
   - "Server 'X' is not running. Start it with: mcp start X"

2. **Technical Errors**: Include context
   - "Failed to serialize request: {error details}"
   - "MCP server error listing prompts: {server error message}"

3. **Return Type**: Result<T, String> for consistency

## Philosophy Compliance

### Ruthless Simplicity
- ✓ Follow existing tools pattern exactly
- ✓ No unnecessary abstractions
- ✓ Direct, clear implementations

### Zero-BS Implementation
- ✓ No stubs or TODOs
- ✓ Every method fully implemented
- ✓ No placeholder error messages

### Modular Design
- ✓ Clear module boundaries (proxy vs commands)
- ✓ Self-contained data structures
- ✓ Testable in isolation

### Bricks & Studs
- ✓ Public API clearly defined
- ✓ Internal methods private
- ✓ Regeneratable from this spec

## Integration with Existing Code

### Lines to Modify

**mcp_proxy.rs:**
- Line 24: Add McpPromptDefinition struct
- Line 38: Add McpPromptArgument struct
- Line 46: Add McpPromptMessage struct
- Line 54: Add McpPromptResult struct
- Line 23 (McpServerInstance): Add prompts field
- Line 105 (register_server): Initialize prompts Vec
- Line 194 (start_server): Call list_prompts_internal
- Line 219 (stop_server): Clear prompts
- Line 280 (after list_tools_internal): Add list_prompts_internal
- Line 295 (after list_tools): Add list_prompts
- Line 365 (after call_tool): Add get_prompt
- Line 450 (tests): Add prompts unit tests

**mcp_commands.rs:**
- Line 175 (after tools method): Add prompts method
- Line 260 (handle_cli_command): Add "prompts" case
- Line 329 (handle_tui_command): Add "prompts" case
- Line 374 (tests): Add prompts slash command tests

### No Breaking Changes
- All additions are new methods/fields
- Existing tools functionality unchanged
- Backward compatible

## Success Criteria Mapping

| Requirement | Implementation | Validation |
|------------|----------------|------------|
| prompts/list RPC | list_prompts_internal | Integration test |
| prompts/get RPC | get_prompt | Integration test |
| Variable substitution | In get_prompt params | Integration test |
| Structured data | McpPromptDefinition | Unit tests |
| Integration tests | prompts_integration_tests module | Run cargo test |
| CLI command | mcp prompts | Manual test |
| TUI command | /mcp-prompts | Manual test |
| Error handling | Result<T, String> pattern | All tests |
| Test coverage > 80% | Unit + integration tests | cargo tarpaulin |

## Risk Mitigation

### Risk: RPC format mismatch
**Mitigation**: Follow MCP spec exactly, validate with integration tests

### Risk: Argument substitution complexity
**Mitigation**: Server handles substitution, we just pass arguments through

### Risk: Breaking existing tools
**Mitigation**: Only additive changes, run all existing tests

### Risk: Performance impact
**Mitigation**: Prompts discovered once during start_server, cached like tools

## Estimated Timeline

Total: 4-6 hours

- Data structures: 30 min
- RPC methods: 1.5 hours
- Commands: 1 hour
- Testing: 1.5 hours
- Documentation: 30 min
- Buffer: 30 min

## References

- MCP Prompts Specification: https://spec.modelcontextprotocol.io/specification/2024-11-05/server/prompts/
- Existing tools implementation: crates/cli/src/plugins/mcp_proxy.rs (lines 224-280)
- Command handler pattern: crates/cli/src/mcp_commands.rs (lines 123-175)
