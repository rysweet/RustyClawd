//! Application state for TUI

use crate::permission_mode::PermissionMode;
use crate::tui::message::Message;
use crate::tui::token_counter::TokenCount;
use std::collections::HashMap;
use std::time::Instant;

/// Maximum debug messages to keep in buffer
const MAX_DEBUG_MESSAGES: usize = 1000;

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
}

/// Result of a tool execution
#[derive(Clone)]
pub struct ToolResult {
    /// Exit code (for bash tools) or success indicator
    pub exit_code: Option<i32>,
    /// Stdout output
    pub stdout: String,
    /// Stderr output
    pub stderr: String,
    /// Whether this was an error
    pub is_error: bool,
}

/// Completion item for slash command autocomplete
#[derive(Clone, Debug)]
pub struct CompletionItem {
    /// Command name (without leading /)
    pub command: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional argument hint
    pub argument_hint: Option<String>,
}

/// Autocomplete state for slash commands
#[derive(Clone, Debug)]
pub struct AutocompleteState {
    /// All available completions
    pub items: Vec<CompletionItem>,
    /// Currently selected index
    pub selected: usize,
}

/// Memory destination for saving user memories
#[derive(Clone, Debug)]
pub struct MemoryDestination {
    /// Display name (e.g., "User memory", "Project memory")
    pub name: String,
    /// File path where memory will be saved
    pub file_path: String,
    /// Optional description/hint (e.g., "Saved in ~/.claude/CLAUDE.md")
    pub description: Option<String>,
    /// Whether this is an imported context file
    pub is_imported: bool,
}

