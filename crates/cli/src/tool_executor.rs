//! Tool executor for Claude API tool calls
//!
//! This module bridges between Anthropic API tool calls and our internal tool implementations.

use crate::hooks;
use crate::permission_mode::PermissionMode;
use crate::terminal_guard::TerminalGuard;
use crate::tool_schema_errors::create_schema_error;

// Import notification types
use crate::hooks::NotificationType;
use crate::notification::NotificationManager;
use anyhow::Result;
use futures::StreamExt;
use rustyclawd_core::client::ClientError;
use rustyclawd_tools::{
    AgentOutputTool, AgentTool, AskUserQuestionTool, BashOutputTool, BashTool, EditTool, GlobTool,
    GrepTool, KillShellTool, ReadTool, SkillTool, SlashCommandTool, TodoWriteTool, Tool,
    ToolContext, ToolEvent, WebFetchTool, WebSearchTool, WriteTool,
};
use serde_json::{json, Value};
use std::sync::Arc;

/// Execute a tool by name with given parameters
///
/// This function takes the tool name and input from Claude's API response,
/// executes the corresponding internal tool, and returns the result as JSON.
pub async fn execute_tool(tool_name: String, tool_input: Value) -> Result<Value, ClientError> {
    execute_tool_with_hooks(
        tool_name,
        tool_input,
        ToolExecutionParams {
            hooks: None,
            session_id: None,
            notification_manager: None,
            tool_use_id: None,
            allowed_tools: vec![],
            disallowed_tools: vec![],
            sdk_transport: None,
            sdk_hook_config: None,
            runtime_agents: std::collections::HashMap::new(),
        },
    )
    .await
}

/// Parameters for tool execution with optional context
pub struct ToolExecutionParams<'a> {
    pub hooks: Option<Arc<hooks::HooksSystem>>,
    pub session_id: Option<String>,
    pub notification_manager: Option<&'a NotificationManager>,
    pub tool_use_id: Option<String>,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    /// SDK bidirectional transport for hook callbacks.
    pub sdk_transport: Option<Arc<crate::sdk_transport::SdkTransport>>,
    /// SDK hook configuration (event matchers and callback IDs).
    pub sdk_hook_config: Option<Arc<crate::sdk_transport::SdkHookConfig>>,
    /// Runtime agents registered via --agents CLI flag.
    pub runtime_agents: std::collections::HashMap<String, rustyclawd_tools::RuntimeAgentInfo>,
}

/// Execute a tool with permission mode checking
///
/// Checks permission mode before execution and blocks tools in Plan mode.
pub async fn execute_tool_with_permission(
    tool_name: String,
    tool_input: Value,
    permission_mode: PermissionMode,
    params: ToolExecutionParams<'_>,
) -> Result<Value, ClientError> {
    // Check permission mode first
    if !permission_mode.allows_tool(&tool_name) {
        return Err(ClientError::ToolExecution(
            permission_mode.blocked_tool_error(&tool_name),
        ));
    }

    // Proceed with normal execution
    execute_tool_with_hooks(tool_name, tool_input, params).await
}

