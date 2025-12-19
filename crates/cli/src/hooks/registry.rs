//! Hook registry for managing and retrieving hooks

use crate::hooks::types::{Hook, HookConfig, HookContext, HookEvent, HooksConfiguration};

/// Hook registry manages hook configurations and lookups
#[derive(Clone)]
pub struct HookRegistry {
    configuration: HooksConfiguration,
}

impl HookRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            configuration: HooksConfiguration::default(),
        }
    }

    /// Register a complete hooks configuration
    pub fn register_configuration(&mut self, config: HooksConfiguration) {
        self.configuration = config;
    }

    /// Get hooks for a specific event and context
    pub fn get_hooks_for_event(&self, event: &HookEvent, context: &HookContext) -> Vec<Hook> {
        let configs = self.configuration.get_hooks_for_event(event);
        let mut hooks = Vec::new();

        // Get tool name from context for matching
        let tool_name = context.tool_name.as_deref().unwrap_or("*");

        // Collect all matching hooks
        for config in configs {
            if config.matcher.matches(tool_name) {
                hooks.extend(config.hooks.clone());
            }
        }

        hooks
    }

    /// Register a single hook for an event
    pub fn register_hook(&mut self, event: HookEvent, config: HookConfig) {
        let hooks = match event {
            HookEvent::SessionStart => &mut self.configuration.session_start,
            HookEvent::SessionEnd => &mut self.configuration.session_end,
            HookEvent::PreToolUse => &mut self.configuration.pre_tool_use,
            HookEvent::PostToolUse => &mut self.configuration.post_tool_use,
            HookEvent::UserPromptSubmit => &mut self.configuration.user_prompt_submit,
            HookEvent::Stop => &mut self.configuration.stop,
            HookEvent::SubagentStop => &mut self.configuration.subagent_stop,
            HookEvent::Notification => &mut self.configuration.notification,
            HookEvent::PreCompact => &mut self.configuration.pre_compact,
        };
        hooks.push(config);
    }

    /// Clear all hooks for a specific event
    pub fn clear_event_hooks(&mut self, event: &HookEvent) {
        let hooks = match event {
            HookEvent::SessionStart => &mut self.configuration.session_start,
            HookEvent::SessionEnd => &mut self.configuration.session_end,
            HookEvent::PreToolUse => &mut self.configuration.pre_tool_use,
            HookEvent::PostToolUse => &mut self.configuration.post_tool_use,
            HookEvent::UserPromptSubmit => &mut self.configuration.user_prompt_submit,
            HookEvent::Stop => &mut self.configuration.stop,
            HookEvent::SubagentStop => &mut self.configuration.subagent_stop,
            HookEvent::Notification => &mut self.configuration.notification,
            HookEvent::PreCompact => &mut self.configuration.pre_compact,
        };
        hooks.clear();
    }

    /// Clear all hooks
    pub fn clear_all(&mut self) {
        self.configuration = HooksConfiguration::default();
    }

    /// Get the current configuration
    pub fn configuration(&self) -> &HooksConfiguration {
        &self.configuration
    }

    /// Count total hooks registered
    pub fn count_total_hooks(&self) -> usize {
        self.configuration
            .session_start
            .iter()
            .map(|c| c.hooks.len())
            .sum::<usize>()
            + self
                .configuration
                .session_end
                .iter()
                .map(|c| c.hooks.len())
                .sum::<usize>()
            + self
                .configuration
                .pre_tool_use
                .iter()
                .map(|c| c.hooks.len())
                .sum::<usize>()
            + self
                .configuration
                .post_tool_use
                .iter()
                .map(|c| c.hooks.len())
                .sum::<usize>()
            + self
                .configuration
                .user_prompt_submit
                .iter()
                .map(|c| c.hooks.len())
                .sum::<usize>()
            + self
                .configuration
                .stop
                .iter()
                .map(|c| c.hooks.len())
                .sum::<usize>()
            + self
                .configuration
                .subagent_stop
                .iter()
                .map(|c| c.hooks.len())
                .sum::<usize>()
            + self
                .configuration
                .notification
                .iter()
                .map(|c| c.hooks.len())
                .sum::<usize>()
            + self
                .configuration
                .pre_compact
                .iter()
                .map(|c| c.hooks.len())
                .sum::<usize>()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::types::HookMatcher;

    #[test]
    fn test_registry_creation() {
        let registry = HookRegistry::new();
        assert_eq!(registry.count_total_hooks(), 0);
    }

    #[test]
    fn test_register_configuration() {
        let mut registry = HookRegistry::new();
        let mut config = HooksConfiguration::default();
        config.session_start.push(HookConfig {
            matcher: HookMatcher::Exact("*".to_string()),
            hooks: vec![Hook::command("echo test".to_string(), Some(60000))],
        });

        registry.register_configuration(config);
        assert_eq!(registry.count_total_hooks(), 1);
    }

    #[test]
    fn test_register_single_hook() {
        let mut registry = HookRegistry::new();
        registry.register_hook(
            HookEvent::SessionStart,
            HookConfig {
                matcher: HookMatcher::Exact("*".to_string()),
                hooks: vec![Hook::command("echo test".to_string(), Some(60000))],
            },
        );
        assert_eq!(registry.count_total_hooks(), 1);
    }

    #[test]
    fn test_get_hooks_for_event_exact_match() {
        let mut registry = HookRegistry::new();
        registry.register_hook(
            HookEvent::PreToolUse,
            HookConfig {
                matcher: HookMatcher::Exact("Write".to_string()),
                hooks: vec![Hook::command("validate.sh".to_string(), Some(60000))],
            },
        );

        let context = HookContext::for_tool(
            "session-123".to_string(),
            "/tmp/transcript".to_string(),
            "/home/user".to_string(),
            "auto".to_string(),
            HookEvent::PreToolUse,
            "Write".to_string(),
            None,
        );

        let hooks = registry.get_hooks_for_event(&HookEvent::PreToolUse, &context);
        assert_eq!(hooks.len(), 1);
    }

    #[test]
    fn test_get_hooks_for_event_no_match() {
        let mut registry = HookRegistry::new();
        registry.register_hook(
            HookEvent::PreToolUse,
            HookConfig {
                matcher: HookMatcher::Exact("Write".to_string()),
                hooks: vec![Hook::command("validate.sh".to_string(), Some(60000))],
            },
        );

        let context = HookContext::for_tool(
            "session-123".to_string(),
            "/tmp/transcript".to_string(),
            "/home/user".to_string(),
            "auto".to_string(),
            HookEvent::PreToolUse,
            "Read".to_string(),
            None,
        );

        let hooks = registry.get_hooks_for_event(&HookEvent::PreToolUse, &context);
        assert_eq!(hooks.len(), 0);
    }

    #[test]
    fn test_get_hooks_wildcard_match() {
        let mut registry = HookRegistry::new();
        registry.register_hook(
            HookEvent::SessionStart,
            HookConfig {
                matcher: HookMatcher::Exact("*".to_string()),
                hooks: vec![Hook::command("init.sh".to_string(), Some(60000))],
            },
        );

        let context = HookContext::for_session(
            "session-123".to_string(),
            "/tmp/transcript".to_string(),
            "/home/user".to_string(),
            "auto".to_string(),
            HookEvent::SessionStart,
        );

        let hooks = registry.get_hooks_for_event(&HookEvent::SessionStart, &context);
        assert_eq!(hooks.len(), 1);
    }

    #[test]
    fn test_get_hooks_regex_match() {
        let mut registry = HookRegistry::new();
        registry.register_hook(
            HookEvent::PreToolUse,
            HookConfig {
                matcher: HookMatcher::Regex("Edit|Write".to_string()),
                hooks: vec![Hook::prompt(None, Some(60000))],
            },
        );

        let context_edit = HookContext::for_tool(
            "session-123".to_string(),
            "/tmp/transcript".to_string(),
            "/home/user".to_string(),
            "auto".to_string(),
            HookEvent::PreToolUse,
            "Edit".to_string(),
            None,
        );

        let context_write = HookContext::for_tool(
            "session-123".to_string(),
            "/tmp/transcript".to_string(),
            "/home/user".to_string(),
            "auto".to_string(),
            HookEvent::PreToolUse,
            "Write".to_string(),
            None,
        );

        let hooks_edit = registry.get_hooks_for_event(&HookEvent::PreToolUse, &context_edit);
        let hooks_write = registry.get_hooks_for_event(&HookEvent::PreToolUse, &context_write);

        assert_eq!(hooks_edit.len(), 1);
        assert_eq!(hooks_write.len(), 1);
    }

    #[test]
    fn test_clear_event_hooks() {
        let mut registry = HookRegistry::new();
        registry.register_hook(
            HookEvent::SessionStart,
            HookConfig {
                matcher: HookMatcher::Exact("*".to_string()),
                hooks: vec![Hook::command("init.sh".to_string(), Some(60000))],
            },
        );

        assert_eq!(registry.count_total_hooks(), 1);
        registry.clear_event_hooks(&HookEvent::SessionStart);
        assert_eq!(registry.count_total_hooks(), 0);
    }

    #[test]
    fn test_clear_all() {
        let mut registry = HookRegistry::new();
        registry.register_hook(
            HookEvent::SessionStart,
            HookConfig {
                matcher: HookMatcher::Exact("*".to_string()),
                hooks: vec![Hook::command("init.sh".to_string(), Some(60000))],
            },
        );
        registry.register_hook(
            HookEvent::SessionEnd,
            HookConfig {
                matcher: HookMatcher::Exact("*".to_string()),
                hooks: vec![Hook::command("cleanup.sh".to_string(), Some(60000))],
            },
        );

        assert_eq!(registry.count_total_hooks(), 2);
        registry.clear_all();
        assert_eq!(registry.count_total_hooks(), 0);
    }

    #[test]
    fn test_multiple_hooks_same_event() {
        let mut registry = HookRegistry::new();
        registry.register_hook(
            HookEvent::SessionStart,
            HookConfig {
                matcher: HookMatcher::Exact("*".to_string()),
                hooks: vec![
                    Hook::command("hook1.sh".to_string(), Some(60000)),
                    Hook::command("hook2.sh".to_string(), Some(60000)),
                ],
            },
        );

        let context = HookContext::for_session(
            "session-123".to_string(),
            "/tmp/transcript".to_string(),
            "/home/user".to_string(),
            "auto".to_string(),
            HookEvent::SessionStart,
        );

        let hooks = registry.get_hooks_for_event(&HookEvent::SessionStart, &context);
        assert_eq!(hooks.len(), 2);
    }

    #[test]
    fn test_mcp_tool_targeting() {
        let mut registry = HookRegistry::new();
        registry.register_hook(
            HookEvent::PreToolUse,
            HookConfig {
                matcher: HookMatcher::Regex("mcp__.*".to_string()),
                hooks: vec![Hook::command("validate_mcp.sh".to_string(), Some(60000))],
            },
        );

        let context = HookContext::for_tool(
            "session-123".to_string(),
            "/tmp/transcript".to_string(),
            "/home/user".to_string(),
            "auto".to_string(),
            HookEvent::PreToolUse,
            "mcp__server__tool".to_string(),
            None,
        );

        let hooks = registry.get_hooks_for_event(&HookEvent::PreToolUse, &context);
        assert_eq!(hooks.len(), 1);
    }
}
