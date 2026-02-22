//! MCP JSON-RPC 2.0 stdio server
//!
//! Runs RustyClawd as an MCP server, reading JSON-RPC requests from stdin
//! and writing responses to stdout. Exposes tool definitions to external
//! MCP clients.

use crate::mcp_commands::McpCommandResult;
use crate::schema_validator::SchemaValidator;
use crate::tool_definitions;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

/// JSON-RPC 2.0 request
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields populated by JSON deserialization
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
pub(crate) async fn serve_mcp_server() -> McpCommandResult {
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
    // Trust tool_definitions.rs to provide correct schemas
    let tool_defs = tool_definitions::get_all_tool_definitions();
    let validator = SchemaValidator::default();
    let mut filtered_count = 0;

    let tools: Vec<serde_json::Value> = tool_defs
        .into_iter()
        .filter_map(|tool| {
            // Validate the input schema
            let validation_result = validator.validate(&tool.input_schema);
            if !validation_result.is_valid() {
                // Log the filtered tool for debugging
                eprintln!(
                    "MCP serve: Filtered out tool '{}' due to incompatible schema: {}",
                    tool.name,
                    validation_result.error_message().unwrap_or_default()
                );
                filtered_count += 1;
                return None;
            }

            Some(serde_json::json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": tool.input_schema
            }))
        })
        .collect();

    // Log summary if any tools were filtered
    if filtered_count > 0 {
        eprintln!(
            "MCP serve: Filtered out {} tool(s) with incompatible schemas",
            filtered_count
        );
    }

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: request.id.clone(),
        result: Some(serde_json::json!({ "tools": tools })),
        error: None,
    }
}

/// Handle tools/call request
///
/// Tool execution in MCP serve mode is not supported because it requires
/// the complete tool_executor context (hooks, session state, permissions).
/// Returns a proper error response per JSON-RPC 2.0 spec.
async fn handle_tools_call(request: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: request.id.clone(),
        result: None,
        error: Some(JsonRpcError {
            code: -32601,
            message: "Method not supported".to_string(),
            data: Some(serde_json::json!({
                "details": "Tool execution requires session context (hooks, permissions, state) which is not available in MCP serve mode. Use CLI mode for tool execution."
            })),
        }),
    }
}
