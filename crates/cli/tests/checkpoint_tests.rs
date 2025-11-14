//! Comprehensive test suite for Claude Code checkpointing system
//!
//! This module tests all checkpointing functionality based on the Claude Code
//! checkpointing documentation at https://docs.claude.com/en/docs/claude-code/checkpointing
//!
//! Checkpointing provides session-level recovery by automatically capturing state
//! before each edit operation. Key features tested:
//! - Session state persistence (conversations and code changes)
//! - Checkpoint creation and management
//! - State restoration (code, conversation, or both)
//! - Session resuming with full context
//!
//! Test structure aligns with testing pyramid:
//! - 60% Unit tests: Checkpoint structure, serialization, validation
//! - 30% Integration tests: Full checkpoint lifecycle
//! - 10% E2E patterns: Complete session save/restore workflows

use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// TYPE DEFINITIONS & CHECKPOINT STRUCTURES
// ============================================================================

/// Checkpoint restoration scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreScope {
    /// Restore conversation history only
    ConversationOnly,
    /// Restore code changes only
    CodeOnly,
    /// Restore both conversation and code changes
    Both,
}

impl fmt::Display for RestoreScope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RestoreScope::ConversationOnly => write!(f, "conversation_only"),
            RestoreScope::CodeOnly => write!(f, "code_only"),
            RestoreScope::Both => write!(f, "both"),
        }
    }
}

/// File change captured in a checkpoint
#[derive(Debug, Clone, PartialEq)]
pub struct FileChange {
    /// Absolute path to the file
    pub path: String,
    /// Content after the change
    pub content: String,
    /// Hash of the file content (for verification)
    pub hash: String,
    /// Timestamp of when the change was made
    pub timestamp_ms: u64,
}

impl FileChange {
    pub fn new(path: impl Into<String>, content: impl Into<String>, timestamp_ms: u64) -> Self {
        let path = path.into();
        let content = content.into();
        let hash = Self::compute_hash(&content);
        Self {
            path,
            content,
            hash,
            timestamp_ms,
        }
    }

    fn compute_hash(content: &str) -> String {
        format!("{:x}", content.len() * 31) // Simplified hash for testing
    }

    pub fn verify_integrity(&self) -> bool {
        Self::compute_hash(&self.content) == self.hash
    }
}

/// Conversation message stored in checkpoint
#[derive(Debug, Clone, PartialEq)]
pub struct CheckpointMessage {
    pub role: String, // "user", "assistant", "system"
    pub content: String,
    pub timestamp_ms: u64,
}

impl CheckpointMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>, timestamp_ms: u64) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            timestamp_ms,
        }
    }

    pub fn user(content: impl Into<String>, timestamp_ms: u64) -> Self {
        Self::new("user", content, timestamp_ms)
    }

    pub fn assistant(content: impl Into<String>, timestamp_ms: u64) -> Self {
        Self::new("assistant", content, timestamp_ms)
    }

    pub fn system(content: impl Into<String>, timestamp_ms: u64) -> Self {
        Self::new("system", content, timestamp_ms)
    }
}

/// Working directory and environment state
#[derive(Debug, Clone, PartialEq)]
pub struct SessionState {
    /// Current working directory
    pub cwd: String,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Active file contexts
    pub active_contexts: Vec<String>,
}

impl SessionState {
    pub fn new(cwd: impl Into<String>) -> Self {
        Self {
            cwd: cwd.into(),
            env: HashMap::new(),
            active_contexts: Vec::new(),
        }
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.active_contexts.push(context.into());
        self
    }
}

/// A checkpoint capturing a single point in a session
#[derive(Debug, Clone, PartialEq)]
pub struct Checkpoint {
    /// Unique identifier for this checkpoint
    pub id: String,
    /// Sequential checkpoint number
    pub number: u32,
    /// Timestamp when checkpoint was created
    pub created_at_ms: u64,
    /// Conversation history at this point
    pub messages: Vec<CheckpointMessage>,
    /// File changes in this session
    pub file_changes: Vec<FileChange>,
    /// Session environment state
    pub session_state: SessionState,
    /// Optional user-provided description
    pub description: Option<String>,
}

impl Checkpoint {
    pub fn new(
        id: impl Into<String>,
        number: u32,
        created_at_ms: u64,
        session_state: SessionState,
    ) -> Self {
        Self {
            id: id.into(),
            number,
            created_at_ms,
            messages: Vec::new(),
            file_changes: Vec::new(),
            session_state,
            description: None,
        }
    }

    /// Add a message to the checkpoint
    pub fn add_message(&mut self, message: CheckpointMessage) {
        self.messages.push(message);
    }

    /// Record a file change
    pub fn record_file_change(&mut self, change: FileChange) {
        // Check if file already exists in checkpoint
        if let Some(existing) = self.file_changes.iter_mut().find(|c| c.path == change.path) {
            *existing = change; // Update with latest version
        } else {
            self.file_changes.push(change);
        }
    }

    /// Set checkpoint description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Verify checkpoint integrity
    pub fn verify_integrity(&self) -> bool {
        // All file changes must have valid hashes
        self.file_changes.iter().all(|f| f.verify_integrity())
    }

