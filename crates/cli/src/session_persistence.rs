//! Session persistence for InteractiveSession
//!
//! Provides auto-save/resume functionality by wiring the checkpoint system
//! to the TUI interactive session. Implements:
//!
//! - Auto-save on exit (Ctrl+D, /exit, signals)
//! - Resume prompt on startup (if session < 24h old)
//! - Manual checkpointing via /save command
//! - Session restoration via /load command
//! - Session listing via /sessions command
//!
//! # Architecture
//!
//! This module bridges the checkpoint system (checkpoint/*) with the
//! interactive TUI session (interactive.rs). It converts between:
//! - Core Message types -> Checkpoint CheckpointMessage types
//! - TUI conversation history -> Checkpoint session state
//!
//! # Usage
//!
//! ```rust,ignore
//! let mut persistence = SessionPersistence::new("interactive-session")?;
//!
//! // Check for resumable session
//! if let Some(info) = persistence.check_resumable_session()? {
//!     if user_wants_to_resume {
//!         persistence.resume_session()?;
//!     }
//! }
//!
//! // Auto-save on exit
//! persistence.auto_save()?;
//! ```

#[cfg(test)]
use crate::checkpoint::storage::CheckpointStorage;
use crate::checkpoint::{
    loader::SessionLoader,
    saver::SessionSaver,
    types::{CheckpointMessage, Session, SessionState},
};
use anyhow::{Context as AnyhowContext, Result};
use rustyclawd_core::{Message, MessageRole};
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum age for a session to be considered resumable (24 hours)
const RESUMABLE_SESSION_MAX_AGE_MS: u64 = 24 * 60 * 60 * 1000;

/// Maximum number of checkpoints to retain per session
const MAX_CHECKPOINTS: usize = 10;

/// Default session ID for interactive sessions
const DEFAULT_SESSION_ID: &str = "interactive-session";

/// Session information for display
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub last_checkpoint_time: u64,
    pub age_hours: f64,
    pub checkpoint_count: usize,
    pub message_count: usize,
}

impl SessionInfo {
    /// Format age for display
    pub fn format_age(&self) -> String {
        if self.age_hours < 1.0 {
            format!("{:.0} minutes ago", self.age_hours * 60.0)
        } else if self.age_hours < 24.0 {
            format!("{:.1} hours ago", self.age_hours)
        } else {
            format!("{:.1} days ago", self.age_hours / 24.0)
        }
    }
}

/// Session persistence manager
pub struct SessionPersistence {
    /// Current session
    session: Session,
    /// Session saver
    saver: SessionSaver,
    /// Session loader
    loader: SessionLoader,
    /// Whether this session was resumed
    is_resumed: bool,
}

impl SessionPersistence {
    /// Create a new session persistence manager
    pub fn new(session_id: impl Into<String>) -> Result<Self> {
        let session_id = session_id.into();
        let session = Session::new(session_id, MAX_CHECKPOINTS);

        #[cfg(not(test))]
        let saver = SessionSaver::with_default_storage()
            .context("Failed to initialize checkpoint storage")?;

        #[cfg(not(test))]
        let loader = SessionLoader::with_default_storage()
            .context("Failed to initialize checkpoint loader")?;

        #[cfg(test)]
        let (saver, loader) = {
            let storage = CheckpointStorage::new(
                std::env::temp_dir()
                    .join("rustyclawd-session-persistence-tests")
                    .join(std::process::id().to_string()),
            );
            (
                SessionSaver::new(storage.clone()),
                SessionLoader::new(storage),
            )
        };

        Ok(Self {
            session,
            saver,
            loader,
            is_resumed: false,
        })
    }

    /// Create with default session ID
    pub fn with_default_id() -> Result<Self> {
        Self::new(DEFAULT_SESSION_ID)
    }

