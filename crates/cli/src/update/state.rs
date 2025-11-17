//! Update state persistence module
//!
//! Manages persistent storage of update state including downloads,
//! verifications, and backup information across application runs.

use crate::update::error::UpdateError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Status of an update operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UpdateStatus {
    /// Pending - update detected but not downloaded yet
    Pending,
    /// Downloading - binary is being downloaded
    Downloading,
    /// Downloaded - binary downloaded, waiting for verification
    Downloaded,
    /// Verified - checksum verified successfully
    Verified,
    /// Backed up - original binary backed up
    BackedUp,
    /// Installed - new binary installed
    Installed,
    /// Failed - update process failed
    Failed,
    /// Rolled back - rolled back to previous version
    RolledBack,
}

impl std::fmt::Display for UpdateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Downloading => write!(f, "Downloading"),
            Self::Downloaded => write!(f, "Downloaded"),
            Self::Verified => write!(f, "Verified"),
            Self::BackedUp => write!(f, "BackedUp"),
            Self::Installed => write!(f, "Installed"),
            Self::Failed => write!(f, "Failed"),
            Self::RolledBack => write!(f, "RolledBack"),
        }
    }
}

/// Record of a single update attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecord {
    /// Target version being updated to
    pub target_version: String,
    /// Current status of the update
    pub status: UpdateStatus,
    /// URL the binary was downloaded from
    pub download_url: Option<String>,
    /// SHA256 checksum of the downloaded binary
    pub checksum: Option<String>,
    /// Path to the downloaded binary
    pub binary_path: Option<PathBuf>,
    /// Path to the backup of the original binary
    pub backup_path: Option<PathBuf>,
    /// Timestamp when the update record was created
    pub created_at: DateTime<Utc>,
    /// Timestamp of the last status update
    pub updated_at: DateTime<Utc>,
    /// Error message if the update failed
    pub error_message: Option<String>,
}

impl UpdateRecord {
    /// Create a new update record for a target version
    pub fn new(target_version: String) -> Self {
        let now = Utc::now();
        Self {
            target_version,
            status: UpdateStatus::Pending,
            download_url: None,
            checksum: None,
            binary_path: None,
            backup_path: None,
            created_at: now,
            updated_at: now,
            error_message: None,
        }
    }

    /// Update the status of the record
    pub fn set_status(&mut self, status: UpdateStatus) {
        self.status = status;
        self.updated_at = Utc::now();
        self.error_message = None;
    }

    /// Set status to failed with an error message
    pub fn set_failed(&mut self, error: String) {
        self.status = UpdateStatus::Failed;
        self.updated_at = Utc::now();
        self.error_message = Some(error);
    }

    /// Check if the update is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.status, UpdateStatus::Installed | UpdateStatus::Failed | UpdateStatus::RolledBack)
    }

    /// Check if the update is in a retryable state
    pub fn is_retryable(&self) -> bool {
        matches!(self.status, UpdateStatus::Pending | UpdateStatus::Failed)
    }
}

/// Persistent state manager for update operations
pub struct UpdateStateManager {
    /// Path to the state file
    state_file: PathBuf,
}

impl UpdateStateManager {
    /// Create a new state manager with the default state file location
    ///
    /// The default state file is `~/.rusty/update_state.json`
    pub fn new() -> Result<Self, UpdateError> {
        let state_file = Self::get_default_state_file()?;
        Self::with_file(state_file)
    }

