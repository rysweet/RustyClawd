//! Tool execution state management for TUI
//!
//! Tracks the lifecycle of tool calls: creation, active execution, and finalization.
//! Owns only tool STATE (name, params, timing, result). Message list integration
//! remains in App, which orchestrates ToolTracker + messages together.

use std::collections::HashMap;
use std::time::Instant;

/// State for an active tool execution message
#[derive(Clone)]
pub struct ToolMessageState {
    /// Index of the message in the messages list
    pub message_index: usize,
    /// Tool name
    pub tool_name: String,
    /// Tool parameters (for display)
    pub params: serde_json::Value,
    /// When the tool started executing
    pub start_time: Instant,
    /// Whether the tool has completed
    pub completed: bool,
    /// Result (if completed)
    pub result: Option<ToolResult>,
    /// Elapsed duration in seconds (captured when completed)
    pub elapsed_duration: Option<u64>,
}

/// Result of a tool execution
#[derive(Clone)]
pub struct ToolResult {
    /// Exit code (for bash tools) or success indicator
    pub exit_code: Option<i32>,
    /// Primary output (stdout for Bash, content for other tools)
    pub stdout: String,
    /// Error output (stderr for Bash, empty for other tools)
    pub stderr: String,
    /// Whether this was an error
    pub is_error: bool,
    /// Raw content (for expanded view - shows full API response)
    pub raw_content: String,
    /// Optional structured JSON content (MCP spec structuredContent field)
    /// Contains typed data conforming to the tool's outputSchema when available
    pub structured_content: Option<serde_json::Value>,
}

/// Manages tool execution state, keyed by tool_id.
/// Pure state container -- does NOT touch the message list.
pub struct ToolTracker {
    tools: HashMap<String, ToolMessageState>,
}

impl ToolTracker {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Insert a new tool state entry. Returns the inserted state for caller use.
    pub fn insert(&mut self, tool_id: String, state: ToolMessageState) {
        self.tools.insert(tool_id, state);
    }

    /// Get tool state by tool_id (read-only).
    pub fn get(&self, tool_id: &str) -> Option<&ToolMessageState> {
        self.tools.get(tool_id)
    }

    /// Iterate over all active (non-completed) tools.
    pub fn active_tools(&self) -> impl Iterator<Item = (&String, &ToolMessageState)> {
        self.tools.iter().filter(|(_, state)| !state.completed)
    }

    /// Check if any tools are currently executing.
    pub fn has_active(&self) -> bool {
        self.tools.iter().any(|(_, state)| !state.completed)
    }

    /// Get name of any active tool (for status bar).
    pub fn active_name(&self) -> Option<String> {
        self.tools
            .iter()
            .find(|(_, state)| !state.completed)
            .map(|(_, state)| state.tool_name.clone())
    }

    /// Find tool state by message index (for rendering).
    pub fn by_message_index(&self, message_index: usize) -> Option<(&String, &ToolMessageState)> {
        self.tools
            .iter()
            .find(|(_, state)| state.message_index == message_index)
    }

    /// Mark a tool as completed with the given result.
    /// Returns (tool_name, elapsed_secs, message_index) for debug logging, or None if tool_id not found.
    pub fn finalize(&mut self, tool_id: &str, result: ToolResult) -> Option<(String, u64, usize)> {
        if let Some(state) = self.tools.get_mut(tool_id) {
            let elapsed = state.start_time.elapsed().as_secs();
            let info = (state.tool_name.clone(), elapsed, state.message_index);
            state.completed = true;
            state.elapsed_duration = Some(elapsed);
            state.result = Some(result);
            Some(info)
        } else {
            None
        }
    }
}