/// Execute a tool with optional hooks system and session context
///
/// Hooks are executed before and after tool execution:
/// - PreToolUse: Can block execution with "deny" decision
/// - PostToolUse: Non-blocking, for logging/monitoring
pub async fn execute_tool_with_hooks(
    tool_name: String,
    tool_input: Value,
    params: ToolExecutionParams<'_>,
) -> Result<Value, ClientError> {
    let hooks = params.hooks;
    let session_id = params.session_id;
    let notification_manager = params.notification_manager;
    let tool_use_id = params.tool_use_id;
    let allowed_tools = params.allowed_tools;
    let disallowed_tools = params.disallowed_tools;
    let sdk_transport = params.sdk_transport;
    let sdk_hook_config = params.sdk_hook_config;
    let runtime_agents = params.runtime_agents;
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
        allowed_tools: allowed_tools.clone(),
        disallowed_tools: disallowed_tools.clone(),
        runtime_agents,
    };

    // Check if tool is explicitly disallowed (takes precedence)
    if !ctx.disallowed_tools.is_empty() && ctx.disallowed_tools.contains(&tool_name) {
        return Err(ClientError::ToolExecution(format!(
            "Tool execution blocked: Tool '{}' is disallowed for this session",
            tool_name
        )));
    }

    // Check if tool is in allowed list (if allowlist is non-empty)
    if !ctx.allowed_tools.is_empty() && !ctx.allowed_tools.contains(&tool_name) {
        return Err(ClientError::ToolExecution(format!(
            "Tool execution blocked: Tool '{}' is not in the allowed tools list for this session",
            tool_name
        )));
    }

    // Execute PreToolUse hook (BLOCKING - can deny execution)
    if let (Some(ref hooks_system), Some(ref sess_id)) = (&hooks, &session_id) {
        let hook_context = hooks::HookContext::for_tool(
            sess_id.clone(),
            format!(".claude/sessions/{}/transcript.json", sess_id),
            ctx.cwd.to_string_lossy().to_string(),
            "ask".to_string(),
            hooks::HookEvent::PreToolUse,
            tool_name.clone(),
            tool_use_id.clone(),
        )
        .with_tool_params(tool_input.clone());

        match hooks_system
            .execute_hooks(hooks::HookEvent::PreToolUse, &hook_context)
            .await
        {
            Ok(results) => {
                for result in results {
                    if let Some(output) = result.parse_output() {
                        // Check permission decision
                        if let Some(decision) = output.permission_decision {
                            // Fire PermissionPrompt notification when Ask decision is detected
                            if decision == hooks::types::PermissionDecision::Ask {
                                if let (Some(notification_mgr), Some(ref sess_id)) =
                                    (notification_manager, &session_id)
                                {
                                    notification_mgr
                                        .notify(
                                            sess_id,
                                            NotificationType::PermissionPrompt,
                                            &format!("Permission required for tool: {}", tool_name),
                                        )
                                        .await;
                                }
                            }

                            if decision == hooks::types::PermissionDecision::Deny {
                                let reason = output
                                    .permission_decision_reason
                                    .unwrap_or_else(|| "Permission denied by hook".to_string());
                                return Err(ClientError::ToolExecution(format!(
                                    "Tool execution blocked: {}",
                                    reason
                                )));
                            }
                        }
                    }
                    if !result.is_success() {
                        tracing::warn!("PreToolUse hook failed: {}", result.stderr);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to execute PreToolUse hooks: {}", e);
                // Non-blocking - continue with execution even if hook fails
            }
        }
    }

    // SDK hook callbacks (PreToolUse) -- bidirectional protocol via stdin/stdout.
    if let (Some(ref transport), Some(ref config)) = (&sdk_transport, &sdk_hook_config) {
        for (callback_id, _pattern) in config.get_matching_callbacks("PreToolUse", &tool_name) {
            let input = json!({
                "tool_name": tool_name,
                "tool_input": tool_input,
            });
            match transport.send_hook_callback(&callback_id, "PreToolUse", &input) {
                Ok(output) => {
                    if let Some(decision) = output.get("decision").and_then(|d| d.as_str()) {
                        if decision == "deny" {
                            let reason = output
                                .get("reason")
                                .and_then(|r| r.as_str())
                                .unwrap_or("Denied by SDK hook");
                            return Err(ClientError::ToolExecution(format!(
                                "SDK hook denied: {}",
                                reason
                            )));
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("SDK hook callback failed for {}: {}", callback_id, e);
                    // Non-blocking -- continue with execution even if callback fails.
                }
            }
        }
    }

    // Execute the tool
    let result = match tool_name.as_str() {
        "Bash" => execute_bash_tool(tool_input.clone(), &ctx).await,
        "BashOutput" => execute_bash_output_tool(tool_input.clone(), &ctx).await,
        "KillShell" => execute_kill_shell_tool(tool_input.clone(), &ctx).await,
        "Read" => execute_read_tool(tool_input.clone(), &ctx).await,
        "Write" => execute_write_tool(tool_input.clone(), &ctx).await,
        "Edit" => execute_edit_tool(tool_input.clone(), &ctx).await,
        "Glob" => execute_glob_tool(tool_input.clone(), &ctx).await,
        "Grep" => execute_grep_tool(tool_input.clone(), &ctx).await,
        "AskUserQuestion" => execute_ask_user_question_tool(tool_input.clone(), &ctx).await,
        "Skill" => execute_skill_tool(tool_input.clone(), &ctx).await,
        "SlashCommand" => execute_slash_command_tool(tool_input.clone(), &ctx).await,
        "Task" => execute_agent_tool(tool_input.clone(), &ctx).await,
        "AgentOutput" => execute_agent_output_tool(tool_input.clone(), &ctx).await,
        "TodoWrite" => execute_todowrite_tool(tool_input.clone(), &ctx).await,
        "WebFetch" => execute_web_fetch_tool(tool_input.clone(), &ctx).await,
        "WebSearch" => execute_web_search_tool(tool_input.clone(), &ctx).await,
        _ => Err(ClientError::ToolExecution(format!(
            "Unknown tool: {}",
            tool_name
        ))),
    };

    // Execute PostToolUse hook (NON-BLOCKING - for logging/monitoring)
    if let (Some(ref hooks_system), Some(ref sess_id)) = (&hooks, &session_id) {
        // Convert result to JSON value for hook context
        let result_value = match &result {
            Ok(val) => val.clone(),
            Err(e) => json!({"error": e.to_string()}),
        };

        let hook_context = hooks::HookContext::for_tool(
            sess_id.clone(),
            format!(".claude/sessions/{}/transcript.json", sess_id),
            ctx.cwd.to_string_lossy().to_string(),
            "ask".to_string(),
            hooks::HookEvent::PostToolUse,
            tool_name.clone(),
            tool_use_id.clone(),
        )
        .with_tool_params(tool_input.clone())
        .with_tool_result(result_value);

        match hooks_system
            .execute_hooks(hooks::HookEvent::PostToolUse, &hook_context)
            .await
        {
            Ok(results) => {
                for hook_result in results {
                    if !hook_result.is_success() {
                        tracing::warn!("PostToolUse hook failed: {}", hook_result.stderr);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to execute PostToolUse hooks: {}", e);
                // Non-blocking - don't affect tool execution result
            }
        }
    }

    // SDK hook callbacks (PostToolUse) -- non-blocking, for logging/monitoring.
    if let (Some(ref transport), Some(ref config)) = (&sdk_transport, &sdk_hook_config) {
        let result_value = match &result {
            Ok(val) => val.clone(),
            Err(e) => json!({"error": e.to_string()}),
        };
        for (callback_id, _pattern) in config.get_matching_callbacks("PostToolUse", &tool_name) {
            let input = json!({
                "tool_name": tool_name,
                "tool_input": tool_input,
                "tool_result": result_value,
            });
            if let Err(e) = transport.send_hook_callback(&callback_id, "PostToolUse", &input) {
                tracing::warn!(
                    "SDK PostToolUse hook callback failed for {}: {}",
                    callback_id,
                    e
                );
            }
        }
    }

    result
}

/// Collect the result from a tool's event stream.
///
/// This is the shared helper that eliminates boilerplate across all tool executors.
/// It iterates the stream, returning the first Result event as a serialized JSON value,
/// or an error if the stream yields an Error event or completes without a result.
async fn collect_tool_stream<T: serde::Serialize>(
    tool_name: &str,
    mut stream: rustyclawd_tools::ToolStream<T>,
) -> Result<Value, ClientError> {
    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::ToolExecution(format!(
                        "Failed to serialize {} output: {}",
                        tool_name, e
                    ))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::ToolExecution(format!(
                    "{} tool error: {}",
                    tool_name, message
                )));
            }
            ToolEvent::Progress { .. } => {}
        }
    }
    Err(ClientError::ToolExecution(format!(
        "{} tool completed without result",
        tool_name
    )))
}

/// Generic tool execution: deserialize params, run tool, collect stream.
///
/// Handles the full lifecycle for any tool that implements the Tool trait.
/// The `tool` parameter is the constructed tool instance (unit struct or ::new()).
async fn execute_tool_generic<T: Tool>(
    tool_name: &str,
    input: Value,
    ctx: &ToolContext,
    tool: T,
) -> Result<Value, ClientError> {
    let params: T::Params = serde_json::from_value(input)
        .map_err(|e| create_schema_error(tool_name, &e.to_string()))?;
    let stream = tool.execute(params, ctx).await.map_err(|e| {
        ClientError::ToolExecution(format!("{} tool execution failed: {}", tool_name, e))
    })?;
    collect_tool_stream(tool_name, stream).await
}

/// Execute Bash tool
async fn execute_bash_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    // NOTE: TerminalGuard is NOT used here because bash tools now execute in background tasks
    // during TUI mode. Suspending terminal state would black out the TUI and break interactivity.
    // Instead, bash subprocesses are isolated from terminal via proper stdio redirection
    // (stdin redirected to /dev/null, stdout/stderr captured).
    execute_tool_generic("Bash", input, ctx, BashTool).await
}

