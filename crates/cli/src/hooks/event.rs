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
    /// Fires when CLAUDE.md or rules files are loaded (v2.1.69)
    InstructionsLoaded,
    /// Fires during initial setup/first run (v2.1.10)
    Setup,
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
            HookEvent::InstructionsLoaded,
            HookEvent::Setup,
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
            HookEvent::InstructionsLoaded => "InstructionsLoaded",
            HookEvent::Setup => "Setup",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_all() {
        let events = HookEvent::all();
        assert_eq!(events.len(), 17);
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

    #[test]
    fn test_instructions_loaded_event() {
        let event = HookEvent::InstructionsLoaded;
        assert_eq!(event.as_str(), "InstructionsLoaded");
        assert!(HookEvent::all().contains(&event));
    }

    #[test]
    fn test_setup_event() {
        let event = HookEvent::Setup;
        assert_eq!(event.as_str(), "Setup");
        assert!(HookEvent::all().contains(&event));
    }

    #[test]
    fn test_instructions_loaded_serde_roundtrip() {
        let event = HookEvent::InstructionsLoaded;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"InstructionsLoaded\"");
        let deserialized: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, event);
    }

    #[test]
    fn test_setup_serde_roundtrip() {
        let event = HookEvent::Setup;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"Setup\"");
        let deserialized: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, event);
    }
}
