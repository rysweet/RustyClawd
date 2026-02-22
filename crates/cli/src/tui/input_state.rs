//! Input state management for TUI
//!
//! Encapsulates the TextArea input widget and all input editing methods.
//! Extracted from app.rs to reduce its size and improve modularity.

use crate::tui::soft_wrap::SoftWrapState;
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders},
};
use std::collections::HashSet;
use tui_textarea::TextArea;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

/// Rust orange color for TUI styling
const RUST_ORANGE: Color = Color::Rgb(222, 165, 132);

/// Create a styled TextArea for the input pane with default Rust-orange styling.
/// This is the single source of truth for input TextArea initialization.
fn make_input_textarea() -> TextArea<'static> {
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
    input
}

/// Input state - manages the TextArea widget and all input editing operations
pub struct InputState {
    /// Current input buffer (multi-line text editor)
    pub input: TextArea<'static>,

    /// Soft wrap state for input text wrapping
    soft_wrap: SoftWrapState,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            input: make_input_textarea(),
            soft_wrap: SoftWrapState::default(),
        }
    }

    // === State queries (immutable) ===

    pub fn input_text(&self) -> String {
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
    pub fn update_input_focus_style(&mut self, is_focused: bool) {
        let border_color = if is_focused {
            Color::White
        } else {
            RUST_ORANGE
        };

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

    // === Input editing methods ===

    pub fn insert_char(&mut self, c: char) {
        self.input.insert_char(c);

        // Reflow if current line exceeds width
        if self.should_reflow() {
            self.reflow_input_content();
        }
    }

    pub fn delete_char(&mut self) {
        self.input.delete_next_char(); // Delete key: delete char AT cursor

        // Reflow to potentially merge lines that are now shorter
        self.reflow_input_content();
    }

    /// Perform backspace operation. Returns debug messages to be logged by caller.
    pub fn backspace(&mut self) -> Vec<String> {
        let mut debug_messages = Vec::new();

        // Delete grapheme cluster (handles emojis with modifiers correctly)
        let (row, col) = self.input.cursor();

        // Clone lines to avoid borrow issues
        let lines: Vec<String> = self.input.lines().iter().map(|s| s.to_string()).collect();

        if row >= lines.len() {
            return debug_messages;
        }

        let line = lines[row].clone();

        // Debug logging
        debug_messages.push(format!(
            "[BACKSPACE] cursor=({},{}), line_len={}, line={:?}",
            row,
            col,
            line.len(),
            line
        ));

        if col == 0 {
            // At start of line - use default behavior to merge with previous line
            self.input.delete_char();
        } else {
            // Delete the grapheme cluster before cursor
            let graphemes: Vec<String> = line.graphemes(true).map(|s| s.to_string()).collect();

            debug_messages.push(format!(
                "[BACKSPACE] grapheme_count={}, first_10={:?}",
                graphemes.len(),
                graphemes.iter().take(10).collect::<Vec<_>>()
            ));

            // Find grapheme index by counting characters
            // TextArea's col is in CHARACTER positions, not display width
            let mut char_count = 0;
            let mut grapheme_idx = 0;

            for (idx, g) in graphemes.iter().enumerate() {
                let g_char_count = g.chars().count();
                if char_count >= col {
                    break;
                }
                char_count += g_char_count;
                grapheme_idx = idx + 1;
            }

            debug_messages.push(format!(
                "[BACKSPACE] char_count={}, grapheme_idx={}",
                char_count, grapheme_idx
            ));

            if grapheme_idx > 0 {
                let deleted_grapheme = graphemes.get(grapheme_idx - 1).cloned().unwrap_or_default();
                let deleted_char_count = deleted_grapheme.chars().count();

                debug_messages.push(format!(
                    "[BACKSPACE] deleting grapheme_idx={}, content={:?}, char_len={}",
                    grapheme_idx - 1,
                    deleted_grapheme,
                    deleted_char_count
                ));

                // Build new line without the previous grapheme
                let mut new_line = String::new();
                for (idx, g) in graphemes.iter().enumerate() {
                    if idx != grapheme_idx - 1 {
                        new_line.push_str(g);
                    }
                }

                // Replace the line
                let mut new_lines = lines.clone();
                new_lines[row] = new_line;

                // Calculate new cursor position (in characters)
                let new_col = col.saturating_sub(deleted_char_count);

                debug_messages.push(format!(
                    "[BACKSPACE] new_col={}, deleted_char_count={}",
                    new_col, deleted_char_count
                ));

                // Update textarea
                self.input = TextArea::from(new_lines);
                self.input
                    .move_cursor(tui_textarea::CursorMove::Jump(row as u16, new_col as u16));
            }
        }

        // Reflow to potentially merge lines that are now shorter
        self.reflow_input_content();

        debug_messages
    }

    pub fn move_cursor_left(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Back);
    }

    pub fn move_cursor_right(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Forward);
    }

    pub fn move_cursor_to_start(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Head);
    }

    pub fn move_cursor_to_end(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::End);
    }

    pub fn insert_newline(&mut self) {
        self.input.insert_newline();
    }

    pub fn move_cursor_up(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Up);
    }

    pub fn move_cursor_down(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Down);
    }

    pub fn move_cursor_word_left(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::WordBack);
    }

    pub fn move_cursor_word_right(&mut self) {
        self.input
            .move_cursor(tui_textarea::CursorMove::WordForward);
    }

    pub fn move_cursor_absolute_start(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Top);
    }

    pub fn move_cursor_absolute_end(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Bottom);
    }

    pub fn move_cursor_to_input_top(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Top);
    }

    pub fn move_cursor_to_input_bottom(&mut self) {
        self.input.move_cursor(tui_textarea::CursorMove::Bottom);
    }

    pub fn scroll_input_viewport_up(&mut self) {
        // TextArea auto-scrolls when cursor moves — move cursor up 5 lines to scroll viewport
        for _ in 0..5 {
            self.input.move_cursor(tui_textarea::CursorMove::Up);
        }
    }

    pub fn scroll_input_viewport_down(&mut self) {
        for _ in 0..5 {
            self.input.move_cursor(tui_textarea::CursorMove::Down);
        }
    }

    /// Clear input without submitting (Ctrl+U behavior)
    pub fn clear_input(&mut self) {
        self.input = make_input_textarea();
        self.soft_wrap.clear(); // Clear soft-break tracking
    }

    /// Submit input: extract text stripping soft breaks, clear buffer, return text
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
        let mut new_input = make_input_textarea();

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
    }

    // === Reflow engine ===

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
                    let first_char_len = remaining.chars().next().unwrap().len_utf8(); // OK to unwrap: remaining is non-empty (checked by while loop)
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
            if visual_width >= self.soft_wrap.inner_width() as usize {
                // Cap cursor at the last safe position
                let safe_col = current_line.chars().count().saturating_sub(1);
                target_col = target_col.min(safe_col);
            }
        }

        // Move cursor to restored position
        new_input.move_cursor(tui_textarea::CursorMove::Jump(
            target_row as u16,
            target_col as u16,
        ));

        self.input = new_input;
        self.soft_wrap.soft_break_lines = new_soft_breaks;
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
}
