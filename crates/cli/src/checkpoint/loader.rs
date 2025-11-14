//! Session restoration logic
//!
//! This module handles:
//! - Loading checkpoints from disk
//! - Session restoration with scope control
//! - Checkpoint verification before restoration
//! - History reconstruction

use super::storage::CheckpointStorage;
use super::types::{Checkpoint, RestoreScope, Session};
use std::io;

/// Session loader for checkpoint restoration
pub struct SessionLoader {
    pub(crate) storage: CheckpointStorage,
}

impl SessionLoader {
    /// Create a new session loader
    pub fn new(storage: CheckpointStorage) -> Self {
        Self { storage }
    }

    /// Create with default storage location
    pub fn with_default_storage() -> io::Result<Self> {
        let storage = CheckpointStorage::with_default_path()?;
        Ok(Self { storage })
    }

    /// List all available sessions
    pub fn list_sessions(&self) -> io::Result<Vec<String>> {
        use std::fs;

        let sessions_dir = &self.storage.base_dir;

        if !sessions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut session_ids = Vec::new();
        for entry in fs::read_dir(sessions_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    session_ids.push(name.to_string());
                }
            }
        }

        session_ids.sort();
        Ok(session_ids)
    }

    /// Load a single checkpoint from disk
    pub fn load_checkpoint(&self, session_id: &str, checkpoint_id: &str) -> io::Result<Checkpoint> {
        let checkpoint = self.storage.load_checkpoint(session_id, checkpoint_id)?;

        // Verify checkpoint integrity
        if !checkpoint.verify_integrity() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Checkpoint {} failed integrity check", checkpoint_id),
            ));
        }

        Ok(checkpoint)
    }

    /// Load all checkpoints for a session and reconstruct history
    pub fn load_session(&self, session_id: &str, max_checkpoints: usize) -> io::Result<Session> {
        let checkpoint_ids = self.storage.list_checkpoints(session_id)?;

        if checkpoint_ids.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("No checkpoints found for session {}", session_id),
            ));
        }

        // Create session
        let mut session = Session::new(session_id, max_checkpoints);

        // Load and add all checkpoints to history
        for checkpoint_id in checkpoint_ids {
            match self.load_checkpoint(session_id, &checkpoint_id) {
                Ok(checkpoint) => {
                    // Update session state from latest checkpoint
                    session.current_state = checkpoint.session_state.clone();
                    session.current_checkpoint_number = checkpoint.number + 1;
                    session.history.record_checkpoint(checkpoint);
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to load checkpoint {}: {}",
                        checkpoint_id, e
                    );
                }
            }
        }

        Ok(session)
    }

    /// Resume a session from the latest checkpoint
    pub fn resume_session(&self, session_id: &str, max_checkpoints: usize) -> io::Result<Session> {
        self.load_session(session_id, max_checkpoints)
    }

    /// Restore a specific checkpoint into a session with scope control
    pub fn restore_checkpoint(
        &self,
        session: &mut Session,
        checkpoint_id: &str,
        scope: RestoreScope,
    ) -> io::Result<()> {
        let checkpoint = self.load_checkpoint(&session.id, checkpoint_id)?;

        match scope {
            RestoreScope::ConversationOnly => {
                // Restore only conversation messages
                // In a full implementation, this would restore message history
                // to the session's conversation state
            }
            RestoreScope::CodeOnly => {
                // Restore only file changes
                // In a full implementation, this would write files to disk
                // from the checkpoint's file_changes
            }
            RestoreScope::Both => {
                // Restore complete session state
                session.current_state = checkpoint.session_state.clone();
            }
        }

        Ok(())
    }

    /// Load checkpoint metadata without full content
    pub fn load_checkpoint_metadata(
        &self,
        session_id: &str,
        checkpoint_id: &str,
    ) -> io::Result<super::storage::CheckpointMetadata> {
        self.storage.checkpoint_metadata(session_id, checkpoint_id)
    }

    /// List all available checkpoints for a session
    pub fn list_checkpoints(&self, session_id: &str) -> io::Result<Vec<String>> {
        self.storage.list_checkpoints(session_id)
    }

    /// Check if a checkpoint can be loaded
    pub fn can_load_checkpoint(&self, session_id: &str, checkpoint_id: &str) -> bool {
        self.storage.checkpoint_exists(session_id, checkpoint_id)
            && self
                .storage
                .verify_checkpoint(session_id, checkpoint_id)
                .unwrap_or(false)
    }

    /// Load the latest checkpoint for a session
    pub fn load_latest_checkpoint(&self, session_id: &str) -> io::Result<Checkpoint> {
        let checkpoint_ids = self.storage.list_checkpoints(session_id)?;

        let latest_id = checkpoint_ids.last().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("No checkpoints found for session {}", session_id),
            )
        })?;

        self.load_checkpoint(session_id, latest_id)
    }

    /// Get the underlying storage
    pub fn storage(&self) -> &CheckpointStorage {
        &self.storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::saver::SessionSaver;
    use crate::checkpoint::types::SessionState;

    fn temp_storage() -> CheckpointStorage {
        // Use a unique name combining process ID and a random component to avoid conflicts
        let unique_id = format!(
            "checkpoint-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let temp_dir = std::env::temp_dir().join(unique_id);
        CheckpointStorage::new(temp_dir)
    }

    #[test]
    fn test_loader_creation() {
        let storage = temp_storage();
        let loader = SessionLoader::new(storage);
        assert!(loader
            .storage()
            .session_dir("test")
            .to_string_lossy()
            .contains("checkpoint-test"));
    }

    #[test]
    fn test_load_checkpoint() {
        let storage = temp_storage();
        let saver = SessionSaver::new(storage.clone());
        let loader = SessionLoader::new(storage);
        let session_id = format!(
            "session-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );

        // Save a checkpoint first
        let state = SessionState::new("/project");
        let checkpoint = Checkpoint::new("cp-001", 1, 1000, state);
        saver.save_checkpoint(&session_id, &checkpoint).unwrap();

        // Load it back
        let loaded = loader.load_checkpoint(&session_id, "cp-001");
        assert!(loaded.is_ok());
        assert_eq!(loaded.unwrap().id, "cp-001");

        // Cleanup
        let _ = std::fs::remove_dir_all(loader.storage().session_dir(&session_id));
    }

    #[test]
    fn test_load_session() {
        let storage = temp_storage();
        let saver = SessionSaver::new(storage.clone());
        let loader = SessionLoader::new(storage);
        let session_id = format!(
            "session-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );

        // Create and save a session
        let mut session = Session::new(&session_id, 10);
        session.create_checkpoint(Some("First".to_string()));
        session.create_checkpoint(Some("Second".to_string()));
        saver.save_session(&session).unwrap();

        // Load it back
        let loaded_session = loader.load_session(&session_id, 10);
        assert!(loaded_session.is_ok());

        let loaded = loaded_session.unwrap();
        assert_eq!(loaded.id, session_id);
        assert_eq!(loaded.history.len(), 2);

        // Cleanup
        let _ = std::fs::remove_dir_all(loader.storage().session_dir(&session_id));
    }

    #[test]
    fn test_can_load_checkpoint() {
        let storage = temp_storage();
        let loader = SessionLoader::new(storage);
        let session_id = format!(
            "session-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );

        // Non-existent checkpoint should return false
        assert!(!loader.can_load_checkpoint(&session_id, "cp-nonexistent"));
    }

    #[test]
    fn test_list_checkpoints() {
        let storage = temp_storage();
        let saver = SessionSaver::new(storage.clone());
        let loader = SessionLoader::new(storage);
        let session_id = format!(
            "session-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );

        // Create and save checkpoints
        let state = SessionState::new("/project");
        let cp1 = Checkpoint::new("cp-001", 1, 1000, state.clone());
        let cp2 = Checkpoint::new("cp-002", 2, 2000, state);

        saver.save_checkpoint(&session_id, &cp1).unwrap();
        saver.save_checkpoint(&session_id, &cp2).unwrap();

        // List checkpoints
        let checkpoint_ids = loader.list_checkpoints(&session_id).unwrap();
        assert_eq!(checkpoint_ids.len(), 2);
        assert!(checkpoint_ids.contains(&"cp-001".to_string()));
        assert!(checkpoint_ids.contains(&"cp-002".to_string()));

        // Cleanup
        let _ = std::fs::remove_dir_all(loader.storage().session_dir(&session_id));
    }
}
