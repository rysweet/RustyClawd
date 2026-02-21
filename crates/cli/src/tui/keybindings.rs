//! Configurable keybinding system for TUI controls
//!
//! Philosophy: Modular, declarative keybindings that prevent control characters
//! from bleeding into text input. All control keys are intercepted BEFORE input.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::LazyLock;

/// Action to take when a keybinding is triggered
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Variants defined for complete keybinding coverage; some not yet bound
pub enum KeyAction {
    /// Exit the application
    Exit,
    /// Toggle debug panel
    ToggleDebug,
    /// Toggle mouse mode (enables/disables mouse capture for terminal text selection)
    ToggleMouseMode,
    /// Cycle permission mode
    CyclePermissionMode,
    /// Clear error message
    ClearError,
    /// Scroll up by N lines
    ScrollUp(usize),
    /// Scroll down by N lines
    ScrollDown(usize),
    /// Jump to bottom (force follow mode)
    JumpToBottom,
    /// Move cursor to start of line
    CursorStart,
    /// Move cursor to end of line
    CursorEnd,
    /// Clear current input line
    ClearLine,
    /// Submit current input
    Submit,
    /// Backspace
    Backspace,
    /// Delete character
    Delete,
    /// Move cursor left
    CursorLeft,
    /// Move cursor right
    CursorRight,

    // === Multi-line input navigation ===
    /// Move cursor left by word
    CursorWordLeft,
    /// Move cursor right by word
    CursorWordRight,
    /// Jump to absolute start of all input text (Ctrl+Home)
    CursorAbsoluteStart,
    /// Jump to absolute end of all input text (Ctrl+End)
    CursorAbsoluteEnd,
    /// Jump to top of input area (PageUp)
    InputPageUp,
    /// Jump to bottom of input area (PageDown)
    InputPageDown,
    /// Scroll input viewport up (Ctrl+Up when > 5 lines)
    InputScrollUp,
    /// Scroll input viewport down (Ctrl+Down when > 5 lines)
    InputScrollDown,
    /// Insert newline (Shift+Enter)
    InsertNewline,
}

/// A keybinding maps key events to actions
#[derive(Debug, Clone)]
#[allow(dead_code)] // description used for help display
pub struct KeyBinding {
    pub key: KeyPattern,
    pub action: KeyAction,
    pub description: &'static str,
}

/// Pattern for matching key events
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPattern {
    pub code: KeyCodePattern,
    pub modifiers: KeyModifiers,
}

/// Pattern for matching key codes
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Complete key code coverage; some not yet used in bindings
pub enum KeyCodePattern {
    Char(char),
    F(u8),
    Enter,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Esc,
}

impl KeyPattern {
    /// Check if this pattern matches a key event
    pub fn matches(&self, event: &KeyEvent) -> bool {
        self.modifiers == event.modifiers && self.code.matches(&event.code)
    }

    /// Convenience constructor for Ctrl+<char>
    pub fn ctrl_char(c: char) -> Self {
        Self {
            code: KeyCodePattern::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }
    }

    /// Convenience constructor for plain key
    pub fn plain(code: KeyCodePattern) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    /// Convenience constructor for Ctrl+<key>
    pub fn ctrl(code: KeyCodePattern) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    /// Convenience constructor for Shift+<key>
    pub fn shift(code: KeyCodePattern) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::SHIFT,
        }
    }

    /// Convenience constructor for F-key
    pub fn f_key(n: u8) -> Self {
        Self {
            code: KeyCodePattern::F(n),
            modifiers: KeyModifiers::NONE,
        }
    }
}