/// Execute Read tool
async fn execute_read_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("Read", input, ctx, ReadTool).await
}

/// Execute Write tool
async fn execute_write_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("Write", input, ctx, WriteTool).await
}

/// Execute Edit tool
async fn execute_edit_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("Edit", input, ctx, EditTool).await
}

/// Execute Glob tool
async fn execute_glob_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("Glob", input, ctx, GlobTool).await
}

/// Execute Grep tool
async fn execute_grep_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("Grep", input, ctx, GrepTool).await
}

/// Execute BashOutput tool
async fn execute_bash_output_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("BashOutput", input, ctx, BashOutputTool).await
}

/// Execute KillShell tool
async fn execute_kill_shell_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("KillShell", input, ctx, KillShellTool).await
}

/// Execute AskUserQuestion tool
///
/// Special case: uses TerminalGuard and prints progress to stderr.
async fn execute_ask_user_question_tool(
    input: Value,
    ctx: &ToolContext,
) -> Result<Value, ClientError> {
    // Protect terminal state during interactive prompts
    let _guard = TerminalGuard::new().map_err(|e| {
        ClientError::ToolExecution(format!("Failed to create terminal guard: {}", e))
    })?;

    let params: rustyclawd_tools::ask_user_question::AskUserQuestionParams =
        serde_json::from_value(input)
            .map_err(|e| create_schema_error("AskUserQuestion", &e.to_string()))?;

    let tool = AskUserQuestionTool;
    let mut stream = tool.execute(params, ctx).await.map_err(|e| {
        ClientError::ToolExecution(format!("AskUserQuestion tool execution failed: {}", e))
    })?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::ToolExecution(format!(
                        "Failed to serialize AskUserQuestion output: {}",
                        e
                    ))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::ToolExecution(format!(
                    "AskUserQuestion tool error: {}",
                    message
                )));
            }
            ToolEvent::Progress { step, percentage } => {
                // Print progress to stderr so user can see what's happening
                if let Some(pct) = percentage {
                    eprintln!("[{:.0}%] {}", pct, step);
                } else {
                    eprintln!("{}", step);
                }
            }
        }
    }

    Err(ClientError::ToolExecution(
        "AskUserQuestion tool completed without result".to_string(),
    ))
    // Guard is automatically dropped here, restoring terminal state
}