    /// Create a new state manager with a custom state file location
    pub fn with_file<P: AsRef<Path>>(state_file: P) -> Result<Self, UpdateError> {
        let state_file = state_file.as_ref().to_path_buf();

        // Create the parent directory if it doesn't exist
        if let Some(parent) = state_file.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| UpdateError::StatePersistenceFailed(format!("Failed to create state directory: {}", e)))?;
            }
        }

        debug!("State manager initialized with file: {:?}", state_file);
        Ok(Self { state_file })
    }

    /// Get the default state file location (~/.rusty/update_state.json)
    fn get_default_state_file() -> Result<PathBuf, UpdateError> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| UpdateError::StatePersistenceFailed("Failed to determine home directory".to_string()))?;

        Ok(home_dir.join(".rusty").join("update_state.json"))
    }

    /// Load update state from the state file
    ///
    /// Returns empty map if the file doesn't exist
    pub fn load_state(&self) -> Result<std::collections::HashMap<String, UpdateRecord>, UpdateError> {
        if !self.state_file.exists() {
            debug!("State file does not exist, returning empty state");
            return Ok(std::collections::HashMap::new());
        }

        let json = fs::read_to_string(&self.state_file)
            .map_err(|e| UpdateError::StatePersistenceFailed(format!("Failed to read state file: {}", e)))?;

        let state: std::collections::HashMap<String, UpdateRecord> = serde_json::from_str(&json)
            .map_err(|e| UpdateError::InvalidStateData(format!("Failed to parse state data: {}", e)))?;

        debug!("Loaded state with {} records", state.len());
        Ok(state)
    }

    /// Save update state to the state file
    pub fn save_state(&self, state: &std::collections::HashMap<String, UpdateRecord>) -> Result<(), UpdateError> {
        let json =
            serde_json::to_string_pretty(state).map_err(|e| UpdateError::InvalidStateData(format!("Failed to serialize state: {}", e)))?;

        fs::write(&self.state_file, json)
            .map_err(|e| UpdateError::StatePersistenceFailed(format!("Failed to write state file: {}", e)))?;

        debug!("Saved state with {} records", state.len());
        Ok(())
    }

    /// Get a specific update record by version
    pub fn get_record(&self, version: &str) -> Result<Option<UpdateRecord>, UpdateError> {
        let state = self.load_state()?;
        Ok(state.get(version).cloned())
    }

    /// Create or update an update record
    pub fn upsert_record(&self, version: &str, record: UpdateRecord) -> Result<(), UpdateError> {
        let mut state = self.load_state()?;
        state.insert(version.to_string(), record);
        self.save_state(&state)
    }

    /// Delete an update record
    pub fn delete_record(&self, version: &str) -> Result<(), UpdateError> {
        let mut state = self.load_state()?;
        state.remove(version);
        self.save_state(&state)
    }

    /// Get all update records
    pub fn get_all_records(&self) -> Result<Vec<UpdateRecord>, UpdateError> {
        let state = self.load_state()?;
        let mut records: Vec<_> = state.into_values().collect();

        // Sort by created_at descending (newest first)
        records.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(records)
    }

    /// Get the most recent update record
    pub fn get_latest_record(&self) -> Result<Option<UpdateRecord>, UpdateError> {
        let records = self.get_all_records()?;
        Ok(records.into_iter().next())
    }

    /// Get incomplete update records (not finished or failed)
    pub fn get_incomplete_records(&self) -> Result<Vec<UpdateRecord>, UpdateError> {
        let records = self.get_all_records()?;
        Ok(records.into_iter().filter(|r| !r.is_complete()).collect())
    }

    /// Get failed update records
    pub fn get_failed_records(&self) -> Result<Vec<UpdateRecord>, UpdateError> {
        let records = self.get_all_records()?;
        Ok(records.into_iter().filter(|r| r.status == UpdateStatus::Failed).collect())
    }

    /// Get retryable update records
    pub fn get_retryable_records(&self) -> Result<Vec<UpdateRecord>, UpdateError> {
        let records = self.get_all_records()?;
        Ok(records.into_iter().filter(|r| r.is_retryable()).collect())
    }

    /// Clear all old records, keeping only the most recent N records per status
    pub fn cleanup_old_records(&self, keep_per_status: usize) -> Result<usize, UpdateError> {
        let mut state = self.load_state()?;
        let original_count = state.len();

        // Group records by status
        let mut records_by_status: std::collections::HashMap<UpdateStatus, Vec<(String, UpdateRecord)>> =
            std::collections::HashMap::new();

        for (version, record) in state.iter() {
            records_by_status
                .entry(record.status.clone())
                .or_default()
                .push((version.clone(), record.clone()));
        }

        // Keep only the most recent N per status
        state.clear();
        for (_, mut records) in records_by_status {
            records.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));
            for (version, record) in records.into_iter().take(keep_per_status) {
                state.insert(version, record);
            }
        }

        self.save_state(&state)?;

        let deleted_count = original_count - state.len();
        info!("Cleaned up {} old update records", deleted_count);

        Ok(deleted_count)
    }

    /// Get the state file path
    pub fn state_file(&self) -> &Path {
        &self.state_file
    }
}