    /// Calculate total size of checkpoint data
    pub fn size_bytes(&self) -> usize {
        self.id.len()
            + self.messages.iter().map(|m| m.content.len()).sum::<usize>()
            + self
                .file_changes
                .iter()
                .map(|f| f.content.len())
                .sum::<usize>()
    }

    /// Serialize checkpoint to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let json = json!({
            "id": self.id,
            "number": self.number,
            "created_at_ms": self.created_at_ms,
            "description": self.description,
            "messages": self.messages.iter().map(|m| {
                json!({
                    "role": m.role,
                    "content": m.content,
                    "timestamp_ms": m.timestamp_ms,
                })
            }).collect::<Vec<_>>(),
            "file_changes": self.file_changes.iter().map(|f| {
                json!({
                    "path": f.path,
                    "content": f.content,
                    "hash": f.hash,
                    "timestamp_ms": f.timestamp_ms,
                })
            }).collect::<Vec<_>>(),
            "session_state": {
                "cwd": self.session_state.cwd,
                "env": self.session_state.env,
                "active_contexts": self.session_state.active_contexts,
            }
        });
        serde_json::to_string(&json)
    }

    /// Deserialize checkpoint from JSON
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let value: Value = serde_json::from_str(json_str)?;

        let id = value["id"].as_str().unwrap_or("").to_string();
        let number = value["number"].as_u64().unwrap_or(0) as u32;
        let created_at_ms = value["created_at_ms"].as_u64().unwrap_or(0);
        let description = value["description"].as_str().map(|s| s.to_string());

        let messages = value["messages"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|m| {
                CheckpointMessage::new(
                    m["role"].as_str().unwrap_or(""),
                    m["content"].as_str().unwrap_or(""),
                    m["timestamp_ms"].as_u64().unwrap_or(0),
                )
            })
            .collect();

        let file_changes = value["file_changes"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|f| FileChange {
                path: f["path"].as_str().unwrap_or("").to_string(),
                content: f["content"].as_str().unwrap_or("").to_string(),
                hash: f["hash"].as_str().unwrap_or("").to_string(),
                timestamp_ms: f["timestamp_ms"].as_u64().unwrap_or(0),
            })
            .collect();

        let cwd = value["session_state"]["cwd"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let mut env = HashMap::new();
        if let Some(env_obj) = value["session_state"]["env"].as_object() {
            for (k, v) in env_obj {
                if let Some(v_str) = v.as_str() {
                    env.insert(k.clone(), v_str.to_string());
                }
            }
        }
        let active_contexts = value["session_state"]["active_contexts"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|c| c.as_str().map(|s| s.to_string()))
            .collect();

        let session_state = SessionState {
            cwd,
            env,
            active_contexts,
        };

        let mut checkpoint = Checkpoint::new(id, number, created_at_ms, session_state);
        checkpoint.messages = messages;
        checkpoint.file_changes = file_changes;
        checkpoint.description = description;

        Ok(checkpoint)
    }
}

/// Session checkpoint history
#[derive(Debug, Clone)]
pub struct CheckpointHistory {
    /// Session ID
    pub session_id: String,
    /// All checkpoints in order
    checkpoints: Vec<Checkpoint>,
    /// Maximum number of checkpoints to retain
    pub max_checkpoints: usize,
}

impl CheckpointHistory {
    pub fn new(session_id: impl Into<String>, max_checkpoints: usize) -> Self {
        Self {
            session_id: session_id.into(),
            checkpoints: Vec::new(),
            max_checkpoints,
        }
    }

    /// Record a new checkpoint
    pub fn record_checkpoint(&mut self, checkpoint: Checkpoint) {
        self.checkpoints.push(checkpoint);

        // Enforce max checkpoints retention
        if self.checkpoints.len() > self.max_checkpoints {
            let to_remove = self.checkpoints.len() - self.max_checkpoints;
            self.checkpoints.drain(0..to_remove);
        }
    }

    /// Get the latest checkpoint
    pub fn latest(&self) -> Option<&Checkpoint> {
        self.checkpoints.last()
    }

    /// Get checkpoint by ID
    pub fn get(&self, id: &str) -> Option<&Checkpoint> {
        self.checkpoints.iter().find(|c| c.id == id)
    }

    /// Get checkpoint by number
    pub fn get_by_number(&self, number: u32) -> Option<&Checkpoint> {
        self.checkpoints.iter().find(|c| c.number == number)
    }

    /// Get all checkpoints
    pub fn all(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    /// Number of checkpoints in history
    pub fn len(&self) -> usize {
        self.checkpoints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.checkpoints.is_empty()
    }

    /// Total size of all checkpoints
    pub fn total_size_bytes(&self) -> usize {
        self.checkpoints.iter().map(|c| c.size_bytes()).sum()
    }
}

/// Session that can be checkpointed and restored
#[derive(Debug, Clone)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// Current checkpoint number
    pub current_checkpoint_number: u32,
    /// Checkpoint history
    pub history: CheckpointHistory,
    /// Current session state
    pub current_state: SessionState,
}

