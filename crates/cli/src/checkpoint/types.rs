//! Core checkpoint types and data structures
//!
//! This module defines the foundational types for Claude Code's checkpointing system:
//! - FileChange: Individual file modifications with hash verification
//! - CheckpointMessage: Conversation messages stored in checkpoints
//! - SessionState: Working directory and environment state
//! - Checkpoint: Complete session state at a point in time
//! - CheckpointHistory: Collection of checkpoints with retention policy
//! - Session: Active session with checkpointing capability
//! - RestoreScope: What to restore from a checkpoint

use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;

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
        // Simplified hash for testing - in production would use SHA256
        format!("{:x}", content.len() * 31)
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
    /// Active conversation context (messages in current session)
    pub context: Vec<CheckpointMessage>,
}

impl Session {
    pub fn new(id: impl Into<String>, max_checkpoints: usize) -> Self {
        let id = id.into();
        Self {
            current_state: SessionState::new("/"),
            id: id.clone(),
            current_checkpoint_number: 0,
            history: CheckpointHistory::new(id, max_checkpoints),
            context: Vec::new(),
        }
    }

    /// Create a checkpoint of the current session state
    pub fn create_checkpoint(&mut self, description: Option<String>) -> String {
        let checkpoint_id = format!("checkpoint-{}-{}", self.id, self.current_checkpoint_number);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut checkpoint = Checkpoint::new(
            checkpoint_id.clone(),
            self.current_checkpoint_number,
            now_ms,
            self.current_state.clone(),
        );

        // Capture current conversation context
        checkpoint.messages = self.context.clone();

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
                // Restore conversation messages to session context
                self.context = checkpoint.messages.clone();
            }
            RestoreScope::CodeOnly => {
                // Restore file changes to disk
                for file_change in &checkpoint.file_changes {
                    // Verify integrity before writing
                    if !file_change.verify_integrity() {
                        return Err(format!(
                            "File change integrity check failed for: {}",
                            file_change.path
                        ));
                    }

                    // Write file to disk
                    if let Err(e) = std::fs::write(&file_change.path, &file_change.content) {
                        return Err(format!(
                            "Failed to restore file {}: {}",
                            file_change.path, e
                        ));
                    }
                }
            }
            RestoreScope::Both => {
                // Restore session state
                self.current_state = checkpoint.session_state.clone();

                // Restore conversation context
                self.context = checkpoint.messages.clone();

                // Restore file changes
                for file_change in &checkpoint.file_changes {
                    // Verify integrity before writing
                    if !file_change.verify_integrity() {
                        return Err(format!(
                            "File change integrity check failed for: {}",
                            file_change.path
                        ));
                    }

                    // Write file to disk
                    if let Err(e) = std::fs::write(&file_change.path, &file_change.content) {
                        return Err(format!(
                            "Failed to restore file {}: {}",
                            file_change.path, e
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the last checkpoint that can be restored
    pub fn last_checkpoint(&self) -> Option<&Checkpoint> {
        self.history.latest()
    }
}
