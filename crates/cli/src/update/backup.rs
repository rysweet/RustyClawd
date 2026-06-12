//! Backup management system for safe binary updates
//!
//! Manages timestamped backups of the current binary before replacement,
//! allowing for easy rollback if needed.

use crate::update::error::UpdateError;
use chrono::{DateTime, Local, TimeZone};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Represents a single backup entry
#[derive(Debug, Clone)]
pub struct BackupEntry {
    /// Original binary path (None when recovered from backup listing)
    pub original_path: Option<PathBuf>,
    /// Backup file path
    pub backup_path: PathBuf,
    /// Timestamp of when the backup was created
    pub timestamp: DateTime<Local>,
    /// File size in bytes
    pub file_size: u64,
}

impl BackupEntry {
    /// Format the timestamp for display
    pub fn timestamp_str(&self) -> String {
        self.timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    /// Get the backup filename
    pub fn backup_filename(&self) -> String {
        self.timestamp.format("rusty.%Y%m%d-%H%M%S").to_string()
    }
}

/// Backup manager for handling binary backups
pub struct BackupManager {
    /// Base backup directory (typically ~/.rusty/backups/)
    backup_dir: PathBuf,
}

impl BackupManager {
    /// Create a new backup manager with the default backup directory
    ///
    /// The default backup directory is `~/.rusty/backups/`
    pub fn new() -> Result<Self, UpdateError> {
        let backup_dir = Self::get_default_backup_dir()?;
        Self::with_directory(backup_dir)
    }

    /// Create a new backup manager with a custom backup directory
    pub fn with_directory<P: AsRef<Path>>(backup_dir: P) -> Result<Self, UpdateError> {
        let backup_dir = backup_dir.as_ref().to_path_buf();

        // Create the backup directory if it doesn't exist
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir).map_err(|e| {
                UpdateError::BackupFailed(format!("Failed to create backup directory: {}", e))
            })?;
        }

