//! Message types for TUI display

use chrono::{DateTime, Local};

/// A message in the conversation
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Local>,
    pub streaming: bool,
    pub status: MessageStatus, // Completion status for rendering indicators

    // Expand/collapse state for system messages and tool calls
    pub collapsible: bool,         // Can this message be collapsed?
    pub collapsed: bool,           // Is it currently collapsed?
    pub collapsed_preview: String, // Text shown when collapsed

    // UI visibility (false = visible, true = hidden)
    // Used to hide slash command injected prompts from UI while keeping them in conversation
    pub hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

/// Message completion status for rendering status indicators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStatus {
    Streaming, // Message is currently being generated
    Complete,  // Message generation complete
    Error,     // Message generation failed
}

impl Message {
    /// Create a standard (non-collapsible) user message
    pub fn user(content: String) -> Self {
        Self {
            role: Role::User,
            content,
            timestamp: Local::now(),
            streaming: false,
            status: MessageStatus::Complete, // User messages start complete
            collapsible: false,
            collapsed: false,
            collapsed_preview: String::new(),
            hidden: false,
        }
    }

    /// Create a hidden user message (for slash command injected prompts)
    /// These are kept in conversation history but not displayed in the UI
    pub fn user_hidden(content: String) -> Self {
        Self {
            role: Role::User,
            content,
            timestamp: Local::now(),
            streaming: false,
            status: MessageStatus::Complete,
            collapsible: false,
            collapsed: false,
            collapsed_preview: String::new(),
            hidden: true,
        }
    }

    /// Create a standard (non-collapsible) assistant message
    pub fn assistant(content: String) -> Self {
        Self {
            role: Role::Assistant,
            content,
            timestamp: Local::now(),
            streaming: false,
            status: MessageStatus::Complete, // Non-streaming messages start complete
            collapsible: false,
            collapsed: false,
            collapsed_preview: String::new(),
            hidden: false,
        }
    }

    /// Create a partial (streaming) assistant message
    pub fn assistant_partial(content: String) -> Self {
        Self {
            role: Role::Assistant,
            content,
            timestamp: Local::now(),
            streaming: true,
            status: MessageStatus::Streaming, // Streaming messages start in Streaming status
            collapsible: false,
            collapsed: false,
            collapsed_preview: String::new(),
            hidden: false,
        }
    }

    /// Create a collapsible system message (starts collapsed)
    pub fn system(content: String) -> Self {
        let preview = if content.len() > 60 {
            format!("{}...", &content[..60])
        } else {
            content.clone()
        };

        Self {
            role: Role::System,
            content,
            timestamp: Local::now(),
            streaming: false,
            status: MessageStatus::Complete, // System messages start complete
            collapsible: true,
            collapsed: true,
            collapsed_preview: preview,
            hidden: false,
        }
    }

    /// Create a collapsible message with custom preview
    pub fn collapsible(role: Role, content: String, preview: String) -> Self {
        Self {
            role,
            content,
            timestamp: Local::now(),
            streaming: false,
            status: MessageStatus::Complete, // Collapsible messages start complete
            collapsible: true,
            collapsed: true,
            collapsed_preview: preview,
            hidden: false,
        }
    }

    /// Toggle collapse state (only works if collapsible)
    pub fn toggle_collapse(&mut self) {
        if self.collapsible {
            self.collapsed = !self.collapsed;
        }
    }

    /// Mark message as streaming complete
    pub fn complete_streaming(&mut self) {
        self.status = MessageStatus::Complete;
        self.streaming = false;
    }

    /// Mark message as error
    pub fn mark_error(&mut self) {
        self.status = MessageStatus::Error;
        self.streaming = false;
    }

    /// Check if this is a tool message (system message that's collapsible)
    pub fn is_tool_message(&self) -> bool {
        self.role == Role::System && self.collapsible
    }

    /// Get display content based on collapse state
    pub fn display_content(&self) -> &str {
        if self.collapsible && self.collapsed {
            &self.collapsed_preview
        } else {
            &self.content
        }
    }

    /// Format message header for display (with timestamp and role prefix)
    pub fn format_header(&self) -> String {
        let time = self.timestamp.format("%H:%M:%S");
        let role = match self.role {
            Role::User => "You",
            Role::Assistant => "Assistant",
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
                    line.len().div_ceil(width)
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
