//! Hook type definitions and data structures

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

/// Hook type - command (bash) or prompt (LLM)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookType {
    Command,
    Prompt,
}

/// Hook lifecycle events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    SessionStart,
    SessionEnd,
    PreToolUse,
    PostToolUse,
    UserPromptSubmit,
    Stop,
    SubagentStop,
    Notification,
    PreCompact,
    /// Fires when a tool requires permission and user would be prompted.
    /// Hook can return allow/deny/ask to automatically handle permission decisions.
    PermissionRequest,
    /// Fires when an agent becomes idle and available for new tasks (multi-agent coordination)
    TeammateIdle,
    /// Fires when an agent completes its assigned task (multi-agent coordination)
    TaskCompleted,
}

impl HookEvent {
    /// Get all possible hook events
    pub fn all() -> Vec<HookEvent> {
        vec![
            HookEvent::SessionStart,
            HookEvent::SessionEnd,
            HookEvent::PreToolUse,
            HookEvent::PostToolUse,
            HookEvent::UserPromptSubmit,
            HookEvent::Stop,
            HookEvent::SubagentStop,
            HookEvent::Notification,
            HookEvent::PreCompact,
            HookEvent::PermissionRequest,
            HookEvent::TeammateIdle,
            HookEvent::TaskCompleted,
        ]
    }

    /// Get event name as string
    pub fn as_str(&self) -> &str {
        match self {
            HookEvent::SessionStart => "SessionStart",
            HookEvent::SessionEnd => "SessionEnd",
            HookEvent::PreToolUse => "PreToolUse",
            HookEvent::PostToolUse => "PostToolUse",
            HookEvent::UserPromptSubmit => "UserPromptSubmit",
            HookEvent::Stop => "Stop",
            HookEvent::SubagentStop => "SubagentStop",
            HookEvent::Notification => "Notification",
            HookEvent::PreCompact => "PreCompact",
            HookEvent::PermissionRequest => "PermissionRequest",
            HookEvent::TeammateIdle => "TeammateIdle",
            HookEvent::TaskCompleted => "TaskCompleted",
        }
    }
}

/// Hook matcher for filtering tools/events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookMatcher {
    /// Match exact string
    Exact(String),
    /// Match regex pattern
    Regex(String),
}

impl Serialize for HookMatcher {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            HookMatcher::Exact(s) | HookMatcher::Regex(s) => s.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for HookMatcher {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        // Determine if it's a regex pattern or exact match
        if s == "*" {
            // "*" is an exact match that matches everything
            Ok(HookMatcher::Exact(s))
        } else if s.contains('|') || s.contains(".*") || s.contains(".") {
            // Contains regex special characters: pipe (alternation), .* (wildcard), dots
            Ok(HookMatcher::Regex(s))
        } else if is_mcp_server_wildcard(&s) {
            // MCP server wildcard pattern: mcp__<server>__*
            Ok(HookMatcher::Regex(s))
        } else {
            // Simple string - exact match
            Ok(HookMatcher::Exact(s))
        }
    }
}

/// Check if a pattern is an MCP server wildcard (mcp__<server>__*)
fn is_mcp_server_wildcard(pattern: &str) -> bool {
    pattern.starts_with("mcp__") && pattern.ends_with("__*") && pattern.matches("__").count() == 2
}

impl HookMatcher {
    /// Check if a tool name matches this matcher
    pub fn matches(&self, tool_name: &str) -> bool {
        match self {
            HookMatcher::Exact(pattern) => {
                // "*" matches everything
                if pattern == "*" {
                    return true;
                }
                tool_name == pattern
            }
            HookMatcher::Regex(pattern) => {
                // Simple regex matching for common patterns
                // Order matters: check specific patterns before generic ones

                // 1. Check exact "mcp__.*" (all MCP tools)
                if pattern == "mcp__.*" {
                    return tool_name.starts_with("mcp__");
                }

                // 2. Check MCP server wildcard pattern: mcp__<server>__*
                if is_mcp_server_wildcard(pattern) {
                    // Extract server name: "mcp__filesystem__*" → "filesystem"
                    let parts: Vec<&str> = pattern.split("__").collect();
                    if parts.len() == 3 {
                        let server_name = parts[1];
                        // Match: "mcp__<server>__<anything>"
                        return tool_name.starts_with(&format!("mcp__{}__", server_name));
                    }
                }

                // 3. Check full MCP pattern: mcp__.*__.*
                if pattern == "mcp__.*__.*" {
                    return tool_name.starts_with("mcp__") && tool_name.matches("__").count() >= 2;
                }

                // 4. Check alternation pattern like "Edit|Write"
                if pattern.contains('|') {
                    return pattern.split('|').any(|p| tool_name.contains(p.trim()));
                }

                // 5. Check generic prefix matching (ends with .*)
                if pattern.ends_with(".*") {
                    let prefix = pattern.trim_end_matches(".*");
                    return tool_name.starts_with(prefix);
                }

                // 6. Default: contains matching
                tool_name.contains(pattern)
            }
        }
    }
}

