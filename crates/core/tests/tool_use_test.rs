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
        content: vec![ContentBlock::Text {
            text: "Result data".to_string(),
        }],
        is_error: None,
    };

    let json_str = serde_json::to_string(&result).expect("Failed to serialize");
    assert!(json_str.contains("tool_result"));
    assert!(json_str.contains("tool_1"));
    assert!(json_str.contains("Result data"));
}

// ===== Structured Content Tests (Issue #148) =====
// These tests will fail until we implement Vec<ContentBlock> for content field

#[test]
fn test_tool_result_with_single_text_block() {
    let result = ContentBlock::ToolResult {
        tool_use_id: "tool_123".to_string(),
        content: vec![ContentBlock::Text {
            text: "Success".to_string(),
        }],
        is_error: None,
    };

    let json_str = serde_json::to_string(&result).expect("Failed to serialize");
    assert!(json_str.contains("tool_result"));
    assert!(json_str.contains("tool_123"));
    assert!(json_str.contains("Success"));
}

#[test]
fn test_tool_result_with_multiple_text_blocks() {
    let result = ContentBlock::ToolResult {
        tool_use_id: "tool_456".to_string(),
        content: vec![
            ContentBlock::Text {
                text: "Line 1".to_string(),
            },
            ContentBlock::Text {
                text: "Line 2".to_string(),
            },
        ],
        is_error: None,
    };

    let json_str = serde_json::to_string(&result).expect("Failed to serialize");
    assert!(json_str.contains("tool_result"));
    assert!(json_str.contains("Line 1"));
    assert!(json_str.contains("Line 2"));
}

#[test]
fn test_tool_result_with_error_flag() {
    let result = ContentBlock::ToolResult {
        tool_use_id: "tool_789".to_string(),
        content: vec![ContentBlock::Text {
            text: "Error occurred".to_string(),
        }],
        is_error: Some(true),
    };

    let json_str = serde_json::to_string(&result).expect("Failed to serialize");
    assert!(json_str.contains("tool_result"));
    assert!(json_str.contains("Error occurred"));
    assert!(json_str.contains("true")); // is_error should be true
}

#[test]
fn test_tool_result_deserialization_with_array() {
    let json_str = r#"{
        "type": "tool_result",
        "tool_use_id": "tool_test",
        "content": [
            {
                "type": "text",
                "text": "Result text"
            }
        ]
    }"#;

    let result: ContentBlock = serde_json::from_str(json_str).expect("Failed to deserialize");

    match result {
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => {
            assert_eq!(tool_use_id, "tool_test");
            assert_eq!(content.len(), 1);
            assert!(is_error.is_none());

            match &content[0] {
                ContentBlock::Text { text } => assert_eq!(text, "Result text"),
                _ => panic!("Expected Text content block"),
            }
        }
        _ => panic!("Expected ToolResult variant"),
    }
}

#[test]
fn test_message_with_tool_result_blocks() {
    let blocks = vec![ContentBlock::ToolResult {
        tool_use_id: "tool_1".to_string(),
        content: vec![ContentBlock::Text {
            text: "Tool output".to_string(),
        }],
        is_error: None,
    }];

    let msg = Message::with_blocks(Role::User, blocks);
    let json_str = serde_json::to_string(&msg).expect("Failed to serialize");
    assert!(json_str.contains("user"));
    assert!(json_str.contains("tool_result"));
    assert!(json_str.contains("Tool output"));
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