impl KeyCodePattern {
    /// Check if this pattern matches a key code
    pub fn matches(&self, code: &KeyCode) -> bool {
        match (self, code) {
            (KeyCodePattern::Char(a), KeyCode::Char(b)) => a == b,
            (KeyCodePattern::F(a), KeyCode::F(b)) => a == b,
            (KeyCodePattern::Enter, KeyCode::Enter) => true,
            (KeyCodePattern::Backspace, KeyCode::Backspace) => true,
            (KeyCodePattern::Delete, KeyCode::Delete) => true,
            (KeyCodePattern::Left, KeyCode::Left) => true,
            (KeyCodePattern::Right, KeyCode::Right) => true,
            (KeyCodePattern::Up, KeyCode::Up) => true,
            (KeyCodePattern::Down, KeyCode::Down) => true,
            (KeyCodePattern::Home, KeyCode::Home) => true,
            (KeyCodePattern::End, KeyCode::End) => true,
            (KeyCodePattern::PageUp, KeyCode::PageUp) => true,
            (KeyCodePattern::PageDown, KeyCode::PageDown) => true,
            (KeyCodePattern::Tab, KeyCode::Tab) => true,
            (KeyCodePattern::BackTab, KeyCode::BackTab) => true,
            (KeyCodePattern::Esc, KeyCode::Esc) => true,
            _ => false,
        }
    }
}

/// Keybinding configuration
#[derive(Debug, Clone)]
pub struct KeyBindings {
    bindings: Vec<KeyBinding>,
}

/// Static default keybindings instance. Created once, reused on every keypress.
static DEFAULT_BINDINGS: LazyLock<KeyBindings> = LazyLock::new(|| {
    let bindings = vec![
        // Exit
        KeyBinding {
            key: KeyPattern::ctrl_char('c'),
            action: KeyAction::Exit,
            description: "Exit application",
        },
        KeyBinding {
            key: KeyPattern::ctrl_char('d'),
            action: KeyAction::Exit,
            description: "Exit application",
        },
        // Debug panel
        KeyBinding {
            key: KeyPattern::f_key(1),
            action: KeyAction::ToggleDebug,
            description: "Toggle debug panel",
        },
        // Mouse mode
        KeyBinding {
            key: KeyPattern::f_key(2),
            action: KeyAction::ToggleMouseMode,
            description: "Toggle mouse mode (F2)",
        },
        // Permission mode
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::BackTab),
            action: KeyAction::CyclePermissionMode,
            description: "Cycle permission mode (Shift+Tab)",
        },
        // Error handling
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::Esc),
            action: KeyAction::ClearError,
            description: "Clear error message",
        },
        // Scrolling
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::Up),
            action: KeyAction::ScrollUp(1),
            description: "Scroll up 1 line",
        },
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::Down),
            action: KeyAction::ScrollDown(1),
            description: "Scroll down 1 line",
        },
        // PageUp/PageDown: Jump to input top/bottom (was scroll, now input navigation)
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::PageUp),
            action: KeyAction::InputPageUp,
            description: "Jump to top of input",
        },
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::PageDown),
            action: KeyAction::InputPageDown,
            description: "Jump to bottom of input",
        },
        // Cursor movement (Emacs-style)
        KeyBinding {
            key: KeyPattern::ctrl_char('a'),
            action: KeyAction::CursorStart,
            description: "Move cursor to start",
        },
        KeyBinding {
            key: KeyPattern::ctrl_char('e'),
            action: KeyAction::CursorEnd,
            description: "Move cursor to end",
        },
        // Line editing
        KeyBinding {
            key: KeyPattern::ctrl_char('u'),
            action: KeyAction::ClearLine,
            description: "Clear line",
        },
        // Newline insertion (MUST come before plain Enter to match first)
        // Note: Many terminals strip SHIFT from Enter, so we provide Alt+Enter as fallback
        KeyBinding {
            key: KeyPattern::shift(KeyCodePattern::Enter),
            action: KeyAction::InsertNewline,
            description: "Insert newline (Shift+Enter)",
        },
        KeyBinding {
            key: KeyPattern {
                code: KeyCodePattern::Enter,
                modifiers: KeyModifiers::ALT,
            },
            action: KeyAction::InsertNewline,
            description: "Insert newline (Alt+Enter)",
        },
        KeyBinding {
            key: KeyPattern::ctrl_char('j'),
            action: KeyAction::InsertNewline,
            description: "Insert newline (Ctrl+J)",
        },
        // Input submission
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::Enter),
            action: KeyAction::Submit,
            description: "Submit input",
        },
        // Basic editing
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::Backspace),
            action: KeyAction::Backspace,
            description: "Delete previous character",
        },
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::Delete),
            action: KeyAction::Delete,
            description: "Delete current character",
        },
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::Left),
            action: KeyAction::CursorLeft,
            description: "Move cursor left",
        },
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::Right),
            action: KeyAction::CursorRight,
            description: "Move cursor right",
        },
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::Home),
            action: KeyAction::CursorStart,
            description: "Move cursor to start",
        },
        KeyBinding {
            key: KeyPattern::plain(KeyCodePattern::End),
            action: KeyAction::CursorEnd,
            description: "Move cursor to end",
        },
        // === Multi-line input navigation ===
        // Word navigation
        KeyBinding {
            key: KeyPattern::ctrl(KeyCodePattern::Left),
            action: KeyAction::CursorWordLeft,
            description: "Move cursor left by word",
        },
        KeyBinding {
            key: KeyPattern::ctrl(KeyCodePattern::Right),
            action: KeyAction::CursorWordRight,
            description: "Move cursor right by word",
        },
        // Absolute navigation
        KeyBinding {
            key: KeyPattern::ctrl(KeyCodePattern::Home),
            action: KeyAction::CursorAbsoluteStart,
            description: "Jump to start of all input",
        },
        KeyBinding {
            key: KeyPattern::ctrl(KeyCodePattern::End),
            action: KeyAction::CursorAbsoluteEnd,
            description: "Jump to end of all input",
        },
        // Viewport scrolling (when input > 5 lines)
        KeyBinding {
            key: KeyPattern::ctrl(KeyCodePattern::Up),
            action: KeyAction::InputScrollUp,
            description: "Scroll input viewport up",
        },
        KeyBinding {
            key: KeyPattern::ctrl(KeyCodePattern::Down),
            action: KeyAction::InputScrollDown,
            description: "Scroll input viewport down",
        },
    ];

    KeyBindings { bindings }
});

