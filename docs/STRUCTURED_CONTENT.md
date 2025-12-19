# Structured Content Support (MCP spec)

## Overview

RustyClawd supports the MCP (Model Context Protocol) specification for structured content in tool results. Tool results can now return arrays of content blocks instead of just plain strings, enabling richer responses with text, images, and other content types.

## ContentBlock::ToolResult Schema

The `ContentBlock::ToolResult` variant now accepts structured content:

```rust
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Vec<ContentBlock>,  // Changed from String
        is_error: Option<bool>,
    },
}
```

## Usage Examples

### Simple Text Response (Backward Compatible)

Tools can still return simple text responses by wrapping them in a Text block:

```rust
ContentBlock::ToolResult {
    tool_use_id: "tool_123".to_string(),
    content: vec![ContentBlock::Text {
        text: "Command executed successfully".to_string(),
    }],
    is_error: None,
}
```

### Mixed Content Response

Tools can return multiple content blocks with different types:

```rust
ContentBlock::ToolResult {
    tool_use_id: "tool_456".to_string(),
    content: vec![
        ContentBlock::Text {
            text: "Analysis complete:".to_string(),
        },
        ContentBlock::Text {
            text: "Found 3 issues".to_string(),
        },
    ],
    is_error: None,
}
```

### Error Response

Error responses work the same way with the `is_error` flag:

```rust
ContentBlock::ToolResult {
    tool_use_id: "tool_789".to_string(),
    content: vec![ContentBlock::Text {
        text: "Tool execution failed: File not found".to_string(),
    }],
    is_error: Some(true),
}
```

## Backward Compatibility

All existing code that creates tool results continues to work. Helper functions automatically convert string results to the new format:

```rust
// Old code (still works)
let result = tool_result_from_string("tool_123", "Success");

// Internally converts to:
ContentBlock::ToolResult {
    tool_use_id: "tool_123".to_string(),
    content: vec![ContentBlock::Text { text: "Success".to_string() }],
    is_error: None,
}
```

## Implementation Notes

- **Serialization**: The `content` field serializes as a JSON array of content blocks
- **Deserialization**: Supports both old string format and new array format for compatibility
- **Tool Implementations**: All tools in `crates/cli/src/tool_executor.rs` have been updated to use the new format
- **Interactive Session**: The interactive session in `crates/cli/src/interactive.rs` handles both formats transparently

## Migration Guide

For tool developers:

1. **No immediate action required**: Existing string-based results continue to work
2. **To use structured content**: Return `Vec<ContentBlock>` instead of `String`
3. **Testing**: Both old and new formats are covered by tests in `crates/core/tests/tool_use_test.rs`

## MCP Spec Compliance

This implementation follows the Model Context Protocol specification for tool results:
- Supports array of content blocks per MCP spec
- Maintains backward compatibility with existing implementations
- Enables richer tool responses with multiple content types
