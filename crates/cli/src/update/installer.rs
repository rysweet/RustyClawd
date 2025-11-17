//! Atomic binary installer with rollback support
//!
//! Handles atomic replacement of the current binary with a new version,
//! with support for rollback in case of failure. Uses platform-specific
//! atomic operations to ensure consistency.

use crate::update::backup::BackupManager;
use crate::update::error::UpdateError;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

/// Configuration for the installer
#[derive(Debug, Clone)]
pub struct InstallerConfig {
    /// Verify binary exists and is executable before replacement
    pub verify_before_install: bool,
    /// Create a backup before replacement
    pub create_backup: bool,
    /// Keep backup after successful install
    pub keep_backup: bool,
}

impl Default for InstallerConfig {
    fn default() -> Self {
        Self {
            verify_before_install: true,
            create_backup: true,
            keep_backup: true,
        }
    }
}

/// Result of an installation operation
#[derive(Debug, Clone)]
pub struct InstallResult {
    /// Path where the new binary was installed
    pub installed_path: PathBuf,
    /// Path to the backup (if one was created)
    pub backup_path: Option<PathBuf>,
    /// Old version path (for rollback purposes)
    pub old_binary_path: Option<PathBuf>,
    /// Whether the installation was successful
    pub success: bool,
    /// Installation timestamp
    pub timestamp: chrono::DateTime<chrono::Local>,
}

/// Atomic binary installer
pub struct BinaryInstaller {
    config: InstallerConfig,
    backup_manager: BackupManager,
}

impl BinaryInstaller {
    /// Create a new installer with default configuration
    pub fn new() -> Result<Self, UpdateError> {
        Self::with_config(InstallerConfig::default())
    }

    /// Create a new installer with custom configuration
    pub fn with_config(config: InstallerConfig) -> Result<Self, UpdateError> {
        let backup_manager = BackupManager::new()?;
        Ok(Self {
            config,
            backup_manager,
        })
    }

    /// Create a new installer with a custom backup directory
    pub fn with_backup_dir<P: AsRef<Path>>(config: InstallerConfig, backup_dir: P) -> Result<Self, UpdateError> {
        let backup_manager = BackupManager::with_directory(&backup_dir)?;
        Ok(Self {
            config,
            backup_manager,
        })
    }

    /// Install a new binary atomically, replacing the current one
    ///
    /// # Arguments
    /// * `new_binary_path` - Path to the new binary to install
    /// * `current_binary_path` - Path to the current binary to replace
    ///
    /// # Returns
    /// InstallResult with installation details
    ///
    /// # Error Behavior
    /// If any step fails, the original binary is left untouched. The operation
    /// is designed to be atomic on supported systems using rename().
    ///
    /// # Example
    /// ```ignore
    /// let installer = BinaryInstaller::new()?;
    /// let result = installer.install_update(
    ///     Path::new("/tmp/new_binary"),
    ///     Path::new("/usr/local/bin/rusty")
    /// )?;
    /// ```
    pub fn install_update(
        &self,
        new_binary_path: &Path,
        current_binary_path: &Path,
    ) -> Result<InstallResult, UpdateError> {
        info!("Starting atomic binary installation");
        debug!(
            "New binary: {:?}, Current binary: {:?}",
            new_binary_path, current_binary_path
        );

        // Verify new binary exists and is readable
        if !new_binary_path.exists() {
            return Err(UpdateError::IoError(format!(
                "New binary not found at: {:?}",
                new_binary_path
            )));
        }

        // Verify current binary exists
        if !current_binary_path.exists() {
            return Err(UpdateError::IoError(format!(
                "Current binary not found at: {:?}",
                current_binary_path
            )));
        }

        // Verify new binary is executable (if configured)
        #[cfg(unix)]
        if self.config.verify_before_install {
            Self::verify_executable(new_binary_path)?;
        }

        // Create backup if configured
        let backup_path = if self.config.create_backup {
            let backup_entry = self.backup_manager.backup_binary(current_binary_path)?;
            debug!("Backup created at: {:?}", backup_entry.backup_path);
            Some(backup_entry.backup_path.clone())
        } else {
            None
        };

        // Perform atomic replacement using platform-specific operations
        match self.atomic_replace(new_binary_path, current_binary_path) {
            Ok(_) => {
                info!("Binary successfully installed at: {:?}", current_binary_path);

                // Verify the new binary after installation
                #[cfg(unix)]
                Self::verify_executable(current_binary_path)?;

                Ok(InstallResult {
                    installed_path: current_binary_path.to_path_buf(),
                    backup_path,
                    old_binary_path: Some(current_binary_path.to_path_buf()),
                    success: true,
                    timestamp: chrono::Local::now(),
                })
            }
            Err(e) => {
                error!("Atomic replacement failed: {}", e);

                // If replacement failed and we have a backup, attempt rollback
                if let Some(ref backup) = backup_path {
                    warn!("Attempting rollback due to installation failure");
                    if let Err(rollback_err) = self.rollback_to_backup(backup, current_binary_path) {
                        error!("Rollback also failed: {}", rollback_err);
                        return Err(UpdateError::IoError(format!(
                            "Installation failed and rollback failed: {}",
                            rollback_err
                        )));
                    }
                    info!("Rollback completed successfully");
                }

                Err(e)
            }
        }
    }

