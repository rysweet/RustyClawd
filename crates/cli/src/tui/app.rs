//! Application state for TUI

use crate::permission_mode::PermissionMode;
use crate::tui::message::Message;
use crate::tui::token_counter::TokenCount;

/// Maximum debug messages to keep in buffer
const MAX_DEBUG_MESSAGES: usize = 1000;

/// Main application state - single source of truth
pub struct App {
    /// Message history (all messages in conversation)
    messages: Vec<Message>,

    /// Current input buffer
    input: String,

    /// Cursor position in input buffer (byte offset)
    cursor_pos: usize,

    /// Scroll offset for message viewport (lines from top)
    scroll_offset: usize,

    /// Auto-follow bottom (true = stick to bottom, false = manual scroll position)
    follow_bottom: bool,

    /// Current permission mode
    permission_mode: PermissionMode,

    /// Active streaming state (if any)
    streaming: Option<StreamingState>,

    /// Active tool execution (if any)
    active_tool: Option<String>,

    /// Whether to exit the application
    should_exit: bool,

    /// Last error message (displayed in status bar)
    error: Option<String>,

    /// Dirty flag - tracks if UI needs re-rendering
    dirty: bool,

    /// Debug panel visibility
    debug_visible: bool,

    /// Debug message buffer (circular buffer)
    debug_messages: Vec<String>,

    /// Dropdown menu open state
    menu_open: bool,
}

/// State for active streaming response
struct StreamingState {
    /// Message index being streamed to
    message_index: usize,

    /// Accumulated content so far
    accumulated: String,

    /// Token count (live updates during streaming)
    token_count: TokenCount,

    /// Thinking indicator (true when waiting for first token)
    thinking: bool,
}

