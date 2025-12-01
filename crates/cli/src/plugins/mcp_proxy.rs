//! MCP Server Proxy - Manages MCP server lifecycle and proxies tool calls
//!
//! Handles starting, stopping, and communicating with MCP servers defined in plugins.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child as TokioChild, Command as TokioCommand};

use crate::plugins::manifest::McpServerDefinition;

/// MCP server instance state
#[derive(Debug)]
pub struct McpServerInstance {
    /// Server definition
    pub definition: McpServerDefinition,
    /// Running process (if started)
    pub process: Option<TokioChild>,
    /// Server capabilities discovered at startup
    pub capabilities: Option<McpCapabilities>,
    /// Available tools from this server
    pub tools: Vec<McpToolDefinition>,
    /// Available resources from this server
    pub resources: Vec<Resource>,
}

/// MCP server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCapabilities {
    /// Tools capability
    #[serde(default)]
    pub tools: bool,
    /// Resources capability
    #[serde(default)]
    pub resources: bool,
    /// Prompts capability
    #[serde(default)]
    pub prompts: bool,
}

/// MCP tool definition from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    /// Tool name (will be prefixed with mcp__{server_id}__)
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for input
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// MCP resource definition from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Resource URI (e.g., file:///path/to/file, http://example.com/resource)
    pub uri: String,
    /// Human-readable resource name
    pub name: String,
    /// Resource description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type of the resource
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP resource contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContents {
    /// Resource URI
    pub uri: String,
    /// MIME type of the contents
    #[serde(rename = "mimeType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Text contents (for text-based resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Binary contents (base64 encoded, for binary resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

/// MCP request message
#[derive(Debug, Serialize)]
struct McpRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// MCP response message
#[derive(Debug, Deserialize)]
struct McpResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<McpError>,
}