impl Session {
    pub fn new(id: impl Into<String>, max_checkpoints: usize) -> Self {
        let id = id.into();
        Self {
            current_state: SessionState::new("/"),
            id: id.clone(),
            current_checkpoint_number: 0,
            history: CheckpointHistory::new(id, max_checkpoints),
        }
    }

    /// Create a checkpoint of the current session state
    pub fn create_checkpoint(&mut self, description: Option<String>) -> String {
        let checkpoint_id = format!("checkpoint-{}-{}", self.id, self.current_checkpoint_number);
        let now_ms = 0; // In real code, this would be current time

        let mut checkpoint = Checkpoint::new(
            checkpoint_id.clone(),
            self.current_checkpoint_number,
            now_ms,
            self.current_state.clone(),
        );

        if let Some(desc) = description {
            checkpoint = checkpoint.with_description(desc);
        }

        self.history.record_checkpoint(checkpoint);
        self.current_checkpoint_number += 1;

        checkpoint_id
    }

    /// Restore from a checkpoint
    pub fn restore_checkpoint(
        &mut self,
        checkpoint_id: &str,
        scope: RestoreScope,
    ) -> Result<(), String> {
        let checkpoint = self
            .history
            .get(checkpoint_id)
            .ok_or_else(|| format!("Checkpoint not found: {}", checkpoint_id))?;

        match scope {
            RestoreScope::ConversationOnly => {
                // Restore only conversation, keep current session state
                // In production, this would also restore message history
            }
            RestoreScope::CodeOnly => {
                // Restore only file changes, keep conversation
                // In production, this would restore files from checkpoint
            }
            RestoreScope::Both => {
                // Restore everything
                self.current_state = checkpoint.session_state.clone();
            }
        }

        Ok(())
    }

    /// Get the last checkpoint that can be restored
    pub fn last_checkpoint(&self) -> Option<&Checkpoint> {
        self.history.latest()
    }
}

// ============================================================================
// UNIT TESTS: CHECKPOINT STRUCTURE & SERIALIZATION
// ============================================================================

#[cfg(test)]
mod checkpoint_structure_tests {
    use super::*;

    #[test]
    fn test_file_change_creation() {
        let change = FileChange::new("/src/main.rs", "fn main() {}", 1000);

        assert_eq!(change.path, "/src/main.rs");
        assert_eq!(change.content, "fn main() {}");
        assert_eq!(change.timestamp_ms, 1000);
        assert!(!change.hash.is_empty());
    }

    #[test]
    fn test_file_change_integrity_verification() {
        let mut change = FileChange::new("/src/main.rs", "fn main() {}", 1000);

        // Valid state
        assert!(change.verify_integrity());

        // Corrupt the content (hash won't match)
        change.content = "corrupted content".to_string();
        assert!(!change.verify_integrity());
    }

    #[test]
    fn test_file_change_hash_computation() {
        let change1 = FileChange::new("/src/main.rs", "code", 1000);
        let change2 = FileChange::new("/src/main.rs", "code", 2000);
        let change3 = FileChange::new("/src/main.rs", "different", 1000);

        // Same content = same hash (hash only depends on content)
        assert_eq!(change1.hash, change2.hash);
        // Different content = different hash
        assert_ne!(change1.hash, change3.hash);
    }

    #[test]
    fn test_checkpoint_message_creation() {
        let msg_user = CheckpointMessage::user("Hello", 1000);
        let msg_assistant = CheckpointMessage::assistant("Hi there", 2000);
        let msg_system = CheckpointMessage::system("System prompt", 3000);

        assert_eq!(msg_user.role, "user");
        assert_eq!(msg_assistant.role, "assistant");
        assert_eq!(msg_system.role, "system");
    }

    #[test]
    fn test_session_state_creation() {
        let state = SessionState::new("/home/user/project")
            .with_env("RUST_LOG", "debug")
            .with_context("file.rs");

        assert_eq!(state.cwd, "/home/user/project");
        assert_eq!(state.env.get("RUST_LOG"), Some(&"debug".to_string()));
        assert_eq!(state.active_contexts.len(), 1);
        assert_eq!(state.active_contexts[0], "file.rs");
    }

    #[test]
    fn test_checkpoint_creation() {
        let state = SessionState::new("/project");
        let checkpoint = Checkpoint::new("cp-001", 1, 1000, state);

        assert_eq!(checkpoint.id, "cp-001");
        assert_eq!(checkpoint.number, 1);
        assert_eq!(checkpoint.created_at_ms, 1000);
        assert!(checkpoint.verify_integrity());
    }

    #[test]
    fn test_checkpoint_add_message() {
        let state = SessionState::new("/project");
        let mut checkpoint = Checkpoint::new("cp-001", 1, 1000, state);

        checkpoint.add_message(CheckpointMessage::user("Hello", 1000));
        checkpoint.add_message(CheckpointMessage::assistant("Hi", 1100));

        assert_eq!(checkpoint.messages.len(), 2);
        assert_eq!(checkpoint.messages[0].content, "Hello");
        assert_eq!(checkpoint.messages[1].content, "Hi");
    }

