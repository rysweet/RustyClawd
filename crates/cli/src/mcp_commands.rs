//! MCP Command UI - CLI and TUI interfaces for MCP server management
//!
//! Provides user-facing commands for managing Model Context Protocol servers:
//! - start: Launch an MCP server
//! - stop: Terminate a running MCP server
//! - list: Show all registered servers and their status
//! - tools: Display available tools from a server
//! - status: Show detailed status of a specific server
//!
//! Used by both CLI (rusty mcp ...) and TUI (/mcp-... commands)

use crate::plugins::mcp_proxy::{McpProxy, McpServerInstance};
use std::sync::Arc;
use tokio::sync::Mutex;

/// MCP command result
pub type McpCommandResult = Result<String, String>;

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
            "Missing subcommand. Usage: mcp <start|stop|list|tools|status> [args]".to_string(),
        );
    }

    let subcommand = args[0].as_str();

    match subcommand {
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
        "status" => {
            if args.len() < 2 {
                return Err("Missing server ID. Usage: mcp status <server-id>".to_string());
            }
            handler.status(&args[1]).await
        }
        "stop-all" => handler.stop_all().await,
        _ => Err(format!(
            "Unknown subcommand: '{}'. Available: start, stop, list, tools, status",
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
        "status" => {
            if args.is_empty() {
                return Err("Missing server ID. Usage: /mcp-status <server-id>".to_string());
            }
            handler.status(&args[0]).await
        }
        _ => Err(format!(
            "Unknown MCP command: '{}'. Available: start, stop, list, tools, status",
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
    fn test_parse_slash_command_invalid() {
        assert!(parse_slash_command("/help").is_none());
        assert!(parse_slash_command("mcp-list").is_none());
        assert!(parse_slash_command("/mcplist").is_none());
    }
}
