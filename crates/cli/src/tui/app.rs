//! Application state for TUI

use crate::commands::permissions_search_state::PermissionsSearchState;
use crate::permission_mode::PermissionMode;
use crate::tui::autocomplete_state::AutocompleteManager;
pub use crate::tui::autocomplete_state::{AutocompleteState, CompletionItem};
use crate::tui::debug_panel::DebugPanel;
use crate::tui::input_state::InputState;
use crate::tui::message::Message;
use crate::tui::modal_state::{MemoryDestination, MemoryModalState, ModalManager};
use crate::tui::streaming_state::StreamingState;
use crate::tui::token_counter::TokenCount;
use crate::tui::tool_messages::ToolTracker;
pub use crate::tui::tool_messages::{ToolMessageState, ToolResult};
use rat_focus::{Focus, FocusFlag};
use ratatui::layout::Rect;
use std::time::Instant;

/// Layout cache - stores pane areas from last render for hit testing
#[derive(Clone, Debug, Default)]
pub struct LayoutCache {
    /// Messages pane area
    pub messages_area: Rect,
    /// Input pane area
    pub input_area: Rect,
    /// Debug panel area (when visible)
    pub debug_area: Option<Rect>,
}

/// Reusable scroll controller for any scrollable panel.
/// Manages offset, follow-bottom mode, and max-scroll clamping.
pub struct ScrollController {
    /// Current scroll offset (lines from top)
    offset: usize,
    /// Auto-follow bottom (true = stick to bottom, false = manual scroll)
    follow_bottom: bool,
    /// Maximum valid scroll offset (updated by renderer each frame)
    max_scroll: usize,
}