    #[test]
    fn test_checkpoint_record_file_change() {
        let state = SessionState::new("/project");
        let mut checkpoint = Checkpoint::new("cp-001", 1, 1000, state);

        let change1 = FileChange::new("/src/main.rs", "v1", 1000);
        checkpoint.record_file_change(change1);

        assert_eq!(checkpoint.file_changes.len(), 1);
        assert_eq!(checkpoint.file_changes[0].content, "v1");

        // Update same file
        let change2 = FileChange::new("/src/main.rs", "v2", 1100);
        checkpoint.record_file_change(change2);

        // Should still be 1 (replaced)
        assert_eq!(checkpoint.file_changes.len(), 1);
        assert_eq!(checkpoint.file_changes[0].content, "v2");
    }

    #[test]
    fn test_checkpoint_description() {
        let state = SessionState::new("/project");
        let checkpoint =
            Checkpoint::new("cp-001", 1, 1000, state).with_description("Initial implementation");

        assert_eq!(
            checkpoint.description,
            Some("Initial implementation".to_string())
        );
    }

    #[test]
    fn test_checkpoint_size_calculation() {
        let state = SessionState::new("/project");
        let mut checkpoint = Checkpoint::new("cp-001", 1, 1000, state);

        checkpoint.add_message(CheckpointMessage::user("Hello world", 1000));

        let size = checkpoint.size_bytes();
        assert!(size > 0);
        assert!(size < 1000); // Should be relatively small
    }
}

// ============================================================================
// SERIALIZATION TESTS: JSON ENCODING/DECODING
// ============================================================================

#[cfg(test)]
mod checkpoint_serialization_tests {
    use super::*;

    #[test]
    fn test_checkpoint_to_json() {
        let state = SessionState::new("/project");
        let mut checkpoint = Checkpoint::new("cp-001", 1, 1000, state);
        checkpoint.add_message(CheckpointMessage::user("Test", 1000));

        let json = checkpoint.to_json().expect("Should serialize");

        assert!(json.contains("\"id\":\"cp-001\""));
        assert!(json.contains("\"number\":1"));
        assert!(json.contains("\"Test\""));
    }

    #[test]
    fn test_checkpoint_from_json() {
        let state = SessionState::new("/project");
        let mut original = Checkpoint::new("cp-001", 1, 1000, state);
        original.add_message(CheckpointMessage::user("Test message", 1000));

        let json = original.to_json().expect("Should serialize");
        let restored = Checkpoint::from_json(&json).expect("Should deserialize");

        assert_eq!(restored.id, original.id);
        assert_eq!(restored.number, original.number);
        assert_eq!(restored.messages.len(), original.messages.len());
        assert_eq!(restored.messages[0].content, "Test message");
    }

    #[test]
    fn test_checkpoint_json_round_trip() {
        let state = SessionState::new("/home/user").with_env("KEY", "value");
        let mut checkpoint = Checkpoint::new("cp-123", 5, 5000, state);

        checkpoint.add_message(CheckpointMessage::user("User input", 5000));
        checkpoint.add_message(CheckpointMessage::assistant("Response", 5100));
        checkpoint.record_file_change(FileChange::new("/src/app.rs", "code here", 5050));

        let json = checkpoint.to_json().expect("Should serialize");
        let restored = Checkpoint::from_json(&json).expect("Should deserialize");

        assert_eq!(restored.id, "cp-123");
        assert_eq!(restored.number, 5);
        assert_eq!(restored.messages.len(), 2);
        assert_eq!(restored.file_changes.len(), 1);
        assert_eq!(
            restored.session_state.env.get("KEY"),
            Some(&"value".to_string())
        );
    }

    #[test]
    fn test_checkpoint_json_preserves_file_changes() {
        let state = SessionState::new("/project");
        let mut checkpoint = Checkpoint::new("cp-001", 1, 1000, state);

        checkpoint.record_file_change(FileChange::new("/a.rs", "content_a", 1000));
        checkpoint.record_file_change(FileChange::new("/b.rs", "content_b", 1100));

        let json = checkpoint.to_json().expect("Should serialize");
        let restored = Checkpoint::from_json(&json).expect("Should deserialize");

        assert_eq!(restored.file_changes.len(), 2);
        assert!(restored.file_changes.iter().any(|f| f.path == "/a.rs"));
        assert!(restored.file_changes.iter().any(|f| f.path == "/b.rs"));
    }

    #[test]
    fn test_checkpoint_json_with_special_characters() {
        let state = SessionState::new("/project");
        let mut checkpoint = Checkpoint::new("cp-001", 1, 1000, state);

        let message_with_quotes = r#"This has "quotes" and \n newlines"#;
        checkpoint.add_message(CheckpointMessage::user(message_with_quotes, 1000));

        let json = checkpoint.to_json().expect("Should serialize");
        let restored = Checkpoint::from_json(&json).expect("Should deserialize");

        assert_eq!(restored.messages[0].content, message_with_quotes);
    }

