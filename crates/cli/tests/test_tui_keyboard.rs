//! TUI Keyboard Navigation Tests
//!
//! Tests for keyboard input handling:
//! - Character input
//! - Cursor movement
//! - Text editing
//! - Special keys
//! - Keyboard shortcuts

mod helpers;
mod tui_test_harness;

use helpers::event_generator::EventGenerator;
use rustyclawd::tui::input_viewport::calculate_viewport;

#[test]
fn test_character_input() {
    // Test basic character input
    let events = EventGenerator::string("hello");

    assert_eq!(events.len(), 5);
    // Verify events can be created
    assert!(events.iter().all(|e| e.modifiers.is_empty()));
}

#[test]
fn test_backspace_handling() {
    // Test backspace key
    let event = EventGenerator::backspace();

    assert_eq!(event.code, crossterm::event::KeyCode::Backspace);
}

#[test]
fn test_enter_key() {
    // Test enter key
    let event = EventGenerator::enter();

    assert_eq!(event.code, crossterm::event::KeyCode::Enter);
}

#[test]
fn test_arrow_key_navigation() {
    // Test arrow keys
    let left = EventGenerator::left();
    let right = EventGenerator::right();
    let up = EventGenerator::up();
    let down = EventGenerator::down();

    assert_eq!(left.code, crossterm::event::KeyCode::Left);
    assert_eq!(right.code, crossterm::event::KeyCode::Right);
    assert_eq!(up.code, crossterm::event::KeyCode::Up);
    assert_eq!(down.code, crossterm::event::KeyCode::Down);
}

#[test]
fn test_home_end_keys() {
    // Test Home and End keys
    let home = EventGenerator::home();
    let end = EventGenerator::end();

    assert_eq!(home.code, crossterm::event::KeyCode::Home);
    assert_eq!(end.code, crossterm::event::KeyCode::End);
}

#[test]
fn test_page_up_down() {
    // Test PageUp and PageDown
    let page_up = EventGenerator::page_up();
    let page_down = EventGenerator::page_down();

    assert_eq!(page_up.code, crossterm::event::KeyCode::PageUp);
    assert_eq!(page_down.code, crossterm::event::KeyCode::PageDown);
}

#[test]
fn test_ctrl_c_shortcut() {
    // Test Ctrl+C
    let ctrl_c = EventGenerator::ctrl_c();

    assert_eq!(ctrl_c.code, crossterm::event::KeyCode::Char('c'));
    assert!(ctrl_c
        .modifiers
        .contains(crossterm::event::KeyModifiers::CONTROL));
}

#[test]
fn test_ctrl_d_shortcut() {
    // Test Ctrl+D
    let ctrl_d = EventGenerator::ctrl_d();

    assert_eq!(ctrl_d.code, crossterm::event::KeyCode::Char('d'));
    assert!(ctrl_d
        .modifiers
        .contains(crossterm::event::KeyModifiers::CONTROL));
}

#[test]
fn test_tab_key() {
    // Test Tab key
    let tab = EventGenerator::tab();

    assert_eq!(tab.code, crossterm::event::KeyCode::Tab);
}

#[test]
fn test_escape_key() {
    // Test Escape key
    let esc = EventGenerator::escape();

    assert_eq!(esc.code, crossterm::event::KeyCode::Esc);
}

#[test]
fn test_unicode_input() {
    // Test Unicode character input
    let events = EventGenerator::string("🦀");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].code, crossterm::event::KeyCode::Char('🦀'));
}

#[test]
fn test_typing_sequence() {
    // Test complete typing sequence
    let events = EventGenerator::typing_sequence("hello");

    assert_eq!(events.len(), 6); // 5 chars + enter
    assert_eq!(events[5].code, crossterm::event::KeyCode::Enter);
}

// Input viewport tests (cursor positioning)
#[test]
fn test_viewport_cursor_at_start() {
    // Test viewport when cursor is at start
    let text = "This is a long text that needs scrolling";
    let viewport = calculate_viewport(text, 0, 20);

    assert_eq!(viewport.viewport_cursor_pos, 0);
    assert_eq!(viewport.viewport_offset, 0);
}

#[test]
fn test_viewport_cursor_at_end() {
    // Test viewport when cursor is at end
    let text = "This is a long text that needs scrolling";
    let text_len = text.chars().count();
    let viewport = calculate_viewport(text, text_len, 20);

    assert_eq!(viewport.viewport_cursor_pos, 20);
}

#[test]
fn test_viewport_cursor_movement_left() {
    // Test cursor movement left
    let text = "hello world";
    let cursor_at_end = text.len();

    let viewport1 = calculate_viewport(text, cursor_at_end, 20);
    let viewport2 = calculate_viewport(text, cursor_at_end - 1, 20);

    assert_eq!(viewport1.viewport_cursor_pos, cursor_at_end);
    assert_eq!(viewport2.viewport_cursor_pos, cursor_at_end - 1);
}

#[test]
fn test_viewport_cursor_movement_right() {
    // Test cursor movement right
    let text = "hello world";

    let viewport1 = calculate_viewport(text, 0, 20);
    let viewport2 = calculate_viewport(text, 1, 20);

    assert_eq!(viewport1.viewport_cursor_pos, 0);
    assert_eq!(viewport2.viewport_cursor_pos, 1);
}

#[test]
fn test_viewport_with_long_text() {
    // Test viewport with text exceeding width
    let text = "This is a very long line of text that definitely exceeds the available width";
    let viewport = calculate_viewport(text, 30, 20);

    // Cursor should be visible within viewport
    assert!(viewport.viewport_cursor_pos <= 20);
    assert!(viewport.viewport_offset > 0);
}

#[test]
fn test_cursor_navigation_sequence() {
    // Test sequence of cursor movements
    let text = "hello world";
    let positions = vec![0, 5, 11, 6, 0];

    for pos in positions {
        let viewport = calculate_viewport(text, pos, 20);
        assert_eq!(viewport.viewport_cursor_pos, pos);
    }
}

#[test]
fn test_keyboard_input_with_special_chars() {
    // Test input with special characters
    let special_text = "Hello! @#$%";
    let events = EventGenerator::string(special_text);

    assert_eq!(events.len(), special_text.chars().count());
}

#[test]
fn test_slash_command_input() {
    // Test slash command input
    let events = EventGenerator::slash_command("exit");

    assert_eq!(events.len(), 6); // / + exit + enter
    assert_eq!(events[0].code, crossterm::event::KeyCode::Char('/'));
}