    /// Check if there's a resumable session available
    pub fn check_resumable_session(&self) -> Result<Option<SessionInfo>> {
        let session_id = &self.session.id;

        // Check if session exists
        let checkpoint_ids = match self.loader.list_checkpoints(session_id) {
            Ok(ids) if !ids.is_empty() => ids,
            _ => return Ok(None),
        };

        // Load the latest checkpoint
        let latest_checkpoint = match self.loader.load_latest_checkpoint(session_id) {
            Ok(cp) => cp,
            Err(_) => return Ok(None),
        };

        // Check age
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let age_ms = now_ms.saturating_sub(latest_checkpoint.created_at_ms);

        if age_ms > RESUMABLE_SESSION_MAX_AGE_MS {
            return Ok(None);
        }

        let age_hours = age_ms as f64 / (60.0 * 60.0 * 1000.0);

        Ok(Some(SessionInfo {
            session_id: session_id.clone(),
            last_checkpoint_time: latest_checkpoint.created_at_ms,
            age_hours,
            checkpoint_count: checkpoint_ids.len(),
            message_count: latest_checkpoint.messages.len(),
        }))
    }

    /// Resume the session from the latest checkpoint
    pub fn resume_session(&mut self) -> Result<Vec<Message>> {
        let session_id = self.session.id.clone();

        // Load the full session
        let loaded_session = self
            .loader
            .load_session(&session_id, MAX_CHECKPOINTS)
            .context("Failed to load session")?;

        // Extract messages from the latest checkpoint
        let messages = if let Some(latest) = loaded_session.history.latest() {
            self.convert_checkpoint_messages_to_core(&latest.messages)
        } else {
            Vec::new()
        };

        // Update internal session
        self.session = loaded_session;
        self.is_resumed = true;

        Ok(messages)
    }

    /// Auto-save current session state
    pub fn auto_save(&mut self, messages: &[Message]) -> Result<()> {
        // Convert messages to checkpoint format
        let checkpoint_messages = self.convert_core_messages_to_checkpoint(messages);

        // Update session context
        self.session.context = checkpoint_messages;

        // Update session state
        let cwd = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        self.session.current_state = SessionState::new(cwd);

        // Create checkpoint
        let checkpoint_id = self
            .session
            .create_checkpoint(Some("Auto-save".to_string()));

        // Save to disk
        if let Some(checkpoint) = self.session.history.get(&checkpoint_id) {
            self.saver
                .save_checkpoint(&self.session.id, checkpoint)
                .context("Failed to save checkpoint")?;
        }

        // Enforce retention policy
        self.enforce_retention_policy()?;

        Ok(())
    }

    /// Manually save a checkpoint with a description
    pub fn save_checkpoint(&mut self, messages: &[Message], description: String) -> Result<String> {
        // Convert messages to checkpoint format
        let checkpoint_messages = self.convert_core_messages_to_checkpoint(messages);

        // Update session context
        self.session.context = checkpoint_messages;

        // Update session state
        let cwd = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        self.session.current_state = SessionState::new(cwd);

        // Create checkpoint
        let checkpoint_id = self.session.create_checkpoint(Some(description));

        // Save to disk
        if let Some(checkpoint) = self.session.history.get(&checkpoint_id) {
            self.saver
                .save_checkpoint(&self.session.id, checkpoint)
                .context("Failed to save checkpoint")?;
        }

        // Enforce retention policy
        self.enforce_retention_policy()?;

        Ok(checkpoint_id)
    }

    /// Load a specific checkpoint
    pub fn load_checkpoint(&mut self, checkpoint_id: &str) -> Result<Vec<Message>> {
        let checkpoint = self
            .loader
            .load_checkpoint(&self.session.id, checkpoint_id)
            .context("Failed to load checkpoint")?;

        let messages = self.convert_checkpoint_messages_to_core(&checkpoint.messages);

        // Update session state
        self.session.current_state = checkpoint.session_state.clone();

        Ok(messages)
    }

    /// List all available sessions with metadata
    pub fn list_all_sessions(&self) -> Result<Vec<SessionInfo>> {
        let session_ids = self.loader.list_sessions()?;
        let mut sessions = Vec::new();

        for session_id in session_ids {
            // Try to load session metadata
            if let Ok(checkpoint_ids) = self.loader.list_checkpoints(&session_id) {
                if checkpoint_ids.is_empty() {
                    continue;
                }

                // Load latest checkpoint for metadata
                if let Ok(latest) = self.loader.load_latest_checkpoint(&session_id) {
                    let now_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    let age_ms = now_ms.saturating_sub(latest.created_at_ms);
                    let age_hours = age_ms as f64 / (60.0 * 60.0 * 1000.0);

                    sessions.push(SessionInfo {
                        session_id,
                        last_checkpoint_time: latest.created_at_ms,
                        age_hours,
                        checkpoint_count: checkpoint_ids.len(),
                        message_count: latest.messages.len(),
                    });
                }
            }
        }

        // Sort by most recent first
        sessions.sort_by_key(|s| std::cmp::Reverse(s.last_checkpoint_time));

        Ok(sessions)
    }

