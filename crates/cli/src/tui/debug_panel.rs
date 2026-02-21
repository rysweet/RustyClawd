//! Debug panel state management
//!
//! Self-contained module for the debug message panel.
//! Manages debug visibility, message buffer, and scroll state.

use std::collections::VecDeque;

use super::app::ScrollController;

/// Maximum debug messages to keep in buffer
const MAX_DEBUG_MESSAGES: usize = 1000;

/// Debug panel state - manages debug visibility, messages, and scrolling
pub struct DebugPanel {
    /// Debug panel visibility
    visible: bool,

    /// Debug message buffer (circular buffer, VecDeque for O(1) pop_front)
    messages: VecDeque<String>,

    /// Scroll controller for debug panel
    scroll: ScrollController,
}

impl DebugPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            messages: VecDeque::new(),
            scroll: ScrollController::new(),
        }
    }

    // === Visibility ===

    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Toggle visibility. Returns whether a dirty flag should be set.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        let status = if self.visible { "ON" } else { "OFF" };
        self.push_message(format!("=== Debug Panel {} ===", status));
    }

    // === Messages ===

    pub fn messages(&self) -> &VecDeque<String> {
        &self.messages
    }

    pub fn push_message(&mut self, message: String) {
        // Circular buffer - remove oldest if at capacity (O(1) with VecDeque)
        if self.messages.len() >= MAX_DEBUG_MESSAGES {
            self.messages.pop_front();
        }
        self.messages.push_back(message);
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    // === Scrolling ===

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll.scroll_up(lines);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll.scroll_down(lines);
    }

    pub fn update_max_scroll(&mut self, max_scroll: usize) {
        self.scroll.update_max_scroll(max_scroll);
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll.offset()
    }

    pub fn follow_bottom(&self) -> bool {
        self.scroll.follow_bottom()
    }
}
