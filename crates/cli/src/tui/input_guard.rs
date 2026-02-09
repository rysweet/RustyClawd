//! Input Guard for Extended Thinking
//!
//! Prevents input during extended thinking phases while allowing Ctrl+C.
//!
//! ## Architecture (Brick Design)
//!
//! Self-contained module for input filtering:
//! - Blocks input during thinking
//! - Allows interruption (Ctrl+C)
//! - Clear visual feedback
//!
//! ## Public API ("Studs")
//!
//! - `should_block_input()`: Check if input should be blocked

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Check if input should be blocked during extended thinking
///
/// Returns `true` if the input should be blocked, `false` if it should be allowed.
///
/// # Blocking Rules
///
/// - Block all input when `is_thinking` is true
/// - EXCEPT: Always allow Ctrl+C for interruption
/// - EXCEPT: Always allow Ctrl+D for exit
///
/// # Arguments
///
/// * `is_thinking` - Whether currently in extended thinking phase
/// * `key_event` - The key event to check
///
/// # Examples
///
/// ```rust
/// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
///
/// let thinking = true;
///
/// // Block regular keys during thinking
/// let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
/// assert!(should_block_input(thinking, &key));
///
/// // Allow Ctrl+C (interruption)
/// let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
/// assert!(!should_block_input(thinking, &key));
///
/// // Allow Ctrl+D (exit)
/// let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
/// assert!(!should_block_input(thinking, &key));
/// ```
pub fn should_block_input(is_thinking: bool, key_event: &KeyEvent) -> bool {
    if !is_thinking {
        // Not thinking - allow all input
        return false;
    }

    // During thinking, only allow Ctrl+C and Ctrl+D
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => false, // Allow Ctrl+C
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => false, // Allow Ctrl+D
        _ => true, // Block everything else
    }
}

/// Get blocked input message to display to user
///
/// Returns a helpful message explaining why input is blocked and how to interrupt.
pub fn get_blocked_input_message() -> &'static str {
    "⚠️  Input blocked during extended thinking (Ctrl+C to interrupt)"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_input_when_not_thinking() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(!should_block_input(false, &key));
    }

    #[test]
    fn test_block_regular_keys_when_thinking() {
        let thinking = true;

        // Block regular character
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert!(should_block_input(thinking, &key));

        // Block Enter
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert!(should_block_input(thinking, &key));

        // Block arrow keys
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert!(should_block_input(thinking, &key));
    }

    #[test]
    fn test_allow_ctrl_c_when_thinking() {
        let thinking = true;
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(!should_block_input(thinking, &key));
    }

    #[test]
    fn test_allow_ctrl_d_when_thinking() {
        let thinking = true;
        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert!(!should_block_input(thinking, &key));
    }

    #[test]
    fn test_block_other_ctrl_keys_when_thinking() {
        let thinking = true;

        // Block Ctrl+A
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert!(should_block_input(thinking, &key));

        // Block Ctrl+Z
        let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert!(should_block_input(thinking, &key));
    }

    #[test]
    fn test_blocked_input_message() {
        let msg = get_blocked_input_message();
        assert!(msg.contains("blocked"));
        assert!(msg.contains("Ctrl+C"));
    }
}
