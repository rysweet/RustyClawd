//! Tool executor for Claude API tool calls
//!
//! This module bridges between Anthropic API tool calls and our internal tool implementations.

use crate::hooks;
use crate::permission_mode::PermissionMode;
use crate::terminal_guard::TerminalGuard;

// Import notification types
use crate::hooks::NotificationType;
use crate::notification::NotificationManager;
use anyhow::Result;
use futures::StreamExt;
use rustyclawd_core::client::ClientError;
use rustyclawd_tools::{
    AgentOutputTool, AgentTool, AskUserQuestionTool, BashOutputTool, BashTool, EditTool, GlobTool,
    GrepTool, KillShellTool, ReadTool, SkillTool, SlashCommandTool, TodoWriteTool, Tool,
    ToolContext, ToolEvent, WriteTool,
};
use serde_json::{json, Value};
use std::sync::Arc;

/// Create an educational error message that teaches Claude the correct schema
fn create_schema_error(tool_name: &str, error_msg: &str) -> ClientError {
    let (required_fields, optional_fields, example) = match tool_name {
        "Write" => (
            vec!["file_path", "content"],
            vec![],
            json!({
                "file_path": "/absolute/path/to/file.txt",
                "content": "The content to write to the file"
            }),
        ),
        "Read" => (
            vec!["file_path"],
            vec!["offset", "limit"],
            json!({
                "file_path": "/absolute/path/to/file.txt",
                "offset": 0,
                "limit": 100
            }),
        ),
        "Edit" => (
            vec!["file_path", "old_string", "new_string"],
            vec!["replace_all"],
            json!({
                "file_path": "/absolute/path/to/file.txt",
                "old_string": "text to replace",
                "new_string": "replacement text",
                "replace_all": false
            }),
        ),
        "Bash" => (
            vec!["command"],
            vec![
                "timeout",
                "description",
                "run_in_background",
                "dangerouslyDisableSandbox",
            ],
            json!({
                "command": "ls -la",
                "timeout": 120000,
                "description": "List files in directory"
            }),
        ),
        "Glob" => (
            vec!["pattern"],
            vec!["path"],
            json!({
                "pattern": "**/*.rs",
                "path": "/path/to/search"
            }),
        ),
        "Grep" => (
            vec!["pattern"],
            vec![
                "path",
                "output_mode",
                "glob",
                "type",
                "-i",
                "-n",
                "-A",
                "-B",
                "-C",
                "multiline",
                "head_limit",
                "offset",
            ],
            json!({
                "pattern": "search.*pattern",
                "path": "/path/to/search",
                "output_mode": "content"
            }),
        ),
        "BashOutput" => (
            vec!["bash_id"],
            vec!["filter"],
            json!({
                "bash_id": "shell_abc123",
                "filter": "ERROR.*"
            }),
        ),
        "KillShell" => (
            vec!["shell_id"],
            vec![],
            json!({
                "shell_id": "shell_abc123"
            }),
        ),
        "AskUserQuestion" => (
            vec!["questions"],
            vec!["answers"],
            json!({
                "questions": [{
                    "question": "What is your choice?",
                    "header": "choice",
                    "multiSelect": false,
                    "options": [
                        {"label": "Option 1", "description": "First option"},
                        {"label": "Option 2", "description": "Second option"}
                    ]
                }],
                "answers": {}
            }),
        ),
        "Skill" => (
            vec!["skill"],
            vec![],
            json!({
                "skill": "skill-name"
            }),
        ),
        "SlashCommand" => (
            vec!["command"],
            vec![],
            json!({
                "command": "/command-name arg1 arg2"
            }),
        ),
        "Task" => (
            vec!["subagent_type", "prompt", "description"],
            vec!["model", "resume", "run_in_background"],
            json!({
                "subagent_type": "agent_name",
                "prompt": "Full task description for the agent",
                "description": "Brief task summary",
                "model": "sonnet",
                "run_in_background": false
            }),
        ),
        "AgentOutput" => (
            vec!["agent_id"],
            vec![],
            json!({
                "agent_id": "agent_builder_t1234567890"
            }),
        ),
        "TodoWrite" => (
            vec!["todos"],
            vec![],
            json!({
                "todos": [
                    {
                        "content": "Task description",
                        "status": "pending",
                        "activeForm": "Present continuous form"
                    },
                    {
                        "content": "Another task",
                        "status": "in_progress",
                        "activeForm": "Doing another task"
                    }
                ]
            }),
        ),
        _ => (vec![], vec![], json!({})),
    };

    let error_response = json!({
        "error": format!("Parameter validation failed for {} tool: {}. Required fields: {:?}", tool_name, error_msg, required_fields),
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

    ClientError::Api(
        serde_json::to_string_pretty(&error_response).unwrap_or_else(|_| error_msg.to_string()),
    )
}

/// Execute a tool by name with given parameters
///
/// This function takes the tool name and input from Claude's API response,
/// executes the corresponding internal tool, and returns the result as JSON.
pub async fn execute_tool(tool_name: String, tool_input: Value) -> Result<Value, ClientError> {
    execute_tool_with_hooks(tool_name, tool_input, None, None, None).await
}

/// Execute a tool with permission mode checking
///
/// Checks permission mode before execution and blocks tools in Plan mode.
pub async fn execute_tool_with_permission(
    tool_name: String,
    tool_input: Value,
    permission_mode: PermissionMode,
    hooks: Option<Arc<hooks::HooksSystem>>,
    session_id: Option<String>,
    notification_manager: Option<&NotificationManager>,
) -> Result<Value, ClientError> {
    // Check permission mode first
    if !permission_mode.allows_tool(&tool_name) {
        return Err(ClientError::Api(
            permission_mode.blocked_tool_error(&tool_name),
        ));
    }

    // Proceed with normal execution
    execute_tool_with_hooks(
        tool_name,
        tool_input,
        hooks,
        session_id,
        notification_manager,
    )
    .await
}

/// Execute a tool with optional hooks system and session context
///
/// Hooks are executed before and after tool execution:
/// - PreToolUse: Can block execution with "deny" decision
/// - PostToolUse: Non-blocking, for logging/monitoring
pub async fn execute_tool_with_hooks(
    tool_name: String,
    tool_input: Value,
    hooks: Option<Arc<hooks::HooksSystem>>,
    session_id: Option<String>,
    notification_manager: Option<&NotificationManager>,
) -> Result<Value, ClientError> {
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

    // Execute PreToolUse hook (BLOCKING - can deny execution)
    if let (Some(ref hooks_system), Some(ref sess_id)) = (&hooks, &session_id) {
        let hook_context = hooks::HookContext::for_tool(
            sess_id.clone(),
            format!(".claude/sessions/{}/transcript.json", sess_id),
            ctx.cwd.to_string_lossy().to_string(),
            "ask".to_string(),
            hooks::HookEvent::PreToolUse,
            tool_name.clone(),
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
                                return Err(ClientError::Api(format!(
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
        _ => Err(ClientError::Api(format!("Unknown tool: {}", tool_name))),
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

    result
}

/// Execute Bash tool
async fn execute_bash_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    // Protect terminal state during bash execution
    let _guard = TerminalGuard::new()
        .map_err(|e| ClientError::Api(format!("Failed to create terminal guard: {}", e)))?;

    let params: rustyclawd_tools::bash::BashParams =
        serde_json::from_value(input).map_err(|e| create_schema_error("Bash", &e.to_string()))?;

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
        serde_json::from_value(input).map_err(|e| create_schema_error("Read", &e.to_string()))?;

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
        serde_json::from_value(input).map_err(|e| create_schema_error("Write", &e.to_string()))?;

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
        serde_json::from_value(input).map_err(|e| create_schema_error("Edit", &e.to_string()))?;

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
        serde_json::from_value(input).map_err(|e| create_schema_error("Glob", &e.to_string()))?;

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
        serde_json::from_value(input).map_err(|e| create_schema_error("Grep", &e.to_string()))?;

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

/// Execute BashOutput tool
async fn execute_bash_output_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::bash_output::BashOutputParams = serde_json::from_value(input)
        .map_err(|e| create_schema_error("BashOutput", &e.to_string()))?;

    let tool = BashOutputTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("BashOutput tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize BashOutput output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!(
                    "BashOutput tool error: {}",
                    message
                )));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "BashOutput tool completed without result".to_string(),
    ))
}

/// Execute KillShell tool
async fn execute_kill_shell_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::kill_shell::KillShellParams = serde_json::from_value(input)
        .map_err(|e| create_schema_error("KillShell", &e.to_string()))?;

    let tool = KillShellTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("KillShell tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize KillShell output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!(
                    "KillShell tool error: {}",
                    message
                )));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "KillShell tool completed without result".to_string(),
    ))
}