impl ScrollController {
    pub fn new() -> Self {
        Self {
            offset: 0,
            follow_bottom: true, // Start in auto-follow mode
            max_scroll: 0,       // Will be updated by renderer
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn follow_bottom(&self) -> bool {
        self.follow_bottom
    }

    pub fn scroll_up(&mut self, lines: usize) {
        // If we're in follow mode, transition to manual scroll mode
        if self.follow_bottom {
            self.follow_bottom = false;
            // Initialize offset to max_scroll so we're actually at the bottom
            self.offset = self.max_scroll;
        }
        // Now scroll up from current position
        self.offset = self.offset.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        // If already following bottom, stay there
        if self.follow_bottom {
            return;
        }
        // Increment scroll offset and clamp to valid range
        self.offset = self.offset.saturating_add(lines);
        // Clamp to max_scroll to prevent phantom accumulation
        if self.offset >= self.max_scroll {
            // At or past the bottom - switch to follow mode
            self.follow_bottom = true;
            self.offset = 0; // Renderer will set to max_scroll
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.follow_bottom = true;
        self.offset = 0; // Will be set to max_scroll during render
    }

    /// Update max_scroll from renderer (called after content height calculation).
    /// Also clamps offset to prevent invalid state after terminal resize.
    pub fn update_max_scroll(&mut self, max_scroll: usize) {
        self.max_scroll = max_scroll;
        // Clamp offset when max_scroll changes (e.g., after terminal resize)
        // Only clamp if NOT in follow_bottom mode (which uses max_scroll directly in render)
        if !self.follow_bottom && self.offset > max_scroll {
            self.offset = max_scroll;
        }
    }
}

/// Main application state - single source of truth
pub struct App {
    /// Message history (all messages in conversation)
    messages: Vec<Message>,

    /// Input state (TextArea + soft wrap)
    pub input_state: InputState,

    /// Scroll controller for message viewport
    message_scroll: ScrollController,

    /// Autocomplete manager
    autocomplete: AutocompleteManager,

    /// Modal manager (memory modal + permissions modal)
    modals: ModalManager,

    /// Current permission mode
    permission_mode: PermissionMode,

    /// Active streaming state (if any)
    streaming: Option<StreamingState>,

    /// Active tool executions (tool_id -> state)
    tools: ToolTracker,

    /// Whether to exit the application
    should_exit: bool,

    /// Last error message (displayed in status bar)
    error: Option<String>,

    /// Dirty flag - tracks if UI needs re-rendering
    dirty: bool,

    /// Debug panel state (visibility, messages, scrolling)
    debug: DebugPanel,

    /// Dropdown menu open state
    menu_open: bool,

    /// Focus state for messages pane
    focus_messages: FocusFlag,

    /// Focus state for input pane
    focus_input: FocusFlag,

    /// Focus state for debug panel
    focus_debug: FocusFlag,

    /// Focus state for autocomplete popup
    focus_autocomplete: FocusFlag,

    /// Focus state for memory modal
    focus_memory_modal: FocusFlag,

    /// Focus state for permissions modal
    focus_permissions_modal: FocusFlag,

    /// Layout cache from last render (for hit testing)
    layout_cache: LayoutCache,

    /// Clickable regions for mouse interactions
    pub click_regions: crate::tui::click_region::ClickableRegions,

    /// Cached focus structure (rebuilt only when focus_dirty is set)
    cached_focus: Option<Focus>,

    /// Flag indicating focus structure needs rebuilding (debug toggle, modal open/close)
    focus_dirty: bool,
}

impl App {
    pub fn new(permission_mode: PermissionMode) -> Self {
        Self {
            messages: Vec::new(),
            input_state: InputState::new(),
            message_scroll: ScrollController::new(),
            autocomplete: AutocompleteManager::new(),
            modals: ModalManager::new(),
            permission_mode,
            streaming: None,
            tools: ToolTracker::new(),
            should_exit: false,
            error: None,
            dirty: true, // Start dirty to trigger initial render
            debug: DebugPanel::new(),
            menu_open: false,
            focus_messages: FocusFlag::new(),
            focus_input: FocusFlag::new(),
            focus_debug: FocusFlag::new(),
            focus_autocomplete: FocusFlag::new(),
            focus_memory_modal: FocusFlag::new(),
            focus_permissions_modal: FocusFlag::new(),
            layout_cache: LayoutCache::default(),
            click_regions: crate::tui::click_region::ClickableRegions::new(),
            cached_focus: None,
            focus_dirty: true, // Start dirty to build on first event
        }
    }

    // === State queries (immutable) ===

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn messages_mut(&mut self) -> &mut [Message] {
        &mut self.messages
    }

    pub fn input(&self) -> String {
        self.input_state.input_text()
    }

    pub fn cursor_pos(&self) -> (usize, usize) {
        self.input_state.cursor_pos()
    }

    pub fn input_line_count(&self) -> usize {
        self.input_state.input_line_count()
    }

    pub fn has_multi_line_input(&self) -> bool {
        self.input_state.has_multi_line_input()
    }

    /// Update TextArea block style based on focus state
    /// Must be called before rendering to show focus-aware border colors
    pub fn update_input_focus_style(&mut self) {
        let is_focused = self.focus_input.get();
        self.input_state.update_input_focus_style(is_focused);
    }

    /// Update soft wrap inner width and reflow if width changed
    pub fn update_soft_wrap_width(&mut self, width: u16) {
        self.input_state.update_soft_wrap_width(width);
    }

    pub fn scroll_offset(&self) -> usize {
        self.message_scroll.offset()
    }

    pub fn follow_bottom(&self) -> bool {
        self.message_scroll.follow_bottom()
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
        self.debug.visible()
    }

    pub fn debug_messages(&self) -> &std::collections::VecDeque<String> {
        self.debug.messages()
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
            message.role,
            message.content.len()
        ));

        // DEBUG: Log first 300 chars of content with escaped newlines
        let debug_sample = message.content.chars().take(300).collect::<String>();
        let escaped = debug_sample
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        self.push_debug_message(format!("[MSG CONTENT] {}", escaped));

        // DEBUG: Show how .lines() will parse this
        let paragraphs: Vec<&str> = message.content.lines().collect();
        self.push_debug_message(format!(
            "[LINES] .lines() produced {} paragraphs",
            paragraphs.len()
        ));
        for (idx, para) in paragraphs.iter().enumerate() {
            if para.is_empty() {
                self.push_debug_message(format!("[LINES] Para {}: <EMPTY>", idx));
            } else {
                let preview = if para.len() > 30 {
                    format!("{}...", &para[..30])
                } else {
                    para.to_string()
                };
                self.push_debug_message(format!(
                    "[LINES] Para {}: {:?} (len={})",
                    idx,
                    preview,
                    para.len()
                ));
            }
        }

        self.messages.push(message);
        self.scroll_to_bottom();
        self.mark_dirty();
    }

    pub fn start_streaming_response(&mut self) -> usize {
        let index = self.messages.len();
        self.push_debug_message("[STREAM] Started".to_string());
        self.messages
            .push(Message::assistant_partial(String::new()));
        self.streaming = Some(StreamingState::new(index));
        self.scroll_to_bottom();
        self.mark_dirty();
        index
    }

    pub fn append_streaming_content(&mut self, content: &str) {
        // Log only significant chunks (> 10 chars) to reduce spam
        if let Some(ref state) = self.streaming {
            if content.len() > 10 {
                let new_len = state.accumulated().len() + content.len();
                self.push_debug_message(format!(
                    "[STREAM] +{} chars (total: {})",
                    content.len(),
                    new_len
                ));
            }
        }

        // Delegate content accumulation to StreamingState, then update messages[]
        let truncated = if let Some(ref mut state) = self.streaming {
            let truncated = state.append_content(content);

            if let Some(msg) = self.messages.get_mut(state.message_index()) {
                *msg = Message::assistant_partial(state.accumulated().to_string());
            }
            // Only auto-scroll if user is already at bottom
            self.scroll_to_bottom_if_at_bottom();
            self.mark_dirty();
            truncated
        } else {
            false
        };

        if truncated {
            self.push_debug_message(
                "[STREAM] Content size limit reached (10MB), truncating".to_string(),
            );
        }
    }

    pub fn finish_streaming(&mut self) {
        if let Some(state) = self.streaming.take() {
            let message_index = state.message_index();
            let accumulated = state.take_accumulated();
            let final_len = accumulated.len();
            if let Some(msg) = self.messages.get_mut(message_index) {
                *msg = Message::assistant(accumulated);
                // Message::assistant() already sets status to Complete
            }
            self.push_debug_message(format!("[STREAM] Finished ({} total chars)", final_len));
            // Maintain scroll position if at bottom
            self.scroll_to_bottom_if_at_bottom();
            self.mark_dirty();
        }
    }

    /// Mark the last streaming message as complete (without replacing content)
    pub fn complete_last_streaming(&mut self) {
        if let Some(state) = &self.streaming {
            if let Some(msg) = self.messages.get_mut(state.message_index()) {
                msg.complete_streaming();
                self.mark_dirty();
            }
        }
    }

    /// Mark the last message as error
    pub fn mark_last_message_error(&mut self) {
        if let Some(message) = self.messages.last_mut() {
            message.mark_error();
            self.mark_dirty();
        }
    }

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

    // === Input buffer management (delegates to InputState) ===

    pub fn insert_char(&mut self, c: char) {
        self.input_state.insert_char(c);
        self.mark_dirty();
    }

    pub fn delete_char(&mut self) {
        self.input_state.delete_char();
        self.mark_dirty();
    }

    pub fn backspace(&mut self) {
        let debug_msgs = self.input_state.backspace();
        for msg in debug_msgs {
            self.push_debug_message(msg);
        }
        self.mark_dirty();
    }

    pub fn move_cursor_left(&mut self) {
        self.input_state.move_cursor_left();
        self.mark_dirty();
    }

    pub fn move_cursor_right(&mut self) {
        self.input_state.move_cursor_right();
        self.mark_dirty();
    }

    pub fn move_cursor_to_start(&mut self) {
        self.input_state.move_cursor_to_start();
        self.mark_dirty();
    }

    pub fn move_cursor_to_end(&mut self) {
        self.input_state.move_cursor_to_end();
        self.mark_dirty();
    }

    pub fn insert_newline(&mut self) {
        self.input_state.insert_newline();
        self.mark_dirty();
    }

    pub fn move_cursor_up(&mut self) {
        self.input_state.move_cursor_up();
        self.mark_dirty();
    }

    pub fn move_cursor_down(&mut self) {
        self.input_state.move_cursor_down();
        self.mark_dirty();
    }

    pub fn move_cursor_word_left(&mut self) {
        self.input_state.move_cursor_word_left();
        self.mark_dirty();
    }

    pub fn move_cursor_word_right(&mut self) {
        self.input_state.move_cursor_word_right();
        self.mark_dirty();
    }

    pub fn move_cursor_absolute_start(&mut self) {
        self.input_state.move_cursor_absolute_start();
        self.mark_dirty();
    }

    pub fn move_cursor_absolute_end(&mut self) {
        self.input_state.move_cursor_absolute_end();
        self.mark_dirty();
    }

    pub fn move_cursor_to_input_top(&mut self) {
        self.input_state.move_cursor_to_input_top();
        self.mark_dirty();
    }

    pub fn move_cursor_to_input_bottom(&mut self) {
        self.input_state.move_cursor_to_input_bottom();
        self.mark_dirty();
    }

    pub fn scroll_input_viewport_up(&mut self) {
        self.input_state.scroll_input_viewport_up();
        self.mark_dirty();
    }

    pub fn scroll_input_viewport_down(&mut self) {
        self.input_state.scroll_input_viewport_down();
        self.mark_dirty();
    }

    /// Clear input without submitting (Ctrl+U behavior)
    pub fn clear_input(&mut self) {
        self.input_state.clear_input();
        self.mark_dirty();
    }

    pub fn submit_input(&mut self) -> Option<String> {
        let result = self.input_state.submit_input();
        self.mark_dirty();
        result
    }

    pub fn set_input(&mut self, text: &str) {
        self.input_state.set_input(text);
        self.mark_dirty();
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.message_scroll.scroll_up(lines);
        self.mark_dirty();
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.message_scroll.scroll_down(lines);
        self.mark_dirty();
    }

    pub fn scroll_to_bottom(&mut self) {
        self.message_scroll.scroll_to_bottom();
        self.mark_dirty();
    }

    /// Update max_scroll from renderer (called after content height calculation)
    pub fn update_max_scroll(&mut self, max_scroll: usize) {
        self.message_scroll.update_max_scroll(max_scroll);
        self.mark_dirty();
    }

    // === Debug panel scrolling ===

    pub fn scroll_debug_up(&mut self, lines: usize) {
        self.debug.scroll_up(lines);
        self.mark_dirty();
    }

    pub fn scroll_debug_down(&mut self, lines: usize) {
        self.debug.scroll_down(lines);
        self.mark_dirty();
    }

    pub fn update_debug_max_scroll(&mut self, max_scroll: usize) {
        self.debug.update_max_scroll(max_scroll);
        self.mark_dirty();
    }

    pub fn debug_scroll_offset(&self) -> usize {
        self.debug.scroll_offset()
    }

    pub fn debug_follow_bottom(&self) -> bool {
        self.debug.follow_bottom()
    }

    /// Scroll to bottom only if already following bottom (preserves manual scroll position)
    fn scroll_to_bottom_if_at_bottom(&mut self) {
        if self.message_scroll.follow_bottom() {
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
        self.debug.toggle();
        self.focus_dirty = true;
        self.mark_dirty();
    }

    pub fn toggle_menu(&mut self) {
        self.menu_open = !self.menu_open;
        self.mark_dirty();
    }

    pub fn push_debug_message(&mut self, message: String) {
        self.debug.push_message(message);
        self.mark_dirty();
    }

    pub fn clear_debug_messages(&mut self) {
        self.debug.clear_messages();
        self.mark_dirty();
    }

    /// Get render debug info (for logging inside render functions)
    pub fn get_render_debug(&self) -> String {
        format!(
            "messages={}, first_content={:?}, scroll={}, streaming={}",
            self.messages.len(),
            self.messages
                .first()
                .map(|m| &m.content[..m.content.len().min(50)]),
            self.message_scroll.offset(),
            self.streaming.is_some()
        )
    }

    /// Update token count during streaming
    pub fn update_token_count(&mut self, input: u32, output: u32) {
        if let Some(ref mut state) = self.streaming {
            state.update_tokens(input, output);
            self.mark_dirty();
        }
    }

    /// Get current token count (if streaming)
    pub fn token_count(&self) -> Option<TokenCount> {
        self.streaming.as_ref().map(|s| s.token_count())
    }

    /// Check if currently in thinking mode (waiting for first token)
    pub fn is_thinking(&self) -> bool {
        self.streaming
            .as_ref()
            .map(|s| s.is_thinking())
            .unwrap_or(false)
    }

    /// Check if in extended thinking phase
    pub fn is_extended_thinking(&self) -> bool {
        self.streaming
            .as_ref()
            .map(|s| s.is_extended_thinking())
            .unwrap_or(false)
    }

    /// Start extended thinking phase (called when ContentBlockStart::Thinking received)
    pub fn start_extended_thinking(&mut self) {
        if let Some(ref mut state) = self.streaming {
            state.start_extended_thinking();
        }
        self.push_debug_message("[THINKING] Extended thinking started".to_string());
        self.mark_dirty();
    }

    /// Note transition to receiving thinking content (called when ThinkingDelta received)
    pub fn append_thinking_content(&mut self) {
        if let Some(ref mut state) = self.streaming {
            state.append_thinking();
        }
        self.mark_dirty();
    }

    /// Stop extended thinking phase (called when ContentBlockStop received)
    pub fn stop_extended_thinking(&mut self) {
        if let Some(ref mut state) = self.streaming {
            state.stop_extended_thinking();
        }
        self.push_debug_message("[THINKING] Extended thinking stopped".to_string());
        self.mark_dirty();
    }

    /// Check if the "input blocked" message has been shown this thinking phase
    pub fn has_shown_blocked_input_message(&self) -> bool {
        self.streaming
            .as_ref()
            .map(|s| s.has_shown_blocked_input_message())
            .unwrap_or(false)
    }

    /// Set the "input blocked" message shown flag
    pub fn set_shown_blocked_input_message(&mut self, shown: bool) {
        if let Some(ref mut state) = self.streaming {
            state.set_shown_blocked_input_message(shown);
        }
    }

    /// Get thinking duration (if in extended thinking phase)
    pub fn thinking_duration(&self) -> Option<std::time::Duration> {
        self.streaming.as_ref().and_then(|s| s.thinking_duration())
    }

    // === Autocomplete management (delegates to AutocompleteManager) ===

    /// Activate autocomplete with given completions
    pub fn activate_autocomplete(&mut self, items: Vec<CompletionItem>) {
        if self.autocomplete.activate(items) {
            self.focus_dirty = true;
        }
        self.mark_dirty();
    }

    /// Clear autocomplete
    pub fn clear_autocomplete(&mut self) {
        if self.autocomplete.clear() {
            self.focus_dirty = true;
        }
        self.mark_dirty();
    }

    /// Navigate autocomplete selection up
    pub fn autocomplete_prev(&mut self) {
        self.autocomplete.prev();
        self.mark_dirty();
    }

    /// Navigate autocomplete selection down
    pub fn autocomplete_next(&mut self) {
        self.autocomplete.next();
        self.mark_dirty();
    }

    /// Get selected autocomplete item
    pub fn autocomplete_selected(&self) -> Option<&CompletionItem> {
        self.autocomplete.selected()
    }

    /// Check if autocomplete is active
    pub fn autocomplete_active(&self) -> bool {
        self.autocomplete.is_active()
    }

    /// Get autocomplete state (for rendering)
    pub fn autocomplete(&self) -> Option<&AutocompleteState> {
        self.autocomplete.state()
    }

    // === Memory modal management (delegates to ModalManager) ===

    /// Activate memory modal with destinations
    pub fn activate_memory_modal(
        &mut self,
        memory_text: String,
        destinations: Vec<MemoryDestination>,
    ) {
        self.modals.activate_memory_modal(memory_text, destinations);
        self.focus_dirty = true;
        self.mark_dirty();
    }

    /// Update memory text without resetting selection
    pub fn update_memory_text(&mut self, memory_text: String) {
        self.modals.update_memory_text(memory_text);
        self.mark_dirty();
    }

    /// Clear memory modal
    pub fn clear_memory_modal(&mut self) {
        self.modals.clear_memory_modal();
        self.focus_dirty = true;
        self.mark_dirty();
    }

    /// Navigate memory modal selection up
    pub fn memory_modal_prev(&mut self) {
        self.modals.memory_modal_prev();
        self.mark_dirty();
    }

    /// Navigate memory modal selection down
    pub fn memory_modal_next(&mut self) {
        self.modals.memory_modal_next();
        self.mark_dirty();
    }

    /// Get selected memory destination
    pub fn memory_modal_selected(&self) -> Option<&MemoryDestination> {
        self.modals.memory_modal_selected()
    }

    /// Check if memory modal is active
    pub fn memory_modal_active(&self) -> bool {
        self.modals.memory_modal_active()
    }

    /// Get memory modal state (for rendering)
    pub fn memory_modal(&self) -> Option<&MemoryModalState> {
        self.modals.memory_modal()
    }

    // === Permissions modal management (delegates to ModalManager) ===

    /// Activate permissions search modal
    pub fn activate_permissions_modal(&mut self) {
        self.modals.activate_permissions_modal();
        self.focus_dirty = true;
        self.mark_dirty();
    }

    /// Clear permissions modal
    pub fn clear_permissions_modal(&mut self) {
        self.modals.clear_permissions_modal();
        self.focus_dirty = true;
        self.mark_dirty();
    }

    /// Check if permissions modal is active
    pub fn permissions_modal_active(&self) -> bool {
        self.modals.permissions_modal_active()
    }

    /// Get mutable reference to permissions modal state
    pub fn permissions_modal_mut(&mut self) -> Option<&mut PermissionsSearchState> {
        self.modals.permissions_modal_mut()
    }

    /// Get permissions modal state (for rendering)
    pub fn permissions_modal(&self) -> Option<&PermissionsSearchState> {
        self.modals.permissions_modal()
    }

    // === Focus management ===

    /// Get focus flag for messages pane
    pub fn focus_messages(&self) -> FocusFlag {
        self.focus_messages.clone()
    }

    /// Get focus flag for input pane
    pub fn focus_input(&self) -> FocusFlag {
        self.focus_input.clone()
    }

    /// Get focus flag for debug panel
    pub fn focus_debug(&self) -> FocusFlag {
        self.focus_debug.clone()
    }

    /// Get focus flag for autocomplete popup
    pub fn focus_autocomplete(&self) -> FocusFlag {
        self.focus_autocomplete.clone()
    }

    /// Get focus flag for memory modal
    pub fn focus_memory_modal(&self) -> FocusFlag {
        self.focus_memory_modal.clone()
    }

    /// Get focus flag for permissions modal
    pub fn focus_permissions_modal(&self) -> FocusFlag {
        self.focus_permissions_modal.clone()
    }

    /// Update layout cache from render
    pub fn update_layout_cache(&mut self, cache: LayoutCache) {
        // Invalidate focus when layout areas change (e.g., terminal resize)
        if self.layout_cache.messages_area != cache.messages_area
            || self.layout_cache.input_area != cache.input_area
            || self.layout_cache.debug_area != cache.debug_area
        {
            self.focus_dirty = true;
        }
        self.layout_cache = cache;
    }

    /// Get layout cache
    pub fn layout_cache(&self) -> &LayoutCache {
        &self.layout_cache
    }

    // === Focus caching ===

    /// Mark the focus structure as needing a rebuild
    pub fn invalidate_focus(&mut self) {
        self.focus_dirty = true;
    }

    /// Check if focus needs rebuilding
    pub fn is_focus_dirty(&self) -> bool {
        self.focus_dirty
    }

    /// Get the cached focus, rebuilding if necessary.
    /// Returns None if layout cache is not initialized (zero-size area).
    pub fn get_or_rebuild_focus(&mut self) -> Option<&mut Focus> {
        let cache = self.layout_cache.clone();

        // Skip if layout cache is not initialized
        if cache.messages_area.width == 0 && cache.messages_area.height == 0 {
            return None;
        }

        if self.focus_dirty || self.cached_focus.is_none() {
            // Rebuild focus structure
            let mut builder = rat_focus::FocusBuilder::default();

            let messages_wrapper = MessagesPaneWrapper {
                focus: self.focus_messages.clone(),
                area: cache.messages_area,
            };
            rat_focus::HasFocus::build(&messages_wrapper, &mut builder);

            let input_wrapper = InputPaneWrapper {
                focus: self.focus_input.clone(),
                area: cache.input_area,
            };
            rat_focus::HasFocus::build(&input_wrapper, &mut builder);

            if let Some(debug_area) = cache.debug_area {
                let debug_wrapper = DebugPaneWrapper {
                    focus: self.focus_debug.clone(),
                    area: debug_area,
                };
                rat_focus::HasFocus::build(&debug_wrapper, &mut builder);
            }

            if self.autocomplete.is_active() {
                let autocomplete_wrapper = AutocompletePopupWrapper {
                    focus: self.focus_autocomplete.clone(),
                    area: cache.input_area,
                };
                rat_focus::HasFocus::build(&autocomplete_wrapper, &mut builder);
            }

            if self.modals.memory_modal_active() {
                let memory_modal_wrapper = MemoryModalWrapper {
                    focus: self.focus_memory_modal.clone(),
                    area: cache.input_area,
                };
                rat_focus::HasFocus::build(&memory_modal_wrapper, &mut builder);
            }

            if self.modals.permissions_modal_active() {
                let permissions_modal_wrapper = PermissionsModalWrapper {
                    focus: self.focus_permissions_modal.clone(),
                    area: cache.input_area,
                };
                rat_focus::HasFocus::build(&permissions_modal_wrapper, &mut builder);
            }

            self.cached_focus = Some(builder.build());
            self.focus_dirty = false;
        }

        self.cached_focus.as_mut()
    }
}

// === HasFocus wrapper structs for rat-focus integration ===

use rat_focus::{FocusBuilder, HasFocus, Navigation};

/// Generic pane wrapper for rat-focus integration.
/// Z_ORDER=0 uses Regular navigation (keyboard Tab); Z_ORDER>0 uses Mouse-only navigation.
pub struct PaneWrapper<const Z_ORDER: u16> {
    pub focus: FocusFlag,
    pub area: Rect,
}

impl<const Z_ORDER: u16> HasFocus for PaneWrapper<Z_ORDER> {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> ratatui::layout::Rect {
        self.area
    }

    fn area_z(&self) -> u16 {
        Z_ORDER
    }

    fn navigable(&self) -> Navigation {
        if Z_ORDER == 0 {
            Navigation::Regular
        } else {
            Navigation::Mouse
        }
    }
}

/// Type aliases preserving the original names for backward compatibility
pub type MessagesPaneWrapper = PaneWrapper<0>;
pub type InputPaneWrapper = PaneWrapper<0>;
pub type DebugPaneWrapper = PaneWrapper<0>;
pub type AutocompletePopupWrapper = PaneWrapper<1>;
pub type MemoryModalWrapper = PaneWrapper<2>;
pub type PermissionsModalWrapper = PaneWrapper<3>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_movement() {
        let mut app = App::new(PermissionMode::default());
        app.insert_char('a');
        app.insert_char('b');
        app.insert_char('c');

        assert_eq!(app.cursor_pos().1, 3);

        app.move_cursor_left();
        assert_eq!(app.cursor_pos().1, 2);

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
        assert_eq!(app.cursor_pos(), (0, 0));
    }

    #[test]
    fn test_empty_input_not_submitted() {
        let mut app = App::new(PermissionMode::default());
        app.insert_char(' ');

        let input = app.submit_input();
        assert_eq!(input, None); // Whitespace-only not submitted
    }
}