/// Individual hook configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hook {
    #[serde(rename = "type")]
    pub hook_type: HookType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(rename = "timeout", skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
}

impl Hook {
    /// Create a new command hook
    pub fn command(command: String, timeout_ms: Option<u32>) -> Self {
        Self {
            hook_type: HookType::Command,
            command: Some(command),
            prompt: None,
            timeout_ms,
        }
    }

    /// Create a new prompt hook
    pub fn prompt(prompt: Option<String>, timeout_ms: Option<u32>) -> Self {
        Self {
            hook_type: HookType::Prompt,
            command: None,
            prompt,
            timeout_ms,
        }
    }

    /// Get the default timeout (60 seconds)
    pub fn default_timeout() -> u32 {
        60000
    }

    /// Get the effective timeout for this hook
    pub fn effective_timeout(&self) -> u32 {
        self.timeout_ms.unwrap_or_else(Self::default_timeout)
    }
}

/// Hook configuration for a specific event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub matcher: HookMatcher,
    pub hooks: Vec<Hook>,
}

/// Complete hooks configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksConfiguration {
    #[serde(rename = "SessionStart", default)]
    pub session_start: Vec<HookConfig>,
    #[serde(rename = "SessionEnd", default)]
    pub session_end: Vec<HookConfig>,
    #[serde(rename = "PreToolUse", default)]
    pub pre_tool_use: Vec<HookConfig>,
    #[serde(rename = "PostToolUse", default)]
    pub post_tool_use: Vec<HookConfig>,
    #[serde(rename = "UserPromptSubmit", default)]
    pub user_prompt_submit: Vec<HookConfig>,
    #[serde(rename = "Stop", default)]
    pub stop: Vec<HookConfig>,
    #[serde(rename = "SubagentStop", default)]
    pub subagent_stop: Vec<HookConfig>,
    #[serde(rename = "Notification", default)]
    pub notification: Vec<HookConfig>,
    #[serde(rename = "PreCompact", default)]
    pub pre_compact: Vec<HookConfig>,
    /// PermissionRequest hooks fire when a tool requires permission and user would be prompted.
    /// Hook can return allow/deny/ask to automatically handle permission decisions.
    #[serde(rename = "PermissionRequest", default)]
    pub permission_request: Vec<HookConfig>,
    /// TeammateIdle hooks fire when an agent becomes idle and available for new tasks.
    /// Enables multi-agent coordination by notifying when agents finish their work.
    #[serde(rename = "TeammateIdle", default)]
    pub teammate_idle: Vec<HookConfig>,
    /// TaskCompleted hooks fire when an agent completes its assigned task.
    /// Allows teams to coordinate task hand-off and track progress.
    #[serde(rename = "TaskCompleted", default)]
    pub task_completed: Vec<HookConfig>,
}

impl HooksConfiguration {
    /// Get hook configs for a specific event
    pub fn get_hooks_for_event(&self, event: &HookEvent) -> &[HookConfig] {
        match event {
            HookEvent::SessionStart => &self.session_start,
            HookEvent::SessionEnd => &self.session_end,
            HookEvent::PreToolUse => &self.pre_tool_use,
            HookEvent::PostToolUse => &self.post_tool_use,
            HookEvent::UserPromptSubmit => &self.user_prompt_submit,
            HookEvent::Stop => &self.stop,
            HookEvent::SubagentStop => &self.subagent_stop,
            HookEvent::Notification => &self.notification,
            HookEvent::PreCompact => &self.pre_compact,
            HookEvent::PermissionRequest => &self.permission_request,
            HookEvent::TeammateIdle => &self.teammate_idle,
            HookEvent::TaskCompleted => &self.task_completed,
        }
    }
}

/// SessionStart matcher types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStartMatcher {
    Startup,
    Resume,
    Clear,
    Compact,
}

/// SessionEnd reasons
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    Clear,
    Logout,
    PromptInputExit,
    Other,
}

/// Notification types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    PermissionPrompt,
    IdlePrompt,
    AuthSuccess,
    ElicitationDialog,
}

