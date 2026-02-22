//! MCP Server Proxy - Manages MCP server lifecycle and proxies tool calls
//!
//! Handles starting, stopping, and communicating with MCP servers defined in plugins.
//! Types are defined in `mcp_types`, transport logic in `mcp_transport`.

use std::collections::HashMap;

use crate::plugins::manifest::{McpServerDefinition, McpTransportConfig};
use crate::plugins::mcp_transport;
use crate::plugins::mcp_types::{
    McpCapabilities, McpConnection, McpRequest, McpServerInstance, McpToolDefinition,
};

// Re-export public types so existing consumers keep working via mcp_proxy::Type
#[allow(unused_imports)]
pub use crate::plugins::mcp_types::{
    McpCallToolResult, McpNotification, McpNotificationType, McpPromptDefinition, McpPromptResult,
    Resource, ResourceContents,
};

/// MCP server proxy manager
pub struct McpProxy {
    servers: HashMap<String, McpServerInstance>,
    next_request_id: u64,
}

impl McpProxy {
    /// Create new MCP proxy
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            next_request_id: 1,
        }
    }

    /// Register an MCP server definition
    pub fn register_server(&mut self, definition: McpServerDefinition) {
        let server_id = definition.id.clone();
        self.servers.insert(
            server_id,
            McpServerInstance {
                definition,
                connection: None,
                capabilities: None,
                tools: Vec::new(),
                resources: Vec::new(),
                prompts: Vec::new(),
            },
        );
    }

    /// Start an MCP server
    pub async fn start_server(&mut self, server_id: &str) -> Result<(), String> {
        // Check if already running
        if let Some(server) = self.servers.get(server_id) {
            if server.connection.is_some() {
                return Ok(());
            }
        }

        // Extract transport configuration (avoiding borrow issues)
        let transport = {
            let server = self
                .servers
                .get(server_id)
                .ok_or_else(|| format!("Server not found: {}", server_id))?;
            server.definition.get_transport()?
        };

        // Create connection based on transport type
        let mut connection = match transport {
            McpTransportConfig::Stdio { command, args } => {
                // Extract env before async call
                let env = self
                    .servers
                    .get(server_id)
                    .map(|s| s.definition.env.clone())
                    .unwrap_or_default();
                mcp_transport::start_stdio_connection(&command, &args, &env).await?
            }
            McpTransportConfig::Http { url, headers } => {
                mcp_transport::start_http_connection(&url, headers.as_ref()).await?
            }
        };

        // Initialize server and list tools
        let (capabilities, tools) = self.initialize_connection(&mut connection).await?;

        // Store state
        let server = self.servers.get_mut(server_id).unwrap();
        server.connection = Some(connection);
        server.capabilities = capabilities;
        server.tools = tools;

        Ok(())
    }

    /// Manually refresh all registries for a server (tools, resources, prompts)
    /// This can be called periodically or after operations to sync with server state
    pub async fn refresh_server_registries(&mut self, server_id: &str) -> Result<(), String> {
        self.refresh_registry(server_id, "tools/list", "tools")
            .await?;
        self.refresh_registry(server_id, "resources/list", "resources")
            .await?;
        self.refresh_registry(server_id, "prompts/list", "prompts")
            .await?;
        Ok(())
    }

    /// Initialize connection and discover capabilities/tools
    async fn initialize_connection(
        &mut self,
        connection: &mut McpConnection,
    ) -> Result<(Option<McpCapabilities>, Vec<McpToolDefinition>), String> {
        // Send initialize request
        let init_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "initialize".to_string(),
            params: serde_json::json!({
                "protocolVersion": "1.0",
                "capabilities": {},
                "clientInfo": {
                    "name": "claude-code-rs",
                    "version": "1.0.0"
                }
            }),
        };
        self.next_request_id += 1;

        let init_response = mcp_transport::send_request(connection, &init_request).await?;

        let capabilities = if let Some(result) = init_response.result {
            serde_json::from_value(result["capabilities"].clone()).ok()
        } else {
            None
        };

        // List tools
        let list_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "tools/list".to_string(),
            params: serde_json::json!({}),
        };
        self.next_request_id += 1;

        let list_response = mcp_transport::send_request(connection, &list_request).await?;

        let tools = if let Some(result) = list_response.result {
            serde_json::from_value(result["tools"].clone())
                .map_err(|e| format!("Failed to parse tools: {}", e))?
        } else {
            Vec::new()
        };

        Ok((capabilities, tools))
    }

    /// Stop an MCP server
    pub async fn stop_server(&mut self, server_id: &str) -> Result<(), String> {
        let server = self
            .servers
            .get_mut(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        if let Some(connection) = server.connection.take() {
            match connection {
                McpConnection::Stdio {
                    mut process,
                    notification_task,
                } => {
                    // Stop notification listener if running
                    if let Some(task) = notification_task {
                        task.abort();
                    }

                    process
                        .kill()
                        .await
                        .map_err(|e| format!("Failed to kill MCP server: {}", e))?;
                }
                McpConnection::Http { .. } => {
                    // HTTP connections don't need explicit cleanup
                }
            }
        }

        server.capabilities = None;
        server.tools.clear();
        server.resources.clear();
        server.prompts.clear();

        Ok(())
    }

    /// Take a connection from a server temporarily
    ///
    /// Helper to extract connection for use, must be followed by restore_connection
    fn take_connection(&mut self, server_id: &str) -> Result<McpConnection, String> {
        let server = self
            .servers
            .get_mut(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        server
            .connection
            .take()
            .ok_or_else(|| format!("Server not started: {}", server_id))
    }

    /// Restore a connection back to a server
    ///
    /// Helper to restore connection after temporary use
    fn restore_connection(&mut self, server_id: &str, connection: McpConnection) {
        let server = self.servers.get_mut(server_id).expect("Server disappeared");
        server.connection = Some(connection);
    }

    /// List all available tools from a server
    pub fn list_tools(&self, server_id: &str) -> Result<Vec<McpToolDefinition>, String> {
        let server = self
            .servers
            .get(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        if server.connection.is_none() {
            return Err(format!("Server not started: {}", server_id));
        }

        Ok(server.tools.clone())
    }

    /// Call a tool on an MCP server
    pub async fn call_tool(
        &mut self,
        server_id: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let call_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": tool_name,
                "arguments": arguments,
            }),
        };
        self.next_request_id += 1;

        // Take connection temporarily
        let mut connection = self.take_connection(server_id)?;

        // Send request through transport
        let response = mcp_transport::send_request(&mut connection, &call_request).await?;

        // Restore connection
        self.restore_connection(server_id, connection);

        if let Some(error) = response.error {
            return Err(format!("MCP tool call error: {}", error.message));
        }

        response
            .result
            .ok_or_else(|| "No result from MCP server".to_string())
    }

    /// List all available resources from a server
    pub fn list_resources(&self, server_id: &str) -> Result<Vec<Resource>, String> {
        let server = self
            .servers
            .get(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        if server.connection.is_none() {
            return Err(format!("Server not started: {}", server_id));
        }

        Ok(server.resources.clone())
    }

    /// List all available prompts from a server
    pub fn list_prompts(&self, server_id: &str) -> Result<Vec<McpPromptDefinition>, String> {
        let server = self
            .servers
            .get(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        if server.connection.is_none() {
            return Err(format!("Server not started: {}", server_id));
        }

        Ok(server.prompts.clone())
    }

    /// Read a resource from an MCP server by URI
    pub async fn read_resource(
        &mut self,
        server_id: &str,
        uri: &str,
    ) -> Result<ResourceContents, String> {
        let read_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "resources/read".to_string(),
            params: serde_json::json!({
                "uri": uri,
            }),
        };
        self.next_request_id += 1;

        // Take connection temporarily
        let mut connection = self.take_connection(server_id)?;

        // Send request through transport
        let response = mcp_transport::send_request(&mut connection, &read_request).await?;

        // Restore connection
        self.restore_connection(server_id, connection);

        if let Some(error) = response.error {
            return Err(format!("MCP resource read error: {}", error.message));
        }

        if let Some(result) = response.result {
            let contents: ResourceContents = serde_json::from_value(result["contents"].clone())
                .map_err(|e| format!("Failed to parse resource contents: {}", e))?;
            return Ok(contents);
        }

        Err("No response from MCP server".to_string())
    }

    /// Get a prompt with arguments
    pub async fn get_prompt(
        &mut self,
        server_id: &str,
        prompt_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpPromptResult, String> {
        let get_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "prompts/get".to_string(),
            params: serde_json::json!({
                "name": prompt_name,
                "arguments": arguments,
            }),
        };
        self.next_request_id += 1;

        // Take connection temporarily
        let mut connection = self.take_connection(server_id)?;

        // Send request through transport
        let response = mcp_transport::send_request(&mut connection, &get_request).await?;

        // Restore connection
        self.restore_connection(server_id, connection);

        if let Some(error) = response.error {
            return Err(format!("MCP prompt error: {}", error.message));
        }

        if let Some(result) = response.result {
            let prompt_result: McpPromptResult = serde_json::from_value(result)
                .map_err(|e| format!("Failed to parse prompt result: {}", e))?;
            return Ok(prompt_result);
        }

        Err("No response from MCP server".to_string())
    }

    /// Get all registered servers
    pub fn list_servers(&self) -> Vec<String> {
        self.servers.keys().cloned().collect()
    }

    /// Check if a server is running
    pub fn is_server_running(&self, server_id: &str) -> bool {
        self.servers
            .get(server_id)
            .map(|s| s.connection.is_some())
            .unwrap_or(false)
    }

    /// Stop all servers
    pub async fn stop_all(&mut self) -> Result<(), String> {
        let server_ids: Vec<_> = self.servers.keys().cloned().collect();
        for server_id in server_ids {
            let _ = self.stop_server(&server_id).await;
        }
        Ok(())
    }

    /// Generic refresh for any registry (tools, resources, or prompts).
    ///
    /// Sends a list request for the given `method` (e.g. "tools/list"),
    /// extracts the array at `result_key` (e.g. "tools") from the response,
    /// and updates the corresponding field on the server instance.
    async fn refresh_registry(
        &mut self,
        server_id: &str,
        method: &str,
        result_key: &str,
    ) -> Result<(), String> {
        let list_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: method.to_string(),
            params: serde_json::json!({}),
        };
        self.next_request_id += 1;

        // Take connection temporarily
        let server = self
            .servers
            .get_mut(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        let mut connection = server
            .connection
            .take()
            .ok_or_else(|| format!("Server not started: {}", server_id))?;

        // Send request
        let response = mcp_transport::send_request(&mut connection, &list_request).await?;

        // Restore connection
        let server = self.servers.get_mut(server_id).unwrap();
        server.connection = Some(connection);

        // Update the appropriate registry based on result_key
        if let Some(result) = response.result {
            match result_key {
                "tools" => {
                    server.tools = serde_json::from_value(result["tools"].clone())
                        .map_err(|e| format!("Failed to parse tools: {}", e))?;
                }
                "resources" => {
                    server.resources = serde_json::from_value(result["resources"].clone())
                        .map_err(|e| format!("Failed to parse resources: {}", e))?;
                }
                "prompts" => {
                    server.prompts = serde_json::from_value(result["prompts"].clone())
                        .map_err(|e| format!("Failed to parse prompts: {}", e))?;
                }
                _ => {
                    return Err(format!("Unknown registry key: {}", result_key));
                }
            }
        }

        Ok(())
    }
}

