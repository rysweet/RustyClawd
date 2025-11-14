//! Session State Management
//!
//! Provides session tracking for command history, statistics, and runtime state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Session statistics tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionStats {
    /// Total number of messages exchanged
    pub message_count: u64,
    /// Total number of user messages
    pub user_message_count: u64,
    /// Total number of assistant messages
    pub assistant_message_count: u64,
    /// Total tokens used (input + output)
    pub total_tokens: u64,
    /// Input tokens used
    pub input_tokens: u64,
    /// Output tokens used
    pub output_tokens: u64,
    /// Number of commands executed
    pub commands_executed: u64,
    /// Number of tool calls made
    pub tool_calls: u64,
    /// Session start time
    pub session_start: DateTime<Utc>,
    /// Session duration in seconds
    pub duration_seconds: u64,
    /// Current model being used
    pub model: String,
}

impl SessionStats {
    /// Create new session statistics
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            message_count: 0,
            user_message_count: 0,
            assistant_message_count: 0,
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            commands_executed: 0,
            tool_calls: 0,
            session_start: Utc::now(),
            duration_seconds: 0,
            model: model.into(),
        }
    }

    /// Add a user message to statistics
    pub fn add_user_message(&mut self, tokens: u64) {
        self.message_count += 1;
        self.user_message_count += 1;
        self.input_tokens += tokens;
        self.total_tokens += tokens;
    }

    /// Add an assistant message to statistics
    pub fn add_assistant_message(&mut self, input_tokens: u64, output_tokens: u64) {
        self.message_count += 1;
        self.assistant_message_count += 1;
        self.input_tokens += input_tokens;
        self.output_tokens += output_tokens;
        self.total_tokens += input_tokens + output_tokens;
    }

    /// Record a command execution
    pub fn add_command(&mut self) {
        self.commands_executed += 1;
    }

    /// Record a tool call
    pub fn add_tool_call(&mut self) {
        self.tool_calls += 1;
    }

    /// Update session duration based on current time
    pub fn update_duration(&mut self) {
        let now = Utc::now();
        self.duration_seconds = (now - self.session_start).num_seconds() as u64;
    }

    /// Set the current model
    pub fn set_model(&mut self, model: impl Into<String>) {
        self.model = model.into();
    }
}

/// Command history entry
#[derive(Debug, Clone)]
pub struct CommandHistoryEntry {
    /// Command that was executed
    pub command: String,
    /// Timestamp of execution
    pub timestamp: DateTime<Utc>,
    /// Whether execution was successful
    pub success: bool,
}

/// Enhanced session state with statistics and history
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Working directory
    pub cwd: String,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Command history
    pub command_history: Vec<CommandHistoryEntry>,
    /// Session statistics
    pub stats: SessionStats,
    /// Active contexts
    pub active_contexts: Vec<String>,
}

impl SessionState {
    /// Create a new session state
    pub fn new(cwd: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            cwd: cwd.into(),
            env: HashMap::new(),
            command_history: Vec::new(),
            stats: SessionStats::new(model),
            active_contexts: Vec::new(),
        }
    }

    /// Add a command to history
    pub fn add_command(&mut self, command: impl Into<String>, success: bool) {
        let entry = CommandHistoryEntry {
            command: command.into(),
            timestamp: Utc::now(),
            success,
        };
        self.command_history.push(entry);
        self.stats.add_command();
    }

    /// Get recent commands (most recent first)
    pub fn get_recent_commands(&self, count: usize) -> Vec<&CommandHistoryEntry> {
        self.command_history.iter().rev().take(count).collect()
    }

    /// Get command history (all commands)
    pub fn get_history(&self) -> &[CommandHistoryEntry] {
        &self.command_history
    }

    /// Get session statistics
    pub fn get_stats(&self) -> &SessionStats {
        &self.stats
    }

    /// Get current model
    pub fn get_model(&self) -> &str {
        &self.stats.model
    }

    /// Set environment variable
    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env.insert(key.into(), value.into());
    }

    /// Get environment variable
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_stats_initialization() {
        let stats = SessionStats::new("claude-sonnet-4-5");

        assert_eq!(stats.message_count, 0);
        assert_eq!(stats.user_message_count, 0);
        assert_eq!(stats.assistant_message_count, 0);
        assert_eq!(stats.total_tokens, 0);
        assert_eq!(stats.model, "claude-sonnet-4-5");
    }

    #[test]
    fn test_add_user_message() {
        let mut stats = SessionStats::new("claude-sonnet-4");
        stats.add_user_message(100);

        assert_eq!(stats.message_count, 1);
        assert_eq!(stats.user_message_count, 1);
        assert_eq!(stats.input_tokens, 100);
        assert_eq!(stats.total_tokens, 100);
    }

    #[test]
    fn test_add_assistant_message() {
        let mut stats = SessionStats::new("claude-sonnet-4");
        stats.add_assistant_message(1000, 500);

        assert_eq!(stats.message_count, 1);
        assert_eq!(stats.assistant_message_count, 1);
        assert_eq!(stats.input_tokens, 1000);
        assert_eq!(stats.output_tokens, 500);
        assert_eq!(stats.total_tokens, 1500);
    }

    #[test]
    fn test_session_state_creation() {
        let state = SessionState::new("/home/user/project", "claude-sonnet-4");

        assert_eq!(state.cwd, "/home/user/project");
        assert_eq!(state.stats.model, "claude-sonnet-4");
        assert!(state.command_history.is_empty());
    }

    #[test]
    fn test_command_history() {
        let mut state = SessionState::new("/home/user", "claude-sonnet-4");

        state.add_command("/help", true);
        state.add_command("/stats", true);

        assert_eq!(state.command_history.len(), 2);
        assert_eq!(state.stats.commands_executed, 2);
    }

    #[test]
    fn test_get_recent_commands() {
        let mut state = SessionState::new("/home/user", "claude-sonnet-4");

        state.add_command("/first", true);
        state.add_command("/second", true);
        state.add_command("/third", true);

        let recent = state.get_recent_commands(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].command, "/third");
        assert_eq!(recent[1].command, "/second");
    }
}
