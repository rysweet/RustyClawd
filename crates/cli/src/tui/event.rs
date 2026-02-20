//! Event handling for TUI

use anyhow::Result;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers,
};
use rat_event::Outcome;
use rat_focus::{FocusBuilder, HasFocus};
use std::io;
use std::time::Duration;

use crate::tui::app::{
    App, AutocompletePopupWrapper, DebugPaneWrapper, InputPaneWrapper, MemoryModalWrapper,
    MessagesPaneWrapper, PermissionsModalWrapper,
};
use crate::tui::click_region::ClickTarget;
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

    /// Toggle message expand/collapse (message index)
    ToggleMessage { index: usize },

    /// Toggle debug pane visibility
    ToggleDebugPane,

    /// Open menu (future implementation)
    OpenMenu,

    /// Terminal resized - need to call autoresize() and clear()
    Resize,
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
    // Build focus structure BEFORE processing events (for Tab/mouse focus handling)
    // Extract data from app first to avoid borrowing conflicts
    let cache = app.layout_cache().clone();

    // Only handle focus if layout cache is initialized (non-zero area)
    // This avoids issues in tests where layout isn't set up
    if cache.messages_area.width > 0 && cache.messages_area.height > 0 {
        let focus_messages = app.focus_messages();
        let focus_input = app.focus_input();
        let focus_debug = app.focus_debug();
        let focus_autocomplete = app.focus_autocomplete();
        let focus_memory_modal = app.focus_memory_modal();
        let focus_permissions_modal = app.focus_permissions_modal();
        let autocomplete_active = app.autocomplete_active();
        let memory_modal_active = app.memory_modal_active();
        let permissions_modal_active = app.permissions_modal_active();

        let mut builder = FocusBuilder::default();

        // Add panes in z-order (bottom to top)
        let messages_wrapper = MessagesPaneWrapper {
            focus: focus_messages,
            area: cache.messages_area,
        };
        messages_wrapper.build(&mut builder);

        let input_wrapper = InputPaneWrapper {
            focus: focus_input,
            area: cache.input_area,
        };
        input_wrapper.build(&mut builder);

        if let Some(debug_area) = cache.debug_area {
            let debug_wrapper = DebugPaneWrapper {
                focus: focus_debug,
                area: debug_area,
            };
            debug_wrapper.build(&mut builder);
        }

        // Add overlays on top (highest z-order)
        if autocomplete_active {
            let autocomplete_wrapper = AutocompletePopupWrapper {
                focus: focus_autocomplete,
                // Autocomplete area is calculated in render, use input area as approximation
                area: cache.input_area,
            };
            autocomplete_wrapper.build(&mut builder);
        }

        if memory_modal_active {
            let memory_modal_wrapper = MemoryModalWrapper {
                focus: focus_memory_modal,
                // Memory modal area is calculated in render, use input area as approximation
                area: cache.input_area,
            };
            memory_modal_wrapper.build(&mut builder);
        }

        if permissions_modal_active {
            let permissions_modal_wrapper = PermissionsModalWrapper {
                focus: focus_permissions_modal,
                // Permissions modal area is calculated in render, use input area as approximation
                area: cache.input_area,
            };
            permissions_modal_wrapper.build(&mut builder);
        }

        let mut focus = builder.build();

        // Handle focus events (Tab navigation, mouse clicks) with rat-focus
        // This processes focus changes BEFORE other event handling
        // The FocusFlags will update automatically based on focus changes
        let focus_outcome = rat_focus::handle_focus(&mut focus, &event);

        // CRITICAL: Check if rat-focus consumed the event
        // If it did (Tab/mouse click for focus), don't process further
        if matches!(focus_outcome, Outcome::Changed) {
            // rat-focus handled this event (Tab or mouse click for focus)
            let msg = format!(
                "🎯 Focus changed: msg={} inp={} dbg={}",
                app.focus_messages().get(),
                app.focus_input().get(),
                app.focus_debug().get()
            );
            app.push_debug_message(msg);
            // Don't pass it to our handlers - return early
            return Ok(EventResult::Continue);
        }
    }

    // Continue with regular event processing
    // Note: The focus state is now updated and will be reflected in the next render
    // We only reach here if rat-focus didn't consume the event (Outcome::NotUsed or Outcome::Unchanged)
    match event {
        Event::Key(key) => handle_key_event(app, key),
        Event::Mouse(mouse) => handle_mouse_event(app, mouse),
        Event::Resize(_, _) => {
            // Terminal resized - signal main loop to call autoresize() and clear()
            Ok(EventResult::Resize)
        }
        _ => Ok(EventResult::Continue),
    }
}