impl Default for McpProxy {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for McpProxy {
    fn drop(&mut self) {
        // Best effort cleanup - stop stdio servers
        for (_, server) in self.servers.iter_mut() {
            if let Some(McpConnection::Stdio {
                process,
                notification_task,
            }) = server.connection.take()
            {
                // Abort notification task
                if let Some(task) = notification_task {
                    task.abort();
                }

                // tokio::process::Child's Drop will kill the process automatically
                // No need for explicit kill command - OS handles cleanup
                drop(process);
            }
            // HTTP connections don't need explicit cleanup
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_proxy_creation() {
        let proxy = McpProxy::new();
        assert_eq!(proxy.list_servers().len(), 0);
    }

    #[test]
    fn test_register_server() {
        let mut proxy = McpProxy::new();
        let definition = McpServerDefinition {
            id: "test-server".to_string(),
            name: "Test Server".to_string(),
            transport: None,
            command: Some("node".to_string()),
            args: vec!["server.js".to_string()],
            env: HashMap::new(),
            description: Some("Test MCP server".to_string()),
            startup_timeout: None,
        };

        proxy.register_server(definition);
        assert_eq!(proxy.list_servers().len(), 1);
        assert!(proxy.list_servers().contains(&"test-server".to_string()));
    }

    #[test]
    fn test_server_not_running_initially() {
        let mut proxy = McpProxy::new();
        let definition = McpServerDefinition {
            id: "test-server".to_string(),
            name: "Test Server".to_string(),
            transport: None,
            command: Some("node".to_string()),
            args: vec![],
            env: HashMap::new(),
            description: None,
            startup_timeout: None,
        };

        proxy.register_server(definition);
        assert!(!proxy.is_server_running("test-server"));
    }

    // ===== Prompts Tests =====

    #[test]
    fn test_server_instance_prompts_initialization() {
        let mut proxy = McpProxy::new();
        let definition = McpServerDefinition {
            id: "test-server".to_string(),
            name: "Test Server".to_string(),
            transport: None,
            command: Some("node".to_string()),
            args: vec![],
            env: HashMap::new(),
            description: None,
            startup_timeout: None,
        };

        proxy.register_server(definition);
        let server = proxy.servers.get("test-server").unwrap();

        // Prompts should be initialized as empty vec
        assert_eq!(server.prompts.len(), 0);
        assert!(server.prompts.is_empty());
    }

    #[test]
    fn test_list_prompts_server_not_found() {
        let proxy = McpProxy::new();
        let result = proxy.list_prompts("nonexistent");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Server not found"));
        assert!(err.contains("nonexistent"));
    }

    #[test]
    fn test_list_prompts_server_not_running() {
        let mut proxy = McpProxy::new();
        let definition = McpServerDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            transport: None,
            command: Some("node".to_string()),
            args: vec![],
            env: HashMap::new(),
            description: None,
            startup_timeout: None,
        };

        proxy.register_server(definition);
        let result = proxy.list_prompts("test");

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not started"));
    }

    #[test]
    fn test_prompt_definition_json() {
        let prompt = McpPromptDefinition {
            name: "test".to_string(),
            description: "Test prompt".to_string(),
            arguments: vec![],
        };

        let json = serde_json::to_string(&prompt).unwrap();
        let deserialized: McpPromptDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, prompt.name);
        assert_eq!(deserialized.description, prompt.description);
    }