/// Execute AskUserQuestion tool
async fn execute_ask_user_question_tool(
    input: Value,
    ctx: &ToolContext,
) -> Result<Value, ClientError> {
    // Protect terminal state during interactive prompts
    let _guard = TerminalGuard::new()
        .map_err(|e| ClientError::Api(format!("Failed to create terminal guard: {}", e)))?;

    let params: rustyclawd_tools::ask_user_question::AskUserQuestionParams =
        serde_json::from_value(input)
            .map_err(|e| create_schema_error("AskUserQuestion", &e.to_string()))?;

    let tool = AskUserQuestionTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("AskUserQuestion tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize AskUserQuestion output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!(
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

    Err(ClientError::Api(
        "AskUserQuestion tool completed without result".to_string(),
    ))
    // Guard is automatically dropped here, restoring terminal state
}

/// Execute Skill tool
async fn execute_skill_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::skill::SkillParams =
        serde_json::from_value(input).map_err(|e| create_schema_error("Skill", &e.to_string()))?;

    let tool = SkillTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("Skill tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize Skill output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!("Skill tool error: {}", message)));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "Skill tool completed without result".to_string(),
    ))
}

/// Execute SlashCommand tool
async fn execute_slash_command_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::slash_command::SlashCommandParams = serde_json::from_value(input)
        .map_err(|e| create_schema_error("SlashCommand", &e.to_string()))?;

    let tool = SlashCommandTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("SlashCommand tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize SlashCommand output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!(
                    "SlashCommand tool error: {}",
                    message
                )));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "SlashCommand tool completed without result".to_string(),
    ))
}

