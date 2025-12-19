//! MCP Command UI - CLI and TUI interfaces for MCP server management
//!
//! Provides user-facing commands for managing Model Context Protocol servers:
//! - serve: Run RustyClawd as an MCP server (exposes tools to external clients)
//! - start: Launch an MCP server
//! - stop: Terminate a running MCP server
//! - list: Show all registered servers and their status
//! - tools: Display available tools from a server
//! - status: Show detailed status of a specific server
//!
//! Used by both CLI (claude mcp ...) and TUI (/mcp-... commands)

use crate::plugins::mcp_proxy::{McpProxy, McpServerInstance};
use crate::tool_definitions;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

/// MCP command result
pub type McpCommandResult = Result<String, String>;

/// JSON-RPC 2.0 request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: serde_json::Value,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

/// Serve RustyClawd as an MCP server
///
/// Reads JSON-RPC 2.0 requests from stdin and writes responses to stdout.
/// Exposes all RustyClawd tools to external MCP clients.
async fn serve_mcp_server() -> McpCommandResult {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Read requests line by line from stdin
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("Failed to read stdin: {}", e))?;

        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                // Send parse error response
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: serde_json::Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: "Parse error".to_string(),
                        data: Some(serde_json::json!({ "details": e.to_string() })),
                    }),
                };

                if let Ok(json) = serde_json::to_string(&error_response) {
                    writeln!(stdout, "{}", json).ok();
                    stdout.flush().ok();
                }
                continue;
            }
        };

        // Handle request based on method
        let response = match request.method.as_str() {
            "initialize" => handle_initialize(&request),
            "tools/list" => handle_tools_list(&request),
            "tools/call" => handle_tools_call(&request).await,
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: Some(serde_json::json!({ "method": request.method })),
                }),
            },
        };

        // Send response
        if let Ok(json) = serde_json::to_string(&response) {
            writeln!(stdout, "{}", json).ok();
            stdout.flush().ok();
        }
    }

    Ok("MCP server stopped".to_string())
}

/// Handle initialize request
fn handle_initialize(request: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: request.id.clone(),
        result: Some(serde_json::json!({
            "protocolVersion": "1.0",
            "capabilities": {
                "tools": true
            },
            "serverInfo": {
                "name": "rustyclawd",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        error: None,
    }
}

/// Handle tools/list request
fn handle_tools_list(request: &JsonRpcRequest) -> JsonRpcResponse {
    // Get all tool definitions and convert to MCP format
    let tool_defs = tool_definitions::get_all_tool_definitions();

    let tools: Vec<serde_json::Value> = tool_defs
        .into_iter()
        .map(|tool| {
            // Ensure inputSchema has "type": "object" at root
            let mut input_schema = tool.input_schema;
            if !input_schema
                .get("type")
                .and_then(|t| t.as_str())
                .eq(&Some("object"))
            {
                // Wrap schema if it doesn't have type: object
                let mut schema_obj = serde_json::Map::new();
                schema_obj.insert(
                    "type".to_string(),
                    serde_json::Value::String("object".to_string()),
                );
                if let Some(obj) = input_schema.as_object() {
                    for (key, value) in obj {
                        schema_obj.insert(key.clone(), value.clone());
                    }
                }
                input_schema = serde_json::Value::Object(schema_obj);
            }

            serde_json::json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": input_schema
            })
        })
        .collect();

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: request.id.clone(),
        result: Some(serde_json::json!({ "tools": tools })),
        error: None,
    }
}

/// Handle tools/call request
async fn handle_tools_call(request: &JsonRpcRequest) -> JsonRpcResponse {
    // Extract tool name and arguments
    let tool_name = match request.params.get("name").and_then(|n| n.as_str()) {
        Some(name) => name,
        None => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Invalid params".to_string(),
                    data: Some(serde_json::json!({ "details": "Missing tool name" })),
                }),
            };
        }
    };

    let _arguments = request
        .params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    // NOTE: Full tool execution requires the complete tool_executor context
    // which includes hooks, session state, permission handling, etc.
    // For initial MCP serve implementation, we return a success response
    // indicating the tool was invoked.
    //
    // Future enhancement: Integrate with crate::tool_executor::execute_tool
    // to provide full tool execution capability.

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: request.id.clone(),
        result: Some(serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!("Tool '{}' invoked with parameters. Full execution requires session context.", tool_name)
            }],
            "isError": false
        })),
        error: None,
    }
}