    // ===== HTTP Transport Tests =====

    #[test]
    fn test_register_http_server() {
        let mut proxy = McpProxy::new();
        let definition = McpServerDefinition {
            id: "http-server".to_string(),
            name: "HTTP Test Server".to_string(),
            transport: Some(McpTransportConfig::Http {
                url: "http://localhost:8080/mcp".to_string(),
                headers: None,
            }),
            command: None,
            args: vec![],
            env: HashMap::new(),
            description: Some("HTTP MCP server".to_string()),
            startup_timeout: None,
        };

        proxy.register_server(definition);
        assert_eq!(proxy.list_servers().len(), 1);
        assert!(proxy.list_servers().contains(&"http-server".to_string()));
        assert!(!proxy.is_server_running("http-server"));
    }

    #[test]
    fn test_http_transport_with_headers() {
        let mut proxy = McpProxy::new();
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer test-token".to_string());
        headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let definition = McpServerDefinition {
            id: "auth-http-server".to_string(),
            name: "Authenticated HTTP Server".to_string(),
            transport: Some(McpTransportConfig::Http {
                url: "https://api.example.com/mcp".to_string(),
                headers: Some(headers),
            }),
            command: None,
            args: vec![],
            env: HashMap::new(),
            description: None,
            startup_timeout: None,
        };

        proxy.register_server(definition);
        assert_eq!(proxy.list_servers().len(), 1);
    }

