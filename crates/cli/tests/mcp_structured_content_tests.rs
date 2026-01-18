//! MCP structuredContent Field Tests
//!
//! Tests for the MCP `structuredContent` field in tool responses.
//! Per MCP spec (2025-11-25), `CallToolResult` includes:
//! - content: Vec<ContentBlock> - human-readable results
//! - structuredContent: Option<serde_json::Value> - typed JSON matching outputSchema
//! - isError: Option<bool> - error indicator

use rustyclawd::plugins::mcp_proxy::McpCallToolResult;
use serde_json::json;

#[test]
fn test_mcp_call_tool_result_serialization_with_structured_content() {
    let result = McpCallToolResult {
        content: vec![json!({
            "type": "text",
            "text": "Found 3 files"
        })],
        structured_content: Some(json!({
            "files": [
                {"path": "/src/main.rs", "size": 1024},
                {"path": "/src/lib.rs", "size": 512}
            ],
            "total_count": 2
        })),
        is_error: None,
    };

    // Test serialization
    let json_str = serde_json::to_string(&result).unwrap();

    // Verify structuredContent is present and in camelCase
    assert!(
        json_str.contains("structuredContent"),
        "Should serialize as camelCase structuredContent"
    );
    assert!(json_str.contains("files"));
    assert!(json_str.contains("/src/main.rs"));
}

#[test]
fn test_mcp_call_tool_result_without_structured_content() {
    let result = McpCallToolResult {
        content: vec![json!({
            "type": "text",
            "text": "Simple result"
        })],
        structured_content: None,
        is_error: None,
    };

    // Test serialization
    let json_str = serde_json::to_string(&result).unwrap();

    // Verify structuredContent is omitted when None
    assert!(
        !json_str.contains("structuredContent"),
        "Should omit structuredContent when None"
    );
    assert!(json_str.contains("content"));
    assert!(json_str.contains("Simple result"));
}

#[test]
fn test_mcp_call_tool_result_deserialization_with_structured_content() {
    // Simulate MCP server response with structuredContent
    let json_str = r#"{
        "content": [{"type": "text", "text": "Analysis complete"}],
        "structuredContent": {
            "issues": [
                {"severity": "warning", "message": "Unused variable"},
                {"severity": "error", "message": "Syntax error"}
            ],
            "summary": {"total": 2, "errors": 1, "warnings": 1}
        },
        "isError": false
    }"#;

    let result: McpCallToolResult = serde_json::from_str(json_str).unwrap();

    assert_eq!(result.content.len(), 1);
    assert!(result.structured_content.is_some());

    let structured = result.structured_content.unwrap();
    assert!(structured.get("issues").is_some());
    assert!(structured.get("summary").is_some());

    let summary = structured.get("summary").unwrap();
    assert_eq!(summary.get("total").unwrap(), 2);
}

#[test]
fn test_mcp_call_tool_result_deserialization_without_structured_content() {
    // Simulate MCP server response without structuredContent (backward compat)
    let json_str = r#"{
        "content": [{"type": "text", "text": "Done"}],
        "isError": false
    }"#;

    let result: McpCallToolResult = serde_json::from_str(json_str).unwrap();

    assert_eq!(result.content.len(), 1);
    assert!(result.structured_content.is_none());
    assert_eq!(result.is_error, Some(false));
}

#[test]
fn test_mcp_call_tool_result_with_complex_structured_content() {
    // Test with nested objects and arrays
    let result = McpCallToolResult {
        content: vec![json!({"type": "text", "text": "Complex result"})],
        structured_content: Some(json!({
            "database": {
                "tables": [
                    {
                        "name": "users",
                        "columns": ["id", "name", "email"],
                        "row_count": 1500
                    },
                    {
                        "name": "orders",
                        "columns": ["id", "user_id", "total"],
                        "row_count": 5000
                    }
                ],
                "version": "14.2"
            },
            "metrics": {
                "query_time_ms": 42,
                "cache_hit": true
            }
        })),
        is_error: None,
    };

    // Round-trip test
    let json_str = serde_json::to_string(&result).unwrap();
    let deserialized: McpCallToolResult = serde_json::from_str(&json_str).unwrap();

    assert!(deserialized.structured_content.is_some());
    let structured = deserialized.structured_content.unwrap();

    let tables = structured["database"]["tables"].as_array().unwrap();
    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0]["name"], "users");
    assert_eq!(structured["metrics"]["query_time_ms"], 42);
}

#[test]
fn test_mcp_call_tool_result_error_with_structured_content() {
    // Error responses can also have structuredContent
    let result = McpCallToolResult {
        content: vec![json!({
            "type": "text",
            "text": "Validation failed"
        })],
        structured_content: Some(json!({
            "errors": [
                {"field": "email", "code": "INVALID_FORMAT"},
                {"field": "age", "code": "OUT_OF_RANGE"}
            ],
            "error_count": 2
        })),
        is_error: Some(true),
    };

    let json_str = serde_json::to_string(&result).unwrap();
    let deserialized: McpCallToolResult = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.is_error, Some(true));
    assert!(deserialized.structured_content.is_some());

    let structured = deserialized.structured_content.unwrap();
    let errors = structured["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 2);
}

#[test]
fn test_mcp_call_tool_result_camel_case_field_names() {
    // Verify MCP spec compliance with camelCase field names
    let json_str = r#"{
        "content": [{"type": "text", "text": "test"}],
        "structuredContent": {"key": "value"},
        "isError": false
    }"#;

    let result: McpCallToolResult = serde_json::from_str(json_str).unwrap();
    assert!(result.structured_content.is_some());
    assert_eq!(result.is_error, Some(false));

    // Serialization should produce camelCase
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(serialized.contains("structuredContent"));
    assert!(serialized.contains("isError"));
    assert!(!serialized.contains("structured_content"));
    assert!(!serialized.contains("is_error"));
}