    /// Perform atomic replacement of the binary
    ///
    /// This uses the atomic rename() operation on Unix-like systems
    /// and copy+delete on Windows.
    #[cfg(unix)]
    fn atomic_replace(&self, new_binary: &Path, current_binary: &Path) -> Result<(), UpdateError> {
        // On Unix, rename() is atomic
        // First, move new binary to a temporary location next to current binary
        let parent = current_binary
            .parent()
            .ok_or_else(|| UpdateError::IoError("Cannot determine parent directory".to_string()))?;

        let temp_name = format!(
            ".{}.tmp.{}",
            current_binary.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("binary"),
            std::process::id()
        );
        let temp_binary = parent.join(&temp_name);

        debug!("Atomic replacement using temp file: {:?}", temp_binary);

        // Copy new binary to temp location in the same directory
        fs::copy(new_binary, &temp_binary).map_err(|e| {
            UpdateError::IoError(format!("Failed to copy new binary to temp location: {}", e))
        })?;

        // Set executable permissions on temp binary
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&temp_binary, permissions).map_err(|e| {
                UpdateError::IoError(format!("Failed to set permissions on temp binary: {}", e))
            })?;
        }

        // Atomic rename - this is the critical operation
        fs::rename(&temp_binary, current_binary).map_err(|e| {
            // Clean up temp file on error
            let _ = fs::remove_file(&temp_binary);
            UpdateError::IoError(format!("Atomic rename failed: {}", e))
        })?;

        debug!("Atomic replacement completed successfully");
        Ok(())
    }

    /// Perform atomic replacement on Windows
    #[cfg(windows)]
    fn atomic_replace(&self, new_binary: &Path, current_binary: &Path) -> Result<(), UpdateError> {
        // On Windows, rename() doesn't overwrite existing files, so we need a different approach
        // Move current binary to a backup location first, then move new binary to target location
        let parent = current_binary
            .parent()
            .ok_or_else(|| UpdateError::IoError("Cannot determine parent directory".to_string()))?;

        let backup_name = format!(
            ".{}.bak.{}",
            current_binary.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("binary"),
            std::process::id()
        );
        let temp_backup = parent.join(&backup_name);

        debug!("Windows atomic replacement using temp backup: {:?}", temp_backup);

        // Move current binary to temp backup location
        fs::rename(current_binary, &temp_backup).map_err(|e| {
            UpdateError::IoError(format!("Failed to move current binary to backup: {}", e))
        })?;

        // Move new binary to target location
        if let Err(e) = fs::rename(new_binary, current_binary) {
            // Restore original binary from backup
            let restore_err = fs::rename(&temp_backup, current_binary);
            return Err(UpdateError::IoError(format!(
                "Failed to move new binary to target (restore: {}): {}",
                restore_err.is_ok(),
                e
            )));
        }

        // Clean up the temporary backup
        let _ = fs::remove_file(&temp_backup);

        debug!("Windows atomic replacement completed successfully");
        Ok(())
    }

    /// Perform atomic replacement on other platforms (fallback)
    #[cfg(not(any(unix, windows)))]
    fn atomic_replace(&self, new_binary: &Path, current_binary: &Path) -> Result<(), UpdateError> {
        // Fallback: simple copy and replace
        // This is not atomic but better than nothing
        fs::copy(new_binary, current_binary).map_err(|e| {
            UpdateError::IoError(format!("Failed to copy new binary: {}", e))
        })?;

        Ok(())
    }

    /// Verify that a binary is executable (Unix only)
    #[cfg(unix)]
    fn verify_executable(binary_path: &Path) -> Result<(), UpdateError> {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(binary_path).map_err(|e| {
            UpdateError::IoError(format!("Failed to read binary metadata: {}", e))
        })?;

        let permissions = metadata.permissions();
        let mode = permissions.mode();

        // Check if any execute bit is set
        if (mode & 0o111) == 0 {
            return Err(UpdateError::IoError(
                "Binary is not executable".to_string(),
            ));
        }

        Ok(())
    }

    /// Verify that a binary is executable (non-Unix fallback)
    #[cfg(not(unix))]
    fn verify_executable(_binary_path: &Path) -> Result<(), UpdateError> {
        // On non-Unix platforms, we can't easily verify executability
        Ok(())
    }

    /// Rollback to a previous binary from backup
    ///
    /// # Arguments
    /// * `backup_path` - Path to the backup binary
    /// * `target_path` - Where to restore the backup to
    ///
    /// # Returns
    /// Ok(()) on successful rollback
    pub fn rollback_to_backup(&self, backup_path: &Path, target_path: &Path) -> Result<(), UpdateError> {
        info!("Initiating rollback from backup");
        debug!("Backup: {:?}, Target: {:?}", backup_path, target_path);

        if !backup_path.exists() {
            return Err(UpdateError::BackupFailed(format!(
                "Backup file not found: {:?}",
                backup_path
            )));
        }

        // For atomic rollback, we use the same technique as installation
        let parent = target_path
            .parent()
            .ok_or_else(|| UpdateError::IoError("Cannot determine parent directory".to_string()))?;

        let temp_name = format!(
            ".{}.rollback.{}",
            target_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("binary"),
            std::process::id()
        );
        let temp_restored = parent.join(&temp_name);

        // Copy backup to temp location
        fs::copy(backup_path, &temp_restored).map_err(|e| {
            UpdateError::BackupFailed(format!("Failed to copy backup for rollback: {}", e))
        })?;

        // Set executable permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&temp_restored, permissions).map_err(|e| {
                UpdateError::BackupFailed(format!("Failed to set rollback binary permissions: {}", e))
            })?;
        }

        // Atomic rename
        fs::rename(&temp_restored, target_path).map_err(|e| {
            let _ = fs::remove_file(&temp_restored);
            UpdateError::BackupFailed(format!("Atomic rollback rename failed: {}", e))
        })?;

        info!("Rollback completed successfully");
        Ok(())
    }

    /// Rollback to a backup by filename
    ///
    /// # Arguments
    /// * `backup_filename` - Name of the backup (e.g., "rusty.20240115-143022")
    /// * `target_path` - Where to restore the backup to
    pub fn rollback_by_backup_name(
        &self,
        backup_filename: &str,
        target_path: &Path,
    ) -> Result<(), UpdateError> {
        let backup_entry = self.backup_manager.get_backup(backup_filename)?;
        self.rollback_to_backup(&backup_entry.backup_path, target_path)
    }

    /// Get the backup manager
    pub fn backup_manager(&self) -> &BackupManager {
        &self.backup_manager
    }
}

