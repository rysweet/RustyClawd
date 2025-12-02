//! Claude Code Hooks System
//!
//! Provides lifecycle hooks for all 9 events with command and prompt-based execution.
//! Hooks can validate, transform, and control tool execution flow.
//!
//! # Hook Events
//! - SessionStart: Called when a new session begins
//! - SessionEnd: Called when a session ends
//! - PreToolUse: Called before tool execution (can block)
//! - PostToolUse: Called after tool execution
//! - UserPromptSubmit: Called when user submits a prompt
//! - Stop: Called when checking if work is complete
//! - SubagentStop: Called when a subagent stops
//! - Notification: Called for notification filtering
//! - PreCompact: Called before compacting conversation history

pub mod executor;
pub mod loader;
pub mod registry;
pub mod types;

pub use executor::HookExecutor;
pub use loader::HookLoader;
pub use registry::HookRegistry;
pub use types::{HookContext, HookEvent, HookResult};

use anyhow::Result;

/// Main hooks system interface
#[derive(Clone)]
pub struct HooksSystem {
    registry: HookRegistry,
    executor: HookExecutor,
}

impl HooksSystem {
    /// Create a new hooks system
    pub fn new() -> Self {
        Self {
            registry: HookRegistry::new(),
            executor: HookExecutor::new(),
        }
    }

    /// Load hooks from configuration file
    pub async fn load_from_file(&mut self, config_path: &str) -> Result<()> {
        let config = HookLoader::load_from_file(config_path).await?;
        self.registry.register_configuration(config);
        Ok(())
    }

    /// Execute hooks for a specific event
    pub async fn execute_hooks(
        &self,
        event: HookEvent,
        context: &HookContext,
    ) -> Result<Vec<HookResult>> {
        let hooks = self.registry.get_hooks_for_event(&event, context);
        self.executor.execute_hooks(&hooks, context).await
    }

    /// Get the registry (for testing)
    pub fn registry(&self) -> &HookRegistry {
        &self.registry
    }

    /// Get mutable registry (for testing)
    pub fn registry_mut(&mut self) -> &mut HookRegistry {
        &mut self.registry
    }
}

impl Default for HooksSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hooks_system_creation() {
        let system = HooksSystem::new();
        assert!(system
            .registry
            .get_hooks_for_event(&HookEvent::SessionStart, &HookContext::default())
            .is_empty());
    }

    #[test]
    fn test_hooks_system_default() {
        let system = HooksSystem::default();
        assert!(system
            .registry
            .get_hooks_for_event(&HookEvent::SessionEnd, &HookContext::default())
            .is_empty());
    }
}