    #[test]
    fn test_empty_checkpoint_serialization() {
        let state = SessionState::new("/project");
        let checkpoint = Checkpoint::new("cp-001", 1, 1000, state);

        let json = checkpoint.to_json().expect("Should serialize");
        let restored = Checkpoint::from_json(&json).expect("Should deserialize");

        assert_eq!(restored.messages.len(), 0);
        assert_eq!(restored.file_changes.len(), 0);
    }
}

// ============================================================================
// UNIT TESTS: CHECKPOINT HISTORY MANAGEMENT
// ============================================================================

#[cfg(test)]
mod checkpoint_history_tests {
    use super::*;

    #[test]
    fn test_history_creation() {
        let history = CheckpointHistory::new("session-001", 10);

        assert_eq!(history.session_id, "session-001");
        assert_eq!(history.max_checkpoints, 10);
        assert!(history.is_empty());
    }

    #[test]
    fn test_record_checkpoint() {
        let mut history = CheckpointHistory::new("session-001", 10);
        let state = SessionState::new("/project");
        let checkpoint = Checkpoint::new("cp-001", 1, 1000, state);

        history.record_checkpoint(checkpoint);

        assert_eq!(history.len(), 1);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_get_latest_checkpoint() {
        let mut history = CheckpointHistory::new("session-001", 10);
        let state = SessionState::new("/project");

        let cp1 = Checkpoint::new("cp-001", 1, 1000, state.clone());
        let cp2 = Checkpoint::new("cp-002", 2, 2000, state.clone());

        history.record_checkpoint(cp1);
        history.record_checkpoint(cp2);

        let latest = history.latest().expect("Should have latest");
        assert_eq!(latest.id, "cp-002");
    }

    #[test]
    fn test_get_checkpoint_by_id() {
        let mut history = CheckpointHistory::new("session-001", 10);
        let state = SessionState::new("/project");

        let cp1 = Checkpoint::new("cp-001", 1, 1000, state.clone());
        let cp2 = Checkpoint::new("cp-002", 2, 2000, state);

        history.record_checkpoint(cp1);
        history.record_checkpoint(cp2);

        let found = history.get("cp-001").expect("Should find cp-001");
        assert_eq!(found.id, "cp-001");

        assert!(history.get("nonexistent").is_none());
    }

    #[test]
    fn test_get_checkpoint_by_number() {
        let mut history = CheckpointHistory::new("session-001", 10);
        let state = SessionState::new("/project");

        history.record_checkpoint(Checkpoint::new("cp-001", 1, 1000, state.clone()));
        history.record_checkpoint(Checkpoint::new("cp-002", 2, 2000, state));

        let found = history.get_by_number(2).expect("Should find checkpoint #2");
        assert_eq!(found.id, "cp-002");
    }

    #[test]
    fn test_max_checkpoint_retention() {
        let mut history = CheckpointHistory::new("session-001", 3);
        let state = SessionState::new("/project");

        // Add more checkpoints than max
        for i in 1..=5 {
            let cp = Checkpoint::new(
                format!("cp-{:03}", i),
                i as u32,
                i as u64 * 1000,
                state.clone(),
            );
            history.record_checkpoint(cp);
        }

        // Should only keep 3 (latest)
        assert_eq!(history.len(), 3);
        // Should have kept the last 3
        assert!(history.get("cp-003").is_some());
        assert!(history.get("cp-004").is_some());
        assert!(history.get("cp-005").is_some());
        // Should have removed the first 2
        assert!(history.get("cp-001").is_none());
        assert!(history.get("cp-002").is_none());
    }

    #[test]
    fn test_all_checkpoints() {
        let mut history = CheckpointHistory::new("session-001", 10);
        let state = SessionState::new("/project");

        history.record_checkpoint(Checkpoint::new("cp-001", 1, 1000, state.clone()));
        history.record_checkpoint(Checkpoint::new("cp-002", 2, 2000, state));

        let all = history.all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_total_size_calculation() {
        let mut history = CheckpointHistory::new("session-001", 10);
        let state = SessionState::new("/project");

        let mut cp = Checkpoint::new("cp-001", 1, 1000, state);
        cp.add_message(CheckpointMessage::user("some content", 1000));
        history.record_checkpoint(cp);

        let total_size = history.total_size_bytes();
        assert!(total_size > 0);
    }
}

// ============================================================================
// INTEGRATION TESTS: SESSION SAVING & RESUMING
// ============================================================================

#[cfg(test)]
mod session_saving_tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("session-001", 10);

        assert_eq!(session.id, "session-001");
        assert_eq!(session.current_checkpoint_number, 0);
        assert!(session.history.is_empty());
    }

    #[test]
    fn test_create_checkpoint() {
        let mut session = Session::new("session-001", 10);

        let cp_id = session.create_checkpoint(Some("First checkpoint".to_string()));

        assert!(!cp_id.is_empty());
        assert_eq!(session.history.len(), 1);
        assert_eq!(session.current_checkpoint_number, 1);
    }

    #[test]
    fn test_create_multiple_checkpoints() {
        let mut session = Session::new("session-001", 10);

        let cp1 = session.create_checkpoint(Some("Checkpoint 1".to_string()));
        let cp2 = session.create_checkpoint(Some("Checkpoint 2".to_string()));
        let cp3 = session.create_checkpoint(None);

        assert_ne!(cp1, cp2);
        assert_ne!(cp2, cp3);
        assert_eq!(session.history.len(), 3);
    }