/// Hook execution context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookContext {
    pub session_id: String,
    pub transcript_path: String,
    pub cwd: String,
    pub permission_mode: String,
    pub hook_event_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_start_matcher: Option<SessionStartMatcher>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_end_reason: Option<SessionEndReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_type: Option<NotificationType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_prompt: Option<String>,
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl HookContext {
    /// Create context for a tool event (PreToolUse/PostToolUse)
    pub fn for_tool(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        event: HookEvent,
        tool_name: String,
        tool_use_id: Option<String>,
    ) -> Self {
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: event.as_str().to_string(),
            tool_name: Some(tool_name),
            tool_use_id,
            tool_params: None,
            tool_result: None,
            session_start_matcher: None,
            session_end_reason: None,
            notification_type: None,
            user_prompt: None,
            additional: HashMap::new(),
        }
    }

    /// Create context for a session event
    pub fn for_session(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        event: HookEvent,
    ) -> Self {
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: event.as_str().to_string(),
            tool_name: None,
            tool_use_id: None,
            tool_params: None,
            tool_result: None,
            session_start_matcher: None,
            session_end_reason: None,
            notification_type: None,
            user_prompt: None,
            additional: HashMap::new(),
        }
    }

    /// Create context for SessionStart event
    pub fn for_session_start(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        matcher: SessionStartMatcher,
    ) -> Self {
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: HookEvent::SessionStart.as_str().to_string(),
            tool_name: None,
            tool_use_id: None,
            tool_params: None,
            tool_result: None,
            session_start_matcher: Some(matcher),
            session_end_reason: None,
            notification_type: None,
            user_prompt: None,
            additional: HashMap::new(),
        }
    }

    /// Create context for SessionEnd event
    pub fn for_session_end(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        reason: SessionEndReason,
    ) -> Self {
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: HookEvent::SessionEnd.as_str().to_string(),
            tool_name: None,
            tool_use_id: None,
            tool_params: None,
            tool_result: None,
            session_start_matcher: None,
            session_end_reason: Some(reason),
            notification_type: None,
            user_prompt: None,
            additional: HashMap::new(),
        }
    }

    /// Create context for Notification event
    pub fn for_notification(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        notification_type: NotificationType,
    ) -> Self {
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: HookEvent::Notification.as_str().to_string(),
            tool_name: None,
            tool_use_id: None,
            tool_params: None,
            tool_result: None,
            session_start_matcher: None,
            session_end_reason: None,
            notification_type: Some(notification_type),
            user_prompt: None,
            additional: HashMap::new(),
        }
    }

    /// Create context for UserPromptSubmit event
    pub fn for_user_prompt(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        user_prompt: String,
    ) -> Self {
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: HookEvent::UserPromptSubmit.as_str().to_string(),
            tool_name: None,
            tool_use_id: None,
            tool_params: None,
            tool_result: None,
            session_start_matcher: None,
            session_end_reason: None,
            notification_type: None,
            user_prompt: Some(user_prompt),
            additional: HashMap::new(),
        }
    }

    /// Create context for PermissionRequest event.
    /// Fires when a tool requires permission and user would be prompted.
    pub fn for_permission_request(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        tool_name: String,
        tool_use_id: Option<String>,
        tool_params: Option<serde_json::Value>,
    ) -> Self {
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: HookEvent::PermissionRequest.as_str().to_string(),
            tool_name: Some(tool_name),
            tool_use_id,
            tool_params,
            tool_result: None,
            session_start_matcher: None,
            session_end_reason: None,
            notification_type: None,
            user_prompt: None,
            additional: HashMap::new(),
        }
    }

    /// Create context for TeammateIdle event.
    /// Fires when an agent becomes idle and available for new tasks.
    pub fn for_teammate_idle(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        agent_id: String,
    ) -> Self {
        let mut additional = HashMap::new();
        additional.insert(
            "agent_id".to_string(),
            serde_json::Value::String(agent_id),
        );
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: HookEvent::TeammateIdle.as_str().to_string(),
            tool_name: None,
            tool_use_id: None,
            tool_params: None,
            tool_result: None,
            session_start_matcher: None,
            session_end_reason: None,
            notification_type: None,
            user_prompt: None,
            additional,
        }
    }

    /// Create context for TaskCompleted event.
    /// Fires when an agent completes its assigned task.
    pub fn for_task_completed(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        agent_id: String,
        agent_type: String,
    ) -> Self {
        let mut additional = HashMap::new();
        additional.insert(
            "agent_id".to_string(),
            serde_json::Value::String(agent_id),
        );
        additional.insert(
            "agent_type".to_string(),
            serde_json::Value::String(agent_type),
        );
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: HookEvent::TaskCompleted.as_str().to_string(),
            tool_name: None,
            tool_use_id: None,
            tool_params: None,
            tool_result: None,
            session_start_matcher: None,
            session_end_reason: None,
            notification_type: None,
            user_prompt: None,
            additional,
        }
    }

    /// Set tool parameters
    pub fn with_tool_params(mut self, params: serde_json::Value) -> Self {
        self.tool_params = Some(params);
        self
    }

    /// Set tool result
    pub fn with_tool_result(mut self, result: serde_json::Value) -> Self {
        self.tool_result = Some(result);
        self
    }
}