/// MCP command handler - shared between CLI and TUI
pub struct McpCommandHandler {
    /// Shared MCP proxy instance
    proxy: Arc<Mutex<McpProxy>>,
}

impl McpCommandHandler {
    /// Create new command handler with shared proxy
    pub fn new(proxy: Arc<Mutex<McpProxy>>) -> Self {
        Self { proxy }
    }

    /// Start an MCP server
    pub async fn start(&self, server_id: &str) -> McpCommandResult {
        let mut proxy = self.proxy.lock().await;

        // Check if server exists
        if !proxy.list_servers().contains(&server_id.to_string()) {
            return Err(format!(
                "Server '{}' not found. Use 'mcp list' to see available servers.",
                server_id
            ));
        }

        // Check if already running
        if proxy.is_server_running(server_id) {
            return Ok(format!("Server '{}' is already running", server_id));
        }

        // Start the server
        proxy.start_server(server_id).await?;

        // Get tools count for success message
        let tools = proxy.list_tools(server_id)?;
        Ok(format!(
            "Successfully started server '{}' with {} tool(s)",
            server_id,
            tools.len()
        ))
    }

    /// Stop an MCP server
    pub async fn stop(&self, server_id: &str) -> McpCommandResult {
        let mut proxy = self.proxy.lock().await;

        // Check if server exists
        if !proxy.list_servers().contains(&server_id.to_string()) {
            return Err(format!(
                "Server '{}' not found. Use 'mcp list' to see available servers.",
                server_id
            ));
        }

        // Check if running
        if !proxy.is_server_running(server_id) {
            return Ok(format!("Server '{}' is not running", server_id));
        }

        // Stop the server
        proxy.stop_server(server_id).await?;
        Ok(format!("Successfully stopped server '{}'", server_id))
    }

    /// List all MCP servers with status
    pub async fn list(&self) -> McpCommandResult {
        let proxy = self.proxy.lock().await;
        let servers = proxy.list_servers();

        if servers.is_empty() {
            return Ok("No MCP servers registered.\n\nRegister servers by adding them to plugin manifests in .claude/plugins/".to_string());
        }

        let mut output = String::from("MCP Servers:\n");
        output.push_str(&format!("{:-<60}\n", ""));

        for server_id in servers {
            let status = if proxy.is_server_running(&server_id) {
                "RUNNING"
            } else {
                "STOPPED"
            };

            let tools_info = if proxy.is_server_running(&server_id) {
                match proxy.list_tools(&server_id) {
                    Ok(tools) => format!("{} tool(s)", tools.len()),
                    Err(_) => "unknown tools".to_string(),
                }
            } else {
                "-".to_string()
            };

            output.push_str(&format!("  {} - {} [{}]\n", server_id, status, tools_info));
        }

        output.push_str(&format!("{:-<60}\n", ""));
        output.push_str("\nCommands:\n");
        output.push_str("  mcp start <server-id>  - Start a server\n");
        output.push_str("  mcp stop <server-id>   - Stop a server\n");
        output.push_str("  mcp tools <server-id>  - List available tools\n");
        output.push_str("  mcp status <server-id> - Show detailed status\n");

        Ok(output)
    }

    /// List tools from a server
    pub async fn tools(&self, server_id: &str) -> McpCommandResult {
        let proxy = self.proxy.lock().await;

        // Check if server exists
        if !proxy.list_servers().contains(&server_id.to_string()) {
            return Err(format!(
                "Server '{}' not found. Use 'mcp list' to see available servers.",
                server_id
            ));
        }

        // Check if running
        if !proxy.is_server_running(server_id) {
            return Err(format!(
                "Server '{}' is not running. Start it with: mcp start {}",
                server_id, server_id
            ));
        }

        // Get tools
        let tools = proxy.list_tools(server_id)?;

        if tools.is_empty() {
            return Ok(format!("Server '{}' has no tools available", server_id));
        }

        let tool_count = tools.len();
        let mut output = format!("Tools from server '{}':\n", server_id);
        output.push_str(&format!("{:-<60}\n", ""));

        for tool in tools {
            output.push_str(&format!("\n  {}\n", tool.name));
            output.push_str(&format!("    {}\n", tool.description));

            // Show input schema summary
            if let Some(props) = tool.input_schema.get("properties") {
                if let Some(props_obj) = props.as_object() {
                    if !props_obj.is_empty() {
                        output.push_str("    Parameters:\n");
                        for (key, _) in props_obj {
                            output.push_str(&format!("      - {}\n", key));
                        }
                    }
                }
            }
        }

        output.push_str(&format!("\n{:-<60}\n", ""));
        output.push_str(&format!("\nTotal: {} tool(s)\n", tool_count));

        Ok(output)
    }

