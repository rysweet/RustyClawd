//! Tool execution state methods for App.
//!
//! Manages tool lifecycle: begin, query, finalize.

use super::app::App;
use crate::tui::message::Message;
use crate::tui::tool_messages::{ToolMessageState, ToolResult};
use std::time::Instant;

impl App {
    // === Tool execution state ===

    /// Begin a new tool execution message.
    /// Orchestrates: creates tool state in ToolTracker, THEN pushes a placeholder message.
    pub fn begin_tool_message(
        &mut self,
        tool_id: String,
        tool_name: String,
        params: serde_json::Value,
    ) -> usize {
        self.push_debug_message(format!("[TOOL] Started: {} (id: {})", tool_name, tool_id));

        // Create a placeholder message (collapsible tool message)
        let preview = format!("🔧 {} ...", tool_name);
        let message = Message::collapsible(
            crate::tui::message::Role::System,
            String::new(), // Will be filled by renderer
            preview,
        );

        let message_index = self.messages.len();
        self.messages.push(message);

        // Track tool state
        let state = ToolMessageState {
            message_index,
            tool_name,
            params,
            start_time: Instant::now(),
            completed: false,
            result: None,
            elapsed_duration: None,
        };

        self.tools.insert(tool_id, state);
        self.mark_dirty();

        message_index
    }

    /// Get tool state by tool_id (read-only)
    pub fn get_tool_message_state(&self, tool_id: &str) -> Option<&ToolMessageState> {
        self.tools.get(tool_id)
    }

    /// Get all active (non-completed) tool messages
    pub fn active_tool_messages(&self) -> impl Iterator<Item = (&String, &ToolMessageState)> {
        self.tools.active_tools()
    }

    /// Finalize a tool execution message with result.
    /// Orchestrates: finalizes in ToolTracker, THEN updates message status.
    pub fn finalize_tool_message(&mut self, tool_id: &str, result: ToolResult) {
        // Finalize in tracker (captures timing, stores result)
        let debug_info = self.tools.finalize(tool_id, result.clone());

        // Update message status based on result
        if let Some((_, _, message_index)) = &debug_info {
            if let Some(message) = self.messages.get_mut(*message_index) {
                if result.is_error {
                    message.mark_error();
                } else {
                    message.complete_streaming();
                }
            }
            self.mark_dirty();
        }

        // Log debug message
        if let Some((tool_name, elapsed, _)) = debug_info {
            self.push_debug_message(format!(
                "[TOOL] Finished: {} ({}s, exit_code: {:?})",
                tool_name, elapsed, result.exit_code
            ));
        }
    }

    /// Check if any tools are currently executing
    pub fn has_active_tools(&self) -> bool {
        self.tools.has_active()
    }

    /// Get name of any active tool (for status bar)
    pub fn active_tool_name(&self) -> Option<String> {
        self.tools.active_name()
    }

    /// Find tool state by message index (for rendering)
    pub fn tool_message_by_index(
        &self,
        message_index: usize,
    ) -> Option<(&String, &ToolMessageState)> {
        self.tools.by_message_index(message_index)
    }
}