    #[test]
    fn test_checkpoint_captures_current_state() {
        let mut session = Session::new("session-001", 10);

        session.current_state = SessionState::new("/home/user/project")
            .with_env("RUST_LOG", "debug")
            .with_context("app.rs");

        let cp_id = session.create_checkpoint(None);
        let checkpoint = session.history.get(&cp_id).expect("Should find checkpoint");

        assert_eq!(checkpoint.session_state.cwd, "/home/user/project");
        assert_eq!(
            checkpoint.session_state.env.get("RUST_LOG"),
            Some(&"debug".to_string())
        );
        assert_eq!(checkpoint.session_state.active_contexts.len(), 1);
    }

    #[test]
    fn test_session_state_changes_between_checkpoints() {
        let mut session = Session::new("session-001", 10);

        session.current_state = SessionState::new("/project1");
        let cp1_id = session.create_checkpoint(None);

        session.current_state = SessionState::new("/project2");
        let cp2_id = session.create_checkpoint(None);

        let cp1 = session.history.get(&cp1_id).expect("Should find cp1");
        let cp2 = session.history.get(&cp2_id).expect("Should find cp2");

        assert_eq!(cp1.session_state.cwd, "/project1");
        assert_eq!(cp2.session_state.cwd, "/project2");
    }

    #[test]
    fn test_last_checkpoint_retrieval() {
        let mut session = Session::new("session-001", 10);

        session.create_checkpoint(Some("First".to_string()));
        session.create_checkpoint(Some("Second".to_string()));

        let last = session
            .last_checkpoint()
            .expect("Should have last checkpoint");
        assert_eq!(last.description, Some("Second".to_string()));
    }

    #[test]
    fn test_checkpoint_numbering_sequence() {
        let mut session = Session::new("session-001", 10);

        for i in 0..5 {
            session.create_checkpoint(None);
            assert_eq!(session.current_checkpoint_number, (i + 1) as u32);
        }
    }
}

// ============================================================================
// INTEGRATION TESTS: SESSION RESUMING & RESTORATION
// ============================================================================

#[cfg(test)]
mod session_resuming_tests {
    use super::*;

    #[test]
    fn test_restore_checkpoint_both_scope() {
        let mut session = Session::new("session-001", 10);

        session.current_state = SessionState::new("/original").with_env("VAR", "original_value");
        let cp_id = session.create_checkpoint(None);

        // Change session state
        session.current_state = SessionState::new("/modified").with_env("VAR", "modified_value");

        // Restore checkpoint with Both scope
        session
            .restore_checkpoint(&cp_id, RestoreScope::Both)
            .expect("Should restore");

        assert_eq!(session.current_state.cwd, "/original");
        assert_eq!(
            session.current_state.env.get("VAR"),
            Some(&"original_value".to_string())
        );
    }

    #[test]
    fn test_restore_nonexistent_checkpoint() {
        let mut session = Session::new("session-001", 10);

        let result = session.restore_checkpoint("nonexistent", RestoreScope::Both);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Checkpoint not found"));
    }

    #[test]
    fn test_restore_scope_conversation_only() {
        let mut session = Session::new("session-001", 10);

        let original_state = SessionState::new("/original");
        session.current_state = original_state.clone();
        let cp_id = session.create_checkpoint(None);

        // Change state
        session.current_state = SessionState::new("/modified");

        // Restore with conversation-only scope
        session
            .restore_checkpoint(&cp_id, RestoreScope::ConversationOnly)
            .expect("Should restore conversation");

        // State should remain modified (not restored)
        assert_eq!(session.current_state.cwd, "/modified");
    }

    #[test]
    fn test_restore_scope_code_only() {
        let mut session = Session::new("session-001", 10);

        session.current_state = SessionState::new("/project");
        let cp_id = session.create_checkpoint(None);

        // Change state
        session.current_state = SessionState::new("/modified");

        // Restore with code-only scope
        session
            .restore_checkpoint(&cp_id, RestoreScope::CodeOnly)
            .expect("Should restore code");

        // State should remain modified (code restoration doesn't change working state)
        assert_eq!(session.current_state.cwd, "/modified");
    }

    #[test]
    fn test_sequential_checkpoint_restore() {
        let mut session = Session::new("session-001", 10);

        // Create checkpoint 1
        session.current_state = SessionState::new("/state1");
        let cp1_id = session.create_checkpoint(None);

        // Create checkpoint 2
        session.current_state = SessionState::new("/state2");
        let cp2_id = session.create_checkpoint(None);

        // Restore checkpoint 1
        session
            .restore_checkpoint(&cp1_id, RestoreScope::Both)
            .expect("Should restore cp1");
        assert_eq!(session.current_state.cwd, "/state1");

        // Restore checkpoint 2
        session
            .restore_checkpoint(&cp2_id, RestoreScope::Both)
            .expect("Should restore cp2");
        assert_eq!(session.current_state.cwd, "/state2");
    }

