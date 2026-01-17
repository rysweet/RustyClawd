# Tool Use Examples

Complete guide to RustyClawd's tool use capabilities with working code examples.

## Quick Reference

| Pattern | Status | Test Location |
|---------|--------|---------------|
| Single tool call | ✅ Complete | tool_use_tests.rs:1137-1178 |
| Multiple tools in one call | ✅ Complete | sdk_compliance_tests.rs:850-885 |
| Parallel tool execution | ✅ Complete | sdk_compliance_tests.rs:846-916 |
| Sequential tool chains | ✅ Complete | sdk_compliance_tests.rs:1772-1781 |
| Tool choice modes | ✅ Complete | types.rs:79-110 |
| Error handling | ✅ Complete | tool_use_tests.rs:986-1126 |
| Stop reasons | ✅ Complete | sdk_compliance_tests.rs:1131-1209 |
| Strict schema validation | ✅ Complete | strict_json_schema_validation.md |
| Extended thinking | ✅ Complete | examples/extended_thinking.rs |

## Single Tool Call

The simplest tool use pattern - define a tool, register it, execute it.

### Example: Weather Tool

```rust
use rustyclawd_core::client::*;
use serde_json::json;

// 1. Define tool
let mut props = HashMap::new();
props.insert("location".to_string(), json!({ "type": "string" }));
let schema = InputSchema::object(props, vec!["location".to_string()]);

let tool_def = ToolDefinition::new(
    "get-weather",
    "Retrieves current weather conditions for a specified location with temperature, humidity, and forecast data",
    schema,
).unwrap();

// 2. Register tool
let mut registry = ToolRegistry::new();
let executor = |input: Value| {
    if let Some(location) = input.get("location") {
        ToolExecutionResult::Success(json!({
            "location": location,
            "temperature": 72,
            "conditions": "Sunny"
        }))
    } else {
        ToolExecutionResult::Error("Missing location".to_string())
    }
};

registry.register(tool_def, executor).unwrap();

// 3. Execute tool
let result = registry.execute("get-weather", json!({"location": "San Francisco"}));

// 4. Verify result
match result.unwrap() {
    ToolExecutionResult::Success(output) => {
        assert_eq!(output.get("location"), Some(&json!("San Francisco")));
        assert_eq!(output.get("temperature"), Some(&json!(72)));
    }
    _ => panic!("Expected success"),
}
```

**Test Evidence**: `tool_use_tests.rs:test_full_tool_lifecycle` (lines 1137-1178)

## Multiple Tools in Single API Call

Claude can use multiple tools in one response. All tool definitions are passed together.

### Example: Multiple Tool Definitions

```rust
use rustyclawd_core::client::*;

let tools = vec![
    ToolDefinition {
        name: "bash".to_string(),
        description: "Execute bash commands".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "command": {"type": "string"}
            },
            "required": ["command"]
        }),
    },
    ToolDefinition {
        name: "read".to_string(),
        description: "Read file contents".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string"}
            },
            "required": ["file_path"]
        }),
    },
];

let request = CreateMessageRequest::new(
    "claude-sonnet-4-5-20250929",
    vec![Message::user("List and read files")],
    1024,
).with_tools(tools);

// Request now includes both tools
assert!(request.tools.is_some());
assert_eq!(request.tools.unwrap().len(), 2);
```

**Test Evidence**: `tool_use_test.rs:test_create_message_request_with_tools`

## Parallel Tool Use

Claude can invoke multiple tools in a single response turn. All results must be returned in one user message.

### Example: Parallel Weather Checks

```rust
use rustyclawd_core::client::*;

// Claude's response with multiple tool_use blocks
let blocks = vec![
    ContentBlock::Text {
        text: "I'll check multiple sources".to_string(),
    },
    ContentBlock::ToolUse {
        id: "toolu_1".to_string(),
        name: "get_weather".to_string(),
        input: json!({"location": "NYC"}),
    },
    ContentBlock::ToolUse {
        id: "toolu_2".to_string(),
        name: "get_weather".to_string(),
        input: json!({"location": "SF"}),
    },
    ContentBlock::ToolUse {
        id: "toolu_3".to_string(),
        name: "get_news".to_string(),
        input: json!({"topic": "weather"}),
    },
];

let msg = Message::with_blocks(Role::Assistant, blocks);

// Verify multiple tool_use blocks
match msg.content {
    MessageContent::Blocks(blocks) => {
        let tool_use_count = blocks
            .iter()
            .filter(|b| matches!(b, ContentBlock::ToolUse { .. }))
            .count();
        assert_eq!(tool_use_count, 3);
    }
    _ => panic!("Expected blocks content"),
}
```