/// MCP error structure
#[derive(Debug, Deserialize)]
struct McpError {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<serde_json::Value>,
}

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
                process: None,
                capabilities: None,
                tools: Vec::new(),
                resources: Vec::new(),
            },
        );
    }

    /// Start an MCP server
    pub async fn start_server(&mut self, server_id: &str) -> Result<(), String> {
        let server = self
            .servers
            .get_mut(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        // Don't restart if already running
        if server.process.is_some() {
            return Ok(());
        }

        // Build command
        let mut cmd = TokioCommand::new(&server.definition.command);
        cmd.args(&server.definition.args);
        cmd.envs(server.definition.env.iter());
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Start process
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start MCP server: {}", e))?;

        // Initialize server (send initialize request)
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

        // Send initialization
        if let Some(stdin) = child.stdin.as_mut() {
            let request_str = serde_json::to_string(&init_request)
                .map_err(|e| format!("Failed to serialize request: {}", e))?;
            stdin
                .write_all(request_str.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to MCP server: {}", e))?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| format!("Failed to write newline: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("Failed to flush: {}", e))?;
        }

        // Read initialization response
        if let Some(stdout) = child.stdout.as_mut() {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| format!("Failed to read initialization response: {}", e))?;

            let response: McpResponse = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse initialization response: {}", e))?;

            if let Some(error) = response.error {
                return Err(format!(
                    "MCP server initialization error: {}",
                    error.message
                ));
            }

            if let Some(result) = response.result {
                server.capabilities = serde_json::from_value(result["capabilities"].clone()).ok();
            }
        }

        // Check if resources capability is available
        let has_resources = server.capabilities.as_ref()
            .map(|caps| caps.resources)
            .unwrap_or(false);

        // List tools
        let tools = self.list_tools_internal(server_id, &mut child).await?;

        // List resources (if capability is available)
        let resources = if has_resources {
            self.list_resources_internal(server_id, &mut child).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        // Store state
        let server = self.servers.get_mut(server_id).unwrap();
        server.tools = tools;
        server.resources = resources;
        server.process = Some(child);

        Ok(())
    }

    /// Stop an MCP server
    pub async fn stop_server(&mut self, server_id: &str) -> Result<(), String> {
        let server = self
            .servers
            .get_mut(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        if let Some(mut process) = server.process.take() {
            process
                .kill()
                .await
                .map_err(|e| format!("Failed to kill MCP server: {}", e))?;
        }

        server.capabilities = None;
        server.tools.clear();
        server.resources.clear();

        Ok(())
    }

    /// List tools from a server (internal helper)
    async fn list_tools_internal(
        &mut self,
        _server_id: &str,
        child: &mut TokioChild,
    ) -> Result<Vec<McpToolDefinition>, String> {
        let list_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "tools/list".to_string(),
            params: serde_json::json!({}),
        };
        self.next_request_id += 1;

        // Send request
        if let Some(stdin) = child.stdin.as_mut() {
            let request_str = serde_json::to_string(&list_request)
                .map_err(|e| format!("Failed to serialize request: {}", e))?;
            stdin
                .write_all(request_str.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to MCP server: {}", e))?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| format!("Failed to write newline: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("Failed to flush: {}", e))?;
        }

        // Read response
        if let Some(stdout) = child.stdout.as_mut() {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| format!("Failed to read tools list: {}", e))?;

            let response: McpResponse = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse tools list response: {}", e))?;

            if let Some(error) = response.error {
                return Err(format!("MCP server error listing tools: {}", error.message));
            }

            if let Some(result) = response.result {
                let tools: Vec<McpToolDefinition> = serde_json::from_value(result["tools"].clone())
                    .map_err(|e| format!("Failed to parse tools: {}", e))?;
                return Ok(tools);
            }
        }

        Ok(Vec::new())
    }

    /// List all available tools from a server
    pub fn list_tools(&self, server_id: &str) -> Result<Vec<McpToolDefinition>, String> {
        let server = self
            .servers
            .get(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        if server.process.is_none() {
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
        let server = self
            .servers
            .get_mut(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        let process = server
            .process
            .as_mut()
            .ok_or_else(|| format!("Server not started: {}", server_id))?;

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

        // Send request
        if let Some(stdin) = process.stdin.as_mut() {
            let request_str = serde_json::to_string(&call_request)
                .map_err(|e| format!("Failed to serialize request: {}", e))?;
            stdin
                .write_all(request_str.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to MCP server: {}", e))?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| format!("Failed to write newline: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("Failed to flush: {}", e))?;
        }

        // Read response
        if let Some(stdout) = process.stdout.as_mut() {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| format!("Failed to read tool call response: {}", e))?;

            let response: McpResponse = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse tool call response: {}", e))?;

            if let Some(error) = response.error {
                return Err(format!("MCP tool call error: {}", error.message));
            }

            if let Some(result) = response.result {
                return Ok(result);
            }
        }

        Err("No response from MCP server".to_string())
    }

    /// List resources from a server (internal helper)
    async fn list_resources_internal(
        &mut self,
        _server_id: &str,
        child: &mut TokioChild,
    ) -> Result<Vec<Resource>, String> {
        let list_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "resources/list".to_string(),
            params: serde_json::json!({}),
        };
        self.next_request_id += 1;

        // Send request
        if let Some(stdin) = child.stdin.as_mut() {
            let request_str = serde_json::to_string(&list_request)
                .map_err(|e| format!("Failed to serialize request: {}", e))?;
            stdin
                .write_all(request_str.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to MCP server: {}", e))?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| format!("Failed to write newline: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("Failed to flush: {}", e))?;
        }

        // Read response
        if let Some(stdout) = child.stdout.as_mut() {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| format!("Failed to read resources list: {}", e))?;

            let response: McpResponse = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse resources list response: {}", e))?;

            if let Some(error) = response.error {
                return Err(format!(
                    "MCP server error listing resources: {}",
                    error.message
                ));
            }

            if let Some(result) = response.result {
                let resources: Vec<Resource> =
                    serde_json::from_value(result["resources"].clone())
                        .map_err(|e| format!("Failed to parse resources: {}", e))?;
                return Ok(resources);
            }
        }

        Ok(Vec::new())
    }

    /// List all available resources from a server
    pub fn list_resources(&self, server_id: &str) -> Result<Vec<Resource>, String> {
        let server = self
            .servers
            .get(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        if server.process.is_none() {
            return Err(format!("Server not started: {}", server_id));
        }

        Ok(server.resources.clone())
    }

    /// Read a resource from an MCP server by URI
    pub async fn read_resource(
        &mut self,
        server_id: &str,
        uri: &str,
    ) -> Result<ResourceContents, String> {
        let server = self
            .servers
            .get_mut(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        let process = server
            .process
            .as_mut()
            .ok_or_else(|| format!("Server not started: {}", server_id))?;

        let read_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "resources/read".to_string(),
            params: serde_json::json!({
                "uri": uri,
            }),
        };
        self.next_request_id += 1;

        // Send request
        if let Some(stdin) = process.stdin.as_mut() {
            let request_str = serde_json::to_string(&read_request)
                .map_err(|e| format!("Failed to serialize request: {}", e))?;
            stdin
                .write_all(request_str.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to MCP server: {}", e))?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| format!("Failed to write newline: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("Failed to flush: {}", e))?;
        }

        // Read response
        if let Some(stdout) = process.stdout.as_mut() {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| format!("Failed to read resource: {}", e))?;

            let response: McpResponse = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse resource read response: {}", e))?;

            if let Some(error) = response.error {
                return Err(format!("MCP resource read error: {}", error.message));
            }

            if let Some(result) = response.result {
                let contents: ResourceContents = serde_json::from_value(result["contents"].clone())
                    .map_err(|e| format!("Failed to parse resource contents: {}", e))?;
                return Ok(contents);
            }
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
            .map(|s| s.process.is_some())
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
}

impl Default for McpProxy {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for McpProxy {
    fn drop(&mut self) {
        // Best effort cleanup - stop all servers synchronously
        for (_, server) in self.servers.iter_mut() {
            if let Some(process) = server.process.take() {
                let _ = std::process::Command::new("kill")
                    .arg(format!("{}", process.id().unwrap_or(0)))
                    .output();
            }
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
            command: "node".to_string(),
            args: vec!["server.js".to_string()],
            env: HashMap::new(),
            description: Some("Test MCP server".to_string()),
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
            command: "node".to_string(),
            args: vec![],
            env: HashMap::new(),
            description: None,
        };

        proxy.register_server(definition);
        assert!(!proxy.is_server_running("test-server"));
    }
}