impl App {
    pub fn new(permission_mode: PermissionMode) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            follow_bottom: true, // Start in auto-follow mode
            permission_mode,
            streaming: None,
            active_tool: None,
            should_exit: false,
            error: None,
            dirty: true, // Start dirty to trigger initial render
            debug_visible: false,
            debug_messages: Vec::new(),
            menu_open: false,
        }
    }

    // === State queries (immutable) ===

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn follow_bottom(&self) -> bool {
        self.follow_bottom
    }

    pub fn permission_mode(&self) -> PermissionMode {
        self.permission_mode
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn is_streaming(&self) -> bool {
        self.streaming.is_some()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn debug_visible(&self) -> bool {
        self.debug_visible
    }

    pub fn debug_messages(&self) -> &[String] {
        &self.debug_messages
    }

    pub fn menu_open(&self) -> bool {
        self.menu_open
    }

    // === State mutations ===

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn add_message(&mut self, message: Message) {
        self.push_debug_message(format!(
            "[MSG] {:?}: {} chars",
            message.role, message.content.len()
        ));
        self.messages.push(message);
        self.scroll_to_bottom();
        self.mark_dirty();
    }

    pub fn start_streaming_response(&mut self) -> usize {
        let index = self.messages.len();
        self.push_debug_message("[STREAM] Started".to_string());
        self.messages
            .push(Message::assistant_partial(String::new()));
        self.streaming = Some(StreamingState {
            message_index: index,
            accumulated: String::new(),
            token_count: TokenCount::default(),
            thinking: true,  // Start in thinking mode
        });
        self.scroll_to_bottom();
        self.mark_dirty();
        index
    }

    pub fn append_streaming_content(&mut self, content: &str) {
        // Log only significant chunks (> 10 chars) to reduce spam
        if let Some(ref state) = self.streaming {
            if content.len() > 10 {
                let new_len = state.accumulated.len() + content.len();
                self.push_debug_message(format!(
                    "[STREAM] +{} chars (total: {})",
                    content.len(), new_len
                ));
            }
        }

        // Now do the actual streaming update
        if let Some(ref mut state) = self.streaming {
            state.accumulated.push_str(content);

            if let Some(msg) = self.messages.get_mut(state.message_index) {
                *msg = Message::assistant_partial(state.accumulated.clone());
            }
            // Only auto-scroll if user is already at bottom
            self.scroll_to_bottom_if_at_bottom();
            self.mark_dirty();
        }
    }

    pub fn finish_streaming(&mut self) {
        if let Some(state) = self.streaming.take() {
            let final_len = state.accumulated.len();
            if let Some(msg) = self.messages.get_mut(state.message_index) {
                *msg = Message::assistant(state.accumulated);
            }
            self.push_debug_message(format!("[STREAM] Finished ({} total chars)", final_len));
            self.mark_dirty();
        }
    }

    // === Tool execution state ===

    pub fn set_active_tool(&mut self, tool_name: String) {
        self.push_debug_message(format!("[TOOL] Started: {}", tool_name));
        self.active_tool = Some(tool_name);
        self.mark_dirty();
    }

    pub fn clear_active_tool(&mut self) {
        if let Some(ref tool_name) = self.active_tool {
            self.push_debug_message(format!("[TOOL] Finished: {}", tool_name));
        }
        self.active_tool = None;
        self.mark_dirty();
    }

    pub fn active_tool(&self) -> Option<&str> {
        self.active_tool.as_deref()
    }

    pub fn has_active_tool(&self) -> bool {
        self.active_tool.is_some()
    }

    // === Input buffer management ===

    pub fn insert_char(&mut self, c: char) {
        // Unicode-aware insertion at cursor position
        let byte_pos = self.cursor_pos;
        if byte_pos <= self.input.len() {
            self.input.insert(byte_pos, c);
            self.cursor_pos += c.len_utf8();
            self.mark_dirty();
        }
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.input.remove(self.cursor_pos);
            self.mark_dirty();
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            // Find previous char boundary
            let mut pos = self.cursor_pos - 1;
            while pos > 0 && !self.input.is_char_boundary(pos) {
                pos -= 1;
            }
            self.input.remove(pos);
            self.cursor_pos = pos;
            self.mark_dirty();
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            let mut pos = self.cursor_pos - 1;
            while pos > 0 && !self.input.is_char_boundary(pos) {
                pos -= 1;
            }
            self.cursor_pos = pos;
            self.mark_dirty();
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            let mut pos = self.cursor_pos + 1;
            while pos < self.input.len() && !self.input.is_char_boundary(pos) {
                pos += 1;
            }
            self.cursor_pos = pos;
            self.mark_dirty();
        }
    }

    pub fn move_cursor_to_start(&mut self) {
        self.cursor_pos = 0;
        self.mark_dirty();
    }

    pub fn move_cursor_to_end(&mut self) {
        self.cursor_pos = self.input.len();
        self.mark_dirty();
    }

    pub fn submit_input(&mut self) -> Option<String> {
        if self.input.trim().is_empty() {
            return None;
        }
        let input = std::mem::take(&mut self.input);
        self.cursor_pos = 0;
        self.mark_dirty();
        Some(input)
    }

    pub fn scroll_up(&mut self, lines: usize) {
        // Disable auto-follow when user scrolls manually
        self.follow_bottom = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        self.mark_dirty();
    }

    pub fn scroll_down(&mut self, lines: usize) {
        // If already following bottom, stay there
        if self.follow_bottom {
            self.mark_dirty();
            return;
        }

        // When scrolling down, we might reach bottom again
        self.scroll_offset = self.scroll_offset.saturating_add(lines);

        // If we've scrolled to a very large offset, assume we're trying to reach bottom
        // This handles the case where user presses "End" or scrolls down repeatedly
        if self.scroll_offset > 100000 {
            self.follow_bottom = true;
        }

        self.mark_dirty();
    }

    pub fn scroll_to_bottom(&mut self) {
        self.follow_bottom = true;
        self.scroll_offset = 0; // Will be set to max_scroll during render
        self.mark_dirty();
    }

    /// Scroll to bottom only if already following bottom (preserves manual scroll position)
    fn scroll_to_bottom_if_at_bottom(&mut self) {
        if self.follow_bottom {
            // Already following bottom, just mark dirty to update
            self.mark_dirty();
        }
        // Otherwise, user has scrolled up - don't force them back
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.mark_dirty();
    }

    pub fn clear_error(&mut self) {
        self.error = None;
        self.mark_dirty();
    }

    pub fn exit(&mut self) {
        self.should_exit = true;
        self.mark_dirty();
    }

    pub fn cycle_permission_mode(&mut self) -> PermissionMode {
        self.permission_mode = self.permission_mode.cycle();
        self.mark_dirty();
        self.permission_mode
    }

    pub fn toggle_debug(&mut self) {
        self.debug_visible = !self.debug_visible;
        let status = if self.debug_visible { "ON" } else { "OFF" };
        self.push_debug_message(format!("=== Debug Panel {} ===", status));
        self.mark_dirty();
    }

    pub fn toggle_menu(&mut self) {
        self.menu_open = !self.menu_open;
        self.mark_dirty();
    }

    pub fn push_debug_message(&mut self, message: String) {
        // Circular buffer - remove oldest if at capacity
        if self.debug_messages.len() >= MAX_DEBUG_MESSAGES {
            self.debug_messages.remove(0);
        }
        self.debug_messages.push(message);
        self.mark_dirty();
    }

    pub fn clear_debug_messages(&mut self) {
        self.debug_messages.clear();
        self.mark_dirty();
    }

    /// Get render debug info (for logging inside render functions)
    pub fn get_render_debug(&self) -> String {
        format!(
            "messages={}, first_content={:?}, scroll={}, streaming={}",
            self.messages.len(),
            self.messages.first().map(|m| &m.content[..m.content.len().min(50)]),
            self.scroll_offset,
            self.streaming.is_some()
        )
    }

    /// Update token count during streaming
    pub fn update_token_count(&mut self, input: u32, output: u32) {
        if let Some(ref mut state) = self.streaming {
            state.token_count.add(input, output);
            // First token received - no longer thinking
            if output > 0 {
                state.thinking = false;
            }
            self.mark_dirty();
        }
    }

    /// Get current token count (if streaming)
    pub fn token_count(&self) -> Option<TokenCount> {
        self.streaming.as_ref().map(|s| s.token_count)
    }

    /// Check if currently in thinking mode (waiting for first token)
    pub fn is_thinking(&self) -> bool {
        self.streaming.as_ref().map(|s| s.thinking).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_movement() {
        let mut app = App::new(PermissionMode::default());
        app.insert_char('a');
        app.insert_char('b');
        app.insert_char('c');

        assert_eq!(app.cursor_pos(), 3);

        app.move_cursor_left();
        assert_eq!(app.cursor_pos(), 2);

        app.delete_char();
        assert_eq!(app.input(), "ab");
    }

    #[test]
    fn test_unicode_input() {
        let mut app = App::new(PermissionMode::default());
        app.insert_char('🦀');
        app.insert_char('🚀');

        assert_eq!(app.input().chars().count(), 2);
        app.move_cursor_left();
        app.insert_char('❤');
        assert_eq!(app.input(), "🦀❤🚀");
    }

    #[test]
    fn test_streaming_lifecycle() {
        let mut app = App::new(PermissionMode::default());

        let idx = app.start_streaming_response();
        assert_eq!(idx, 0);
        assert!(app.is_streaming());

        app.append_streaming_content("Hello");
        app.append_streaming_content(" world");

        app.finish_streaming();
        assert!(!app.is_streaming());
        assert_eq!(app.messages()[0].content, "Hello world");
        assert!(!app.messages()[0].streaming);
    }

    #[test]
    fn test_input_submission() {
        let mut app = App::new(PermissionMode::default());
        app.insert_char('h');
        app.insert_char('i');

        let input = app.submit_input();
        assert_eq!(input, Some("hi".to_string()));
        assert_eq!(app.input(), "");
        assert_eq!(app.cursor_pos(), 0);
    }

    #[test]
    fn test_empty_input_not_submitted() {
        let mut app = App::new(PermissionMode::default());
        app.insert_char(' ');

        let input = app.submit_input();
        assert_eq!(input, None); // Whitespace-only not submitted
    }
}