fn handle_mouse_event(app: &mut App, mouse: event::MouseEvent) -> Result<EventResult> {
    use event::{MouseButton, MouseEventKind};

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            // Route scroll to focused panel
            // If input focused, scroll message panel (input doesn't have meaningful scroll)
            // If debug focused, scroll debug panel
            // Otherwise, scroll message panel (default)
            if app.focus_debug().get() {
                app.scroll_debug_up(3);
            } else {
                // Input or message focus -> scroll message panel
                app.scroll_up(3);
            }
        }
        MouseEventKind::ScrollDown => {
            // Route scroll to focused panel (same logic as ScrollUp)
            if app.focus_debug().get() {
                app.scroll_debug_down(3);
            } else {
                // Input or message focus -> scroll message panel
                app.scroll_down(3);
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            // Left-click: check what was clicked using hit-testing
            // hit_test() automatically translates coordinates for each panel
            app.push_debug_message(format!(
                "[CLICK] Raw mouse=({}, {})",
                mouse.column, mouse.row
            ));

            let target = app
                .click_regions
                .hit_test(mouse.column, mouse.row, app.layout_cache());

            match target {
                ClickTarget::Message { index } => {
                    // DEBUG: Log click hit and extract message data first (avoid borrow conflicts)
                    let msg_info = app
                        .messages()
                        .get(index)
                        .map(|m| (m.role, m.collapsible, m.collapsed));

                    app.push_debug_message(format!(
                        "[CLICK] Hit msg_idx={} at screen=({}, {})",
                        index, mouse.column, mouse.row
                    ));

                    if let Some((role, collapsible, collapsed)) = msg_info {
                        app.push_debug_message(format!(
                            "[CLICK] Message role={:?} collapsible={} collapsed={}",
                            role, collapsible, collapsed
                        ));
                    }

                    // Toggle message expand/collapse
                    if let Some(message) = app.messages_mut().get_mut(index) {
                        message.toggle_collapse();
                    }
                    // Return Continue - message state already updated
                }
                ClickTarget::DebugMessage { index } => {
                    // Future: Handle debug message clicks (collapsible debug entries)
                    app.push_debug_message(format!(
                        "[CLICK] Hit debug msg_idx={} (not yet implemented)",
                        index
                    ));
                }
                ClickTarget::StatusBarItem { id } => {
                    // Route status bar clicks to appropriate actions
                    match id.as_str() {
                        "debug" => return Ok(EventResult::ToggleDebugPane),
                        "menu" => return Ok(EventResult::OpenMenu),
                        _ => {} // Unknown status bar item, ignore
                    }
                }
                ClickTarget::Background => {
                    // Clicked empty space, nothing to do
                }
            }
        }
        _ => {
            // Ignore other mouse events (right-click, drags, etc.)
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

    // Block input during extended thinking (except Ctrl+C interruption)
    let is_extended_thinking = app.is_extended_thinking();
    if crate::tui::input_guard::should_block_input(is_extended_thinking, &key) {
        // Emit blocked-input debug message only once per thinking phase
        // to avoid flooding the debug panel on repeated keypresses.
        if !app.has_shown_blocked_input_message() {
            let msg = crate::tui::input_guard::get_blocked_input_message();
            app.push_debug_message(msg.to_string());
            app.set_shown_blocked_input_message(true);
        }
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
    let is_printable = bindings.is_printable_char(&key);
    let is_streaming = app.is_streaming();

    if is_printable && !is_streaming {
        if let crossterm::event::KeyCode::Char(c) = key.code {
            // Route character input to permissions modal if active and searching
            if app.permissions_modal_active() {
                if let Some(permissions) = app.permissions_modal_mut() {
                    if permissions.is_searching() {
                        permissions.handle_char_input(c);
                        app.mark_dirty();
                    } else if c == '/' {
                        // '/' activates search mode
                        permissions.enter_search_mode();
                        app.mark_dirty();
                    }
                }
            } else {
                app.insert_char(c);
            }
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
        KeyAction::ToggleMouseMode => {
            // Toggle mouse mode and update crossterm capture state
            let new_mode = !app.mouse_mode_enabled();
            app.set_mouse_mode(new_mode);

            if new_mode {
                // Enable mouse capture (clicks go to app, terminal selection blocked)
                crossterm::execute!(io::stdout(), EnableMouseCapture)?;
            } else {
                // Disable mouse capture (terminal selection works, no app clicks)
                crossterm::execute!(io::stdout(), DisableMouseCapture)?;
            }
        }
        KeyAction::CyclePermissionMode => {
            app.cycle_permission_mode();
        }
        KeyAction::ClearError => {
            // Escape key clears errors, modals, and autocomplete
            app.clear_error();
            if app.permissions_modal_active() {
                // Check if we're in search mode
                if let Some(permissions) = app.permissions_modal() {
                    if permissions.is_searching() {
                        // Exit search mode but keep modal open
                        if let Some(p) = app.permissions_modal_mut() {
                            p.exit_search_mode();
                            app.mark_dirty();
                        }
                    } else {
                        // Close modal entirely
                        app.clear_permissions_modal();
                    }
                }
            } else if app.memory_modal_active() {
                app.clear_memory_modal();
            } else if app.autocomplete_active() {
                app.clear_autocomplete();
            }
        }
        KeyAction::ScrollUp(n) => {
            // Priority: permissions modal > memory modal > autocomplete > multi-line input > message scrolling
            if app.permissions_modal_active() {
                if let Some(permissions) = app.permissions_modal_mut() {
                    permissions.select_previous();
                    app.mark_dirty();
                }
            } else if app.memory_modal_active() {
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
            // Priority: permissions modal > memory modal > autocomplete > multi-line input > message scrolling
            if app.permissions_modal_active() {
                if let Some(permissions) = app.permissions_modal_mut() {
                    permissions.select_next();
                    app.mark_dirty();
                }
            } else if app.memory_modal_active() {
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
                // Route backspace to permissions modal if active and searching
                if app.permissions_modal_active() {
                    if let Some(permissions) = app.permissions_modal_mut() {
                        if permissions.is_searching() {
                            permissions.handle_backspace();
                            app.mark_dirty();
                        }
                    }
                } else {
                    app.backspace();
                }
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
        match handle_event(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        )
        .unwrap()
        {
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