### Example: Returning Parallel Results

```rust
// All tool results MUST be returned in a single user message
let results = vec![
    ContentBlock::ToolResult {
        tool_use_id: "toolu_1".to_string(),
        content: vec![ContentBlock::Text {
            text: "NYC: 70°F".to_string(),
        }],
        is_error: None,
    },
    ContentBlock::ToolResult {
        tool_use_id: "toolu_2".to_string(),
        content: vec![ContentBlock::Text {
            text: "SF: 65°F".to_string(),
        }],
        is_error: None,
    },
    ContentBlock::ToolResult {
        tool_use_id: "toolu_3".to_string(),
        content: vec![ContentBlock::Text {
            text: "News: Sunny today".to_string(),
        }],
        is_error: None,
    },
];

let msg = Message::with_blocks(Role::User, results);
assert_eq!(msg.role, Role::User);
```

**Test Evidence**:
- `sdk_compliance_tests.rs SECTION 8: PARALLEL TOOL USE` (lines 846-916)
- `test_parallel_tool_use_multiple_tool_use_blocks`
- `test_parallel_tool_use_multiple_results_in_one_message`

## Sequential Tool Execution

Later tools can depend on results from earlier tools in the conversation.

### Example: Search Then Read

```rust
use rustyclawd_core::client::*;

let conversation = vec![
    // User request
    Message::user("Find and read the config file"),

    // Claude first searches
    Message::with_blocks(
        Role::Assistant,
        vec![ContentBlock::ToolUse {
            id: "t1".to_string(),
            name: "glob".to_string(),
            input: json!({"pattern": "*.config"}),
        }],
    ),

    // User returns search results
    Message::with_blocks(
        Role::User,
        vec![ContentBlock::ToolResult {
            tool_use_id: "t1".to_string(),
            content: vec![ContentBlock::Text {
                text: "app.config".to_string(),
            }],
            is_error: None,
        }],
    ),

    // Claude then reads the file
    Message::with_blocks(
        Role::Assistant,
        vec![ContentBlock::ToolUse {
            id: "t2".to_string(),
            name: "read".to_string(),
            input: json!({"file_path": "app.config"}),
        }],
    ),
];

// Verify conversation structure
assert_eq!(conversation.len(), 4);
```

**Test Evidence**: `sdk_compliance_tests.rs SECTION 22: SEQUENTIAL TOOL EXECUTION` (lines 1772-1781)

## Tool Choice Modes

Control whether and which tools Claude uses.

### Example: Auto Mode (Claude Decides)

```rust
use rustyclawd_core::client::*;

let choice = ToolChoice::auto();
let json_str = serde_json::to_string(&choice).unwrap();
assert!(json_str.contains("auto"));

// Claude decides whether to use tools
let request = CreateMessageRequest::new(
    "claude-sonnet-4-5-20250929",
    vec![Message::user("What is 2+2?")],  // No tools needed
    1024,
).with_tool_choice(choice);
```

### Example: Any Mode (Must Use Tool)

```rust
let choice = ToolChoice::any();
let json_str = serde_json::to_string(&choice).unwrap();
assert!(json_str.contains("any"));

// Claude MUST use a tool
let request = CreateMessageRequest::new(
    "claude-sonnet-4-5-20250929",
    vec![Message::user("Search for Rust documentation")],
    1024,
).with_tool_choice(choice)
 .with_tools(vec![search_tool]);
```

### Example: Specific Tool Mode

```rust
let choice = ToolChoice::tool("calculator");
let json_str = serde_json::to_string(&choice).unwrap();
assert!(json_str.contains("tool"));
assert!(json_str.contains("calculator"));

// Claude MUST use the "calculator" tool
let request = CreateMessageRequest::new(
    "claude-sonnet-4-5-20250929",
    vec![Message::user("Calculate 15 * 23")],
    1024,
).with_tool_choice(choice)
 .with_tools(vec![calculator_tool]);
```

**Test Evidence**:
- `types.rs:79-110` - ToolChoice enum implementation
- `tool_use_test.rs:test_tool_choice_auto`
- `tool_use_test.rs:test_tool_choice_any`
- `tool_use_test.rs:test_tool_choice_specific`

