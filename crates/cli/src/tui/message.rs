//! Message types for TUI display

use chrono::{DateTime, Local};

/// A message in the conversation
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Local>,
    pub streaming: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

impl Message {
    pub fn user(content: String) -> Self {
        Self {
            role: Role::User,
            content,
            timestamp: Local::now(),
            streaming: false,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: Role::Assistant,
            content,
            timestamp: Local::now(),
            streaming: false,
        }
    }

    pub fn assistant_partial(content: String) -> Self {
        Self {
            role: Role::Assistant,
            content,
            timestamp: Local::now(),
            streaming: true,
        }
    }

    pub fn system(content: String) -> Self {
        Self {
            role: Role::System,
            content,
            timestamp: Local::now(),
            streaming: false,
        }
    }

    /// Format message header for display (with timestamp and role prefix)
    pub fn format_header(&self) -> String {
        let time = self.timestamp.format("%H:%M:%S");
        let role = match self.role {
            Role::User => "You",
            Role::Assistant => "Claude",
            Role::System => "System",
        };
        format!("[{}] {}", time, role)
    }

    /// Get number of display lines needed (accounting for wrapping)
    pub fn display_lines(&self, width: usize) -> usize {
        if width == 0 {
            return 1;
        }

        let header_lines = 1;
        let content_lines = self
            .content
            .lines()
            .map(|line| {
                if line.is_empty() {
                    1
                } else {
                    (line.len() + width - 1) / width
                }
            })
            .sum::<usize>();
        header_lines + content_lines + 1 // +1 for separator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello".to_string());
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");
        assert!(!msg.streaming);
    }

    #[test]
    fn test_streaming_message() {
        let msg = Message::assistant_partial("Partial".to_string());
        assert_eq!(msg.role, Role::Assistant);
        assert!(msg.streaming);
    }

    #[test]
    fn test_display_lines() {
        let msg = Message::user("Hello world this is a long message".to_string());
        // With width 10: "Hello worl" + "d this is " + "a long mes" + "sage" = 4 lines
        // + 1 header + 1 separator = 6 lines
        assert_eq!(msg.display_lines(10), 6);
    }

    #[test]
    fn test_display_lines_multiline() {
        let msg = Message::user("Line 1\nLine 2\nLine 3".to_string());
        // 3 content lines + 1 header + 1 separator = 5
        assert_eq!(msg.display_lines(80), 5);
    }
}
