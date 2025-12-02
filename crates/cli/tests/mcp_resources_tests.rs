//! MCP Resources Unit Tests
//!
//! Tests for MCP resources capability including:
//! - Resource data structure validation
//! - ResourceContents data structure validation
//! - MCP proxy resource management
//! - Error handling for missing resources

use rustyclawd::plugins::manifest::McpServerDefinition;
use rustyclawd::plugins::mcp_proxy::{McpProxy, Resource, ResourceContents};
use std::collections::HashMap;

#[test]
fn test_resource_serialization() {
    let resource = Resource {
        uri: "file:///test/document.txt".to_string(),
        name: "Test Document".to_string(),
        description: Some("A test document".to_string()),
        mime_type: Some("text/plain".to_string()),
    };

    // Test serialization
    let json = serde_json::to_string(&resource).unwrap();
    assert!(json.contains("file:///test/document.txt"));
    assert!(json.contains("Test Document"));
    assert!(json.contains("mimeType")); // camelCase

    // Test deserialization
    let deserialized: Resource = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.uri, resource.uri);
    assert_eq!(deserialized.name, resource.name);
    assert_eq!(deserialized.description, resource.description);
    assert_eq!(deserialized.mime_type, resource.mime_type);
}

#[test]
fn test_resource_contents_text_serialization() {
    let contents = ResourceContents {
        uri: "file:///test/document.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        text: Some("This be test content, arr!".to_string()),
        blob: None,
    };

    // Test serialization
    let json = serde_json::to_string(&contents).unwrap();
    assert!(json.contains("file:///test/document.txt"));
    assert!(json.contains("test content"));
    assert!(!json.contains("blob")); // Should be omitted when None

    // Test deserialization
    let deserialized: ResourceContents = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.uri, contents.uri);
    assert_eq!(deserialized.text, contents.text);
    assert!(deserialized.blob.is_none());
}

#[test]
fn test_resource_contents_binary_serialization() {
    let contents = ResourceContents {
        uri: "file:///test/image.png".to_string(),
        mime_type: Some("image/png".to_string()),
        text: None,
        blob: Some("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJ".to_string()),
    };

    // Test serialization
    let json = serde_json::to_string(&contents).unwrap();
    assert!(json.contains("file:///test/image.png"));
    assert!(json.contains("iVBORw0KGgo"));
    assert!(!json.contains("\"text\"")); // Should be omitted when None

    // Test deserialization
    let deserialized: ResourceContents = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.uri, contents.uri);
    assert_eq!(deserialized.blob, contents.blob);
    assert!(deserialized.text.is_none());
}

#[test]
fn test_mcp_proxy_resource_initialization() {
    let mut proxy = McpProxy::new();

    let server_def = McpServerDefinition {
        id: "test-server".to_string(),
        name: "Test Server".to_string(),
        transport: None,
        command: Some("node".to_string()),
        args: vec!["server.js".to_string()],
        env: HashMap::new(),
        description: Some("Test MCP server".to_string()),
    };

    proxy.register_server(server_def);

    // Server should exist but not be running
    let servers = proxy.list_servers();
    assert_eq!(servers.len(), 1);
    assert!(servers.contains(&"test-server".to_string()));
    assert!(!proxy.is_server_running("test-server"));
}

#[test]
fn test_mcp_proxy_list_resources_not_started() {
    let mut proxy = McpProxy::new();

    let server_def = McpServerDefinition {
        id: "not-started".to_string(),
        name: "Not Started".to_string(),
        transport: None,
        command: Some("node".to_string()),
        args: vec![],
        env: HashMap::new(),
        description: None,
    };

    proxy.register_server(server_def);

    // Try to list resources without starting server
    let result = proxy.list_resources("not-started");
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.contains("not started"),
        "Error should mention 'not started': {}",
        error
    );
}

#[tokio::test]
async fn test_mcp_proxy_read_resource_not_started() {
    let mut proxy = McpProxy::new();

    let server_def = McpServerDefinition {
        id: "not-started".to_string(),
        name: "Not Started".to_string(),
        transport: None,
        command: Some("node".to_string()),
        args: vec![],
        env: HashMap::new(),
        description: None,
    };

    proxy.register_server(server_def);

    // Try to read resource without starting server
    let result = proxy.read_resource("not-started", "file:///test.txt").await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.contains("not started"),
        "Error should mention 'not started': {}",
        error
    );
}

#[test]
fn test_mcp_proxy_list_resources_not_found() {
    let proxy = McpProxy::new();

    // Try to list resources from non-existent server
    let result = proxy.list_resources("nonexistent");
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.contains("not found"),
        "Error should mention 'not found': {}",
        error
    );
}

#[tokio::test]
async fn test_mcp_proxy_read_resource_not_found() {
    let mut proxy = McpProxy::new();

    // Try to read resource from non-existent server
    let result = proxy.read_resource("nonexistent", "file:///test.txt").await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.contains("not found"),
        "Error should mention 'not found': {}",
        error
    );
}

