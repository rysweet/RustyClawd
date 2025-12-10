//! Event handling for TUI

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;

use crate::tui::app::App;
use crate::tui::keybindings::{KeyAction, KeyBindings};

/// Result of event handling
#[derive(Debug, PartialEq, Eq)]
pub enum EventResult {
    /// Continue running
    Continue,

    /// Submit user input (returns the input string)
    Submit(String),

    /// Save memory to file (memory_text, file_path)
    SaveMemory(String, String),

    /// Exit application
    Exit,
}

/// Poll for events with timeout
pub fn poll_event(timeout: Duration) -> Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Handle a single event, mutating app state
pub fn handle_event(app: &mut App, event: Event) -> Result<EventResult> {
    match event {
        Event::Key(key) => handle_key_event(app, key),
        Event::Mouse(mouse) => handle_mouse_event(app, mouse),
        Event::Resize(_, _) => {
            // Terminal resized - will be handled in next render
            Ok(EventResult::Continue)
        }
        _ => Ok(EventResult::Continue),
    }
}

fn handle_mouse_event(app: &mut App, mouse: event::MouseEvent) -> Result<EventResult> {
    use event::MouseEventKind;

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            // Scroll up 3 lines (smooth scrolling)
            app.scroll_up(3);
        }
        MouseEventKind::ScrollDown => {
            // Scroll down 3 lines (smooth scrolling)
            app.scroll_down(3);
        }
        _ => {
            // Ignore other mouse events (clicks, drags, etc.)
        }
    }

    Ok(EventResult::Continue)
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<EventResult> {
    // CRITICAL: Filter out key release events on Windows/WSL
    // Crossterm reports BOTH KeyPress and KeyRelease, causing duplicate processing
    if key.kind != KeyEventKind::Press {
        return Ok(EventResult::Continue);
    }


    // Special handling: backslash-escaped Enter inserts newline
    // Check if Enter key pressed WITHOUT Shift modifier AND input ends with backslash
    if matches!(key.code, KeyCode::Enter)
        && !key.modifiers.contains(KeyModifiers::SHIFT)
        && !app.is_streaming()
    {
        let input = app.input();
        if input.ends_with('\\') {
            // Remove trailing backslash and insert newline
            app.backspace();
            app.insert_newline();
            return Ok(EventResult::Continue);
        }
    }

    // Get keybindings configuration
    let bindings = KeyBindings::default();

    // Try to find a keybinding action for this key
    if let Some(action) = bindings.find_action(&key) {
        return handle_key_action(app, action);
    }

    // If no binding found AND it's a printable character AND not streaming, insert it
    if bindings.is_printable_char(&key) && !app.is_streaming() {
        if let crossterm::event::KeyCode::Char(c) = key.code {
            app.insert_char(c);
        }
    }

    // All other keys (control keys without bindings) are silently ignored
    // This prevents garbage characters from leaking into input

    Ok(EventResult::Continue)
}

