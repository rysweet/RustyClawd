//! Notification system with hook integration
//!
//! Provides user notifications with Notification hook execution.
//! Handles 4 critical notification types:
//! - PermissionPrompt: Tool requires permission
//! - IdlePrompt: TUI awaits user input
//! - AuthSuccess: API authentication succeeded
//! - ElicitationDialog: Claude asks clarifying questions

use crate::hooks::{HookContext, HookEvent, HooksSystem};
use crate::hooks::types::NotificationType;
use std::sync::Arc;

/// Sanitize message to prevent terminal injection attacks
fn sanitize_message(msg: &str) -> String {
    msg.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

/// Notification manager with hook integration
pub struct NotificationManager {
    hooks: Arc<HooksSystem>,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new(hooks: Arc<HooksSystem>) -> Self {
        Self { hooks }
    }

    /// Send a notification (fires hook BEFORE displaying to user)
    pub async fn notify(
        &self,
        session_id: &str,
        notification_type: NotificationType,
        message: &str,
    ) {
        // Create hook context
        let context = HookContext::for_notification(
            session_id.to_string(),
            format!(".claude/sessions/{}/transcript.json", session_id),
            std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_default(),
            "ask".to_string(),
            notification_type.clone(),
        );

        // Execute Notification hooks
        match self.hooks.execute_hooks(HookEvent::Notification, &context).await {
            Ok(results) => {
                for result in results {
                    if !result.is_success() {
                        tracing::warn!("Notification hook failed: {}", result.stderr);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to execute Notification hooks: {}", e);
                // Non-blocking - continue with notification even if hook fails
            }
        }

        // Display notification to user (stderr keeps stdout clean)
        eprintln!("🔔 {}", sanitize_message(message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_notification_manager_creation() {
        let hooks = Arc::new(HooksSystem::new());
        let manager = NotificationManager::new(hooks);

        // Basic smoke test - notification should not panic
        manager.notify(
            "test-session",
            NotificationType::AuthSuccess,
            "Test notification"
        ).await;
    }

    #[tokio::test]
    async fn test_notification_types() {
        let hooks = Arc::new(HooksSystem::new());
        let manager = NotificationManager::new(hooks);

        // Test all notification types
        manager.notify(
            "test-session",
            NotificationType::PermissionPrompt,
            "Permission required"
        ).await;

        manager.notify(
            "test-session",
            NotificationType::IdlePrompt,
            "Awaiting input"
        ).await;

        manager.notify(
            "test-session",
            NotificationType::AuthSuccess,
            "Authentication successful"
        ).await;

        manager.notify(
            "test-session",
            NotificationType::ElicitationDialog,
            "Claude is asking questions"
        ).await;
    }
}