    #[test]
    fn test_mixed_transport_servers() {
        let mut proxy = McpProxy::new();

        // Register stdio server
        let stdio_def = McpServerDefinition {
            id: "stdio-server".to_string(),
            name: "Stdio Server".to_string(),
            transport: Some(McpTransportConfig::Stdio {
                command: "node".to_string(),
                args: vec!["server.js".to_string()],
            }),
            command: None,
            args: vec![],
            env: HashMap::new(),
            description: None,
            startup_timeout: None,
        };

        // Register HTTP server
        let http_def = McpServerDefinition {
            id: "http-server".to_string(),
            name: "HTTP Server".to_string(),
            transport: Some(McpTransportConfig::Http {
                url: "http://localhost:3000/mcp".to_string(),
                headers: None,
            }),
            command: None,
            args: vec![],
            env: HashMap::new(),
            description: None,
            startup_timeout: None,
        };

        proxy.register_server(stdio_def);
        proxy.register_server(http_def);

        assert_eq!(proxy.list_servers().len(), 2);
        assert!(proxy.list_servers().contains(&"stdio-server".to_string()));
        assert!(proxy.list_servers().contains(&"http-server".to_string()));
    }

    #[test]
    fn test_backward_compatible_command_field() {
        let mut proxy = McpProxy::new();

        // Old format - using command field directly
        let definition = McpServerDefinition {
            id: "legacy-server".to_string(),
            name: "Legacy Server".to_string(),
            transport: None,
            command: Some("python".to_string()),
            args: vec!["-m".to_string(), "mcp_server".to_string()],
            env: HashMap::new(),
            description: None,
            startup_timeout: None,
        };

        proxy.register_server(definition.clone());

        // Verify it can be registered
        assert_eq!(proxy.list_servers().len(), 1);

        // Verify get_transport returns stdio config
        let transport = definition.get_transport().unwrap();
        match transport {
            McpTransportConfig::Stdio { command, args } => {
                assert_eq!(command, "python");
                assert_eq!(args, vec!["-m", "mcp_server"]);
            }
            _ => panic!("Expected Stdio transport"),
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_http_connection_initialization() {
        // Start mock HTTP server
        let mock_server = MockServer::start().await;

        // Mock all requests to the /mcp endpoint with generic responses
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
                                "tools": true,
                                "resources": false,
                                "prompts": false
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
                                    "name": "test_tool",
                                    "description": "A test tool",
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

        // Create proxy and register HTTP server
        let mut proxy = McpProxy::new();
        let definition = McpServerDefinition {
            id: "test-http".to_string(),
            name: "Test HTTP Server".to_string(),
            transport: Some(McpTransportConfig::Http {
                url: format!("{}/mcp", mock_server.uri()),
                headers: None,
            }),
            command: None,
            args: vec![],
            env: HashMap::new(),
            description: None,
            startup_timeout: None,
        };

        proxy.register_server(definition);
    }
}