## Stop Reasons

The `stop_reason` field indicates why Claude stopped generating.

### Example: End Turn

```rust
let response = MessageResponse {
    id: "msg_1".to_string(),
    type_field: "message".to_string(),
    role: Role::Assistant,
    content: vec![ContentBlock::Text {
        text: "Done".to_string(),
    }],
    model: "claude-3-5-sonnet-20241022".to_string(),
    stop_reason: Some("end_turn".to_string()),
    stop_sequence: None,
    usage: Usage {
        input_tokens: 10,
        output_tokens: 5,
    },
};

assert_eq!(response.stop_reason, Some("end_turn".to_string()));
```

### Example: Tool Use

```rust
let response = MessageResponse {
    stop_reason: Some("tool_use".to_string()),
    content: vec![ContentBlock::ToolUse {
        id: "t1".to_string(),
        name: "calculator".to_string(),
        input: json!({"expression": "2+2"}),
    }],
    // ... other fields
};

assert_eq!(response.stop_reason, Some("tool_use".to_string()));
```

### Example: Max Tokens

```rust
let response = MessageResponse {
    stop_reason: Some("max_tokens".to_string()),
    content: vec![ContentBlock::Text {
        text: "Long text that was cut off...".to_string(),
    }],
    usage: Usage {
        input_tokens: 100,
        output_tokens: 1024,  // Hit the limit
    },
    // ... other fields
};

assert_eq!(response.stop_reason, Some("max_tokens".to_string()));
```

### Example: Stop Sequence

```rust
let response = MessageResponse {
    stop_reason: Some("stop_sequence".to_string()),
    stop_sequence: Some("\n---\n".to_string()),
    content: vec![ContentBlock::Text {
        text: "Content before stop sequence".to_string(),
    }],
    // ... other fields
};

assert_eq!(response.stop_reason, Some("stop_sequence".to_string()));
assert_eq!(response.stop_sequence, Some("\n---\n".to_string()));
```

**Test Evidence**: `sdk_compliance_tests.rs SECTION 12: STOP REASONS` (lines 1131-1209)

## Error Handling

Comprehensive error handling for tool execution failures.

### Example: Invalid Input Error

```rust
use rustyclawd_core::client::*;

let mut registry = ToolRegistry::new();

// Tool requires "temperature" parameter
let mut props = HashMap::new();
props.insert("temperature".to_string(), json!({ "type": "number" }));
let schema = InputSchema::object(props, vec!["temperature".to_string()]);

let tool_def = ToolDefinition::new(
    "validate-input",
    "Tool that validates input parameters with comprehensive error checking",
    schema,
).unwrap();

let executor = |input: Value| {
    if let Some(_temp) = input.get("temperature") {
        ToolExecutionResult::Success(json!({"valid": true}))
    } else {
        ToolExecutionResult::Error("Missing required parameter: temperature".to_string())
    }
};

registry.register(tool_def, executor).unwrap();

// Call without required parameter
let result = registry.execute("validate-input", json!({}));

match result.unwrap() {
    ToolExecutionResult::Error(msg) => {
        assert!(msg.contains("temperature"));
    }
    _ => panic!("Expected error"),
}
```

### Example: Timeout Simulation

```rust
let executor = |_input: Value| {
    ToolExecutionResult::Error("Timeout: Request exceeded 30 seconds".to_string())
};

registry.register(timeout_tool, executor).unwrap();

let result = registry.execute("timeout-tool", json!({}));
match result.unwrap() {
    ToolExecutionResult::Error(msg) => {
        assert!(msg.contains("Timeout"));
    }
    _ => panic!("Expected error"),
}
```

### Example: Retry on Error

```rust
// Simulate Claude's retry logic: 2-3 retries
for attempt in 0..3 {
    let result = registry.execute("retry-tool", json!({}));

    match result.unwrap() {
        ToolExecutionResult::Error(msg) => {
            if attempt < 2 {
                // Could retry
                println!("Attempt {} failed: {}", attempt, msg);
            } else {
                // Final failure
                return Err(msg);
            }
        }
        ToolExecutionResult::Success(data) => {
            return Ok(data);
        }
    }
}
```

### Example: Malformed Tool Result