impl Default for BinaryInstaller {
    fn default() -> Self {
        Self::new().expect("Failed to create default BinaryInstaller")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installer_config_default() {
        let config = InstallerConfig::default();
        assert!(config.verify_before_install);
        assert!(config.create_backup);
        assert!(config.keep_backup);
    }

    #[test]
    fn test_install_result_structure() {
        let result = InstallResult {
            installed_path: PathBuf::from("/usr/local/bin/rusty"),
            backup_path: Some(PathBuf::from("/home/user/.rusty/backups/rusty.20240115-143022")),
            old_binary_path: Some(PathBuf::from("/usr/local/bin/rusty")),
            success: true,
            timestamp: chrono::Local::now(),
        };

        assert!(result.success);
        assert!(result.backup_path.is_some());
    }

    #[test]
    fn test_binary_installer_creation() {
        let installer = BinaryInstaller::new();
        assert!(installer.is_ok());
    }

    #[test]
    fn test_binary_installer_with_config() {
        let config = InstallerConfig {
            verify_before_install: false,
            create_backup: false,
            keep_backup: false,
        };
        let installer = BinaryInstaller::with_config(config);
        assert!(installer.is_ok());
    }

    #[test]
    fn test_install_update_missing_new_binary() {
        let installer = BinaryInstaller::new().expect("Failed to create installer");
        let result = installer.install_update(
            Path::new("/nonexistent/new_binary"),
            Path::new("/usr/local/bin/rusty"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_install_update_missing_current_binary() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let new_binary = temp_dir.path().join("new_binary");
        fs::write(&new_binary, b"new binary content").expect("Failed to write new binary");

        let installer = BinaryInstaller::new().expect("Failed to create installer");
        let result = installer.install_update(
            &new_binary,
            Path::new("/nonexistent/current_binary"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_atomic_replace_success() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");

        // Create a current binary
        let current_binary = temp_dir.path().join("rusty");
        fs::write(&current_binary, b"current binary content").expect("Failed to write current binary");

        // Set executable permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&current_binary, permissions).expect("Failed to set permissions");
        }

        // Create a new binary
        let new_binary = temp_dir.path().join("new_binary");
        fs::write(&new_binary, b"new binary content").expect("Failed to write new binary");

        // Set executable permissions on new binary
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&new_binary, permissions).expect("Failed to set permissions");
        }

        let backup_dir = temp_dir.path().join("backups");
        let installer = BinaryInstaller::with_backup_dir(
            InstallerConfig {
                verify_before_install: false,
                create_backup: true,
                keep_backup: true,
            },
            &backup_dir,
        )
        .expect("Failed to create installer");

        let result = installer.install_update(&new_binary, &current_binary);
        assert!(result.is_ok());

        let install_result = result.unwrap();
        assert!(install_result.success);
        assert!(install_result.backup_path.is_some());

        // Verify new binary content is at current location
        let content = fs::read(&current_binary).expect("Failed to read current binary");
        assert_eq!(content, b"new binary content");
    }

    #[test]
    fn test_rollback_to_backup_success() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");

        // Create a current binary
        let current_binary = temp_dir.path().join("rusty");
        fs::write(&current_binary, b"current binary content").expect("Failed to write current binary");

        // Create backup directory and backup file
        let backup_dir = temp_dir.path().join("backups");
        fs::create_dir(&backup_dir).expect("Failed to create backup dir");

        let backup_binary = backup_dir.join("rusty.20240115-143022");
        fs::write(&backup_binary, b"backup content").expect("Failed to write backup");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&backup_binary, permissions).expect("Failed to set permissions");
        }