impl KeyBindings {
    /// Get the static default keybindings (zero allocation after first call)
    pub fn defaults() -> &'static KeyBindings {
        &DEFAULT_BINDINGS
    }

    /// Find action for a key event
    /// Returns None if no binding matches (key should be inserted as text)
    pub fn find_action(&self, event: &KeyEvent) -> Option<&KeyAction> {
        self.bindings
            .iter()
            .find(|b| b.key.matches(event))
            .map(|b| &b.action)
    }

    /// Check if a key event is a control key (should NOT be inserted as text)
    #[allow(dead_code)] // Utility for input handling; will be used when text input is refined
    pub fn is_control_key(&self, event: &KeyEvent) -> bool {
        // Any key with modifiers (except Shift for uppercase) is a control key
        if event.modifiers.contains(KeyModifiers::CONTROL)
            || event.modifiers.contains(KeyModifiers::ALT)
        {
            return true;
        }

        // Special keys are control keys
        matches!(
            event.code,
            KeyCode::F(_)
                | KeyCode::Enter
                | KeyCode::Backspace
                | KeyCode::Delete
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Tab
                | KeyCode::BackTab
                | KeyCode::Esc
        )
    }

    /// Check if a character should be inserted as text
    pub fn is_printable_char(&self, event: &KeyEvent) -> bool {
        matches!(event.code, KeyCode::Char(c) if !c.is_control())
            && !event.modifiers.contains(KeyModifiers::CONTROL)
            && !event.modifiers.contains(KeyModifiers::ALT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctrl_c_matches() {
        let bindings = KeyBindings::defaults();
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action = bindings.find_action(&event);
        assert!(matches!(action, Some(KeyAction::Exit)));
    }

    #[test]
    fn test_f1_matches() {
        let bindings = KeyBindings::defaults();
        let event = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        let action = bindings.find_action(&event);
        assert!(matches!(action, Some(KeyAction::ToggleDebug)));
    }

    #[test]
    fn test_printable_char() {
        let bindings = KeyBindings::defaults();
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(bindings.is_printable_char(&event));
    }

    #[test]
    fn test_control_char_not_printable() {
        let bindings = KeyBindings::defaults();
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert!(!bindings.is_printable_char(&event));
    }

    #[test]
    fn test_f_key_is_control() {
        let bindings = KeyBindings::defaults();
        let event = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert!(bindings.is_control_key(&event));
    }
}