/// Execute a key action on the app state
fn handle_key_action(app: &mut App, action: &KeyAction) -> Result<EventResult> {
    match action {
        KeyAction::Exit => {
            app.exit();
            return Ok(EventResult::Exit);
        }
        KeyAction::ToggleDebug => {
            app.toggle_debug();
        }
        KeyAction::CyclePermissionMode => {
            app.cycle_permission_mode();
        }
        KeyAction::ClearError => {
            // Escape key clears errors, memory modal, and autocomplete
            app.clear_error();
            if app.memory_modal_active() {
                app.clear_memory_modal();
            }
            if app.autocomplete_active() {
                app.clear_autocomplete();
            }
        }
        KeyAction::ScrollUp(n) => {
            // Priority: memory modal > autocomplete > multi-line input > message scrolling
            if app.memory_modal_active() {
                app.memory_modal_prev();
            } else if app.autocomplete_active() {
                app.autocomplete_prev();
            } else if !app.is_streaming() && app.has_multi_line_input() {
                // NEW: Move cursor up in multi-line input
                app.move_cursor_up();
            } else {
                app.scroll_up(*n);
            }
        }
        KeyAction::ScrollDown(n) => {
            // Priority: memory modal > autocomplete > multi-line input > message scrolling
            if app.memory_modal_active() {
                app.memory_modal_next();
            } else if app.autocomplete_active() {
                app.autocomplete_next();
            } else if !app.is_streaming() && app.has_multi_line_input() {
                // NEW: Move cursor down in multi-line input
                app.move_cursor_down();
            } else {
                app.scroll_down(*n);
            }
        }
        KeyAction::JumpToBottom => {
            app.scroll_to_bottom();
        }
        KeyAction::CursorStart => {
            if !app.is_streaming() {
                app.move_cursor_to_start();
            }
        }
        KeyAction::CursorEnd => {
            if !app.is_streaming() {
                app.move_cursor_to_end();
            }
        }
        KeyAction::ClearLine => {
            if !app.is_streaming() {
                app.clear_input();
            }
        }
        KeyAction::Submit => {
            if !app.is_streaming() {
                // Priority: memory modal > autocomplete > normal submit
                if app.memory_modal_active() {
                    // Save memory to selected destination
                    if let Some(modal) = app.memory_modal() {
                        if let Some(dest) = app.memory_modal_selected() {
                            let memory_text = modal.memory_text.clone();
                            let file_path = dest.file_path.clone();
                            app.clear_memory_modal();
                            app.submit_input(); // Clear the input
                            return Ok(EventResult::SaveMemory(memory_text, file_path));
                        }
                    }
                } else if app.autocomplete_active() {
                    // If autocomplete is active, select the highlighted item
                    if let Some(item) = app.autocomplete_selected() {
                        // Replace input with selected command
                        let command_text = format!("/{}", item.command);
                        app.set_input(&command_text);
                        app.move_cursor_to_end();
                        app.clear_autocomplete();
                    }
                } else {
                    // Normal submit
                    if let Some(input) = app.submit_input() {
                        return Ok(EventResult::Submit(input));
                    }
                }
            }
        }
        KeyAction::Backspace => {
            if !app.is_streaming() {
                app.backspace();
            }
        }
        KeyAction::Delete => {
            if !app.is_streaming() {
                app.delete_char();
            }
        }
        KeyAction::CursorLeft => {
            if !app.is_streaming() {
                app.move_cursor_left();
            }
        }
        KeyAction::CursorRight => {
            if !app.is_streaming() {
                app.move_cursor_right();
            }
        }
        // === NEW: Multi-line input navigation ===
        KeyAction::CursorWordLeft => {
            if !app.is_streaming() {
                app.move_cursor_word_left();
            }
        }
        KeyAction::CursorWordRight => {
            if !app.is_streaming() {
                app.move_cursor_word_right();
            }
        }
        KeyAction::CursorAbsoluteStart => {
            if !app.is_streaming() {
                app.move_cursor_absolute_start();
            }
        }
        KeyAction::CursorAbsoluteEnd => {
            if !app.is_streaming() {
                app.move_cursor_absolute_end();
            }
        }
        KeyAction::InputPageUp => {
            if !app.is_streaming() {
                app.move_cursor_to_input_top();
            }
        }
        KeyAction::InputPageDown => {
            if !app.is_streaming() {
                app.move_cursor_to_input_bottom();
            }
        }
        KeyAction::InputScrollUp => {
            if !app.is_streaming() {
                app.scroll_input_viewport_up();
            }
        }
        KeyAction::InputScrollDown => {
            if !app.is_streaming() {
                app.scroll_input_viewport_down();
            }
        }
        KeyAction::InsertNewline => {
            if !app.is_streaming() {
                app.insert_newline();
            }
        }
    }

    Ok(EventResult::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission_mode::PermissionMode;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn make_key_event(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    #[test]
    fn test_input_submission_flow() {
        let mut app = App::new(PermissionMode::default());

        // Simulate typing "hello"
        for c in "hello".chars() {
            let event = make_key_event(KeyCode::Char(c), KeyModifiers::NONE);
            let result = handle_event(&mut app, event).unwrap();
            assert_eq!(result, EventResult::Continue);
        }

        assert_eq!(app.input(), "hello");

        // Simulate Enter
        let event = make_key_event(KeyCode::Char('\r'), KeyModifiers::NONE);
        match handle_event(&mut app, Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))).unwrap() {
            EventResult::Submit(input) => {
                assert_eq!(input, "hello");
                assert_eq!(app.input(), ""); // Input cleared
            }
            _ => panic!("Expected Submit"),
        }
    }

    #[test]
    fn test_ctrl_c_exits() {
        let mut app = App::new(PermissionMode::default());
        let event = make_key_event(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let result = handle_event(&mut app, event).unwrap();
        assert_eq!(result, EventResult::Exit);
    }

    #[test]
    fn test_ctrl_d_sets_exit_flag() {
        let mut app = App::new(PermissionMode::default());
        let event = make_key_event(KeyCode::Char('d'), KeyModifiers::CONTROL);
        handle_event(&mut app, event).unwrap();
        assert!(app.should_exit());
    }

    #[test]
    fn test_backspace() {
        let mut app = App::new(PermissionMode::default());
        app.insert_char('a');
        app.insert_char('b');

        let event = make_key_event(KeyCode::Backspace, KeyModifiers::NONE);
        handle_event(&mut app, event).unwrap();

        assert_eq!(app.input(), "a");
    }

    #[test]
    fn test_cursor_movement() {
        let mut app = App::new(PermissionMode::default());
        app.insert_char('a');
        app.insert_char('b');
        app.insert_char('c');

        // Left - check column position (second element of tuple)
        let event = make_key_event(KeyCode::Left, KeyModifiers::NONE);
        handle_event(&mut app, event).unwrap();
        assert_eq!(app.cursor_pos().1, 2);

        // Right
        let event = make_key_event(KeyCode::Right, KeyModifiers::NONE);
        handle_event(&mut app, event).unwrap();
        assert_eq!(app.cursor_pos().1, 3);

        // Home
        let event = make_key_event(KeyCode::Home, KeyModifiers::NONE);
        handle_event(&mut app, event).unwrap();
        assert_eq!(app.cursor_pos().1, 0);

        // End
        let event = make_key_event(KeyCode::End, KeyModifiers::NONE);
        handle_event(&mut app, event).unwrap();
        assert_eq!(app.cursor_pos().1, 3);
    }

    #[test]
    fn test_no_input_during_streaming() {
        let mut app = App::new(PermissionMode::default());
        app.start_streaming_response();

        // Try to type
        let event = make_key_event(KeyCode::Char('a'), KeyModifiers::NONE);
        handle_event(&mut app, event).unwrap();

        // Input should still be empty
        assert_eq!(app.input(), "");
    }

    #[test]
    fn test_shift_enter_inserts_newline() {
        let mut app = App::new(PermissionMode::default());

        // Type "line1"
        for c in "line1".chars() {
            app.insert_char(c);
        }
        assert_eq!(app.input(), "line1");

        // Press Shift+Enter
        let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        let result = handle_event(&mut app, event).unwrap();

        // Should continue (not submit)
        assert_eq!(result, EventResult::Continue);

        // Type "line2"
        for c in "line2".chars() {
            app.insert_char(c);
        }

        // Input should be multi-line
        assert_eq!(app.input(), "line1\nline2");
        assert_eq!(app.input_line_count(), 2);
    }

    #[test]
    fn test_backslash_escape_newline() {
        let mut app = App::new(PermissionMode::default());

        // Type "line1\"
        for c in "line1\\".chars() {
            app.insert_char(c);
        }
        assert_eq!(app.input(), "line1\\");

        // Press Enter (without Shift)
        let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let result = handle_event(&mut app, event).unwrap();

        // Should continue (not submit) and insert newline
        assert_eq!(result, EventResult::Continue);

        // Backslash should be removed, newline inserted
        assert_eq!(app.input(), "line1\n");

        // Type "line2"
        for c in "line2".chars() {
            app.insert_char(c);
        }

        // Input should be multi-line
        assert_eq!(app.input(), "line1\nline2");
    }
}
