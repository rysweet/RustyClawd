//! Input buffer delegation methods for App.
//!
//! Extracted from app.rs to keep the main App module focused.
//! Each method delegates to InputState and marks the UI dirty.

use super::App;

impl App {
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
}