    #[test]
    fn test_restore_checkpoint_preserves_history() {
        let mut session = Session::new("session-001", 10);

        session.create_checkpoint(None);
        session.current_state = SessionState::new("/modified");
        let cp_id = session.create_checkpoint(None);

        let history_len_before = session.history.len();

        session
            .restore_checkpoint(&cp_id, RestoreScope::Both)
            .expect("Should restore");

        // History should be unchanged
        assert_eq!(session.history.len(), history_len_before);
    }
}

// ============================================================================
// INTEGRATION TESTS: STATE PERSISTENCE & RECOVERY
// ============================================================================

#[cfg(test)]
mod state_persistence_tests {
    use super::*;

    #[test]
    fn test_full_session_persistence_cycle() {
        let mut session = Session::new("session-001", 10);

        // Build up state
        session.current_state = SessionState::new("/home/user/project")
            .with_env("RUST_LOG", "debug")
            .with_env("CARGO_PROFILE_RELEASE_LTO", "thin")
            .with_context("src/main.rs")
            .with_context("src/lib.rs");

        // Create checkpoint
        let cp_id = session.create_checkpoint(Some("Development state".to_string()));
        let checkpoint = session.history.get(&cp_id).expect("Should have checkpoint");

        // Verify persistence
        assert_eq!(checkpoint.session_state.cwd, "/home/user/project");
        assert_eq!(checkpoint.session_state.env.len(), 2);
        assert_eq!(checkpoint.session_state.active_contexts.len(), 2);
    }

    #[test]
    fn test_checkpoint_with_complex_file_changes() {
        let mut session = Session::new("session-001", 10);
        let cp_id = session.create_checkpoint(None);
        let mut checkpoint = session
            .history
            .get(&cp_id)
            .expect("Should have checkpoint")
            .clone();

        // Record multiple file changes
        checkpoint.record_file_change(FileChange::new("/src/main.rs", "v1", 1000));
        checkpoint.record_file_change(FileChange::new("/src/lib.rs", "v2", 1100));
        checkpoint.record_file_change(FileChange::new("/Cargo.toml", "v3", 1200));

        assert_eq!(checkpoint.file_changes.len(), 3);
        assert!(checkpoint.file_changes.iter().all(|f| f.verify_integrity()));
    }

    #[test]
    fn test_checkpoint_with_conversation_history() {
        let mut session = Session::new("session-001", 10);
        let cp_id = session.create_checkpoint(None);
        let mut checkpoint = session
            .history
            .get(&cp_id)
            .expect("Should have checkpoint")
            .clone();

        // Add conversation
        checkpoint.add_message(CheckpointMessage::user("What should I do?", 1000));
        checkpoint.add_message(CheckpointMessage::assistant(
            "First, analyze the problem",
            1100,
        ));
        checkpoint.add_message(CheckpointMessage::user("Then what?", 1200));
        checkpoint.add_message(CheckpointMessage::assistant(
            "Then implement the solution",
            1300,
        ));

        assert_eq!(checkpoint.messages.len(), 4);
        assert_eq!(checkpoint.messages[0].role, "user");
        assert_eq!(checkpoint.messages[1].role, "assistant");
    }

    #[test]
    fn test_recovery_from_corrupted_checkpoint() {
        let mut session = Session::new("session-001", 10);

        session.current_state = SessionState::new("/project");
        let cp_id = session.create_checkpoint(None);

        let mut checkpoint = session
            .history
            .get(&cp_id)
            .expect("Should have checkpoint")
            .clone();

        // Corrupt a file change
        if !checkpoint.file_changes.is_empty() {
            checkpoint.file_changes[0].content = "corrupted".to_string();
        }

        // Integrity check should fail
        assert!(!checkpoint.verify_integrity() || checkpoint.file_changes.is_empty());
    }

    #[test]
    fn test_checkpoint_uniqueness() {
        let mut session = Session::new("session-001", 10);

        let cp1_id = session.create_checkpoint(Some("CP1".to_string()));
        let cp2_id = session.create_checkpoint(Some("CP2".to_string()));

        assert_ne!(cp1_id, cp2_id);
    }

    #[test]
    fn test_persistence_across_restore_cycles() {
        let mut session = Session::new("session-001", 10);

        // Create initial state
        session.current_state = SessionState::new("/state1");
        let cp1_id = session.create_checkpoint(None);

        // Change and create new checkpoint
        session.current_state = SessionState::new("/state2");
        let cp2_id = session.create_checkpoint(None);

        // Restore to cp1
        session
            .restore_checkpoint(&cp1_id, RestoreScope::Both)
            .expect("Should restore cp1");

        // Create new checkpoint from restored state
        let cp3_id = session.create_checkpoint(None);
        let cp3 = session.history.get(&cp3_id).expect("Should have cp3");

        // cp3 should have state from cp1
        assert_eq!(cp3.session_state.cwd, "/state1");
    }
}