    /// Show detailed status of a server
    pub async fn status(&self, server_id: &str) -> McpCommandResult {
        let proxy = self.proxy.lock().await;

        // Check if server exists
        if !proxy.list_servers().contains(&server_id.to_string()) {
            return Err(format!(
                "Server '{}' not found. Use 'mcp list' to see available servers.",
                server_id
            ));
        }

        let is_running = proxy.is_server_running(server_id);

        let mut output = format!("Server Status: {}\n", server_id);
        output.push_str(&format!("{:-<60}\n", ""));
        output.push_str(&format!(
            "  Status: {}\n",
            if is_running { "RUNNING" } else { "STOPPED" }
        ));

        if is_running {
            // Get tools
            match proxy.list_tools(server_id) {
                Ok(tools) => {
                    output.push_str(&format!("  Tools: {}\n", tools.len()));

                    if !tools.is_empty() {
                        output.push_str("\n  Available Tools:\n");
                        for tool in tools {
                            output
                                .push_str(&format!("    - {}: {}\n", tool.name, tool.description));
                        }
                    }
                }
                Err(e) => {
                    output.push_str(&format!("  Tools: Error - {}\n", e));
                }
            }
        } else {
            output.push_str("  Tools: Not available (server stopped)\n");
            output.push_str(&format!("\n  Start with: mcp start {}\n", server_id));
        }

        output.push_str(&format!("{:-<60}\n", ""));

        Ok(output)
    }

    /// List prompts from a server
    pub async fn prompts(&self, server_id: &str) -> McpCommandResult {
        let proxy = self.proxy.lock().await;

        // Check if server exists
        if !proxy.list_servers().contains(&server_id.to_string()) {
            return Err(format!(
                "Server '{}' not found. Use 'mcp list' to see available servers.",
                server_id
            ));
        }

        // Check if running
        if !proxy.is_server_running(server_id) {
            return Err(format!(
                "Server '{}' is not running. Start it with: mcp start {}",
                server_id, server_id
            ));
        }

        // Get prompts
        let prompts = proxy.list_prompts(server_id)?;

        if prompts.is_empty() {
            return Ok(format!("Server '{}' has no prompts available", server_id));
        }

        let prompt_count = prompts.len();
        let mut output = format!("Prompts from server '{}':\n", server_id);
        output.push_str(&format!("{:-<60}\n", ""));

        for prompt in prompts {
            output.push_str(&format!("\n  {}\n", prompt.name));
            output.push_str(&format!("    {}\n", prompt.description));

            if !prompt.arguments.is_empty() {
                output.push_str("    Arguments:\n");
                for arg in prompt.arguments {
                    let required = if arg.required { "required" } else { "optional" };
                    output.push_str(&format!("      - {} ({})\n", arg.name, required));
                }
            }
        }

        output.push_str(&format!("\n{:-<60}\n", ""));
        output.push_str(&format!("\nTotal: {} prompt(s)\n", prompt_count));

        Ok(output)
    }

    /// Stop all running servers
    pub async fn stop_all(&self) -> McpCommandResult {
        let mut proxy = self.proxy.lock().await;
        proxy.stop_all().await?;
        Ok("All MCP servers stopped".to_string())
    }
}

