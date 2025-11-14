//! File system storage operations for checkpoints
//!
//! This module handles:
//! - Directory structure (.claude/sessions/)
//! - Checkpoint file persistence
//! - Metadata management
//! - File integrity verification

use super::types::Checkpoint;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Storage backend for checkpoint persistence
#[derive(Clone)]
pub struct CheckpointStorage {
    /// Base directory for sessions (.claude/sessions/)
    pub(crate) base_dir: PathBuf,
}

impl CheckpointStorage {
    /// Create a new storage with the given base directory
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Create storage in default location (.claude/sessions/)
    pub fn default() -> io::Result<Self> {
        let cwd = std::env::current_dir()?;
        let base_dir = cwd.join(".claude").join("sessions");
        Ok(Self { base_dir })
    }

    /// Ensure the sessions directory exists
    pub fn ensure_dir(&self) -> io::Result<()> {
        fs::create_dir_all(&self.base_dir)
    }

    /// Get the directory path for a specific session
    pub fn session_dir(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(session_id)
    }

    /// Get the file path for a specific checkpoint
    pub fn checkpoint_path(&self, session_id: &str, checkpoint_id: &str) -> PathBuf {
        self.session_dir(session_id)
            .join(format!("{}.json", checkpoint_id))
    }

    /// Save a checkpoint to disk
    pub fn save_checkpoint(&self, session_id: &str, checkpoint: &Checkpoint) -> io::Result<()> {
        // Ensure session directory exists
        let session_dir = self.session_dir(session_id);
        fs::create_dir_all(&session_dir)?;

        // Serialize checkpoint to JSON
        let json = checkpoint.to_json().map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Serialization failed: {}", e),
            )
        })?;

        // Write to file
        let checkpoint_path = self.checkpoint_path(session_id, &checkpoint.id);
        fs::write(checkpoint_path, json)?;

        Ok(())
    }

    /// Load a checkpoint from disk
    pub fn load_checkpoint(&self, session_id: &str, checkpoint_id: &str) -> io::Result<Checkpoint> {
        let checkpoint_path = self.checkpoint_path(session_id, checkpoint_id);
        let json = fs::read_to_string(checkpoint_path)?;

        Checkpoint::from_json(&json).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Deserialization failed: {}", e),
            )
        })
    }

    /// List all checkpoints for a session
    pub fn list_checkpoints(&self, session_id: &str) -> io::Result<Vec<String>> {
        let session_dir = self.session_dir(session_id);

        if !session_dir.exists() {
            return Ok(Vec::new());
        }

        let mut checkpoint_ids = Vec::new();
        for entry in fs::read_dir(session_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    checkpoint_ids.push(stem.to_string());
                }
            }
        }

        checkpoint_ids.sort();
        Ok(checkpoint_ids)
    }

    /// Delete a checkpoint
    pub fn delete_checkpoint(&self, session_id: &str, checkpoint_id: &str) -> io::Result<()> {
        let checkpoint_path = self.checkpoint_path(session_id, checkpoint_id);
        fs::remove_file(checkpoint_path)
    }

    /// Check if a checkpoint exists
    pub fn checkpoint_exists(&self, session_id: &str, checkpoint_id: &str) -> bool {
        self.checkpoint_path(session_id, checkpoint_id).exists()
    }

    /// Get checkpoint metadata without loading full content
    pub fn checkpoint_metadata(
        &self,
        session_id: &str,
        checkpoint_id: &str,
    ) -> io::Result<CheckpointMetadata> {
        let checkpoint_path = self.checkpoint_path(session_id, checkpoint_id);
        let metadata = fs::metadata(&checkpoint_path)?;

        Ok(CheckpointMetadata {
            checkpoint_id: checkpoint_id.to_string(),
            file_size: metadata.len(),
            modified: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64),
        })
    }

    /// Verify checkpoint file integrity
    pub fn verify_checkpoint(&self, session_id: &str, checkpoint_id: &str) -> io::Result<bool> {
        let checkpoint = self.load_checkpoint(session_id, checkpoint_id)?;
        Ok(checkpoint.verify_integrity())
    }

    /// Clean up old checkpoints for a session (keeping only the N most recent)
    pub fn cleanup_old_checkpoints(
        &self,
        session_id: &str,
        keep_count: usize,
    ) -> io::Result<usize> {
        let mut checkpoint_ids = self.list_checkpoints(session_id)?;

        if checkpoint_ids.len() <= keep_count {
            return Ok(0);
        }

        // Sort by name (which includes number) and remove oldest
        checkpoint_ids.sort();
        let to_remove = checkpoint_ids.len() - keep_count;
        let mut removed = 0;

        for checkpoint_id in checkpoint_ids.iter().take(to_remove) {
            if self.delete_checkpoint(session_id, checkpoint_id).is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Get total storage size for a session
    pub fn session_size(&self, session_id: &str) -> io::Result<u64> {
        let session_dir = self.session_dir(session_id);

        if !session_dir.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;
        for entry in fs::read_dir(session_dir)? {
            let entry = entry?;
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }
}

/// Metadata about a stored checkpoint
#[derive(Debug, Clone)]
pub struct CheckpointMetadata {
    pub checkpoint_id: String,
    pub file_size: u64,
    pub modified: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_creation() {
        let storage = CheckpointStorage::new("/tmp/test-sessions");
        assert_eq!(storage.base_dir, PathBuf::from("/tmp/test-sessions"));
    }

    #[test]
    fn test_session_dir_path() {
        let storage = CheckpointStorage::new("/tmp/test-sessions");
        let session_dir = storage.session_dir("session-001");
        assert_eq!(session_dir, PathBuf::from("/tmp/test-sessions/session-001"));
    }

    #[test]
    fn test_checkpoint_path() {
        let storage = CheckpointStorage::new("/tmp/test-sessions");
        let checkpoint_path = storage.checkpoint_path("session-001", "checkpoint-001");
        assert_eq!(
            checkpoint_path,
            PathBuf::from("/tmp/test-sessions/session-001/checkpoint-001.json")
        );
    }

    #[test]
    fn test_checkpoint_exists_false() {
        let storage = CheckpointStorage::new("/tmp/nonexistent-sessions");
        assert!(!storage.checkpoint_exists("session-001", "checkpoint-001"));
    }
}