/// Hook execution result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl HookResult {
    /// Check if hook succeeded (exit code 0)
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Check if hook is blocking error (exit code 2)
    pub fn is_blocking(&self) -> bool {
        self.exit_code == 2
    }

    /// Check if hook is non-blocking error (exit code 1)
    pub fn is_non_blocking_error(&self) -> bool {
        self.exit_code == 1
    }

    /// Parse hook output as JSON
    pub fn parse_output(&self) -> Option<HookOutput> {
        if self.stdout.is_empty() {
            return None;
        }
        serde_json::from_str(&self.stdout).ok()
    }
}

/// Permission decision for PreToolUse hooks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

/// Stop decision for Stop/SubagentStop hooks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StopDecision {
    Approve,
    Block,
}

/// Hook output structure (JSON response)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookOutput {
    /// Whether to continue execution (default: true)
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_execution: Option<bool>,
    /// Message to show when continue=false
    #[serde(rename = "stopReason", skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Hide output from transcript (default: false)
    #[serde(rename = "suppressOutput", skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
    /// Warning message to show to user
    #[serde(rename = "systemMessage", skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
    /// Permission decision for PreToolUse hooks
    #[serde(rename = "permissionDecision", skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<PermissionDecision>,
    /// Reason for permission decision
    #[serde(
        rename = "permissionDecisionReason",
        skip_serializing_if = "Option::is_none"
    )]
    pub permission_decision_reason: Option<String>,
    /// Stop decision for Stop/SubagentStop hooks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<StopDecision>,
    /// Reason for decision (required when blocking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Additional context to inject
    #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
    /// Hook-specific output nested structure
    #[serde(rename = "hookSpecificOutput", skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