    /// List checkpoints for the current session
    pub fn list_checkpoints(&self) -> Result<Vec<(String, SessionInfo)>> {
        let checkpoint_ids = self.loader.list_checkpoints(&self.session.id)?;
        let mut checkpoints = Vec::new();

        for checkpoint_id in checkpoint_ids {
            if let Ok(checkpoint) = self
                .loader
                .load_checkpoint(&self.session.id, &checkpoint_id)
            {
                let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let age_ms = now_ms.saturating_sub(checkpoint.created_at_ms);
                let age_hours = age_ms as f64 / (60.0 * 60.0 * 1000.0);

                let info = SessionInfo {
                    session_id: self.session.id.clone(),
                    last_checkpoint_time: checkpoint.created_at_ms,
                    age_hours,
                    checkpoint_count: 1,
                    message_count: checkpoint.messages.len(),
                };

                let description = checkpoint
                    .description
                    .unwrap_or_else(|| checkpoint_id.clone());
                checkpoints.push((description, info));
            }
        }

        Ok(checkpoints)
    }

    /// Get whether this session was resumed
    pub fn is_resumed(&self) -> bool {
        self.is_resumed
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session.id
    }

    /// Enforce retention policy (keep only MAX_CHECKPOINTS)
    fn enforce_retention_policy(&self) -> Result<()> {
        let storage = self.loader.storage();
        storage
            .cleanup_old_checkpoints(&self.session.id, MAX_CHECKPOINTS)
            .context("Failed to enforce retention policy")?;
        Ok(())
    }