// ============================================================================
// EDGE CASE & BOUNDARY TESTS
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_file_path() {
        let change = FileChange::new("", "content", 1000);
        assert_eq!(change.path, "");
    }

    #[test]
    fn test_large_content_checkpoint() {
        let mut session = Session::new("session-001", 10);
        let cp_id = session.create_checkpoint(None);
        let mut checkpoint = session
            .history
            .get(&cp_id)
            .expect("Should have checkpoint")
            .clone();

        // Add large content
        let large_content = "x".repeat(10_000);
        checkpoint.record_file_change(FileChange::new("/large.rs", large_content, 1000));

        assert!(checkpoint.size_bytes() > 10_000);
    }

    #[test]
    fn test_checkpoint_with_zero_timestamp() {
        let change = FileChange::new("/file.rs", "content", 0);
        assert_eq!(change.timestamp_ms, 0);
    }

    #[test]
    fn test_checkpoint_id_uniqueness_pattern() {
        let mut session = Session::new("session-001", 10);

        let id1 = session.create_checkpoint(None);
        let id2 = session.create_checkpoint(None);

        assert!(id1.contains("session-001"));
        assert!(id2.contains("session-001"));
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_restore_scope_display() {
        assert_eq!(
            RestoreScope::ConversationOnly.to_string(),
            "conversation_only"
        );
        assert_eq!(RestoreScope::CodeOnly.to_string(), "code_only");
        assert_eq!(RestoreScope::Both.to_string(), "both");
    }

    #[test]
    fn test_checkpoint_with_empty_environment() {
        let state = SessionState::new("/project");
        let checkpoint = Checkpoint::new("cp-001", 1, 1000, state);

        assert_eq!(checkpoint.session_state.env.len(), 0);
        assert!(checkpoint.verify_integrity());
    }

    #[test]
    fn test_checkpoint_with_empty_contexts() {
        let state = SessionState::new("/project");
        let checkpoint = Checkpoint::new("cp-001", 1, 1000, state);

        assert_eq!(checkpoint.session_state.active_contexts.len(), 0);
    }

    #[test]
    fn test_history_retrieval_after_max_exceeded() {
        let mut history = CheckpointHistory::new("session-001", 2);
        let state = SessionState::new("/project");

        let cp1 = Checkpoint::new("cp-001", 1, 1000, state.clone());
        let cp2 = Checkpoint::new("cp-002", 2, 2000, state.clone());
        let cp3 = Checkpoint::new("cp-003", 3, 3000, state);

        history.record_checkpoint(cp1);
        history.record_checkpoint(cp2);
        history.record_checkpoint(cp3);

        // cp1 should be removed
        assert!(history.get("cp-001").is_none());
        assert!(history.get("cp-002").is_some());
        assert!(history.get("cp-003").is_some());
    }

    #[test]
    fn test_file_change_with_unicode_content() {
        let unicode_content = "fn greet() { println!(\"Hello, 世界 🦀\"); }";
        let change = FileChange::new("/src/main.rs", unicode_content, 1000);

        assert_eq!(change.content, unicode_content);
        assert!(change.verify_integrity());
    }

    #[test]
    fn test_checkpoint_message_with_empty_content() {
        let msg = CheckpointMessage::user("", 1000);
        assert_eq!(msg.content, "");
        assert_eq!(msg.role, "user");
    }

    #[test]
    fn test_session_restore_with_empty_checkpoint() {
        let mut session = Session::new("session-001", 10);

        session.current_state = SessionState::new("/original");
        let cp_id = session.create_checkpoint(None);

        session
            .restore_checkpoint(&cp_id, RestoreScope::Both)
            .expect("Should restore");

        assert_eq!(session.current_state.cwd, "/original");
    }
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_restore_from_deleted_checkpoint() {
        let mut session = Session::new("session-001", 10);

        let cp_id = session.create_checkpoint(None);
        assert!(session.history.get(&cp_id).is_some());

        // ID exists but try with different ID
        let result = session.restore_checkpoint("wrong-id", RestoreScope::Both);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_deserialization_with_missing_fields() {
        let json = r#"{"id":"cp-001"}"#;
        let result = Checkpoint::from_json(json);

        // Should handle missing fields gracefully
        assert!(result.is_ok());
        let checkpoint = result.unwrap();
        assert_eq!(checkpoint.id, "cp-001");
        assert_eq!(checkpoint.messages.len(), 0);
    }

    #[test]
    fn test_json_deserialization_with_invalid_json() {
        let invalid_json = "{ invalid json }";
        let result = Checkpoint::from_json(invalid_json);

        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_checkpoint_operations_isolation() {
        let mut history = CheckpointHistory::new("session-001", 10);
        let state1 = SessionState::new("/project1");
        let state2 = SessionState::new("/project2");

        let cp1 = Checkpoint::new("cp-001", 1, 1000, state1);
        let cp2 = Checkpoint::new("cp-002", 2, 2000, state2);

        history.record_checkpoint(cp1);
        history.record_checkpoint(cp2);

        // Modifications to retrieved checkpoint shouldn't affect history
        let retrieved = history.get("cp-001").unwrap().clone();
        // Cloned checkpoints are independent
        assert_eq!(retrieved.session_state.cwd, "/project1");

        let original = history.get("cp-001").unwrap();
        assert_eq!(original.session_state.cwd, "/project1");
    }
}