/// Hook-specific output for PreToolUse hooks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookSpecificOutput {
    /// Permission decision
    #[serde(rename = "permissionDecision", skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<PermissionDecision>,
    /// Reason for permission decision
    #[serde(
        rename = "permissionDecisionReason",
        skip_serializing_if = "Option::is_none"
    )]
    pub permission_decision_reason: Option<String>,
    /// Updated tool parameters
    #[serde(rename = "updatedInput", skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_all() {
        let events = HookEvent::all();
        assert_eq!(events.len(), 12);
        assert!(events.contains(&HookEvent::SessionStart));
        assert!(events.contains(&HookEvent::Stop));
        assert!(events.contains(&HookEvent::PermissionRequest));
        assert!(events.contains(&HookEvent::TeammateIdle));
        assert!(events.contains(&HookEvent::TaskCompleted));
    }

    #[test]
    fn test_hook_matcher_exact() {
        let matcher = HookMatcher::Exact("Write".to_string());
        assert!(matcher.matches("Write"));
        assert!(!matcher.matches("Read"));
    }

    #[test]
    fn test_hook_matcher_wildcard() {
        let matcher = HookMatcher::Exact("*".to_string());
        assert!(matcher.matches("Write"));
        assert!(matcher.matches("Read"));
        assert!(matcher.matches("anything"));
    }

    #[test]
    fn test_hook_matcher_regex_alternation() {
        let matcher = HookMatcher::Regex("Edit|Write".to_string());
        assert!(matcher.matches("Edit"));
        assert!(matcher.matches("Write"));
        assert!(!matcher.matches("Read"));
    }

    #[test]
    fn test_hook_matcher_mcp_prefix() {
        let matcher = HookMatcher::Regex("mcp__.*".to_string());
        assert!(matcher.matches("mcp__server__tool"));
        assert!(!matcher.matches("Bash"));
    }

    #[test]
    fn test_hook_result_success() {
        let result = HookResult {
            exit_code: 0,
            stdout: "success".to_string(),
            stderr: String::new(),
        };
        assert!(result.is_success());
        assert!(!result.is_blocking());
        assert!(!result.is_non_blocking_error());
    }

    #[test]
    fn test_hook_result_blocking() {
        let result = HookResult {
            exit_code: 2,
            stdout: String::new(),
            stderr: "blocked".to_string(),
        };
        assert!(!result.is_success());
        assert!(result.is_blocking());
        assert!(!result.is_non_blocking_error());
    }

    #[test]
    fn test_hook_output_parse() {
        let result = HookResult {
            exit_code: 0,
            stdout: r#"{"continue": true, "permissionDecision": "allow"}"#.to_string(),
            stderr: String::new(),
        };
        let output = result.parse_output().unwrap();
        assert_eq!(output.continue_execution, Some(true));
        assert_eq!(output.permission_decision, Some(PermissionDecision::Allow));
    }

    #[test]
    fn test_permission_request_event_as_str() {
        assert_eq!(HookEvent::PermissionRequest.as_str(), "PermissionRequest");
    }

    #[test]
    fn test_permission_request_context() {
        let ctx = HookContext::for_permission_request(
            "session-123".to_string(),
            "/path/to/transcript".to_string(),
            "/cwd".to_string(),
            "ask".to_string(),
            "Bash".to_string(),
            Some("tool-use-456".to_string()),
            Some(serde_json::json!({"command": "ls -la"})),
        );
        assert_eq!(ctx.hook_event_name, "PermissionRequest");
        assert_eq!(ctx.tool_name, Some("Bash".to_string()));
        assert_eq!(ctx.tool_use_id, Some("tool-use-456".to_string()));
        assert!(ctx.tool_params.is_some());
    }

    #[test]
    fn test_hooks_configuration_permission_request() {
        let json = r#"{
            "PermissionRequest": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "scripts/auto-approve.sh",
                            "timeout": 5000
                        }
                    ]
                }
            ]
        }"#;

        let config: HooksConfiguration = serde_json::from_str(json).unwrap();
        assert_eq!(config.permission_request.len(), 1);
        assert_eq!(
            config
                .get_hooks_for_event(&HookEvent::PermissionRequest)
                .len(),
            1
        );
    }

    #[test]
    fn test_permission_request_output_allow() {
        let json = r#"{"permissionDecision": "allow", "permissionDecisionReason": "Safe command"}"#;
        let output: HookOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.permission_decision, Some(PermissionDecision::Allow));
        assert_eq!(
            output.permission_decision_reason,
            Some("Safe command".to_string())
        );
    }

    #[test]
    fn test_permission_request_output_deny() {
        let json = r#"{"permissionDecision": "deny", "permissionDecisionReason": "Dangerous command detected"}"#;
        let output: HookOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.permission_decision, Some(PermissionDecision::Deny));
        assert_eq!(
            output.permission_decision_reason,
            Some("Dangerous command detected".to_string())
        );
    }

    #[test]
    fn test_permission_request_output_ask() {
        let json =
            r#"{"permissionDecision": "ask", "permissionDecisionReason": "Needs user review"}"#;
        let output: HookOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.permission_decision, Some(PermissionDecision::Ask));
        assert_eq!(
            output.permission_decision_reason,
            Some("Needs user review".to_string())
        );
    }

    #[test]
    fn test_teammate_idle_context() {
        let ctx = HookContext::for_teammate_idle(
            "session-123".to_string(),
            "/path/to/transcript".to_string(),
            "/cwd".to_string(),
            "auto".to_string(),
            "agent-42".to_string(),
        );
        assert_eq!(ctx.hook_event_name, "TeammateIdle");
        assert_eq!(
            ctx.additional.get("agent_id").unwrap(),
            &serde_json::Value::String("agent-42".to_string())
        );
        assert!(ctx.tool_name.is_none());
    }

    #[test]
    fn test_task_completed_context() {
        let ctx = HookContext::for_task_completed(
            "session-456".to_string(),
            "/path/to/transcript".to_string(),
            "/cwd".to_string(),
            "auto".to_string(),
            "agent-99".to_string(),
            "builder".to_string(),
        );
        assert_eq!(ctx.hook_event_name, "TaskCompleted");
        assert_eq!(
            ctx.additional.get("agent_id").unwrap(),
            &serde_json::Value::String("agent-99".to_string())
        );
        assert_eq!(
            ctx.additional.get("agent_type").unwrap(),
            &serde_json::Value::String("builder".to_string())
        );
        assert!(ctx.tool_name.is_none());
    }
}