/// Execute Skill tool
async fn execute_skill_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("Skill", input, ctx, SkillTool).await
}

/// Execute SlashCommand tool
async fn execute_slash_command_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("SlashCommand", input, ctx, SlashCommandTool).await
}

/// Execute TodoWrite tool
async fn execute_todowrite_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("TodoWrite", input, ctx, TodoWriteTool).await
}

/// Execute Agent/Task tool
async fn execute_agent_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("Task", input, ctx, AgentTool).await
}

/// Execute AgentOutput tool
async fn execute_agent_output_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("AgentOutput", input, ctx, AgentOutputTool).await
}

/// Execute WebFetch tool
async fn execute_web_fetch_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("WebFetch", input, ctx, WebFetchTool::new()).await
}

/// Execute WebSearch tool
async fn execute_web_search_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    execute_tool_generic("WebSearch", input, ctx, WebSearchTool::new()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_execute_agent_tool_invalid_params_missing_required() {
        let ctx = ToolContext {
            cwd: std::env::current_dir().unwrap_or_default(),
            debug: false,
            metadata: serde_json::Value::Null,
            execution_context: rustyclawd_tools::ExecutionContext::NonInteractive,
            allowed_tools: vec![],
            disallowed_tools: vec![],
            runtime_agents: std::collections::HashMap::new(),
        };

        // Missing required fields
        let invalid_input = json!({
            "subagent_type": "test_agent"
            // Missing "prompt" and "description"
        });

        let result = execute_agent_tool(invalid_input, &ctx).await;
        assert!(result.is_err(), "Should fail with missing required fields");

        if let Err(ClientError::ToolExecution(msg)) = result {
            assert!(
                msg.contains("required"),
                "Error should mention required fields"
            );
        }
    }

    #[tokio::test]
    async fn test_execute_agent_tool_invalid_params_wrong_types() {
        let ctx = ToolContext {
            cwd: std::env::current_dir().unwrap_or_default(),
            debug: false,
            metadata: serde_json::Value::Null,
            execution_context: rustyclawd_tools::ExecutionContext::NonInteractive,
            allowed_tools: vec![],
            disallowed_tools: vec![],
            runtime_agents: std::collections::HashMap::new(),
        };

        // Wrong type for subagent_type (should be string)
        let invalid_input = json!({
            "subagent_type": 123,
            "prompt": "test",
            "description": "test"
        });

        let result = execute_agent_tool(invalid_input, &ctx).await;
        assert!(result.is_err(), "Should fail with wrong parameter types");
    }

    #[tokio::test]
    async fn test_execute_tool_routes_to_task() {
        // Test that the execute_tool function correctly routes to Task
        let result = execute_tool(
            "Task".to_string(),
            json!({
                "subagent_type": "test",
                // Missing required fields to trigger error quickly
            }),
        )
        .await;

        assert!(result.is_err(), "Should fail with missing parameters");
        // If it routes correctly, we should get a schema error, not "Unknown tool" error
        if let Err(ClientError::ToolExecution(msg)) = result {
            assert!(
                !msg.contains("Unknown tool"),
                "Should not be unknown tool error"
            );
        }
    }
}