```rust
// Try to add result with mismatched ID
let mut content = ContentArray::new();
let tool_use = ToolUseBlock {
    id: "call_1".to_string(),
    name: "test".to_string(),
    input: json!({}),
};

content.add_tool_use(tool_use).unwrap();

// Mismatched ID - should fail
let mismatched_result = ToolResultBlock::success("call_2", json!({}));
assert!(content.add_tool_result(mismatched_result).is_err());
```

**Test Evidence**: `tool_use_tests.rs INTEGRATION TESTS - Error Handling` (lines 986-1126)

## Tool Result Formatting

Tool results must follow specific formatting rules.

### Example: Success Result

```rust
let result = ToolResultBlock::success("call_123", json!({"temperature": 72}));

assert_eq!(result.tool_use_id, "call_123");
assert_eq!(result.is_error, Some(false));
```

### Example: Error Result

```rust
let result = ToolResultBlock::error("call_123", "Connection timeout");

assert_eq!(result.tool_use_id, "call_123");
assert_eq!(result.is_error, Some(true));
assert_eq!(result.content, Value::String("Connection timeout".to_string()));
```

### Example: Complex Content

```rust
let content = json!({
    "status": "success",
    "data": {
        "temperature": 72,
        "humidity": 65,
        "conditions": "Sunny"
    }
});

let result = ToolResultBlock::success("call_789", content.clone());
assert_eq!(result.content, content);
```

## Tool Definition Best Practices

### Descriptive Names

Tool names must match regex: `^[a-zA-Z0-9_-]{1,64}$`

```rust
// Valid names
assert!(ToolName::new("weather-api").is_ok());
assert!(ToolName::new("get_temperature").is_ok());
assert!(ToolName::new("HTTP2Client").is_ok());

// Invalid names
assert!(ToolName::new("weather api").is_err());  // space
assert!(ToolName::new("weather.api").is_err());  // dot
```

### Detailed Descriptions

Descriptions must be at least 20 characters and describe functionality clearly.

```rust
// Too short - fails
let result = ToolDefinition::new("tool", "Short", schema);
assert!(result.is_err());

// Good description - passes
let result = ToolDefinition::new(
    "get-weather",
    "Retrieves weather information for a given location. Returns temperature, humidity, and forecast.",
    schema,
);
assert!(result.is_ok());
```

### Complete Schemas

```rust
let mut props = HashMap::new();
props.insert("query".to_string(), json!({
    "type": "string",
    "description": "The search query"
}));
props.insert("limit".to_string(), json!({
    "type": "number",
    "description": "Maximum results to return"
}));

let schema = InputSchema::object(props, vec!["query".to_string()]);

let tool_def = ToolDefinition::new(
    "search",
    "Performs a comprehensive search across documents. Returns matching results ranked by relevance.",
    schema,
).unwrap();
```

## Strict Schema Validation

**Status**: ✅ Fully implemented

RustyClawd supports Anthropic's strict JSON schema validation for tool inputs.

### Example: Strict Tool Definition

```rust
use rustyclawd_core::client::*;

// Define strict schema with additionalProperties: false
let schema = json!({
    "type": "object",
    "properties": {
        "location": {
            "type": "string",
            "description": "City and state"
        },
        "unit": {
            "type": "string",
            "enum": ["celsius", "fahrenheit"]
        }
    },
    "required": ["location"],
    "additionalProperties": false  // Reject extra fields
});

// Create tool with strict validation enabled
let tool = ToolDefinition::new(
    "get_weather",
    "Get the current weather in a location",
    schema
).with_strict(true);

// Automatic beta header management
let request = CreateMessageRequest::new(
    "claude-sonnet-4-5",
    vec![Message::user("What's the weather in Tokyo?")],
    1024,
).with_tools(vec![tool]);

// The beta header is automatically added when needed
let response = client.create_message(request).await?;
```

### Key Features

- **Opt-in per tool**: Use `.with_strict(true)` on any ToolDefinition
- **Automatic beta headers**: `anthropic-beta: structured-outputs-2025-11-13` added automatically
- **Mixed mode support**: Can mix strict and lenient tools in same request
- **Type safety**: Guaranteed schema compliance at API level
- **Backward compatible**: Existing code works without changes (defaults to lenient)

### When to Use Strict Mode

- **Use strict** for: financial transactions, configuration changes, production systems
- **Use lenient** for: prototyping, exploration, backward compatibility

**Complete documentation**: See `docs/strict_json_schema_validation.md` for full examples and best practices.

