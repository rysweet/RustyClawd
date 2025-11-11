//! Conversation context management
//!
//! The Context struct holds the conversation state including messages,
//! available tools, and metadata. It implements memory windowing to
//! prevent unbounded growth (a key improvement over the JavaScript version).

use crate::message::Message;
use serde::{Deserialize, Serialize};

/// Maximum number of messages to keep in memory
/// This prevents the unbounded growth issue found in the JS version
const MAX_MESSAGES: usize = 1000;

/// Number of messages to remove when limit is reached
const PRUNE_COUNT: usize = 100;

/// Conversation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Conversation messages with automatic windowing
    messages: Vec<Message>,

    /// Metadata for the context
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
}

impl Context {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            metadata: None,
        }
    }

    /// Add a message to the context
    ///
    /// Automatically prunes old messages if the limit is exceeded.
    /// This implements memory windowing missing from the JS version.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);

        // Implement memory windowing
        if self.messages.len() > MAX_MESSAGES {
            tracing::warn!(
                "Context exceeded {} messages, pruning oldest {}",
                MAX_MESSAGES,
                PRUNE_COUNT
            );
            self.messages.drain(0..PRUNE_COUNT);
        }
    }

    /// Get all messages in the context
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Fork the context (clone for agent isolation)
    pub fn fork(&self) -> Self {
        self.clone()
    }

    /// Prepend a system message (used by agents)
    pub fn prepend_system_message(&mut self, content: impl Into<String>) {
        let system_msg = Message::system(content);
        self.messages.insert(0, system_msg);
    }

    /// Estimate total memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.messages.iter()
            .map(|m| m.estimated_size())
            .sum::<usize>()
    }

    /// Get number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = Context::new();
        assert_eq!(ctx.message_count(), 0);
        assert_eq!(ctx.memory_usage(), 0);
    }

    #[test]
    fn test_add_message() {
        let mut ctx = Context::new();
        ctx.add_message(Message::user("Hello"));
        assert_eq!(ctx.message_count(), 1);
    }

    #[test]
    fn test_memory_windowing() {
        let mut ctx = Context::new();

        // Add more than MAX_MESSAGES
        for i in 0..1050 {
            ctx.add_message(Message::user(format!("Message {}", i)));
        }

        // Should have pruned (1050 messages, pruned PRUNE_COUNT when > MAX_MESSAGES)
        // After 1001st message: prune 100, leaving 901, then add remaining 49
        assert_eq!(ctx.message_count(), 950); // MAX_MESSAGES - PRUNE_COUNT + remaining

        // Verify we didn't hit the limit again
        assert!(ctx.message_count() <= MAX_MESSAGES);
    }

    #[test]
    fn test_context_fork() {
        let mut ctx = Context::new();
        ctx.add_message(Message::user("Original"));

        let mut forked = ctx.fork();
        forked.add_message(Message::assistant("Forked response"));

        // Original unchanged
        assert_eq!(ctx.message_count(), 1);
        // Fork has new message
        assert_eq!(forked.message_count(), 2);
    }

    #[test]
    fn test_prepend_system_message() {
        let mut ctx = Context::new();
        ctx.add_message(Message::user("Hello"));
        ctx.prepend_system_message("You are helpful");

        assert_eq!(ctx.message_count(), 2);
        assert_eq!(ctx.messages()[0].content, "You are helpful");
    }
}