/// CLI entry point for MCP subcommands
pub async fn handle_cli_command(proxy: Arc<Mutex<McpProxy>>, args: &[String]) -> McpCommandResult {
    let handler = McpCommandHandler::new(proxy);

    if args.is_empty() {
        return Err(
            "Missing subcommand. Usage: mcp <start|stop|list|tools|prompts|status> [args]"
                .to_string(),
        );
    }

    let subcommand = args[0].as_str();

    match subcommand {
        "serve" => serve_mcp_server().await,
        "start" => {
            if args.len() < 2 {
                return Err("Missing server ID. Usage: mcp start <server-id>".to_string());
            }
            handler.start(&args[1]).await
        }
        "stop" => {
            if args.len() < 2 {
                return Err("Missing server ID. Usage: mcp stop <server-id>".to_string());
            }
            handler.stop(&args[1]).await
        }
        "list" => handler.list().await,
        "tools" => {
            if args.len() < 2 {
                return Err("Missing server ID. Usage: mcp tools <server-id>".to_string());
            }
            handler.tools(&args[1]).await
        }
        "prompts" => {
            if args.len() < 2 {
                return Err("Missing server ID. Usage: mcp prompts <server-id>".to_string());
            }
            handler.prompts(&args[1]).await
        }
        "status" => {
            if args.len() < 2 {
                return Err("Missing server ID. Usage: mcp status <server-id>".to_string());
            }
            handler.status(&args[1]).await
        }
        "stop-all" => handler.stop_all().await,
        _ => Err(format!(
            "Unknown subcommand: '{}'. Available: serve, start, stop, list, tools, prompts, status",
            subcommand
        )),
    }
}

/// Parse slash command for TUI
/// Examples: "/mcp-list", "/mcp-start filesystem", "/mcp-tools filesystem"
pub fn parse_slash_command(input: &str) -> Option<(String, Vec<String>)> {
    let input = input.trim();

    if !input.starts_with("/mcp-") {
        return None;
    }

    let parts: Vec<&str> = input[1..].split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    // Convert /mcp-start -> start
    let command = parts[0].strip_prefix("mcp-")?;

    // Rest are arguments
    let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    Some((command.to_string(), args))
}

/// TUI entry point for /mcp-* commands
pub async fn handle_tui_command(
    proxy: Arc<Mutex<McpProxy>>,
    command: &str,
    args: Vec<String>,
) -> McpCommandResult {
    let handler = McpCommandHandler::new(proxy);

    match command {
        "start" => {
            if args.is_empty() {
                return Err("Missing server ID. Usage: /mcp-start <server-id>".to_string());
            }
            handler.start(&args[0]).await
        }
        "stop" => {
            if args.is_empty() {
                return Err("Missing server ID. Usage: /mcp-stop <server-id>".to_string());
            }
            handler.stop(&args[0]).await
        }
        "list" => handler.list().await,
        "tools" => {
            if args.is_empty() {
                return Err("Missing server ID. Usage: /mcp-tools <server-id>".to_string());
            }
            handler.tools(&args[0]).await
        }
        "prompts" => {
            if args.is_empty() {
                return Err("Missing server ID. Usage: /mcp-prompts <server-id>".to_string());
            }
            handler.prompts(&args[0]).await
        }
        "status" => {
            if args.is_empty() {
                return Err("Missing server ID. Usage: /mcp-status <server-id>".to_string());
            }
            handler.status(&args[0]).await
        }
        _ => Err(format!(
            "Unknown MCP command: '{}'. Available: start, stop, list, tools, prompts, status",
            command
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slash_command_list() {
        let result = parse_slash_command("/mcp-list");
        assert!(result.is_some());
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "list");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_slash_command_start() {
        let result = parse_slash_command("/mcp-start filesystem");
        assert!(result.is_some());
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "start");
        assert_eq!(args, vec!["filesystem"]);
    }

    #[test]
    fn test_parse_slash_command_tools() {
        let result = parse_slash_command("/mcp-tools filesystem");
        assert!(result.is_some());
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "tools");
        assert_eq!(args, vec!["filesystem"]);
    }

    #[test]
    fn test_parse_slash_command_prompts() {
        let result = parse_slash_command("/mcp-prompts filesystem");
        assert!(result.is_some());
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "prompts");
        assert_eq!(args, vec!["filesystem"]);
    }

    #[test]
    fn test_parse_slash_command_invalid() {
        assert!(parse_slash_command("/help").is_none());
        assert!(parse_slash_command("mcp-list").is_none());
        assert!(parse_slash_command("/mcplist").is_none());
    }
}