    /// Convert core Message types to Checkpoint CheckpointMessage types
    fn convert_core_messages_to_checkpoint(&self, messages: &[Message]) -> Vec<CheckpointMessage> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::System => "system",
                };
                CheckpointMessage::new(role, msg.content.clone(), now_ms)
            })
            .collect()
    }

    /// Convert Checkpoint CheckpointMessage types to core Message types
    fn convert_checkpoint_messages_to_core(&self, messages: &[CheckpointMessage]) -> Vec<Message> {
        messages
            .iter()
            .map(|msg| {
                let content = msg.content.clone();
                match msg.role.as_str() {
                    "user" => Message::user(content),
                    "assistant" => Message::assistant(content),
                    "system" => Message::system(content),
                    _ => Message::user(content), // Fallback
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_session_id() -> String {
        format!(
            "test-session-{}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    }

    #[test]
    fn test_session_persistence_creation() {
        let session_id = temp_session_id();
        let persistence = SessionPersistence::new(&session_id);
        assert!(persistence.is_ok());

        let persistence = persistence.unwrap();
        assert_eq!(persistence.session_id(), session_id);
        assert!(!persistence.is_resumed());
    }

    #[test]
    fn test_auto_save() {
        let session_id = temp_session_id();
        let mut persistence = SessionPersistence::new(&session_id).unwrap();

        let messages = vec![Message::user("Hello"), Message::assistant("Hi there!")];

        let result = persistence.auto_save(&messages);
        assert!(result.is_ok());

        // Verify checkpoint was created
        let checkpoints = persistence.list_checkpoints().unwrap();
        assert_eq!(checkpoints.len(), 1);

        // Cleanup
        let _ = std::fs::remove_dir_all(persistence.loader.storage().session_dir(&session_id));
    }

    #[test]
    fn test_save_and_load_checkpoint() {
        let session_id = temp_session_id();
        let mut persistence = SessionPersistence::new(&session_id).unwrap();

        let messages = vec![
            Message::user("Test message"),
            Message::assistant("Test response"),
        ];

        // Save checkpoint
        let checkpoint_id = persistence
            .save_checkpoint(&messages, "Test checkpoint".to_string())
            .unwrap();

        // Load checkpoint
        let loaded_messages = persistence.load_checkpoint(&checkpoint_id).unwrap();

        assert_eq!(loaded_messages.len(), 2);
        assert_eq!(loaded_messages[0].content, "Test message");
        assert_eq!(loaded_messages[1].content, "Test response");

        // Cleanup
        let _ = std::fs::remove_dir_all(persistence.loader.storage().session_dir(&session_id));
    }

    #[test]
    fn test_retention_policy() {
        let session_id = temp_session_id();
        let mut persistence = SessionPersistence::new(&session_id).unwrap();

        // Create more than MAX_CHECKPOINTS
        for i in 0..15 {
            let messages = vec![Message::user(format!("Message {}", i))];
            persistence
                .save_checkpoint(&messages, format!("Checkpoint {}", i))
                .unwrap();
        }

        // Verify only MAX_CHECKPOINTS remain
        let checkpoints = persistence.list_checkpoints().unwrap();
        assert!(checkpoints.len() <= MAX_CHECKPOINTS);

        // Cleanup
        let _ = std::fs::remove_dir_all(persistence.loader.storage().session_dir(&session_id));
    }

    #[test]
    fn test_check_resumable_session() {
        let session_id = temp_session_id();
        let mut persistence = SessionPersistence::new(&session_id).unwrap();

        // No session should exist yet
        let info = persistence.check_resumable_session().unwrap();
        assert!(info.is_none());

        // Create a session
        let messages = vec![Message::user("Test")];
        persistence.auto_save(&messages).unwrap();

        // Now session should be resumable
        let info = persistence.check_resumable_session().unwrap();
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.session_id, session_id);
        assert!(info.age_hours < 1.0);

        // Cleanup
        let _ = std::fs::remove_dir_all(persistence.loader.storage().session_dir(&session_id));
    }

    #[test]
    fn test_resume_session() {
        let session_id = temp_session_id();
        let mut persistence = SessionPersistence::new(&session_id).unwrap();

        // Save a session
        let original_messages = vec![
            Message::user("Original message"),
            Message::assistant("Original response"),
        ];
        persistence.auto_save(&original_messages).unwrap();

        // Create a new persistence instance (simulating app restart)
        let mut new_persistence = SessionPersistence::new(&session_id).unwrap();

        // Resume
        let resumed_messages = new_persistence.resume_session().unwrap();

        assert_eq!(resumed_messages.len(), 2);
        assert_eq!(resumed_messages[0].content, "Original message");
        assert!(new_persistence.is_resumed());

        // Cleanup
        let _ = std::fs::remove_dir_all(persistence.loader.storage().session_dir(&session_id));
    }

    #[test]
    fn test_list_all_sessions() {
        let session_id1 = temp_session_id();
        let session_id2 = format!("{}-2", session_id1);

        let mut persistence1 = SessionPersistence::new(&session_id1).unwrap();
        let mut persistence2 = SessionPersistence::new(&session_id2).unwrap();

        // Create sessions
        persistence1
            .auto_save(&[Message::user("Session 1")])
            .unwrap();
        persistence2
            .auto_save(&[Message::user("Session 2")])
            .unwrap();

        // List all sessions
        let sessions = persistence1.list_all_sessions().unwrap();
        assert!(sessions.len() >= 2);

        // Cleanup
        let _ = std::fs::remove_dir_all(persistence1.loader.storage().session_dir(&session_id1));
        let _ = std::fs::remove_dir_all(persistence2.loader.storage().session_dir(&session_id2));
    }

    #[test]
    fn test_message_conversion() {
        let session_id = temp_session_id();
        let persistence = SessionPersistence::new(&session_id).unwrap();

        let original = vec![
            Message::user("User message"),
            Message::assistant("Assistant message"),
            Message::system("System message"),
        ];

        let checkpoint_msgs = persistence.convert_core_messages_to_checkpoint(&original);
        assert_eq!(checkpoint_msgs.len(), 3);
        assert_eq!(checkpoint_msgs[0].role, "user");
        assert_eq!(checkpoint_msgs[1].role, "assistant");
        assert_eq!(checkpoint_msgs[2].role, "system");

        let converted = persistence.convert_checkpoint_messages_to_core(&checkpoint_msgs);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0].content, "User message");
        assert_eq!(converted[1].content, "Assistant message");
        assert_eq!(converted[2].content, "System message");
    }
}
