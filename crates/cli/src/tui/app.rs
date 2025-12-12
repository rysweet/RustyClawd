//! Application state for TUI

use crate::permission_mode::PermissionMode;
use crate::tui::message::Message;
use crate::tui::token_counter::TokenCount;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders},
};
use rat_focus::FocusFlag;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tui_textarea::TextArea;
use unicode_width::UnicodeWidthChar;

/// Maximum debug messages to keep in buffer
const MAX_DEBUG_MESSAGES: usize = 1000;

/// Rust orange color for TUI styling
const RUST_ORANGE: Color = Color::Rgb(222, 165, 132);

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

/// Soft wrap state - tracks which lines are auto-wrapped (not user newlines)
#[derive(Clone, Debug)]
pub struct SoftWrapState {
    /// Set of line indices that END with a soft break
    /// (i.e., line was created by auto-wrap, not user Enter)
    soft_break_lines: HashSet<usize>,

    /// Cached inner width of input area (excluding borders)
    inner_width: u16,

    /// Tab width for width calculations (synced with TextArea's tab_length)
    tab_width: u8,
}

impl Default for SoftWrapState {
    fn default() -> Self {
        Self {
            soft_break_lines: HashSet::new(),
            inner_width: 80, // Default, will be updated from layout
            tab_width: 4, // Match TextArea default
        }
    }
}

impl SoftWrapState {
    /// Check if a line ends with a soft break
    pub fn is_soft_break(&self, line_idx: usize) -> bool {
        self.soft_break_lines.contains(&line_idx)
    }

    /// Mark a line as ending with a soft break
    pub fn add_soft_break(&mut self, line_idx: usize) {
        self.soft_break_lines.insert(line_idx);
    }

    /// Remove soft break marker from a line
    pub fn remove_soft_break(&mut self, line_idx: usize) {
        self.soft_break_lines.remove(&line_idx);
    }

    /// Clear all soft break markers
    pub fn clear(&mut self) {
        self.soft_break_lines.clear();
    }

    /// Update inner width (called when layout changes)
    pub fn update_width(&mut self, width: u16) {
        self.inner_width = width;
    }

    /// Get current inner width
    pub fn inner_width(&self) -> u16 {
        self.inner_width
    }

    /// Calculate visual width of a line accounting for tabs and Unicode
    pub fn calculate_line_width(&self, line: &str) -> usize {
        let mut width = 0;
        for c in line.chars() {
            if c == '\t' {
                // Tab advances to next tab stop
                let tab_width = self.tab_width as usize;
                width += tab_width - (width % tab_width);
            } else {
                width += UnicodeWidthChar::width(c).unwrap_or(0);
            }
        }
        width
    }
}

/// Main application state - single source of truth
pub struct App {
    /// Message history (all messages in conversation)
    messages: Vec<Message>,

    /// Current input buffer (multi-line text editor)
    pub input: TextArea<'static>,

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

    /// Debug panel scroll offset (lines from top)
    debug_scroll_offset: usize,

    /// Maximum valid scroll offset for debug panel (updated by renderer)
    debug_max_scroll: usize,

    /// Auto-follow bottom in debug panel (like message panel)
    debug_follow_bottom: bool,

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

    /// Layout cache from last render (for hit testing)
    layout_cache: LayoutCache,

    /// Clickable regions for mouse interactions
    pub click_regions: crate::tui::click_region::ClickableRegions,