        debug!(
            "Backup manager initialized with directory: {:?}",
            backup_dir
        );
        Ok(Self { backup_dir })
    }

    /// Get the default backup directory (~/.rusty/backups/)
    fn get_default_backup_dir() -> Result<PathBuf, UpdateError> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            UpdateError::BackupFailed("Failed to determine home directory".to_string())
        })?;

        Ok(home_dir.join(".rusty").join("backups"))
    }

    /// Create a backup of the current binary
    ///
    /// # Arguments
    /// * `binary_path` - Path to the current binary to backup
    ///
    /// # Returns
    /// BackupEntry containing information about the created backup
    pub fn backup_binary<P: AsRef<Path>>(
        &self,
        binary_path: P,
    ) -> Result<BackupEntry, UpdateError> {
        let binary_path = binary_path.as_ref();

        if !binary_path.exists() {
            return Err(UpdateError::BackupFailed(format!(
                "Binary not found at: {:?}",
                binary_path
            )));
        }

        let file_size = fs::metadata(binary_path)
            .map_err(|e| {
                UpdateError::BackupFailed(format!("Failed to read binary metadata: {}", e))
            })?
            .len();

        let now = Local::now();
        let backup_filename = now.format("rusty.%Y%m%d-%H%M%S").to_string();
        let backup_path = self.backup_dir.join(&backup_filename);

        info!("Creating backup of {:?} to {:?}", binary_path, backup_path);

        fs::copy(binary_path, &backup_path).map_err(|e| {
            UpdateError::BackupFailed(format!("Failed to copy binary to backup: {}", e))
        })?;

        // Set executable permissions on the backup (Unix-like systems)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&backup_path, permissions).map_err(|e| {
                UpdateError::BackupFailed(format!("Failed to set backup permissions: {}", e))
            })?;
        }

        debug!("Backup created successfully: {:?}", backup_path);

        Ok(BackupEntry {
            original_path: Some(binary_path.to_path_buf()),
            backup_path,
            timestamp: now,
            file_size,
        })
    }

    /// List all backups in the backup directory
    pub fn list_backups(&self) -> Result<Vec<BackupEntry>, UpdateError> {
        let mut backups = Vec::new();

        let entries = fs::read_dir(&self.backup_dir).map_err(|e| {
            UpdateError::BackupFailed(format!("Failed to read backup directory: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                UpdateError::BackupFailed(format!("Failed to read backup entry: {}", e))
            })?;

            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("rusty."))
                    == Some(true)
            {
                if let Ok(metadata) = fs::metadata(&path) {
                    let file_size = metadata.len();

                    // Try to extract timestamp from filename
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        // Format: rusty.YYYYMMDD-HHMMSS
                        let timestamp = Self::parse_backup_filename(filename);

                        if let Some(ts) = timestamp {
                            backups.push(BackupEntry {
                                original_path: None,
                                backup_path: path,
                                timestamp: ts,
                                file_size,
                            });
                        }
                    }
                }
            }
        }

        // Sort by timestamp descending (newest first)
        backups.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        debug!("Found {} backups", backups.len());

        Ok(backups)
    }

    /// Get a specific backup by filename
    pub fn get_backup<S: AsRef<str>>(&self, filename: S) -> Result<BackupEntry, UpdateError> {
        let filename = filename.as_ref();
        let backup_path = self.backup_dir.join(filename);

        if !backup_path.exists() {
            return Err(UpdateError::BackupFailed(format!(
                "Backup not found: {}",
                filename
            )));
        }

        let metadata = fs::metadata(&backup_path).map_err(|e| {
            UpdateError::BackupFailed(format!("Failed to read backup metadata: {}", e))
        })?;

        let timestamp = Self::parse_backup_filename(filename).ok_or_else(|| {
            UpdateError::BackupFailed(format!("Invalid backup filename format: {}", filename))
        })?;

        Ok(BackupEntry {
            original_path: None,
            backup_path,
            timestamp,
            file_size: metadata.len(),
        })
    }

    /// Restore a backup to a target location
    ///
    /// # Arguments
    /// * `backup_entry` - The backup to restore
    /// * `target_path` - Where to restore the backup to
    pub fn restore_backup(
        &self,
        backup_entry: &BackupEntry,
        target_path: &Path,
    ) -> Result<(), UpdateError> {
        if !backup_entry.backup_path.exists() {
            return Err(UpdateError::BackupFailed(format!(
                "Backup file not found: {:?}",
                backup_entry.backup_path
            )));
        }

        info!(
            "Restoring backup from {:?} to {:?}",
            backup_entry.backup_path, target_path
        );

        fs::copy(&backup_entry.backup_path, target_path)
            .map_err(|e| UpdateError::BackupFailed(format!("Failed to restore backup: {}", e)))?;

        // Set executable permissions on the restored binary (Unix-like systems)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(target_path, permissions).map_err(|e| {
                UpdateError::BackupFailed(format!(
                    "Failed to set restored binary permissions: {}",
                    e
                ))
            })?;
        }

        debug!("Backup restored successfully");
        Ok(())
    }

    /// Delete a specific backup
    pub fn delete_backup(&self, backup_entry: &BackupEntry) -> Result<(), UpdateError> {
        if !backup_entry.backup_path.exists() {
            return Err(UpdateError::BackupFailed(format!(
                "Backup file not found: {:?}",
                backup_entry.backup_path
            )));
        }

        info!("Deleting backup: {:?}", backup_entry.backup_path);

        fs::remove_file(&backup_entry.backup_path)
            .map_err(|e| UpdateError::BackupFailed(format!("Failed to delete backup: {}", e)))?;

        debug!("Backup deleted successfully");
        Ok(())
    }

    /// Clean up old backups, keeping only the most recent N
    ///
    /// # Arguments
    /// * `keep_count` - Number of most recent backups to keep
    pub fn cleanup_old_backups(&self, keep_count: usize) -> Result<usize, UpdateError> {
        let backups = self.list_backups()?;

        if backups.len() <= keep_count {
            debug!("No old backups to clean up");
            return Ok(0);
        }

        let mut deleted_count = 0;
        for backup in backups.iter().skip(keep_count) {
            self.delete_backup(backup)?;
            deleted_count += 1;
        }

        info!("Cleaned up {} old backups", deleted_count);
        Ok(deleted_count)
    }

    /// Parse a backup filename and extract the timestamp
    ///
    /// Expected format: rusty.YYYYMMDD-HHMMSS
    fn parse_backup_filename(filename: &str) -> Option<DateTime<Local>> {
        if !filename.starts_with("rusty.") {
            return None;
        }

        let timestamp_str = filename.strip_prefix("rusty.")?;

        // Parse YYYYMMDD-HHMMSS
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y%m%d-%H%M%S") {
            if let chrono::MappedLocalTime::Single(datetime) = Local.from_local_datetime(&dt) {
                return Some(datetime);
            }
        }

        None
    }

    /// Get the backup directory path
    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }
}

impl Default for BackupManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default BackupManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_entry_timestamp_str() {
        let now = Local::now();
        let entry = BackupEntry {
            original_path: Some(PathBuf::from("/tmp/original")),
            backup_path: PathBuf::from("/tmp/backup"),
            timestamp: now,
            file_size: 1024,
        };

