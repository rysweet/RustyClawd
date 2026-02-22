//! MCP command dispatch for CLI and TUI entry points
//!
//! Routes MCP subcommands from both the CLI (`mcp start ...`) and TUI
//! (`/mcp-start ...`) to the shared `McpCommandHandler`.

use crate::mcp_commands::{McpCommandHandler, McpCommandResult};
use crate::mcp_serve::serve_mcp_server;
use crate::plugins::mcp_proxy::McpProxy;
use std::sync::Arc;
use tokio::sync::Mutex;

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