/// Memory modal state
#[derive(Clone, Debug)]
pub struct MemoryModalState {
    /// Memory text to be saved
    pub memory_text: String,
    /// Available destinations
    pub destinations: Vec<MemoryDestination>,
    /// Currently selected index
    pub selected: usize,
}

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

    /// Maximum valid scroll offset (updated by renderer each frame)
    /// Allows scroll operations to clamp properly without magic numbers
    max_scroll: usize,

    /// Autocomplete state
    autocomplete: Option<AutocompleteState>,

    /// Memory modal state
    memory_modal: Option<MemoryModalState>,

    /// Current permission mode
    permission_mode: PermissionMode,

    /// Active streaming state (if any)
    streaming: Option<StreamingState>,

    /// Active tool executions (tool_id -> state)
    tool_messages: HashMap<String, ToolMessageState>,

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
            max_scroll: 0, // Will be updated by renderer
            autocomplete: None,
            memory_modal: None,
            permission_mode,
            streaming: None,
            tool_messages: HashMap::new(),
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
            // Maintain scroll position if at bottom
            self.scroll_to_bottom_if_at_bottom();
            self.mark_dirty();
        }
    }

    // === Tool execution state ===

    /// Begin a new tool execution message (creates placeholder that will be updated dynamically)
    pub fn begin_tool_message(&mut self, tool_id: String, tool_name: String, params: serde_json::Value) -> usize {
        self.push_debug_message(format!("[TOOL] Started: {} (id: {})", tool_name, tool_id));

        // Create a placeholder message
        let message = Message {
            role: crate::tui::message::Role::System,
            content: String::new(), // Will be filled by renderer
            timestamp: chrono::Local::now(),
            streaming: false,
        };

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
        };

        self.tool_messages.insert(tool_id, state);
        self.mark_dirty();

        message_index
    }

    /// Update tool message content (called by renderer to show dynamic content with timer/throbber)
    pub fn get_tool_message_state(&self, tool_id: &str) -> Option<&ToolMessageState> {
        self.tool_messages.get(tool_id)
    }

    /// Get all active (non-completed) tool messages
    pub fn active_tool_messages(&self) -> impl Iterator<Item = (&String, &ToolMessageState)> {
        self.tool_messages.iter().filter(|(_, state)| !state.completed)
    }

    /// Finalize a tool execution message with result
    pub fn finalize_tool_message(&mut self, tool_id: &str, result: ToolResult) {
        // Get tool info for debug message before mutation
        let debug_info = self.tool_messages.get(tool_id).map(|state| {
            (state.tool_name.clone(), state.start_time.elapsed().as_secs())
        });

        // Update tool state
        if let Some(state) = self.tool_messages.get_mut(tool_id) {
            state.completed = true;
            state.result = Some(result.clone());
            self.mark_dirty();
        }

        // Log debug message after mutation
        if let Some((tool_name, elapsed)) = debug_info {
            self.push_debug_message(format!(
                "[TOOL] Finished: {} ({}s, exit_code: {:?})",
                tool_name,
                elapsed,
                result.exit_code
            ));
        }
    }

    /// Check if any tools are currently executing
    pub fn has_active_tools(&self) -> bool {
        self.tool_messages.iter().any(|(_, state)| !state.completed)
    }

    /// Get name of any active tool (for status bar)
    pub fn active_tool_name(&self) -> Option<String> {
        self.tool_messages
            .iter()
            .find(|(_, state)| !state.completed)
            .map(|(_, state)| state.tool_name.clone())
    }

    /// Find tool state by message index (for rendering)
    pub fn tool_message_by_index(&self, message_index: usize) -> Option<(&String, &ToolMessageState)> {
        self.tool_messages
            .iter()
            .find(|(_, state)| state.message_index == message_index)
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

    pub fn set_input(&mut self, text: &str) {
        self.input = text.to_string();
        self.mark_dirty();
    }

    pub fn scroll_up(&mut self, lines: usize) {
        // If we're in follow mode, transition to manual scroll mode
        if self.follow_bottom {
            self.follow_bottom = false;
            // Initialize scroll_offset to max_scroll so we're actually at the bottom
            self.scroll_offset = self.max_scroll;
        }

        // Now scroll up from current position
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        self.mark_dirty();
    }

    pub fn scroll_down(&mut self, lines: usize) {
        // If already following bottom, stay there
        if self.follow_bottom {
            self.mark_dirty();
            return;
        }

        // Increment scroll offset and clamp to valid range
        self.scroll_offset = self.scroll_offset.saturating_add(lines);

        // Clamp to max_scroll to prevent phantom accumulation
        if self.scroll_offset >= self.max_scroll {
            // At or past the bottom - switch to follow mode
            self.follow_bottom = true;
            self.scroll_offset = 0; // Renderer will set to max_scroll
        }

        self.mark_dirty();
    }

    pub fn scroll_to_bottom(&mut self) {
        self.follow_bottom = true;
        self.scroll_offset = 0; // Will be set to max_scroll during render
        self.mark_dirty();
    }

    /// Update max_scroll from renderer (called after content height calculation)
    /// This allows scroll operations to clamp properly
    pub fn update_max_scroll(&mut self, max_scroll: usize) {
        self.max_scroll = max_scroll;
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

    // === Autocomplete management ===

    /// Activate autocomplete with given completions
    pub fn activate_autocomplete(&mut self, items: Vec<CompletionItem>) {
        if items.is_empty() {
            self.autocomplete = None;
        } else {
            self.autocomplete = Some(AutocompleteState {
                items,
                selected: 0,
            });
        }
        self.mark_dirty();
    }

    /// Clear autocomplete
    pub fn clear_autocomplete(&mut self) {
        self.autocomplete = None;
        self.mark_dirty();
    }

    /// Navigate autocomplete selection up
    pub fn autocomplete_prev(&mut self) {
        if let Some(ref mut ac) = self.autocomplete {
            if ac.selected > 0 {
                ac.selected -= 1;
            } else {
                // Wrap to bottom
                ac.selected = ac.items.len().saturating_sub(1);
            }
            self.mark_dirty();
        }
    }

    /// Navigate autocomplete selection down
    pub fn autocomplete_next(&mut self) {
        if let Some(ref mut ac) = self.autocomplete {
            if ac.selected < ac.items.len().saturating_sub(1) {
                ac.selected += 1;
            } else {
                // Wrap to top
                ac.selected = 0;
            }
            self.mark_dirty();
        }
    }

    /// Get selected autocomplete item
    pub fn autocomplete_selected(&self) -> Option<&CompletionItem> {
        self.autocomplete.as_ref().and_then(|ac| ac.items.get(ac.selected))
    }

    /// Check if autocomplete is active
    pub fn autocomplete_active(&self) -> bool {
        self.autocomplete.is_some()
    }

    /// Get autocomplete state (for rendering)
    pub fn autocomplete(&self) -> Option<&AutocompleteState> {
        self.autocomplete.as_ref()
    }

    // === Memory modal management ===

    /// Activate memory modal with destinations
    pub fn activate_memory_modal(&mut self, memory_text: String, destinations: Vec<MemoryDestination>) {
        if destinations.is_empty() {
            self.memory_modal = None;
        } else {
            self.memory_modal = Some(MemoryModalState {
                memory_text,
                destinations,
                selected: 0,
            });
        }
        self.mark_dirty();
    }

    /// Update memory text without resetting selection
    pub fn update_memory_text(&mut self, memory_text: String) {
        if let Some(ref mut modal) = self.memory_modal {
            modal.memory_text = memory_text;
            self.mark_dirty();
        }
    }

    /// Clear memory modal
    pub fn clear_memory_modal(&mut self) {
        self.memory_modal = None;
        self.mark_dirty();
    }

    /// Navigate memory modal selection up
    pub fn memory_modal_prev(&mut self) {
        if let Some(ref mut modal) = self.memory_modal {
            if modal.selected > 0 {
                modal.selected -= 1;
            } else {
                // Wrap to bottom
                modal.selected = modal.destinations.len().saturating_sub(1);
            }
            self.mark_dirty();
        }
    }

    /// Navigate memory modal selection down
    pub fn memory_modal_next(&mut self) {
        if let Some(ref mut modal) = self.memory_modal {
            if modal.selected < modal.destinations.len().saturating_sub(1) {
                modal.selected += 1;
            } else {
                // Wrap to top
                modal.selected = 0;
            }
            self.mark_dirty();
        }
    }

    /// Get selected memory destination
    pub fn memory_modal_selected(&self) -> Option<&MemoryDestination> {
        self.memory_modal.as_ref().and_then(|modal| modal.destinations.get(modal.selected))
    }

    /// Check if memory modal is active
    pub fn memory_modal_active(&self) -> bool {
        self.memory_modal.is_some()
    }

    /// Get memory modal state (for rendering)
    pub fn memory_modal(&self) -> Option<&MemoryModalState> {
        self.memory_modal.as_ref()
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