impl Default for UpdateStateManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default UpdateStateManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_status_display() {
        assert_eq!(UpdateStatus::Pending.to_string(), "Pending");
        assert_eq!(UpdateStatus::Downloading.to_string(), "Downloading");
        assert_eq!(UpdateStatus::Installed.to_string(), "Installed");
        assert_eq!(UpdateStatus::Failed.to_string(), "Failed");
    }

    #[test]
    fn test_update_record_new() {
        let record = UpdateRecord::new("1.2.0".to_string());
        assert_eq!(record.target_version, "1.2.0");
        assert_eq!(record.status, UpdateStatus::Pending);
        assert!(record.download_url.is_none());
        assert!(record.error_message.is_none());
    }

    #[test]
    fn test_update_record_set_status() {
        let mut record = UpdateRecord::new("1.2.0".to_string());
        let initial_time = record.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        record.set_status(UpdateStatus::Downloaded);

        assert_eq!(record.status, UpdateStatus::Downloaded);
        assert!(record.updated_at > initial_time);
        assert!(record.error_message.is_none());
    }

    #[test]
    fn test_update_record_set_failed() {
        let mut record = UpdateRecord::new("1.2.0".to_string());
        record.set_failed("Network error".to_string());

        assert_eq!(record.status, UpdateStatus::Failed);
        assert_eq!(record.error_message, Some("Network error".to_string()));
    }

    #[test]
    fn test_update_record_is_complete() {
        let mut record = UpdateRecord::new("1.2.0".to_string());
        assert!(!record.is_complete());

        record.status = UpdateStatus::Installed;
        assert!(record.is_complete());

        record.status = UpdateStatus::Failed;
        assert!(record.is_complete());

        record.status = UpdateStatus::Downloading;
        assert!(!record.is_complete());
    }

    #[test]
    fn test_update_record_is_retryable() {
        let mut record = UpdateRecord::new("1.2.0".to_string());
        assert!(record.is_retryable());

        record.status = UpdateStatus::Failed;
        assert!(record.is_retryable());

        record.status = UpdateStatus::Installed;
        assert!(!record.is_retryable());
    }

    #[test]
    fn test_update_state_manager_with_file() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("state.json");

        let manager = UpdateStateManager::with_file(&state_file);
        assert!(manager.is_ok());
        assert!(state_file.parent().unwrap().exists());
    }

    #[test]
    fn test_update_state_manager_creates_directory() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("nested").join("dir").join("state.json");

        assert!(!state_file.parent().unwrap().exists());
        let _manager = UpdateStateManager::with_file(&state_file).expect("Failed to create manager");
        assert!(state_file.parent().unwrap().exists());
    }

    #[test]
    fn test_load_state_empty() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("state.json");

        let manager = UpdateStateManager::with_file(&state_file).expect("Failed to create manager");
        let state = manager.load_state().expect("Failed to load state");

        assert!(state.is_empty());
    }

    #[test]
    fn test_save_and_load_state() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("state.json");

        let manager = UpdateStateManager::with_file(&state_file).expect("Failed to create manager");

        let mut state = std::collections::HashMap::new();
        let record = UpdateRecord::new("1.2.0".to_string());
        state.insert("1.2.0".to_string(), record);

        manager.save_state(&state).expect("Failed to save state");
        let loaded = manager.load_state().expect("Failed to load state");

        assert_eq!(loaded.len(), 1);
        assert!(loaded.contains_key("1.2.0"));
    }

    #[test]
    fn test_upsert_record() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("state.json");

        let manager = UpdateStateManager::with_file(&state_file).expect("Failed to create manager");

        let record = UpdateRecord::new("1.2.0".to_string());
        manager.upsert_record("1.2.0", record).expect("Failed to upsert record");

        let loaded = manager.get_record("1.2.0").expect("Failed to get record");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().target_version, "1.2.0");
    }

    #[test]
    fn test_delete_record() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("state.json");

        let manager = UpdateStateManager::with_file(&state_file).expect("Failed to create manager");

        let record = UpdateRecord::new("1.2.0".to_string());
        manager.upsert_record("1.2.0", record).expect("Failed to upsert record");

        manager.delete_record("1.2.0").expect("Failed to delete record");

        let loaded = manager.get_record("1.2.0").expect("Failed to get record");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_get_all_records() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("state.json");

        let manager = UpdateStateManager::with_file(&state_file).expect("Failed to create manager");

        for i in 0..3 {
            let record = UpdateRecord::new(format!("1.{}.0", i));
            manager.upsert_record(&format!("1.{}.0", i), record).expect("Failed to upsert record");
        }

        let records = manager.get_all_records().expect("Failed to get records");
        assert_eq!(records.len(), 3);
    }

    #[test]
    fn test_get_incomplete_records() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("state.json");

        let manager = UpdateStateManager::with_file(&state_file).expect("Failed to create manager");

        let mut record1 = UpdateRecord::new("1.0.0".to_string());
        record1.status = UpdateStatus::Pending;
        manager.upsert_record("1.0.0", record1).expect("Failed to upsert record");

        let mut record2 = UpdateRecord::new("1.1.0".to_string());
        record2.status = UpdateStatus::Installed;
        manager.upsert_record("1.1.0", record2).expect("Failed to upsert record");

        let incomplete = manager.get_incomplete_records().expect("Failed to get incomplete");
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].target_version, "1.0.0");
    }

    #[test]
    fn test_cleanup_old_records() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let state_file = temp_dir.path().join("state.json");

        let manager = UpdateStateManager::with_file(&state_file).expect("Failed to create manager");

        for i in 0..5 {
            let record = UpdateRecord::new(format!("1.{}.0", i));
            manager.upsert_record(&format!("1.{}.0", i), record).expect("Failed to upsert record");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let deleted = manager.cleanup_old_records(2).expect("Failed to cleanup");
        assert!(deleted > 0);

        let remaining = manager.get_all_records().expect("Failed to get records");
        assert!(remaining.len() <= 2);
    }
}
