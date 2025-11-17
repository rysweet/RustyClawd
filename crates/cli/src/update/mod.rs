//! Update mechanism module
//!
//! Handles version detection, checking for updates from GitHub releases,
//! and managing update configuration.
//!
//! # Example
//!
//! ```ignore
//! use rustyclawd::update::{Version, GitHubClient, UpdateConfig};
//!
//! let current = Version::current();
//! let mut config = UpdateConfig::new();
//!
//! let client = GitHubClient::new("rysweet", "RustyClawd");
//! if let Ok(Some(update_info)) = client.get_update_info(&current).await {
//!     println!("Update available: {}", update_info.summary());
//! }
//! ```

pub mod backup;
pub mod config;
pub mod downloader;
pub mod error;
pub mod github_client;
pub mod handler;
pub mod installer;
pub mod scheduler;
pub mod state;
pub mod version;

// Re-export public API
pub use backup::{BackupEntry, BackupManager};
pub use config::UpdateConfig;
pub use downloader::{BinaryDownload, BinaryDownloader, DownloadConfig};
pub use error::UpdateError;
pub use github_client::{GitHubClient, Release, ReleaseAsset, UpdateInfo};
pub use handler::{
    format_update_message, handle_check_updates, handle_install_update, handle_rollback,
    UpdateOperationResult,
};
pub use installer::{BinaryInstaller, InstallResult, InstallerConfig};
pub use scheduler::{ScheduledCheckResult, UpdateScheduler};
pub use state::{UpdateRecord, UpdateStateManager, UpdateStatus};
pub use version::Version;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_version_and_config_integration() {
        let current = Version::new(1, 0, 0);
        let mut config = UpdateConfig::new();

        assert!(config.should_check_now());
        config.update_last_check();
        assert!(!config.should_check_now());

        assert!(current.is_less_than(&Version::new(1, 1, 0)));
        assert!(Version::new(1, 1, 0).is_update_available(&current));
    }

    #[test]
    fn test_public_api_exports() {
        let version = Version::new(1, 0, 0);
        let config = UpdateConfig::new();

        assert_eq!(version.major, 1);
        assert!(config.auto_check);
    }

    #[test]
    fn test_update_record_lifecycle() {
        let mut record = UpdateRecord::new("1.2.0".to_string());
        assert_eq!(record.status, UpdateStatus::Pending);

        record.set_status(UpdateStatus::Downloaded);
        assert_eq!(record.status, UpdateStatus::Downloaded);

        record.set_status(UpdateStatus::Verified);
        assert_eq!(record.status, UpdateStatus::Verified);

        record.download_url = Some("https://example.com/binary".to_string());
        record.checksum = Some("abc123".to_string());
        record.binary_path = Some(PathBuf::from("/tmp/binary"));

        record.set_status(UpdateStatus::BackedUp);
        record.backup_path = Some(PathBuf::from("/tmp/backup"));

        record.set_status(UpdateStatus::Installed);
        assert!(record.is_complete());
    }

    #[test]
    fn test_binary_download_structure() {
        let download = BinaryDownload::new(
            "https://github.com/example/releases/download/v1.0.0/binary".to_string(),
            "abc123def456".to_string(),
            PathBuf::from("/tmp/binary"),
        );

        assert_eq!(download.url, "https://github.com/example/releases/download/v1.0.0/binary");
        assert_eq!(download.expected_sha256, "abc123def456");
        assert!(!download.verified);
    }

    #[test]
    fn test_download_config_structure() {
        let config = DownloadConfig {
            timeout_secs: 60,
            report_progress: true,
        };

        assert_eq!(config.timeout_secs, 60);
        assert!(config.report_progress);
    }

    // Phase 3 Integration Tests: Atomic Replacement and Rollback

    #[test]
    fn test_full_update_cycle_with_backup_and_rollback() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");

        // Setup: Create initial binary with v1.0.0 content
        let binary_path = temp_dir.path().join("rusty");
        fs::write(&binary_path, b"version 1.0.0 binary").expect("Failed to write binary");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&binary_path, permissions).expect("Failed to set permissions");
        }

        // Step 1: Create and test a new binary (v1.1.0)
        let new_binary = temp_dir.path().join("new_rusty");
        fs::write(&new_binary, b"version 1.1.0 binary").expect("Failed to write new binary");

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

        // Step 2: Install new version
        let install_result = installer.install_update(&new_binary, &binary_path)
            .expect("Failed to install update");

        assert!(install_result.success);
        assert!(install_result.backup_path.is_some());

        // Verify new binary is installed
        let current_content = fs::read(&binary_path).expect("Failed to read current binary");
        assert_eq!(current_content, b"version 1.1.0 binary");

        // Step 3: Verify backup was created and contains old version
        let backup_path = install_result.backup_path.unwrap();
        assert!(backup_path.exists());
        let backup_content = fs::read(&backup_path).expect("Failed to read backup");
        assert_eq!(backup_content, b"version 1.0.0 binary");

        // Step 4: Simulate update failure by corrupting the binary
        fs::write(&binary_path, b"corrupted").expect("Failed to corrupt binary");

        // Step 5: Rollback to previous version
        installer.rollback_to_backup(&backup_path, &binary_path)
            .expect("Failed to rollback");

        // Verify rollback restored original version
        let rolled_back_content = fs::read(&binary_path).expect("Failed to read rolled back binary");
        assert_eq!(rolled_back_content, b"version 1.0.0 binary");
    }

    #[test]
    fn test_update_mechanism_with_state_persistence() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");

        // Setup binary and state
        let binary_path = temp_dir.path().join("rusty");
        fs::write(&binary_path, b"v1.0.0").expect("Failed to write binary");

        let state_file = temp_dir.path().join("update_state.json");
        let state_manager = UpdateStateManager::with_file(&state_file)
            .expect("Failed to create state manager");

        // Step 1: Create and persist update record
        let mut record = UpdateRecord::new("1.1.0".to_string());
        record.download_url = Some("https://example.com/rusty-1.1.0".to_string());
        record.set_status(UpdateStatus::Downloaded);

        state_manager.upsert_record("1.1.0", record.clone())
            .expect("Failed to upsert record");

        // Step 2: Simulate installation process with status updates
        let mut record = state_manager.get_record("1.1.0")
            .expect("Failed to get record")
            .unwrap();

        record.set_status(UpdateStatus::BackedUp);
        let backup_entry = BackupManager::new().unwrap().backup_binary(&binary_path)
            .expect("Failed to create backup");
        record.backup_path = Some(backup_entry.backup_path.clone());

        state_manager.upsert_record("1.1.0", record.clone())
            .expect("Failed to upsert updated record");

        // Step 3: Verify state persists
        let persisted = state_manager.get_record("1.1.0")
            .expect("Failed to get record")
            .unwrap();

        assert_eq!(persisted.status, UpdateStatus::BackedUp);
        assert!(persisted.backup_path.is_some());

        // Step 4: Complete the update
        let mut record = persisted;
        record.set_status(UpdateStatus::Installed);
        state_manager.upsert_record("1.1.0", record)
            .expect("Failed to upsert final record");

        // Verify final state
        let final_record = state_manager.get_record("1.1.0")
            .expect("Failed to get final record")
            .unwrap();
        assert!(final_record.is_complete());
    }

    #[test]
    fn test_multiple_backups_and_selective_rollback() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");

        let binary_path = temp_dir.path().join("rusty");
        let backup_dir = temp_dir.path().join("backups");

        let backup_manager = BackupManager::with_directory(&backup_dir)
            .expect("Failed to create backup manager");

        // Create initial binary
        fs::write(&binary_path, b"v1.0.0").expect("Failed to write binary");

        // Create multiple backups
        let _backup1 = backup_manager.backup_binary(&binary_path)
            .expect("Failed to create backup 1");

        std::thread::sleep(std::time::Duration::from_millis(1100));

        fs::write(&binary_path, b"v1.1.0").expect("Failed to update binary");
        let backup2 = backup_manager.backup_binary(&binary_path)
            .expect("Failed to create backup 2");

        std::thread::sleep(std::time::Duration::from_millis(1100));

        fs::write(&binary_path, b"v1.2.0").expect("Failed to update binary");
        let _backup3 = backup_manager.backup_binary(&binary_path)
            .expect("Failed to create backup 3");

        // List all backups
        let backups = backup_manager.list_backups()
            .expect("Failed to list backups");
        assert_eq!(backups.len(), 3);

        // Rollback to second backup (v1.1.0)
        let restore_path = temp_dir.path().join("restored");
        backup_manager.restore_backup(&backup2, &restore_path)
            .expect("Failed to restore backup 2");

        let restored_content = fs::read(&restore_path).expect("Failed to read restored");
        assert_eq!(restored_content, b"v1.1.0");

        // Cleanup old backups, keep only latest 2
        let deleted = backup_manager.cleanup_old_backups(2)
            .expect("Failed to cleanup");
        assert_eq!(deleted, 1);

        // Verify only 2 backups remain
        let remaining = backup_manager.list_backups()
            .expect("Failed to list remaining backups");
        assert_eq!(remaining.len(), 2);
    }

    #[test]
    fn test_installer_with_no_backup_option() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");

        let binary_path = temp_dir.path().join("rusty");
        fs::write(&binary_path, b"v1.0.0").expect("Failed to write binary");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&binary_path, permissions).expect("Failed to set permissions");
        }

        let new_binary = temp_dir.path().join("new_rusty");
        fs::write(&new_binary, b"v1.1.0").expect("Failed to write new binary");

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
                create_backup: false,  // No backup
                keep_backup: false,
            },
            &backup_dir,
        )
        .expect("Failed to create installer");

        let result = installer.install_update(&new_binary, &binary_path)
            .expect("Failed to install");

        assert!(result.success);
        assert!(result.backup_path.is_none());  // No backup should be created

        let content = fs::read(&binary_path).expect("Failed to read binary");
        assert_eq!(content, b"v1.1.0");
    }

    #[test]
    fn test_atomic_replacement_with_state_tracking() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");

        let binary_path = temp_dir.path().join("rusty");
        fs::write(&binary_path, b"v1.0.0").expect("Failed to write binary");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&binary_path, permissions).expect("Failed to set permissions");
        }

        let new_binary = temp_dir.path().join("new_rusty");
        fs::write(&new_binary, b"v1.1.0").expect("Failed to write new binary");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&new_binary, permissions).expect("Failed to set permissions");
        }

        // Setup state tracking
        let state_file = temp_dir.path().join("state.json");
        let state_manager = UpdateStateManager::with_file(&state_file)
            .expect("Failed to create state manager");

        let mut record = UpdateRecord::new("1.1.0".to_string());

        // Track backup operation
        let backup_dir = temp_dir.path().join("backups");
        let installer = BinaryInstaller::with_backup_dir(InstallerConfig::default(), &backup_dir)
            .expect("Failed to create installer");

        let backup_entry = installer.backup_manager()
            .backup_binary(&binary_path)
            .expect("Failed to backup");

        record.set_status(UpdateStatus::BackedUp);
        record.backup_path = Some(backup_entry.backup_path.clone());
        state_manager.upsert_record("1.1.0", record.clone())
            .expect("Failed to upsert record");

        // Perform installation
        let install_result = installer.install_update(&new_binary, &binary_path)
            .expect("Failed to install");

        assert!(install_result.success);

        // Update state with installation result
        record.set_status(UpdateStatus::Installed);
        record.binary_path = Some(install_result.installed_path);
        state_manager.upsert_record("1.1.0", record)
            .expect("Failed to upsert final record");

        // Verify final state
        let final_record = state_manager.get_record("1.1.0")
            .expect("Failed to get record")
            .unwrap();
        assert_eq!(final_record.status, UpdateStatus::Installed);
    }
}
