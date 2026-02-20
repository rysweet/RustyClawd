//! MCP Transport Layer - Stdio and HTTP communication with MCP servers
//!
//! Handles the low-level transport details:
//! - Starting stdio and HTTP connections
//! - Sending JSON-RPC requests over stdio pipes
//! - Sending JSON-RPC requests over HTTP
//! - Connection lifecycle (take/restore pattern for borrow-checker compliance)

use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child as TokioChild, Command as TokioCommand};

use super::mcp_types::{McpConnection, McpRequest, McpResponse};

/// Start a stdio connection to an MCP server process
pub async fn start_stdio_connection(
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

/// Start an HTTP connection to an MCP server
pub async fn start_http_connection(
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

/// Send a JSON-RPC request via stdio to a child process
pub async fn send_stdio_request(
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

        let response: McpResponse =
            serde_json::from_str(&line).map_err(|e| format!("Failed to parse response: {}", e))?;

        return Ok(response);
    }

    Err("No response from MCP server".to_string())
}

/// Send a JSON-RPC request via HTTP
pub async fn send_http_request(
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

/// Send a request through the appropriate transport based on connection type
pub async fn send_request(
    connection: &mut McpConnection,
    request: &McpRequest,
) -> Result<McpResponse, String> {
    match connection {
        McpConnection::Stdio {
            process,
            notification_task: _,
        } => send_stdio_request(process, request).await,
        McpConnection::Http { client, url } => send_http_request(client, url, request).await,
    }
}