#[test]
fn test_resource_uri_formats() {
    // Test various URI formats
    let file_resource = Resource {
        uri: "file:///home/user/document.txt".to_string(),
        name: "Local File".to_string(),
        description: None,
        mime_type: Some("text/plain".to_string()),
    };

    let http_resource = Resource {
        uri: "http://example.com/resource".to_string(),
        name: "HTTP Resource".to_string(),
        description: None,
        mime_type: Some("application/json".to_string()),
    };

    let https_resource = Resource {
        uri: "https://api.example.com/data".to_string(),
        name: "HTTPS Resource".to_string(),
        description: None,
        mime_type: Some("application/json".to_string()),
    };

    // Verify all URI formats serialize correctly
    let file_json = serde_json::to_string(&file_resource).unwrap();
    assert!(file_json.contains("file:///"));

    let http_json = serde_json::to_string(&http_resource).unwrap();
    assert!(http_json.contains("http://"));

    let https_json = serde_json::to_string(&https_resource).unwrap();
    assert!(https_json.contains("https://"));
}

#[test]
fn test_resource_optional_fields() {
    // Test resource with minimal fields
    let minimal_resource = Resource {
        uri: "file:///test.txt".to_string(),
        name: "Test".to_string(),
        description: None,
        mime_type: None,
    };

    let json = serde_json::to_string(&minimal_resource).unwrap();
    // Optional fields should be omitted when None
    assert!(!json.contains("description"));
    assert!(!json.contains("mimeType"));

    // Should deserialize correctly
    let deserialized: Resource = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.uri, "file:///test.txt");
    assert_eq!(deserialized.name, "Test");
    assert!(deserialized.description.is_none());
    assert!(deserialized.mime_type.is_none());
}

#[test]
fn test_resource_contents_mime_type_camel_case() {
    // Verify mimeType is serialized in camelCase (MCP spec requirement)
    let contents = ResourceContents {
        uri: "file:///test.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        text: Some("content".to_string()),
        blob: None,
    };

    let json = serde_json::to_string(&contents).unwrap();
    assert!(json.contains("mimeType")); // camelCase
    assert!(!json.contains("mime_type")); // Not snake_case

    // Verify it deserializes from camelCase
    let json_with_camel = r#"{"uri":"file:///test.txt","mimeType":"text/plain","text":"content"}"#;
    let deserialized: ResourceContents = serde_json::from_str(json_with_camel).unwrap();
    assert_eq!(deserialized.mime_type, Some("text/plain".to_string()));
}

#[test]
fn test_mcp_proxy_multiple_servers() {
    let mut proxy = McpProxy::new();

    // Register multiple servers
    for i in 1..=3 {
        let server_def = McpServerDefinition {
            id: format!("server-{}", i),
            name: format!("Server {}", i),
            transport: None,
            command: Some("node".to_string()),
            args: vec![],
            env: HashMap::new(),
            description: None,
        };
        proxy.register_server(server_def);
    }

    let servers = proxy.list_servers();
    assert_eq!(servers.len(), 3);
    assert!(servers.contains(&"server-1".to_string()));
    assert!(servers.contains(&"server-2".to_string()));
    assert!(servers.contains(&"server-3".to_string()));
}

#[test]
fn test_resource_deserialization_from_mcp_response() {
    // Test deserializing a resource list response from an MCP server
    let mcp_response = r#"{
        "resources": [
            {
                "uri": "file:///test/doc.txt",
                "name": "Document",
                "description": "A test document",
                "mimeType": "text/plain"
            },
            {
                "uri": "file:///test/data.json",
                "name": "Data",
                "mimeType": "application/json"
            }
        ]
    }"#;

    let json: serde_json::Value = serde_json::from_str(mcp_response).unwrap();
    let resources: Vec<Resource> = serde_json::from_value(json["resources"].clone()).unwrap();

    assert_eq!(resources.len(), 2);
    assert_eq!(resources[0].uri, "file:///test/doc.txt");
    assert_eq!(resources[0].name, "Document");
    assert_eq!(
        resources[0].description,
        Some("A test document".to_string())
    );
    assert_eq!(resources[0].mime_type, Some("text/plain".to_string()));

    assert_eq!(resources[1].uri, "file:///test/data.json");
    assert_eq!(resources[1].name, "Data");
    assert!(resources[1].description.is_none());
    assert_eq!(resources[1].mime_type, Some("application/json".to_string()));
}

#[test]
fn test_resource_contents_deserialization_from_mcp_response() {
    // Test deserializing text resource contents
    let text_response = r#"{
        "contents": {
            "uri": "file:///test/doc.txt",
            "mimeType": "text/plain",
            "text": "This is the file content"
        }
    }"#;

    let json: serde_json::Value = serde_json::from_str(text_response).unwrap();
    let contents: ResourceContents = serde_json::from_value(json["contents"].clone()).unwrap();

    assert_eq!(contents.uri, "file:///test/doc.txt");
    assert_eq!(contents.mime_type, Some("text/plain".to_string()));
    assert_eq!(contents.text, Some("This is the file content".to_string()));
    assert!(contents.blob.is_none());

    // Test deserializing binary resource contents
    let binary_response = r#"{
        "contents": {
            "uri": "file:///test/image.png",
            "mimeType": "image/png",
            "blob": "iVBORw0KGgoAAAANSUhEUg=="
        }
    }"#;

    let json: serde_json::Value = serde_json::from_str(binary_response).unwrap();
    let contents: ResourceContents = serde_json::from_value(json["contents"].clone()).unwrap();

    assert_eq!(contents.uri, "file:///test/image.png");
    assert_eq!(contents.mime_type, Some("image/png".to_string()));
    assert_eq!(contents.blob, Some("iVBORw0KGgoAAAANSUhEUg==".to_string()));
    assert!(contents.text.is_none());
}
