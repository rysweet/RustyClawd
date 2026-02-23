//! Hook execution context and related types

use crate::hooks::event::HookEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// Last assistant message content (included in Stop/SubagentStop events, v2.1.47)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_assistant_message: Option<String>,
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
            last_assistant_message: None,
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
            last_assistant_message: None,
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
            last_assistant_message: None,
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
            last_assistant_message: None,
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
            last_assistant_message: None,
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
            last_assistant_message: None,
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
            last_assistant_message: None,
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
        additional.insert("agent_id".to_string(), serde_json::Value::String(agent_id));
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
            last_assistant_message: None,
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
        additional.insert("agent_id".to_string(), serde_json::Value::String(agent_id));
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
            last_assistant_message: None,
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

    /// Set last assistant message (for Stop/SubagentStop events, v2.1.47)
    pub fn with_last_assistant_message(mut self, message: String) -> Self {
        self.last_assistant_message = Some(message);
        self
    }

    /// Create context for WorktreeCreate event (v2.1.50).
    /// Fires when a git worktree is created for agent isolation.
    pub fn for_worktree_create(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        worktree_path: String,
        branch_name: String,
    ) -> Self {
        let mut additional = HashMap::new();
        additional.insert(
            "worktree_path".to_string(),
            serde_json::Value::String(worktree_path),
        );
        additional.insert(
            "branch_name".to_string(),
            serde_json::Value::String(branch_name),
        );
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: HookEvent::WorktreeCreate.as_str().to_string(),
            tool_name: None,
            tool_use_id: None,
            tool_params: None,
            tool_result: None,
            session_start_matcher: None,
            session_end_reason: None,
            notification_type: None,
            user_prompt: None,
            last_assistant_message: None,
            additional,
        }
    }

    /// Create context for WorktreeRemove event (v2.1.50).
    /// Fires when a git worktree is removed after agent completes.
    pub fn for_worktree_remove(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        worktree_path: String,
    ) -> Self {
        let mut additional = HashMap::new();
        additional.insert(
            "worktree_path".to_string(),
            serde_json::Value::String(worktree_path),
        );
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: HookEvent::WorktreeRemove.as_str().to_string(),
            tool_name: None,
            tool_use_id: None,
            tool_params: None,
            tool_result: None,
            session_start_matcher: None,
            session_end_reason: None,
            notification_type: None,
            user_prompt: None,
            last_assistant_message: None,
            additional,
        }
    }

    /// Create context for ConfigChange event (v2.1.49).
    /// Fires when a configuration file changes on disk.
    pub fn for_config_change(
        session_id: String,
        transcript_path: String,
        cwd: String,
        permission_mode: String,
        config_path: String,
    ) -> Self {
        let mut additional = HashMap::new();
        additional.insert(
            "config_path".to_string(),
            serde_json::Value::String(config_path),
        );
        Self {
            session_id,
            transcript_path,
            cwd,
            permission_mode,
            hook_event_name: HookEvent::ConfigChange.as_str().to_string(),
            tool_name: None,
            tool_use_id: None,
            tool_params: None,
            tool_result: None,
            session_start_matcher: None,
            session_end_reason: None,
            notification_type: None,
            user_prompt: None,
            last_assistant_message: None,
            additional,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_worktree_create_context() {
        let ctx = HookContext::for_worktree_create(
            "session-123".to_string(),
            "/path/to/transcript".to_string(),
            "/cwd".to_string(),
            "auto".to_string(),
            "/tmp/worktree-1".to_string(),
            "feat/my-feature".to_string(),
        );
        assert_eq!(ctx.hook_event_name, "WorktreeCreate");
        assert_eq!(
            ctx.additional.get("worktree_path").unwrap(),
            &serde_json::Value::String("/tmp/worktree-1".to_string())
        );
        assert_eq!(
            ctx.additional.get("branch_name").unwrap(),
            &serde_json::Value::String("feat/my-feature".to_string())
        );
    }

    #[test]
    fn test_worktree_remove_context() {
        let ctx = HookContext::for_worktree_remove(
            "session-123".to_string(),
            "/path/to/transcript".to_string(),
            "/cwd".to_string(),
            "auto".to_string(),
            "/tmp/worktree-1".to_string(),
        );
        assert_eq!(ctx.hook_event_name, "WorktreeRemove");
        assert_eq!(
            ctx.additional.get("worktree_path").unwrap(),
            &serde_json::Value::String("/tmp/worktree-1".to_string())
        );
    }

    #[test]
    fn test_config_change_context() {
        let ctx = HookContext::for_config_change(
            "session-123".to_string(),
            "/path/to/transcript".to_string(),
            "/cwd".to_string(),
            "auto".to_string(),
            ".claude/settings.json".to_string(),
        );
        assert_eq!(ctx.hook_event_name, "ConfigChange");
        assert_eq!(
            ctx.additional.get("config_path").unwrap(),
            &serde_json::Value::String(".claude/settings.json".to_string())
        );
    }

    #[test]
    fn test_last_assistant_message_builder() {
        let ctx = HookContext::for_session(
            "session-123".to_string(),
            "/path/to/transcript".to_string(),
            "/cwd".to_string(),
            "auto".to_string(),
            HookEvent::Stop,
        )
        .with_last_assistant_message("Final response text".to_string());
        assert_eq!(
            ctx.last_assistant_message,
            Some("Final response text".to_string())
        );
    }
}