/// Execute TodoWrite tool
async fn execute_todowrite_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::todo_write::TodoWriteParams = serde_json::from_value(input)
        .map_err(|e| create_schema_error("TodoWrite", &e.to_string()))?;

    let tool = TodoWriteTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("TodoWrite tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize TodoWrite output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!(
                    "TodoWrite tool error: {}",
                    message
                )));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "TodoWrite tool completed without result".to_string(),
    ))
}

/// Execute Agent/Task tool
async fn execute_agent_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::agent::AgentParams =
        serde_json::from_value(input).map_err(|e| create_schema_error("Task", &e.to_string()))?;

    let tool = AgentTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("Task tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize Task output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!("Task tool error: {}", message)));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "Task tool completed without result".to_string(),
    ))
}

/// Execute AgentOutput tool
async fn execute_agent_output_tool(input: Value, ctx: &ToolContext) -> Result<Value, ClientError> {
    let params: rustyclawd_tools::agent_output::AgentOutputParams =
        serde_json::from_value(input)
            .map_err(|e| create_schema_error("AgentOutput", &e.to_string()))?;

    let tool = AgentOutputTool;
    let mut stream = tool
        .execute(params, ctx)
        .await
        .map_err(|e| ClientError::Api(format!("AgentOutput tool execution failed: {}", e)))?;

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(output) => {
                return serde_json::to_value(&output).map_err(|e| {
                    ClientError::Api(format!("Failed to serialize AgentOutput output: {}", e))
                });
            }
            ToolEvent::Error { message } => {
                return Err(ClientError::Api(format!(
                    "AgentOutput tool error: {}",
                    message
                )));
            }
            ToolEvent::Progress { .. } => {}
        }
    }

    Err(ClientError::Api(
        "AgentOutput tool completed without result".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_schema_error_for_task_tool() {
        let error = create_schema_error("Task", "Missing required field");
        let error_msg = match error {
            ClientError::Api(msg) => msg,
            _ => panic!("Expected ClientError::Api"),
        };

        // Parse the error message as JSON
        let error_json: serde_json::Value =
            serde_json::from_str(&error_msg).expect("Error message should be valid JSON");

        // Verify required fields are present
        assert!(error_json.get("error").is_some());
        assert!(error_json.get("required_fields").is_some());
        assert!(error_json.get("optional_fields").is_some());
        assert!(error_json.get("example").is_some());

        // Verify Task-specific required fields
        let required = error_json["required_fields"].as_array().unwrap();
        assert!(required.contains(&json!("subagent_type")));
        assert!(required.contains(&json!("prompt")));
        assert!(required.contains(&json!("description")));
    }

    #[test]
    fn test_task_schema_error_includes_optional_fields() {
        let error = create_schema_error("Task", "Test error");
        let error_msg = match error {
            ClientError::Api(msg) => msg,
            _ => panic!("Expected ClientError::Api"),
        };

        let error_json: serde_json::Value = serde_json::from_str(&error_msg).unwrap();
        let optional = error_json["optional_fields"].as_array().unwrap();

        assert!(optional.contains(&json!("model")));
        assert!(optional.contains(&json!("resume")));
    }

    #[test]
    fn test_task_schema_error_includes_example() {
        let error = create_schema_error("Task", "Test error");
        let error_msg = match error {
            ClientError::Api(msg) => msg,
            _ => panic!("Expected ClientError::Api"),
        };

        let error_json: serde_json::Value = serde_json::from_str(&error_msg).unwrap();
        let example = &error_json["example"];

        assert!(example.get("subagent_type").is_some());
        assert!(example.get("prompt").is_some());
        assert!(example.get("description").is_some());
        assert_eq!(example["subagent_type"], "agent_name");
    }

    #[tokio::test]
    async fn test_execute_agent_tool_invalid_params_missing_required() {
        let ctx = ToolContext {
            cwd: std::env::current_dir().unwrap_or_default(),
            debug: false,
            metadata: serde_json::Value::Null,
            execution_context: rustyclawd_tools::ExecutionContext::NonInteractive,
        };

        // Missing required fields
        let invalid_input = json!({
            "subagent_type": "test_agent"
            // Missing "prompt" and "description"
        });

        let result = execute_agent_tool(invalid_input, &ctx).await;
        assert!(result.is_err(), "Should fail with missing required fields");

        if let Err(ClientError::Api(msg)) = result {
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
        if let Err(ClientError::Api(msg)) = result {
            assert!(
                !msg.contains("Unknown tool"),
                "Should not be unknown tool error"
            );
        }
    }

    #[test]
    fn test_all_schema_error_tools_include_task() {
        // Verify that Task is handled in create_schema_error
        let error = create_schema_error("Task", "test");
        let error_msg = match error {
            ClientError::Api(msg) => msg,
            _ => panic!("Expected ClientError::Api"),
        };

        let error_json: serde_json::Value = serde_json::from_str(&error_msg).unwrap();
        let required = error_json["required_fields"].as_array().unwrap();

        // Should not be empty (default case)
        assert!(
            !required.is_empty(),
            "Task tool should have specific schema error handling"
        );
    }

    #[test]
    fn test_task_schema_error_help_message() {
        let error = create_schema_error("Task", "test");
        let error_msg = match error {
            ClientError::Api(msg) => msg,
            _ => panic!("Expected ClientError::Api"),
        };

        let error_json: serde_json::Value = serde_json::from_str(&error_msg).unwrap();
        let help = error_json["help"].as_str().unwrap();

        assert!(
            help.contains("Task"),
            "Help message should mention Task tool"
        );
        assert!(
            help.contains("subagent_type"),
            "Help should list required fields"
        );
        assert!(help.contains("prompt"), "Help should list required fields");
        assert!(
            help.contains("description"),
            "Help should list required fields"
        );
    }
}