## Extended Thinking (ContentBlock::Thinking)

**Status**: ✅ Fully implemented (Issue #130)

RustyClawd supports Claude's extended thinking capability, allowing you to see Claude's internal reasoning process.

### Example: Extended Thinking

```rust
use rustyclawd_core::client::*;

// Extended thinking is enabled automatically when model supports it
let request = CreateMessageRequest::new(
    "claude-sonnet-4-5-20250929",  // Supports extended thinking
    vec![Message::user("Solve this complex problem...")],
    4096,
);

let response = client.create_message(request).await?;

// Process thinking blocks
for (i, block) in response.content.iter().enumerate() {
    match block {
        ContentBlock::Thinking { thinking, signature } => {
            println!("--- [Block {}]: THINKING PROCESS ---", i);
            if let Some(sig) = signature {
                println!("Signature: {}...\n", &sig[..sig.len().min(32)]);
            }
            println!("{}\n", thinking);
        }
        ContentBlock::Text { text } => {
            println!("--- [Block {}]: FINAL ANSWER ---", i);
            println!("{}\n", text);
        }
        _ => {}
    }
}
```

### Key Features

- **ContentBlock::Thinking variant**: Captures Claude's reasoning process
- **Signature field**: Optional cryptographic signature for thinking authenticity
- **Streaming support**: Thinking blocks stream in real-time
- **Non-streaming support**: Full thinking content in response
- **Model detection**: Automatically enabled for compatible models

### When to Use Extended Thinking

Extended thinking provides visibility into Claude's reasoning for:
- Complex problem solving
- Multi-step analysis
- Debugging Claude's decision process
- Understanding model behavior
- Educational demonstrations

**Complete example**: See `crates/core/examples/extended_thinking.rs` for working demonstration.

## Test Coverage Summary

| Feature | Tests | Lines |
|---------|-------|-------|
| Schema validation | 11 tests | tool_use_tests.rs:316-412 |
| Tool use blocks | 7 tests | tool_use_tests.rs:418-481 |
| Content array validation | 10 tests | tool_use_tests.rs:487-612 |
| Tool registry | 7 tests | tool_use_tests.rs:618-748 |
| Execution flow | 5 tests | tool_use_tests.rs:754-942 |
| Tool choice control | 4 tests | tool_use_tests.rs:948-979 |
| Error handling | 6 tests | tool_use_tests.rs:986-1126 |
| E2E lifecycle | 6 tests | tool_use_tests.rs:1133-1312 |
| Edge cases | 4 tests | tool_use_tests.rs:1318-1445 |
| Parallel use | 3 tests | sdk_compliance_tests.rs:846-916 |
| Sequential execution | 1 test | sdk_compliance_tests.rs:1772-1781 |
| Stop reasons | 4 tests | sdk_compliance_tests.rs:1131-1209 |

**Total**: 68 comprehensive tests covering all tool use patterns.

## How to Verify

### Run All Tool Tests

```bash
cd /home/azureuser/src/RustyClawd

# Core tool use tests
cargo test --package rustyclawd-tools --lib tool_use_tests

# SDK compliance tests
cargo test --package rustyclawd-core --lib sdk_compliance_tests

# Integration tests
cargo test --package rustyclawd-core --lib tool_use_test
```

### Run Specific Pattern Tests

```bash
# Parallel tool use
cargo test --package rustyclawd-core test_parallel_tool_use

# Sequential execution
cargo test --package rustyclawd-core test_sequential_tool_calls

# Error handling
cargo test --package rustyclawd-tools test_tool_retry_on_error

# Stop reasons
cargo test --package rustyclawd-core test_stop_reason
```

### Manual Testing

```bash
# Build RustyClawd
cargo build --release

# Test tool use with actual API
./target/release/rusty "create a file test.txt with 'hello world'"

# Verify tool was called
cat test.txt  # Should show "hello world"
```

## References

- [Claude Tool Use Documentation](https://docs.claude.com/en/docs/agents-and-tools/tool-use)
- [Parallel Tool Use Guide](https://docs.claude.com/en/docs/agents-and-tools/tool-use#parallel-tool-use)
- RustyClawd source: `crates/core/src/client/types.rs`
- Test suite: `crates/tools/tests/tool_use_tests.rs`
- SDK tests: `crates/core/tests/sdk_compliance_tests.rs`
