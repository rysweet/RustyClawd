//! Session saving logic
//!
//! This module handles:
//! - Automatic checkpoint creation before edits
//! - On-demand checkpoint saving
//! - Batch checkpoint operations
//! - Checkpoint retention policy enforcement

use super::storage::CheckpointStorage;
use super::types::{Checkpoint, Session};
use std::io;

/// Session saver for checkpoint creation and persistence
pub struct SessionSaver {
    storage: CheckpointStorage,
}

impl SessionSaver {
    /// Create a new session saver
    pub fn new(storage: CheckpointStorage) -> Self {
        Self { storage }
    }

    /// Create with default storage location
    pub fn default() -> io::Result<Self> {
        let storage = CheckpointStorage::default()?;
        Ok(Self { storage })
    }

    /// Save a single checkpoint to disk
    pub fn save_checkpoint(&self, session_id: &str, checkpoint: &Checkpoint) -> io::Result<()> {
        // Verify checkpoint integrity before saving
        if !checkpoint.verify_integrity() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Checkpoint integrity check failed",
            ));
        }

        self.storage.save_checkpoint(session_id, checkpoint)
    }

    /// Save an entire session (all checkpoints in history)
    pub fn save_session(&self, session: &Session) -> io::Result<()> {
        self.storage.ensure_dir()?;

        let session_id = &session.id;
        for checkpoint in session.history.all() {
            self.save_checkpoint(session_id, checkpoint)?;
        }

        Ok(())
    }

    /// Create and save a checkpoint automatically
    pub fn auto_checkpoint(
        &self,
        session: &mut Session,
        description: Option<String>,
    ) -> io::Result<String> {
        let checkpoint_id = session.create_checkpoint(description);

        // Save the newly created checkpoint
        if let Some(checkpoint) = session.history.get(&checkpoint_id) {
            self.save_checkpoint(&session.id, checkpoint)?;
        }

        Ok(checkpoint_id)
    }

    /// Save a checkpoint and enforce retention policy
    pub fn save_with_retention(
        &self,
        session_id: &str,
        checkpoint: &Checkpoint,
        max_checkpoints: usize,
    ) -> io::Result<()> {
        // Save the new checkpoint
        self.save_checkpoint(session_id, checkpoint)?;

        // Clean up old checkpoints
        self.storage
            .cleanup_old_checkpoints(session_id, max_checkpoints)?;

        Ok(())
    }

    /// Batch save multiple checkpoints
    pub fn save_batch(&self, session_id: &str, checkpoints: &[Checkpoint]) -> io::Result<usize> {
        let mut saved = 0;
        for checkpoint in checkpoints {
            if self.save_checkpoint(session_id, checkpoint).is_ok() {
                saved += 1;
            }
        }
        Ok(saved)
    }

    /// Get the underlying storage
    pub fn storage(&self) -> &CheckpointStorage {
        &self.storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::types::{SessionState};
    use std::path::PathBuf;

    fn temp_storage() -> CheckpointStorage {
        // Use a unique name combining process ID and a random component to avoid conflicts
        let unique_id = format!("checkpoint-test-{}-{}", std::process::id(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos());
        let temp_dir = std::env::temp_dir().join(unique_id);
        CheckpointStorage::new(temp_dir)
    }

    #[test]
    fn test_saver_creation() {
        let storage = temp_storage();
        let saver = SessionSaver::new(storage);
        assert!(saver.storage().session_dir("test").to_string_lossy().contains("checkpoint-test"));
    }

    #[test]
    fn test_save_checkpoint() {
        let storage = temp_storage();
        let saver = SessionSaver::new(storage);
        let session_id = format!("session-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos());

        let state = SessionState::new("/project");
        let checkpoint = Checkpoint::new("cp-001", 1, 1000, state);

        let result = saver.save_checkpoint(&session_id, &checkpoint);
        assert!(result.is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(saver.storage().session_dir(&session_id));
    }

    #[test]
    fn test_save_session() {
        let storage = temp_storage();
        let saver = SessionSaver::new(storage);
        let session_id = format!("session-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos());
        let mut session = Session::new(&session_id, 10);

        session.create_checkpoint(Some("First checkpoint".to_string()));
        session.create_checkpoint(Some("Second checkpoint".to_string()));

        let result = saver.save_session(&session);
        assert!(result.is_ok());

        // Verify checkpoints were saved
        let checkpoint_ids = saver.storage().list_checkpoints(&session_id).unwrap();
        assert_eq!(checkpoint_ids.len(), 2);

        // Cleanup
        let _ = std::fs::remove_dir_all(saver.storage().session_dir(&session_id));
    }
}