    /// Soft wrap state for input text wrapping
    soft_wrap: SoftWrapState,
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
        // Configure TextArea with styling ONCE during initialization
        let mut input = TextArea::default();
        input.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(RUST_ORANGE))
                .title(vec![
                    Span::styled("✏️  ", Style::default().fg(RUST_ORANGE)),
                    Span::styled(
                        "Input",
                        Style::default()
                            .fg(RUST_ORANGE)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
        );
        // Remove underline from cursor line (default is underlined)
        input.set_cursor_line_style(Style::default());

        Self {
            messages: Vec::new(),
            input,
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
            debug_scroll_offset: 0,
            debug_max_scroll: 0,
            debug_follow_bottom: true, // Start in auto-follow mode
            menu_open: false,
            focus_messages: FocusFlag::new(),
            focus_input: FocusFlag::new(),
            focus_debug: FocusFlag::new(),
            focus_autocomplete: FocusFlag::new(),
            focus_memory_modal: FocusFlag::new(),
            layout_cache: LayoutCache::default(),
            click_regions: crate::tui::click_region::ClickableRegions::new(),
            soft_wrap: SoftWrapState::default(),
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
        self.input.lines().join("\n")
    }

    pub fn cursor_pos(&self) -> (usize, usize) {
        self.input.cursor()
    }

    pub fn input_line_count(&self) -> usize {
        self.input.lines().len().max(1)
    }

    pub fn has_multi_line_input(&self) -> bool {
        self.input.lines().len() > 1
    }

    /// Update TextArea block style based on focus state
    /// Must be called before rendering to show focus-aware border colors
    pub fn update_input_focus_style(&mut self) {
        let is_focused = self.focus_input.get();
        let border_color = if is_focused { Color::White } else { RUST_ORANGE };

        self.input.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(vec![
                    Span::styled("✏️  ", Style::default().fg(border_color)),
                    Span::styled(
                        "Input",
                        Style::default()
                            .fg(border_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
        );
    }

    /// Update soft wrap inner width and reflow if width changed
    pub fn update_soft_wrap_width(&mut self, width: u16) {
        let old_width = self.soft_wrap.inner_width();
        if width != old_width {
            self.soft_wrap.update_width(width);
            // Reflow content with new width
            self.reflow_input_content();
        }
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
            if let Some(msg) = self.messages.get_mut(state.message_index) {
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

    /// Begin a new tool execution message (creates placeholder that will be updated dynamically)
    pub fn begin_tool_message(&mut self, tool_id: String, tool_name: String, params: serde_json::Value) -> usize {
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
            (state.tool_name.clone(), state.start_time.elapsed().as_secs(), state.message_index)
        });

        // Update tool state
        if let Some(state) = self.tool_messages.get_mut(tool_id) {
            state.completed = true;
            state.result = Some(result.clone());
            state.elapsed_duration = Some(state.start_time.elapsed().as_secs());

            // Update message status based on result
            if let Some(message) = self.messages.get_mut(state.message_index) {
                if result.is_error {
                    message.mark_error();
                } else {
                    message.complete_streaming();
                }
            }

            self.mark_dirty();
        }

        // Log debug message after mutation
        if let Some((tool_name, elapsed, _)) = debug_info {
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
        self.input.insert_char(c);

        // Reflow if current line exceeds width
        if self.should_reflow() {
            self.reflow_input_content();
        }

        self.mark_dirty();
    }

    pub fn delete_char(&mut self) {
        self.input.delete_next_char();  // Delete key: delete char AT cursor

        // Reflow to potentially merge lines that are now shorter
        self.reflow_input_content();

        self.mark_dirty();
    }

    pub fn backspace(&mut self) {
        self.input.delete_char();  // Backspace key: delete char BEFORE cursor

        // Reflow to potentially merge lines that are now shorter
        self.reflow_input_content();

        self.mark_dirty();
    }

    pub fn move_cursor_left(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Back);
        self.mark_dirty();
    }

    pub fn move_cursor_right(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Forward);
        self.mark_dirty();
    }

    pub fn move_cursor_to_start(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Head);
        self.mark_dirty();
    }

    pub fn move_cursor_to_end(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::End);
        self.mark_dirty();
    }

    // === NEW: Multi-line input methods ===

    pub fn insert_newline(&mut self) {
        self.input.insert_newline();
        self.mark_dirty();
    }

    pub fn move_cursor_up(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Up);
        self.mark_dirty();
    }

    pub fn move_cursor_down(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Down);
        self.mark_dirty();
    }

    pub fn move_cursor_word_left(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::WordBack);
        self.mark_dirty();
    }

    pub fn move_cursor_word_right(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::WordForward);
        self.mark_dirty();
    }

    pub fn move_cursor_absolute_start(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Top);
        self.mark_dirty();
    }

    pub fn move_cursor_absolute_end(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Bottom);
        self.mark_dirty();
    }

    pub fn move_cursor_to_input_top(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Top);
        self.mark_dirty();
    }

    pub fn move_cursor_to_input_bottom(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Bottom);
        self.mark_dirty();
    }

    pub fn scroll_input_viewport_up(&mut self) {
        // TODO: Investigate if TextArea has scroll() method or if auto-handled
        // For now, just mark dirty in case TextArea needs repaint
        self.mark_dirty();
    }

    pub fn scroll_input_viewport_down(&mut self) {
        // TODO: Same as above
        self.mark_dirty();
    }

    /// Clear input without submitting (Ctrl+U behavior)
    pub fn clear_input(&mut self) {
        // Reset TextArea - recreate with same styling
        let mut new_input = TextArea::default();
        new_input.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(RUST_ORANGE))
                .title(vec![
                    Span::styled("✏️  ", Style::default().fg(RUST_ORANGE)),
                    Span::styled(
                        "Input",
                        Style::default()
                            .fg(RUST_ORANGE)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
        );
        // Remove underline from cursor line (default is underlined)
        new_input.set_cursor_line_style(Style::default());
        self.input = new_input;
        self.soft_wrap.clear(); // Clear soft-break tracking
        self.mark_dirty();
    }

    /// Reflow input content to apply/update soft line wrapping
    /// Joins lines at soft breaks, then splits at width boundaries
    fn reflow_input_content(&mut self) {
        let inner_width = self.soft_wrap.inner_width() as usize;
        if inner_width == 0 {
            return; // Can't wrap with zero width
        }

        // Get current cursor position
        let (cursor_row, cursor_col) = self.input.cursor();

        // Calculate absolute character position (counting only content, not newlines)
        // This ensures cursor position is preserved when soft-breaks are added/removed
        // NOTE: Using .chars().count() not .len() because tui-textarea uses CHARACTER indices
        let lines = self.input.lines();
        let mut char_pos = 0;
        for (idx, line) in lines.iter().enumerate() {
            if idx < cursor_row {
                char_pos += line.chars().count(); // CHARACTER count, not bytes
            } else if idx == cursor_row {
                char_pos += cursor_col;
                break;
            }
        }

        // Step 1: Join lines at soft breaks to reconstruct logical paragraphs
        let mut paragraphs: Vec<String> = Vec::new();
        let mut current_paragraph = String::new();

        for (idx, line) in lines.iter().enumerate() {
            current_paragraph.push_str(line);

            if idx == lines.len() - 1 {
                // Last line - always end paragraph
                paragraphs.push(current_paragraph.clone());
                break;
            } else if self.soft_wrap.is_soft_break(idx) {
                // Soft break - continue building paragraph (no newline)
                continue;
            } else {
                // Hard break (user Enter) - end paragraph
                paragraphs.push(current_paragraph.clone());
                current_paragraph.clear();
            }
        }

        // Step 2: Split paragraphs at width boundaries to create soft-wrapped lines
        let mut new_lines: Vec<String> = Vec::new();
        let mut new_soft_breaks: HashSet<usize> = HashSet::new();

        for paragraph in paragraphs {
            if paragraph.is_empty() {
                new_lines.push(String::new());
                continue;
            }

            let mut remaining = paragraph.as_str();

            while !remaining.is_empty() {
                let line_width = self.soft_wrap.calculate_line_width(remaining);

                if line_width < inner_width {
                    // Rest of paragraph fits on one line with room for cursor at end
                    new_lines.push(remaining.to_string());
                    break;
                }

                // Need to wrap - find break point at or before inner_width
                let mut break_pos = 0;
                let mut accumulated_width = 0;
                let mut last_space_pos = None;
                let mut last_space_width = 0;

                for (char_idx, ch) in remaining.char_indices() {
                    let char_width = if ch == '\t' {
                        let tab_width = self.soft_wrap.tab_width as usize;
                        tab_width - (accumulated_width % tab_width)
                    } else {
                        UnicodeWidthChar::width(ch).unwrap_or(0)
                    };

                    // Break BEFORE reaching inner_width to leave room for cursor
                    // Use >= instead of > to ensure accumulated_width stays < inner_width
                    if accumulated_width + char_width >= inner_width {
                        break;
                    }

                    accumulated_width += char_width;
                    break_pos = char_idx + ch.len_utf8();

                    // Track last space for word-boundary wrapping
                    if ch.is_whitespace() {
                        last_space_pos = Some(char_idx + ch.len_utf8());
                        last_space_width = accumulated_width;
                    }
                }

                // Prefer breaking at last space if one exists within 80% of width
                let final_break_pos = if let Some(space_pos) = last_space_pos {
                    if last_space_width >= (inner_width * 4 / 5) {
                        space_pos
                    } else {
                        break_pos // Mid-word break
                    }
                } else {
                    break_pos // No spaces, mid-word break
                };

                if final_break_pos == 0 {
                    // Edge case: single character exceeds width
                    // Take at least one character
                    let first_char_len = remaining.chars().next().unwrap().len_utf8();
                    new_lines.push(remaining[..first_char_len].to_string());
                    remaining = &remaining[first_char_len..];
                } else {
                    new_lines.push(remaining[..final_break_pos].to_string());
                    remaining = &remaining[final_break_pos..];
                }

                // Mark this line as ending with a soft break (if more content follows)
                // CRITICAL: Mark ALL wrapped lines, including first line of paragraph
                if !remaining.is_empty() {
                    new_soft_breaks.insert(new_lines.len() - 1);
                }
            }
        }

        // Step 3: Update TextArea with reflowed content
        let new_content = new_lines.join("\n");

        // Replace TextArea content while preserving styling
        // Clone new_lines since we might need to modify it later for cursor overflow fix
        let mut new_input = TextArea::from(new_lines.clone());
        new_input.set_block(self.input.block().cloned().unwrap_or_default());
        new_input.set_cursor_line_style(Style::default());
        new_input.set_cursor_style(self.input.cursor_style());

        // Step 4: Restore cursor to equivalent character position (counting only content)
        let mut remaining_chars = char_pos;
        let mut target_row = 0;
        let mut target_col = 0;

        let content_lines: Vec<&str> = new_content.lines().collect();
        for (line_idx, line) in content_lines.iter().enumerate() {
            let line_len = line.chars().count(); // CHARACTER count, not bytes
            if remaining_chars <= line_len {
                target_row = line_idx;
                target_col = remaining_chars;
                break;
            }
            remaining_chars -= line_len; // Don't subtract newline - we only counted content chars
            target_row = line_idx + 1;
        }

        // Fix visual cursor issue: if cursor would overflow, ensure content wraps
        // This should rarely trigger now that wrap logic uses >= instead of >
        if target_row < content_lines.len() {
            let current_line = content_lines[target_row];

            // Get substring from line start to cursor position (CHARACTER-aware slicing)
            let text_before_cursor = {
                match current_line.char_indices().nth(target_col) {
                    Some((byte_idx, _)) => &current_line[..byte_idx],
                    None => current_line, // target_col >= line length
                }
            };

            // Calculate visual width of that text
            let visual_width = self.soft_wrap.calculate_line_width(text_before_cursor);

            // If cursor would overflow, force it to stay at the end of the line
            // This prevents cursor from rendering beyond the boundary
            if visual_width >= self.soft_wrap.inner_width as usize {
                // Cap cursor at the last safe position
                let safe_col = current_line.chars().count().saturating_sub(1);
                target_col = target_col.min(safe_col);
            }
        }

        // Move cursor to restored position
        new_input.move_cursor(tui_textarea::CursorMove::Jump(target_row as u16, target_col as u16));

        self.input = new_input;
        self.soft_wrap.soft_break_lines = new_soft_breaks;
        self.mark_dirty();
    }

    /// Check if reflow is needed after character insertion
    fn should_reflow(&self) -> bool {
        let (row, _col) = self.input.cursor();
        if let Some(line) = self.input.lines().get(row) {
            let line_width = self.soft_wrap.calculate_line_width(line);
            line_width >= self.soft_wrap.inner_width() as usize
        } else {
            false
        }
    }

    pub fn submit_input(&mut self) -> Option<String> {
        // Extract text, stripping soft breaks
        let lines = self.input.lines();
        let mut result = String::new();

        for (idx, line) in lines.iter().enumerate() {
            result.push_str(line);

            // Add newline only if this is NOT a soft break
            if idx < lines.len() - 1 && !self.soft_wrap.is_soft_break(idx) {
                result.push('\n');
            }
        }

        if result.trim().is_empty() {
            return None;
        }

        // Clear input and return text
        self.clear_input();
        Some(result)
    }

    pub fn set_input(&mut self, text: &str) {
        // Clear existing and set new text
        let mut new_input = TextArea::default();
        new_input.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(RUST_ORANGE))
                .title(vec![
                    Span::styled("✏️  ", Style::default().fg(RUST_ORANGE)),
                    Span::styled(
                        "Input",
                        Style::default()
                            .fg(RUST_ORANGE)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
        );
        // Remove underline from cursor line (default is underlined)
        new_input.set_cursor_line_style(Style::default());

        // Insert the text line by line
        for (i, line) in text.lines().enumerate() {
            if i > 0 {
                new_input.insert_newline();
            }
            for ch in line.chars() {
                new_input.insert_char(ch);
            }
        }

        self.input = new_input;

        // Reflow the newly set content
        self.reflow_input_content();

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
        self.mark_dirty(); // Trigger UI refresh when max_scroll changes
    }

    // === Debug panel scrolling ===

    pub fn scroll_debug_up(&mut self, lines: usize) {
        // If we're in follow mode, transition to manual scroll mode
        if self.debug_follow_bottom {
            self.debug_follow_bottom = false;
            // Initialize scroll_offset to max_scroll so we're actually at the bottom
            self.debug_scroll_offset = self.debug_max_scroll;
        }

        // Now scroll up from current position
        self.debug_scroll_offset = self.debug_scroll_offset.saturating_sub(lines);
        self.mark_dirty();
    }

    pub fn scroll_debug_down(&mut self, lines: usize) {
        // If already following bottom, stay there
        if self.debug_follow_bottom {
            self.mark_dirty();
            return;
        }

        // Increment scroll offset and clamp to valid range
        self.debug_scroll_offset = self.debug_scroll_offset.saturating_add(lines);

        // Clamp to max_scroll to prevent phantom accumulation
        if self.debug_scroll_offset >= self.debug_max_scroll {
            // At or past the bottom - switch to follow mode
            self.debug_follow_bottom = true;
            self.debug_scroll_offset = 0; // Renderer will set to max_scroll
        }

        self.mark_dirty();
    }

    pub fn update_debug_max_scroll(&mut self, max_scroll: usize) {
        self.debug_max_scroll = max_scroll;
        self.mark_dirty();
    }

    pub fn debug_scroll_offset(&self) -> usize {
        self.debug_scroll_offset
    }

    pub fn debug_follow_bottom(&self) -> bool {
        self.debug_follow_bottom
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

    /// Update layout cache from render
    pub fn update_layout_cache(&mut self, cache: LayoutCache) {
        self.layout_cache = cache;
    }

    /// Get layout cache
    pub fn layout_cache(&self) -> &LayoutCache {
        &self.layout_cache
    }
}

// === HasFocus wrapper structs for rat-focus integration ===

use rat_focus::{FocusBuilder, HasFocus, Navigation};

/// Wrapper for messages pane to implement HasFocus
/// Holds FocusFlag directly to avoid borrowing conflicts
pub struct MessagesPaneWrapper {
    pub focus: FocusFlag,
    pub area: Rect,
}

impl HasFocus for MessagesPaneWrapper {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> ratatui::layout::Rect {
        self.area
    }

    fn navigable(&self) -> Navigation {
        Navigation::Regular
    }
}

/// Wrapper for input pane to implement HasFocus
/// Holds FocusFlag directly to avoid borrowing conflicts
pub struct InputPaneWrapper {
    pub focus: FocusFlag,
    pub area: Rect,
}

impl HasFocus for InputPaneWrapper {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> ratatui::layout::Rect {
        self.area
    }

    fn navigable(&self) -> Navigation {
        Navigation::Regular
    }
}

/// Wrapper for debug panel to implement HasFocus
/// Holds FocusFlag directly to avoid borrowing conflicts
pub struct DebugPaneWrapper {
    pub focus: FocusFlag,
    pub area: Rect,
}

impl HasFocus for DebugPaneWrapper {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> ratatui::layout::Rect {
        self.area
    }

    fn navigable(&self) -> Navigation {
        Navigation::Regular
    }
}

/// Wrapper for autocomplete popup to implement HasFocus with z-ordering
/// Holds FocusFlag directly to avoid borrowing conflicts
pub struct AutocompletePopupWrapper {
    pub focus: FocusFlag,
    pub area: Rect,
}

impl HasFocus for AutocompletePopupWrapper {
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
        1 // Z-order 1 (above panes)
    }

    fn navigable(&self) -> Navigation {
        Navigation::Mouse // Mouse-only navigation
    }
}

/// Wrapper for memory modal to implement HasFocus with z-ordering
/// Holds FocusFlag directly to avoid borrowing conflicts
pub struct MemoryModalWrapper {
    pub focus: FocusFlag,
    pub area: Rect,
}

impl HasFocus for MemoryModalWrapper {
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
        2 // Z-order 2 (above autocomplete)
    }

    fn navigable(&self) -> Navigation {
        Navigation::Mouse // Mouse-only navigation
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
