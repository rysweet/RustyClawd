//! Event generation utilities for testing
//!
//! Provides deterministic event sequences for keyboard input simulation

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Event generator for deterministic testing
pub struct EventGenerator;

impl EventGenerator {
    /// Create key event for regular character
    pub fn char(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    /// Create key event with modifiers
    pub fn char_with_modifiers(c: char, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), modifiers)
    }

    /// Create Ctrl+C event
    pub fn ctrl_c() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
    }

    /// Create Ctrl+D event
    pub fn ctrl_d() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)
    }

    /// Create Enter key event
    pub fn enter() -> KeyEvent {
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
    }

    /// Create Backspace key event
    pub fn backspace() -> KeyEvent {
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)
    }

    /// Create Left arrow key event
    pub fn left() -> KeyEvent {
        KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)
    }

    /// Create Right arrow key event
    pub fn right() -> KeyEvent {
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)
    }

    /// Create Up arrow key event
    pub fn up() -> KeyEvent {
        KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)
    }

    /// Create Down arrow key event
    pub fn down() -> KeyEvent {
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)
    }

    /// Create Home key event
    pub fn home() -> KeyEvent {
        KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)
    }

    /// Create End key event
    pub fn end() -> KeyEvent {
        KeyEvent::new(KeyCode::End, KeyModifiers::NONE)
    }

    /// Create PageUp key event
    pub fn page_up() -> KeyEvent {
        KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)
    }

    /// Create PageDown key event
    pub fn page_down() -> KeyEvent {
        KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)
    }

    /// Create Tab key event
    pub fn tab() -> KeyEvent {
        KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)
    }

    /// Create Escape key event
    pub fn escape() -> KeyEvent {
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
    }

    /// Create sequence of character events from string
    pub fn string(s: &str) -> Vec<KeyEvent> {
        s.chars().map(Self::char).collect()
    }

    /// Create typing sequence (string + enter)
    pub fn typing_sequence(s: &str) -> Vec<KeyEvent> {
        let mut events = Self::string(s);
        events.push(Self::enter());
        events
    }

    /// Create slash command sequence
    pub fn slash_command(cmd: &str) -> Vec<KeyEvent> {
        Self::typing_sequence(&format!("/{}", cmd))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_event() {
        let event = EventGenerator::char('a');
        assert_eq!(event.code, KeyCode::Char('a'));
        assert_eq!(event.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_char_with_modifiers() {
        let event = EventGenerator::char_with_modifiers('c', KeyModifiers::CONTROL);
        assert_eq!(event.code, KeyCode::Char('c'));
        assert_eq!(event.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_ctrl_c() {
        let event = EventGenerator::ctrl_c();
        assert_eq!(event.code, KeyCode::Char('c'));
        assert_eq!(event.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_ctrl_d() {
        let event = EventGenerator::ctrl_d();
        assert_eq!(event.code, KeyCode::Char('d'));
        assert_eq!(event.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_enter() {
        let event = EventGenerator::enter();
        assert_eq!(event.code, KeyCode::Enter);
    }

    #[test]
    fn test_backspace() {
        let event = EventGenerator::backspace();
        assert_eq!(event.code, KeyCode::Backspace);
    }

    #[test]
    fn test_arrow_keys() {
        assert_eq!(EventGenerator::left().code, KeyCode::Left);
        assert_eq!(EventGenerator::right().code, KeyCode::Right);
        assert_eq!(EventGenerator::up().code, KeyCode::Up);
        assert_eq!(EventGenerator::down().code, KeyCode::Down);
    }

    #[test]
    fn test_navigation_keys() {
        assert_eq!(EventGenerator::home().code, KeyCode::Home);
        assert_eq!(EventGenerator::end().code, KeyCode::End);
        assert_eq!(EventGenerator::page_up().code, KeyCode::PageUp);
        assert_eq!(EventGenerator::page_down().code, KeyCode::PageDown);
    }

    #[test]
    fn test_special_keys() {
        assert_eq!(EventGenerator::tab().code, KeyCode::Tab);
        assert_eq!(EventGenerator::escape().code, KeyCode::Esc);
    }

    #[test]
    fn test_string_sequence() {
        let events = EventGenerator::string("hello");
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].code, KeyCode::Char('h'));
        assert_eq!(events[1].code, KeyCode::Char('e'));
        assert_eq!(events[4].code, KeyCode::Char('o'));
    }

    #[test]
    fn test_typing_sequence() {
        let events = EventGenerator::typing_sequence("test");
        assert_eq!(events.len(), 5); // 4 chars + enter
        assert_eq!(events[0].code, KeyCode::Char('t'));
        assert_eq!(events[4].code, KeyCode::Enter);
    }

    #[test]
    fn test_slash_command() {
        let events = EventGenerator::slash_command("exit");
        assert_eq!(events.len(), 6); // / + exit + enter
        assert_eq!(events[0].code, KeyCode::Char('/'));
        assert_eq!(events[1].code, KeyCode::Char('e'));
        assert_eq!(events[5].code, KeyCode::Enter);
    }

    #[test]
    fn test_unicode_string() {
        let events = EventGenerator::string("🦀");
        assert_eq!(events.len(), 1); // Single grapheme cluster
        assert_eq!(events[0].code, KeyCode::Char('🦀'));
    }
}
