//! Hook lifecycle event definitions

use serde::{Deserialize, Serialize};

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
    /// Fires when a git worktree is created for agent isolation (v2.1.50)
    WorktreeCreate,
    /// Fires when a git worktree is removed after agent completes (v2.1.50)
    WorktreeRemove,
    /// Fires when a configuration file changes on disk (v2.1.49)
    ConfigChange,
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
            HookEvent::WorktreeCreate,
            HookEvent::WorktreeRemove,
            HookEvent::ConfigChange,
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
            HookEvent::WorktreeCreate => "WorktreeCreate",
            HookEvent::WorktreeRemove => "WorktreeRemove",
            HookEvent::ConfigChange => "ConfigChange",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_all() {
        let events = HookEvent::all();
        assert_eq!(events.len(), 15);
        assert!(events.contains(&HookEvent::SessionStart));
        assert!(events.contains(&HookEvent::Stop));
        assert!(events.contains(&HookEvent::PermissionRequest));
        assert!(events.contains(&HookEvent::TeammateIdle));
        assert!(events.contains(&HookEvent::TaskCompleted));
        assert!(events.contains(&HookEvent::WorktreeCreate));
        assert!(events.contains(&HookEvent::WorktreeRemove));
        assert!(events.contains(&HookEvent::ConfigChange));
    }

    #[test]
    fn test_permission_request_event_as_str() {
        assert_eq!(HookEvent::PermissionRequest.as_str(), "PermissionRequest");
    }

    #[test]
    fn test_worktree_events_as_str() {
        assert_eq!(HookEvent::WorktreeCreate.as_str(), "WorktreeCreate");
        assert_eq!(HookEvent::WorktreeRemove.as_str(), "WorktreeRemove");
    }

    #[test]
    fn test_config_change_event_as_str() {
        assert_eq!(HookEvent::ConfigChange.as_str(), "ConfigChange");
    }
}
