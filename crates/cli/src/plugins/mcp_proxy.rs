//! MCP Server Proxy - Manages MCP server lifecycle and proxies tool calls
//!
//! Handles starting, stopping, and communicating with MCP servers defined in plugins.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child as TokioChild, Command as TokioCommand};

use crate::plugins::manifest::{McpServerDefinition, McpTransportConfig};

/// MCP connection type
#[derive(Debug)]
pub enum McpConnection {
    /// Standard I/O connection with child process
    Stdio {
        process: TokioChild,
        /// Handle to notification listener task (if started)
        notification_task: Option<tokio::task::JoinHandle<()>>,
    },
    /// HTTP connection with reqwest client
    Http {
        client: reqwest::Client,
        url: String,
    },
}

/// MCP server instance state
#[derive(Debug)]
pub struct McpServerInstance {
    /// Server definition
    pub definition: McpServerDefinition,
    /// Active connection (if started)
    pub connection: Option<McpConnection>,
    /// Server capabilities discovered at startup
    pub capabilities: Option<McpCapabilities>,
    /// Available tools from this server
    pub tools: Vec<McpToolDefinition>,
    /// Available resources from this server
    pub resources: Vec<Resource>,
    /// Available prompts from this server
    pub prompts: Vec<McpPromptDefinition>,
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

/// MCP CallToolResult per MCP spec (2025-11-25)
///
/// Represents the result of executing a tool via tools/call.
/// Includes both human-readable content and optional structured JSON data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallToolResult {
    /// Array of content blocks (text, images, etc.) for human-readable output
    pub content: Vec<serde_json::Value>,
    /// Optional structured JSON result matching the tool's declared outputSchema.
    /// Use this when returning typed data that callers can parse programmatically.
    #[serde(rename = "structuredContent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<serde_json::Value>,
    /// Whether this is an error response
    #[serde(rename = "isError")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
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

/// Message in a prompt response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptMessage {
    /// Message role (user, assistant, system)
    pub role: String,
    /// Message content (structured JSON)
    pub content: serde_json::Value,
}

/// Result from prompts/get
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptResult {
    /// Prompt description
    #[serde(default)]
    pub description: Option<String>,
    /// List of messages
    pub messages: Vec<McpPromptMessage>,
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
#[allow(dead_code)] // Fields populated by JSON deserialization
struct McpResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<McpError>,
}

/// MCP notification message (no id field)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC message (can be response or notification)
#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)] // Used by serde for untagged deserialization dispatch
enum JsonRpcMessage {
    Response(McpResponse),
    Notification(McpNotification),
}

/// Types of MCP notifications we handle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpNotificationType {
    ToolsListChanged,
    ResourcesListChanged,
    PromptsListChanged,
    Unknown(String),
}

impl McpNotificationType {
    /// Parse notification type from method string
    pub fn from_method(method: &str) -> Self {
        match method {
            "notifications/tools/list_changed" => Self::ToolsListChanged,
            "notifications/resources/list_changed" => Self::ResourcesListChanged,
            "notifications/prompts/list_changed" => Self::PromptsListChanged,
            _ => Self::Unknown(method.to_string()),
        }
    }

    /// Convert to method string
    pub fn to_method(&self) -> String {
        match self {
            Self::ToolsListChanged => "notifications/tools/list_changed".to_string(),
            Self::ResourcesListChanged => "notifications/resources/list_changed".to_string(),
            Self::PromptsListChanged => "notifications/prompts/list_changed".to_string(),
            Self::Unknown(method) => method.clone(),
        }
    }
}