        let ts_str = entry.timestamp_str();
        assert!(!ts_str.is_empty());
        assert!(ts_str.contains(":"));
    }

    #[test]
    fn test_backup_entry_filename() {
        let now = Local::now();
        let entry = BackupEntry {
            original_path: Some(PathBuf::from("/tmp/original")),
            backup_path: PathBuf::from("/tmp/backup"),
            timestamp: now,
            file_size: 1024,
        };

        let filename = entry.backup_filename();
        assert!(filename.starts_with("rusty."));
        assert!(filename.contains("-"));
    }

    #[test]
    fn test_backup_manager_with_directory() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let backup_dir = temp_dir.path().to_path_buf();

        let manager = BackupManager::with_directory(&backup_dir);
        assert!(manager.is_ok());
        assert!(backup_dir.exists());
    }

    #[test]
    fn test_backup_manager_creates_directory() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let backup_dir = temp_dir.path().join("backups").join("nested");

        assert!(!backup_dir.exists());
        let _manager =
            BackupManager::with_directory(&backup_dir).expect("Failed to create manager");
        assert!(backup_dir.exists());
    }

    #[test]
    fn test_parse_backup_filename_valid() {
        let timestamp = BackupManager::parse_backup_filename("rusty.20240115-143022");
        assert!(timestamp.is_some());
    }

    #[test]
    fn test_parse_backup_filename_invalid_format() {
        let timestamp = BackupManager::parse_backup_filename("rusty.invalid");
        assert!(timestamp.is_none());
    }

    #[test]
    fn test_parse_backup_filename_wrong_prefix() {
        let timestamp = BackupManager::parse_backup_filename("backup.20240115-143022");
        assert!(timestamp.is_none());
    }

    #[test]
    fn test_backup_binary() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let binary_path = temp_dir.path().join("test_binary");
        fs::write(&binary_path, b"test content").expect("Failed to write test binary");

        let backup_dir = temp_dir.path().join("backups");
        let manager = BackupManager::with_directory(&backup_dir).expect("Failed to create manager");

        let backup_entry = manager
            .backup_binary(&binary_path)
            .expect("Failed to create backup");

        assert!(backup_entry.backup_path.exists());
        assert_eq!(backup_entry.file_size, 12);
    }

    #[test]
    fn test_backup_binary_nonexistent() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let backup_dir = temp_dir.path().join("backups");
        let manager = BackupManager::with_directory(&backup_dir).expect("Failed to create manager");

        let result = manager.backup_binary("/nonexistent/binary");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_backups() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let binary_path = temp_dir.path().join("test_binary");
        fs::write(&binary_path, b"test content").expect("Failed to write test binary");

        let backup_dir = temp_dir.path().join("backups");
        let manager = BackupManager::with_directory(&backup_dir).expect("Failed to create manager");

        // Create a few backups
        manager
            .backup_binary(&binary_path)
            .expect("Failed to create backup 1");
        std::thread::sleep(std::time::Duration::from_millis(1100)); // Sleep 1.1 seconds to ensure different timestamps
        manager
            .backup_binary(&binary_path)
            .expect("Failed to create backup 2");

        let backups = manager.list_backups().expect("Failed to list backups");
        assert_eq!(backups.len(), 2);
    }

    #[test]
    fn test_restore_backup() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let binary_path = temp_dir.path().join("test_binary");
        fs::write(&binary_path, b"original content").expect("Failed to write test binary");

        let backup_dir = temp_dir.path().join("backups");
        let manager = BackupManager::with_directory(&backup_dir).expect("Failed to create manager");

        let backup_entry = manager
            .backup_binary(&binary_path)
            .expect("Failed to create backup");

        // Modify the original
        fs::write(&binary_path, b"modified content").expect("Failed to modify original");

        // Restore the backup
        let restore_path = temp_dir.path().join("restored");
        manager
            .restore_backup(&backup_entry, &restore_path)
            .expect("Failed to restore backup");

        let restored_content = fs::read(&restore_path).expect("Failed to read restored file");
        assert_eq!(restored_content, b"original content");
    }

    #[test]
    fn test_cleanup_old_backups() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let binary_path = temp_dir.path().join("test_binary");
        fs::write(&binary_path, b"test content").expect("Failed to write test binary");

        let backup_dir = temp_dir.path().join("backups");
        let manager = BackupManager::with_directory(&backup_dir).expect("Failed to create manager");

        // Create 5 backups with enough delay between them to ensure unique timestamps
        for _ in 0..5 {
            manager
                .backup_binary(&binary_path)
                .expect("Failed to create backup");
            std::thread::sleep(std::time::Duration::from_millis(1100)); // Sleep 1.1 seconds
        }

        let deleted = manager.cleanup_old_backups(2).expect("Failed to cleanup");
        assert_eq!(deleted, 3);

        let remaining = manager.list_backups().expect("Failed to list backups");
        assert_eq!(remaining.len(), 2);
    }
}
