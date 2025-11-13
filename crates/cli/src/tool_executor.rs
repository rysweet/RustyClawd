//! Tool executor for Claude API tool calls
//!
//! This module bridges between Anthropic API tool calls and our internal tool implementations.

use anyhow::Result;
use rustyclawd_core::client::ClientError;
use rustyclawd_tools::{
    BashTool, EditTool, GlobTool, GrepTool, ReadTool, Tool, ToolContext, ToolEvent, WriteTool,
};
use futures::StreamExt;
use serde_json::{json, Value};
use crate::terminal_guard::TerminalGuard;

/// Create an educational error message that teaches Claude the correct schema
fn create_schema_error(tool_name: &str, error_msg: &str) -> ClientError {
    let (required_fields, optional_fields, example) = match tool_name {
        "Write" => (
            vec!["file_path", "content"],
            vec![],
            json!({
                "file_path": "/absolute/path/to/file.txt",
                "content": "The content to write to the file"
            })
        ),
        "Read" => (
            vec!["file_path"],
            vec!["offset", "limit"],
            json!({
                "file_path": "/absolute/path/to/file.txt",
                "offset": 0,
                "limit": 100
            })
        ),
        "Edit" => (
            vec!["file_path", "old_string", "new_string"],
            vec!["replace_all"],
            json!({
                "file_path": "/absolute/path/to/file.txt",
                "old_string": "text to replace",
                "new_string": "replacement text",
                "replace_all": false
            })
        ),
        "Bash" => (
            vec!["command"],
            vec!["timeout", "description", "run_in_background", "dangerouslyDisableSandbox"],
            json!({
                "command": "ls -la",
                "timeout": 120000,
                "description": "List files in directory"
            })
        ),
        "Glob" => (
            vec!["pattern"],
            vec!["path"],
            json!({
                "pattern": "**/*.rs",
                "path": "/path/to/search"
            })
        ),
        "Grep" => (
            vec!["pattern"],
            vec!["path", "output_mode", "glob", "type", "-i", "-n", "-A", "-B", "-C", "multiline", "head_limit", "offset"],
            json!({
                "pattern": "search.*pattern",
                "path": "/path/to/search",
                "output_mode": "content"
            })
        ),
        _ => (vec![], vec![], json!({}))
    };

    let error_response = json!({
        "error": format!("Failed to parse {} tool parameters", tool_name),
        "details": error_msg,
        "required_fields": required_fields,
        "optional_fields": optional_fields,
        "example": example,
        "help": format!(
            "The {} tool requires these fields: {}. Please ensure all required fields are provided with the correct types.",
            tool_name,
            required_fields.join(", ")
        )
    });

    ClientError::Api(serde_json::to_string_pretty(&error_response).unwrap_or_else(|_| error_msg.to_string()))
}

/// Execute a tool by name with given parameters
///
/// This function takes the tool name and input from Claude's API response,
/// executes the corresponding internal tool, and returns the result as JSON.
pub async fn execute_tool(tool_name: String, tool_input: Value) -> Result<Value, ClientError> {
    // Create tool context with execution context from global state
    use crate::terminal_guard::{get_execution_context, ExecutionContext as GuardContext};

    let execution_context = get_execution_context();
    let ctx = ToolContext {
        cwd: std::env::current_dir().unwrap_or_default(),
        debug: false,
        metadata: serde_json::Value::Null,
        execution_context: match execution_context {
            GuardContext::Tui => rustyclawd_tools::ExecutionContext::Tui,
            GuardContext::NonInteractive => rustyclawd_tools::ExecutionContext::NonInteractive,
        },
    };

    match tool_name.as_str() {
        "Bash" => execute_bash_tool(tool_input, &ctx).await,
        "Read" => execute_read_tool(tool_input, &ctx).await,
        "Write" => execute_write_tool(tool_input, &ctx).await,
        "Edit" => execute_edit_tool(tool_input, &ctx).await,
        "Glob" => execute_glob_tool(tool_input, &ctx).await,
        "Grep" => execute_grep_tool(tool_input, &ctx).await,
        _ => Err(ClientError::Api(format!("Unknown tool: {}", tool_name))),
    }
}

/// Execute Bash tool
async fn execute_bash_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    // Protect terminal state during bash execution
    let _guard = TerminalGuard::new()
        .map_err(|e| ClientError::Api(format!("Failed to create terminal guard: {}", e)))?;

    let params: rustyclawd_tools::bash::BashParams =
        serde_json::from_value(input).map_err(|e| {
            create_schema_error("Bash", &e.to_string())
        })?;

    let tool = BashTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("Bash tool execution failed: {}", e)))?;

    // Collect the result from the stream
    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize Bash output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!("Bash tool error: {}", message)));
            }
            ToolEvent::Progress { .. } => {
                // Log progress but continue
            }
        }
    }

    Err(ClientError::Api(
        "Bash tool completed without result".to_string(),
    ))
    // Guard is automatically dropped here, restoring terminal state
}

/// Execute Read tool
async fn execute_read_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::read::ReadParams =
        serde_json::from_value(input).map_err(|e| {
            create_schema_error("Read", &e.to_string())
        })?;

    let tool = ReadTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("Read tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize Read output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!("Read tool error: {}", message)));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "Read tool completed without result".to_string(),
    ))
}

/// Execute Write tool
async fn execute_write_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::write::WriteParams =
        serde_json::from_value(input).map_err(|e| {
            create_schema_error("Write", &e.to_string())
        })?;

    let tool = WriteTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("Write tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize Write output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!("Write tool error: {}", message)));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "Write tool completed without result".to_string(),
    ))
}

/// Execute Edit tool
async fn execute_edit_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::edit::EditParams =
        serde_json::from_value(input).map_err(|e| {
            create_schema_error("Edit", &e.to_string())
        })?;

    let tool = EditTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("Edit tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize Edit output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!("Edit tool error: {}", message)));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "Edit tool completed without result".to_string(),
    ))
}

/// Execute Glob tool
async fn execute_glob_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::glob_tool::GlobParams =
        serde_json::from_value(input).map_err(|e| {
            create_schema_error("Glob", &e.to_string())
        })?;

    let tool = GlobTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("Glob tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize Glob output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!("Glob tool error: {}", message)));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "Glob tool completed without result".to_string(),
    ))
}

/// Execute Grep tool
async fn execute_grep_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::grep::GrepParams =
        serde_json::from_value(input).map_err(|e| {
            create_schema_error("Grep", &e.to_string())
        })?;

    let tool = GrepTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("Grep tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize Grep output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!("Grep tool error: {}", message)));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "Grep tool completed without result".to_string(),
    ))
}