/// MCP error structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields populated by JSON deserialization
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
                self.start_stdio_connection(&command, &args, &env).await?
            }
            McpTransportConfig::Http { url, headers } => {
                self.start_http_connection(&url, headers.as_ref()).await?
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
        // Refresh all three registries
        self.refresh_tools(server_id).await?;
        self.refresh_resources(server_id).await?;
        self.refresh_prompts(server_id).await?;
        Ok(())
    }

    /// Start stdio connection to MCP server
    async fn start_stdio_connection(
        &self,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<McpConnection, String> {
        let mut cmd = TokioCommand::new(command);
        cmd.args(args);
        cmd.envs(env.iter());
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start MCP server: {}", e))?;

        Ok(McpConnection::Stdio {
            process: child,
            notification_task: None,
        })
    }

    /// Start HTTP connection to MCP server
    async fn start_http_connection(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<McpConnection, String> {
        let mut client_builder = reqwest::Client::builder();

        // Add default headers
        let mut header_map = reqwest::header::HeaderMap::new();
        header_map.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        // Add custom headers if provided
        if let Some(headers) = headers {
            for (key, value) in headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                    .map_err(|e| format!("Invalid header name: {}", e))?;
                let header_value = reqwest::header::HeaderValue::from_str(value)
                    .map_err(|e| format!("Invalid header value: {}", e))?;
                header_map.insert(header_name, header_value);
            }
        }

        client_builder = client_builder.default_headers(header_map);

        let client = client_builder
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        Ok(McpConnection::Http {
            client,
            url: url.to_string(),
        })
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

        let init_response = match connection {
            McpConnection::Stdio {
                process,
                notification_task: _,
            } => self.send_stdio_request_mut(process, &init_request).await?,
            McpConnection::Http { client, url } => {
                self.send_http_request(client, url, &init_request).await?
            }
        };

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

        let list_response = match connection {
            McpConnection::Stdio {
                process,
                notification_task: _,
            } => self.send_stdio_request_mut(process, &list_request).await?,
            McpConnection::Http { client, url } => {
                self.send_http_request(client, url, &list_request).await?
            }
        };

        let tools = if let Some(result) = list_response.result {
            serde_json::from_value(result["tools"].clone())
                .map_err(|e| format!("Failed to parse tools: {}", e))?
        } else {
            Vec::new()
        };

        Ok((capabilities, tools))
    }

    /// Send request via HTTP
    async fn send_http_request(
        &self,
        client: &reqwest::Client,
        url: &str,
        request: &McpRequest,
    ) -> Result<McpResponse, String> {
        let response = client
            .post(url)
            .json(request)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        response
            .json::<McpResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
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

        // Send request through appropriate transport
        let response = match &mut connection {
            McpConnection::Stdio {
                process,
                notification_task: _,
            } => self.send_stdio_request_mut(process, &call_request).await?,
            McpConnection::Http { client, url } => {
                self.send_http_request(client, url, &call_request).await?
            }
        };

        // Restore connection
        self.restore_connection(server_id, connection);

        if let Some(error) = response.error {
            return Err(format!("MCP tool call error: {}", error.message));
        }

        response
            .result
            .ok_or_else(|| "No result from MCP server".to_string())
    }

    /// Send request via stdio with mutable process access
    async fn send_stdio_request_mut(
        &self,
        process: &mut TokioChild,
        request: &McpRequest,
    ) -> Result<McpResponse, String> {
        // Send request
        if let Some(stdin) = process.stdin.as_mut() {
            let request_str = serde_json::to_string(request)
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
                .map_err(|e| format!("Failed to read response: {}", e))?;

            let response: McpResponse = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            return Ok(response);
        }

        Err("No response from MCP server".to_string())
    }

    /// List resources from a server (internal helper)
    #[allow(dead_code)] // MCP resource listing not yet wired into CLI commands
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
                let resources: Vec<Resource> = serde_json::from_value(result["resources"].clone())
                    .map_err(|e| format!("Failed to parse resources: {}", e))?;
                return Ok(resources);
            }
        }

        Ok(Vec::new())
    }

    /// List prompts from a server (internal helper)
    #[allow(dead_code)] // MCP prompt listing not yet wired into CLI commands
    async fn list_prompts_internal(
        &mut self,
        _server_id: &str,
        child: &mut TokioChild,
    ) -> Result<Vec<McpPromptDefinition>, String> {
        let list_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "prompts/list".to_string(),
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
                .map_err(|e| format!("Failed to read prompts list: {}", e))?;

            let response: McpResponse = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse prompts list response: {}", e))?;

            if let Some(error) = response.error {
                return Err(format!(
                    "MCP server error listing prompts: {}",
                    error.message
                ));
            }

            if let Some(result) = response.result {
                let prompts: Vec<McpPromptDefinition> =
                    serde_json::from_value(result["prompts"].clone())
                        .map_err(|e| format!("Failed to parse prompts: {}", e))?;
                return Ok(prompts);
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

        // Send request through appropriate transport
        let response = match &mut connection {
            McpConnection::Stdio {
                process,
                notification_task: _,
            } => self.send_stdio_request_mut(process, &read_request).await?,
            McpConnection::Http { client, url } => {
                self.send_http_request(client, url, &read_request).await?
            }
        };

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

        // Send request through appropriate transport
        let response = match &mut connection {
            McpConnection::Stdio {
                process,
                notification_task: _,
            } => self.send_stdio_request_mut(process, &get_request).await?,
            McpConnection::Http { client, url } => {
                self.send_http_request(client, url, &get_request).await?
            }
        };

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

    /// Handle a notification from an MCP server
    #[allow(dead_code)] // MCP notification handling not yet wired into event loop
    async fn handle_notification(
        &mut self,
        server_id: &str,
        notification: McpNotification,
    ) -> Result<(), String> {
        let notification_type = McpNotificationType::from_method(&notification.method);

        match notification_type {
            McpNotificationType::ToolsListChanged => {
                self.refresh_tools(server_id).await?;
            }
            McpNotificationType::ResourcesListChanged => {
                self.refresh_resources(server_id).await?;
            }
            McpNotificationType::PromptsListChanged => {
                self.refresh_prompts(server_id).await?;
            }
            McpNotificationType::Unknown(method) => {
                // Log unknown notifications but don't fail
                eprintln!(
                    "Received unknown notification from server '{}': {}",
                    server_id, method
                );
            }
        }

        Ok(())
    }

    /// Refresh tools list for a server
    async fn refresh_tools(&mut self, server_id: &str) -> Result<(), String> {
        // Create list request
        let list_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "tools/list".to_string(),
            params: serde_json::json!({}),
        };
        self.next_request_id += 1;

        // Get server and extract connection temporarily
        let server = self
            .servers
            .get_mut(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        let mut connection = server
            .connection
            .take()
            .ok_or_else(|| format!("Server not started: {}", server_id))?;

        // Send request
        let response = match &mut connection {
            McpConnection::Stdio {
                process,
                notification_task: _,
            } => self.send_stdio_request_mut(process, &list_request).await?,
            McpConnection::Http { client, url } => {
                self.send_http_request(client, url, &list_request).await?
            }
        };

        // Restore connection
        let server = self.servers.get_mut(server_id).unwrap();
        server.connection = Some(connection);

        // Update tools list
        if let Some(result) = response.result {
            let tools: Vec<McpToolDefinition> = serde_json::from_value(result["tools"].clone())
                .map_err(|e| format!("Failed to parse tools: {}", e))?;
            server.tools = tools;
        }

        Ok(())
    }

    /// Refresh resources list for a server
    async fn refresh_resources(&mut self, server_id: &str) -> Result<(), String> {
        // Create list request
        let list_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "resources/list".to_string(),
            params: serde_json::json!({}),
        };
        self.next_request_id += 1;

        // Get server and extract connection temporarily
        let server = self
            .servers
            .get_mut(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        let mut connection = server
            .connection
            .take()
            .ok_or_else(|| format!("Server not started: {}", server_id))?;

        // Send request
        let response = match &mut connection {
            McpConnection::Stdio {
                process,
                notification_task: _,
            } => self.send_stdio_request_mut(process, &list_request).await?,
            McpConnection::Http { client, url } => {
                self.send_http_request(client, url, &list_request).await?
            }
        };

        // Restore connection
        let server = self.servers.get_mut(server_id).unwrap();
        server.connection = Some(connection);

        // Update resources list
        if let Some(result) = response.result {
            let resources: Vec<Resource> = serde_json::from_value(result["resources"].clone())
                .map_err(|e| format!("Failed to parse resources: {}", e))?;
            server.resources = resources;
        }

        Ok(())
    }

    /// Refresh prompts list for a server
    async fn refresh_prompts(&mut self, server_id: &str) -> Result<(), String> {
        // Create list request
        let list_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_request_id,
            method: "prompts/list".to_string(),
            params: serde_json::json!({}),
        };
        self.next_request_id += 1;

        // Get server and extract connection temporarily
        let server = self
            .servers
            .get_mut(server_id)
            .ok_or_else(|| format!("Server not found: {}", server_id))?;

        let mut connection = server
            .connection
            .take()
            .ok_or_else(|| format!("Server not started: {}", server_id))?;

        // Send request
        let response = match &mut connection {
            McpConnection::Stdio {
                process,
                notification_task: _,
            } => self.send_stdio_request_mut(process, &list_request).await?,
            McpConnection::Http { client, url } => {
                self.send_http_request(client, url, &list_request).await?
            }
        };

        // Restore connection
        let server = self.servers.get_mut(server_id).unwrap();
        server.connection = Some(connection);

        // Update prompts list
        if let Some(result) = response.result {
            let prompts: Vec<McpPromptDefinition> =
                serde_json::from_value(result["prompts"].clone())
                    .map_err(|e| format!("Failed to parse prompts: {}", e))?;
            server.prompts = prompts;
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
        };

        proxy.register_server(definition);
    }
}
