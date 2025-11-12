//! Tool executor for Claude API tool calls
//!
//! This module bridges between Anthropic API tool calls and our internal tool implementations.

use anyhow::Result;
use claude_code_core::client::ClientError;
use claude_code_tools::{
    BashTool, EditTool, GlobTool, GrepTool, ReadTool, Tool, ToolContext, ToolEvent, WriteTool,
};
use futures::StreamExt;
use serde_json::Value;

/// Execute a tool by name with given parameters
///
/// This function takes the tool name and input from Claude's API response,
/// executes the corresponding internal tool, and returns the result as JSON.
pub async fn execute_tool(tool_name: String, tool_input: Value) -> Result<Value, ClientError> {
    let ctx = ToolContext::default();

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
    let params: claude_code_tools::bash::BashParams =
        serde_json::from_value(input).map_err(|e| {
            ClientError::Api(format!("Failed to parse Bash tool parameters: {}", e))
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
}

/// Execute Read tool
async fn execute_read_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: claude_code_tools::read::ReadParams =
        serde_json::from_value(input).map_err(|e| {
            ClientError::Api(format!("Failed to parse Read tool parameters: {}", e))
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
    let params: claude_code_tools::write::WriteParams =
        serde_json::from_value(input).map_err(|e| {
            ClientError::Api(format!("Failed to parse Write tool parameters: {}", e))
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
    let params: claude_code_tools::edit::EditParams =
        serde_json::from_value(input).map_err(|e| {
            ClientError::Api(format!("Failed to parse Edit tool parameters: {}", e))
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
    let params: claude_code_tools::glob_tool::GlobParams =
        serde_json::from_value(input).map_err(|e| {
            ClientError::Api(format!("Failed to parse Glob tool parameters: {}", e))
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
    let params: claude_code_tools::grep::GrepParams =
        serde_json::from_value(input).map_err(|e| {
            ClientError::Api(format!("Failed to parse Grep tool parameters: {}", e))
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
