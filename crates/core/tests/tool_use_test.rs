//! Integration tests for tool use functionality

use rustyclawd_core::client::*;
use serde_json::json;

#[test]
fn test_tool_definition_serialization() {
    let tool = ToolDefinition {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "param1": {
                    "type": "string",
                    "description": "A string parameter"
                }
            },
            "required": ["param1"]
        }),
    };

    let json_str = serde_json::to_string(&tool).expect("Failed to serialize");
    assert!(json_str.contains("test_tool"));
    assert!(json_str.contains("A test tool"));
}

#[test]
fn test_tool_choice_auto() {
    let choice = ToolChoice::auto();
    let json_str = serde_json::to_string(&choice).expect("Failed to serialize");
    assert!(json_str.contains("auto"));
}

#[test]
fn test_tool_choice_any() {
    let choice = ToolChoice::any();
    let json_str = serde_json::to_string(&choice).expect("Failed to serialize");
    assert!(json_str.contains("any"));
}

#[test]
fn test_tool_choice_specific() {
    let choice = ToolChoice::tool("my_tool");
    let json_str = serde_json::to_string(&choice).expect("Failed to serialize");
    assert!(json_str.contains("tool"));
    assert!(json_str.contains("my_tool"));
}

#[test]
fn test_message_with_text_content() {
    let msg = Message::user("Hello");
    let json_str = serde_json::to_string(&msg).expect("Failed to serialize");
    assert!(json_str.contains("user"));
    assert!(json_str.contains("Hello"));
}

#[test]
fn test_message_with_blocks() {
    let blocks = vec![
        ContentBlock::Text {
            text: "Hello".to_string(),
        },
        ContentBlock::ToolUse {
            id: "tool_1".to_string(),
            name: "test_tool".to_string(),
            input: json!({"param": "value"}),
        },
    ];

    let msg = Message::with_blocks(Role::Assistant, blocks);
    let json_str = serde_json::to_string(&msg).expect("Failed to serialize");
    assert!(json_str.contains("assistant"));
    assert!(json_str.contains("tool_use"));
    assert!(json_str.contains("test_tool"));
}

#[test]
fn test_content_block_tool_result() {
    let result = ContentBlock::ToolResult {
        tool_use_id: "tool_1".to_string(),
        content: "Result data".to_string(),
        is_error: None,
    };

    let json_str = serde_json::to_string(&result).expect("Failed to serialize");
    assert!(json_str.contains("tool_result"));
    assert!(json_str.contains("tool_1"));
    assert!(json_str.contains("Result data"));
}

#[test]
fn test_create_message_request_with_tools() {
    let tools = vec![ToolDefinition {
        name: "bash".to_string(),
        description: "Execute bash commands".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string"
                }
            },
            "required": ["command"]
        }),
    }];

    let request = CreateMessageRequest::new(
        "claude-sonnet-4-5-20250929",
        vec![Message::user("Hello")],
        1024,
    )
    .with_tools(tools);

    let json_str = serde_json::to_string(&request).expect("Failed to serialize");
    assert!(json_str.contains("tools"));
    assert!(json_str.contains("bash"));
}