        let installer = BinaryInstaller::with_backup_dir(InstallerConfig::default(), &backup_dir)
            .expect("Failed to create installer");

        // Simulate a bad update
        fs::write(&current_binary, b"bad binary content").expect("Failed to write bad binary");

        // Rollback
        let result = installer.rollback_to_backup(&backup_binary, &current_binary);
        assert!(result.is_ok());

        // Verify backup was restored
        let content = fs::read(&current_binary).expect("Failed to read current binary");
        assert_eq!(content, b"backup content");
    }

    #[test]
    fn test_rollback_missing_backup() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let current_binary = temp_dir.path().join("rusty");
        let backup_dir = temp_dir.path().join("backups");
        fs::create_dir(&backup_dir).expect("Failed to create backup dir");

        let installer = BinaryInstaller::with_backup_dir(InstallerConfig::default(), &backup_dir)
            .expect("Failed to create installer");

        let result = installer.rollback_to_backup(
            Path::new("/nonexistent/backup"),
            &current_binary,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_install_update_with_backup_creation() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");

        // Create current binary
        let current_binary = temp_dir.path().join("rusty");
        fs::write(&current_binary, b"old content").expect("Failed to write current binary");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&current_binary, permissions).expect("Failed to set permissions");
        }

        // Create new binary
        let new_binary = temp_dir.path().join("new_rusty");
        fs::write(&new_binary, b"new content").expect("Failed to write new binary");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&new_binary, permissions).expect("Failed to set permissions");
        }

        let backup_dir = temp_dir.path().join("backups");
        let installer = BinaryInstaller::with_backup_dir(
            InstallerConfig {
                verify_before_install: false,
                create_backup: true,
                keep_backup: true,
            },
            &backup_dir,
        )
        .expect("Failed to create installer");

        let result = installer.install_update(&new_binary, &current_binary);
        assert!(result.is_ok());

        let install_result = result.unwrap();
        assert!(install_result.success);
        assert!(install_result.backup_path.is_some());

        // Verify backup exists
        let backup_path = install_result.backup_path.unwrap();
        assert!(backup_path.exists());

        // Verify backup contains old content
        let backup_content = fs::read(&backup_path).expect("Failed to read backup");
        assert_eq!(backup_content, b"old content");

        // Verify current binary has new content
        let current_content = fs::read(&current_binary).expect("Failed to read current binary");
        assert_eq!(current_content, b"new content");
    }

    #[test]
    fn test_install_update_without_backup() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");

        // Create current binary
        let current_binary = temp_dir.path().join("rusty");
        fs::write(&current_binary, b"old content").expect("Failed to write current binary");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&current_binary, permissions).expect("Failed to set permissions");
        }

        // Create new binary
        let new_binary = temp_dir.path().join("new_rusty");
        fs::write(&new_binary, b"new content").expect("Failed to write new binary");

        let backup_dir = temp_dir.path().join("backups");
        let installer = BinaryInstaller::with_backup_dir(
            InstallerConfig {
                verify_before_install: false,
                create_backup: false,
                keep_backup: false,
            },
            &backup_dir,
        )
        .expect("Failed to create installer");

        let result = installer.install_update(&new_binary, &current_binary);
        assert!(result.is_ok());

        let install_result = result.unwrap();
        assert!(install_result.success);
        assert!(install_result.backup_path.is_none());
    }
}
