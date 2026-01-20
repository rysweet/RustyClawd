//! Tests for MCP `list_changed` notification handling
//!
//! Validates that RustyClawd correctly handles notifications from MCP servers
//! and updates registries dynamically without requiring reconnection.

use rustyclawd::plugins::{McpNotification, McpNotificationType};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn test_notification_types_parsing() {
    // Test tools notification
    let tools_notif = McpNotificationType::from_method("notifications/tools/list_changed");
    assert_eq!(tools_notif, McpNotificationType::ToolsListChanged);
    assert_eq!(tools_notif.to_method(), "notifications/tools/list_changed");

    // Test resources notification
    let resources_notif =
        McpNotificationType::from_method("notifications/resources/list_changed");
    assert_eq!(resources_notif, McpNotificationType::ResourcesListChanged);
    assert_eq!(
        resources_notif.to_method(),
        "notifications/resources/list_changed"
    );

    // Test prompts notification
    let prompts_notif = McpNotificationType::from_method("notifications/prompts/list_changed");
    assert_eq!(prompts_notif, McpNotificationType::PromptsListChanged);
    assert_eq!(
        prompts_notif.to_method(),
        "notifications/prompts/list_changed"
    );

    // Test unknown notification
    let unknown = McpNotificationType::from_method("notifications/unknown/something");
    assert!(matches!(unknown, McpNotificationType::Unknown(_)));
}

#[tokio::test]
async fn test_notification_json_parsing() {
    // Test parsing tools/list_changed notification
    let json = r#"{
        "jsonrpc": "2.0",
        "method": "notifications/tools/list_changed",
        "params": {}
    }"#;

    let notification: McpNotification = serde_json::from_str(json).unwrap();
    assert_eq!(notification.jsonrpc, "2.0");
    assert_eq!(notification.method, "notifications/tools/list_changed");

    // Test parsing resources/list_changed notification
    let json = r#"{
        "jsonrpc": "2.0",
        "method": "notifications/resources/list_changed",
        "params": {}
    }"#;

    let notification: McpNotification = serde_json::from_str(json).unwrap();
    assert_eq!(notification.method, "notifications/resources/list_changed");

    // Test parsing prompts/list_changed notification
    let json = r#"{
        "jsonrpc": "2.0",
        "method": "notifications/prompts/list_changed",
        "params": {}
    }"#;

    let notification: McpNotification = serde_json::from_str(json).unwrap();
    assert_eq!(notification.method, "notifications/prompts/list_changed");
}

#[tokio::test]
async fn test_refresh_tools_registry() {
    // Mock HTTP MCP server
    let mock_server = MockServer::start().await;

    // Initial tools/list response
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(|req: &wiremock::Request| {
            let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
            let method_name = body["method"].as_str().unwrap_or("");
            let req_id = body["id"].as_u64().unwrap_or(1);

            let response = match method_name {
                "initialize" => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "protocolVersion": "1.0",
                        "capabilities": {
                            "tools": true
                        },
                        "serverInfo": {
                            "name": "test-server",
                            "version": "1.0.0"
                        }
                    }
                }),
                "tools/list" => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "tools": [
                            {
                                "name": "initial_tool",
                                "description": "Initial tool",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {}
                                }
                            }
                        ]
                    }
                }),
                _ => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {}
                }),
            };

            ResponseTemplate::new(200).set_body_json(response)
        })
        .mount(&mock_server)
        .await;

    // Test will be completed when implementation is ready
    // For now, validates that test infrastructure works
}
