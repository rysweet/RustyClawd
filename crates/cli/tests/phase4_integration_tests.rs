//! Phase 4 Integration Tests: Update Mechanism - Scheduler and CLI Commands
//!
//! Tests the complete end-to-end update flow including:
//! - Auto-check scheduler functionality
//! - CLI update commands (update, update --check, update --rollback)
//! - Integration with main application
//! - Scheduled checks on startup
//! - Update notifications and messages

use rustyclawd::update::{
    UpdateConfig, UpdateScheduler, Version, UpdateStateManager, UpdateRecord, UpdateStatus,
    BinaryInstaller, InstallerConfig, BackupManager,
};
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_scheduler_initialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("update_config.json");

    let scheduler = UpdateScheduler::with_config_path(config_path.clone())
        .expect("Failed to create scheduler");

    // Verify scheduler was created with default config
    assert!(scheduler.config().auto_check);
    assert_eq!(scheduler.config().check_interval_hours, 24);
}

#[test]
fn test_scheduler_should_check_on_first_startup() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("update_config.json");

    let scheduler = UpdateScheduler::with_config_path(config_path)
        .expect("Failed to create scheduler");

    // First time - should check
    assert!(scheduler.should_check_on_startup());
}

#[test]
fn test_scheduler_config_persistence_across_restarts() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("update_config.json");

    // First instance: create scheduler and update last check
    {
        let mut scheduler = UpdateScheduler::with_config_path(config_path.clone())
            .expect("Failed to create scheduler");

        scheduler.config_mut().update_last_check();
        scheduler.save_config().expect("Failed to save config");
    }

    // Second instance: verify last check was persisted
    {
        let scheduler = UpdateScheduler::with_config_path(config_path)
            .expect("Failed to create scheduler");

        assert!(scheduler.config().last_check_timestamp > 0);
    }
}

#[test]
fn test_scheduler_respects_24_hour_interval() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("update_config.json");

    let mut scheduler = UpdateScheduler::with_config_path(config_path)
        .expect("Failed to create scheduler");

    // First check
    assert!(scheduler.should_check_on_startup());

    // Simulate just updated
    scheduler.config_mut().update_last_check();

    // Should NOT check again immediately
    assert!(!scheduler.should_check_on_startup());

    // Verify interval is 24 hours
    assert_eq!(scheduler.config().check_interval_hours, 24);
}

#[test]
fn test_scheduler_auto_check_can_be_disabled() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("update_config.json");

    let mut scheduler = UpdateScheduler::with_config_path(config_path)
        .expect("Failed to create scheduler");

    // Disable auto-check
    scheduler
        .set_auto_check(false)
        .expect("Failed to disable auto-check");

    // Should never check
    assert!(!scheduler.should_check_on_startup());
}

#[test]
fn test_scheduler_can_customize_interval() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("update_config.json");

    let mut scheduler = UpdateScheduler::with_config_path(config_path)
        .expect("Failed to create scheduler");

    // Change interval to 12 hours
    scheduler
        .set_check_interval(12)
        .expect("Failed to set interval");

    assert_eq!(scheduler.config().check_interval_hours, 12);
}

#[test]
fn test_scheduler_time_until_next_check_calculation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("update_config.json");

    let mut scheduler = UpdateScheduler::with_config_path(config_path)
        .expect("Failed to create scheduler");

    // First check - time until should be 0
    let time_until = scheduler.time_until_next_check();
    assert_eq!(time_until, 0);

    // After updating last check
    scheduler.config_mut().update_last_check();
    let time_until = scheduler.time_until_next_check();

    // Should be approximately 24 hours (86400 seconds)
    // Allow some variance for test execution time
    assert!(time_until > 86200);
    assert!(time_until <= 86400);
}

#[test]
fn test_update_state_tracking_through_phases() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let state_file = temp_dir.path().join("update_state.json");

    let state_manager = UpdateStateManager::with_file(&state_file)
        .expect("Failed to create state manager");

    // Create initial record
    let mut record = UpdateRecord::new("1.1.0".to_string());
    assert_eq!(record.status, UpdateStatus::Pending);

    // Simulate progression through phases
    record.set_status(UpdateStatus::Downloaded);
    state_manager
        .upsert_record("1.1.0", record.clone())
        .expect("Failed to save record");

    // Verify persistence
    let loaded = state_manager
        .get_record("1.1.0")
        .expect("Failed to load record")
        .expect("Record should exist");

    assert_eq!(loaded.status, UpdateStatus::Downloaded);
}

#[test]
fn test_complete_backup_restore_cycle() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let binary_path = temp_dir.path().join("rusty");
    let backup_dir = temp_dir.path().join("backups");

    // Create initial binary
    fs::write(&binary_path, b"v1.0.0 content").expect("Failed to write binary");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&binary_path, permissions).expect("Failed to set permissions");
    }

    // Create backup manager
    let backup_manager = BackupManager::with_directory(&backup_dir)
        .expect("Failed to create backup manager");

    // Create backup
    let backup_entry = backup_manager
        .backup_binary(&binary_path)
        .expect("Failed to create backup");

    // Verify backup exists
    assert!(backup_entry.backup_path.exists());

    // Modify binary
    fs::write(&binary_path, b"v1.1.0 content").expect("Failed to modify binary");

    // Restore from backup
    let restore_path = temp_dir.path().join("restored");
    backup_manager
        .restore_backup(&backup_entry, &restore_path)
        .expect("Failed to restore backup");

    // Verify restored content matches original
    let restored_content = fs::read(&restore_path).expect("Failed to read restored");
    assert_eq!(restored_content, b"v1.0.0 content");
}

#[test]
fn test_atomic_binary_replacement_with_installer() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let current_binary = temp_dir.path().join("rusty");
    let new_binary = temp_dir.path().join("rusty-new");
    let backup_dir = temp_dir.path().join("backups");

    // Create current binary
    fs::write(&current_binary, b"v1.0.0").expect("Failed to write current");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&current_binary, permissions).expect("Failed to set permissions");
    }

    // Create new binary
    fs::write(&new_binary, b"v1.1.0").expect("Failed to write new");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&new_binary, permissions).expect("Failed to set permissions");
    }

    // Create installer with backup
    let installer = BinaryInstaller::with_backup_dir(
        InstallerConfig {
            verify_before_install: false,
            create_backup: true,
            keep_backup: true,
        },
        &backup_dir,
    )
    .expect("Failed to create installer");

    // Install update
    let result = installer
        .install_update(&new_binary, &current_binary)
        .expect("Failed to install");

    assert!(result.success);
    assert!(result.backup_path.is_some());

    // Verify new binary is in place
    let current_content = fs::read(&current_binary).expect("Failed to read current");
    assert_eq!(current_content, b"v1.1.0");

    // Verify backup contains old version
    let backup_content = fs::read(result.backup_path.unwrap()).expect("Failed to read backup");
    assert_eq!(backup_content, b"v1.0.0");
}

#[test]
fn test_rollback_after_failed_update() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let binary_path = temp_dir.path().join("rusty");
    let backup_dir = temp_dir.path().join("backups");

    // Create original binary
    fs::write(&binary_path, b"v1.0.0 working").expect("Failed to write binary");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&binary_path, permissions).expect("Failed to set permissions");
    }

    // Create installer
    let installer = BinaryInstaller::with_backup_dir(InstallerConfig::default(), &backup_dir)
        .expect("Failed to create installer");

    // Create backup of original
    let backup_entry = installer
        .backup_manager()
        .backup_binary(&binary_path)
        .expect("Failed to create backup");

    // Simulate failed update (corrupt the binary)
    fs::write(&binary_path, b"corrupted").expect("Failed to corrupt binary");

    // Rollback
    installer
        .rollback_to_backup(&backup_entry.backup_path, &binary_path)
        .expect("Failed to rollback");

    // Verify rolled back content
    let rolled_back = fs::read(&binary_path).expect("Failed to read rolled back");
    assert_eq!(rolled_back, b"v1.0.0 working");
}

#[test]
fn test_multiple_update_records_management() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let state_file = temp_dir.path().join("update_state.json");

    let state_manager = UpdateStateManager::with_file(&state_file)
        .expect("Failed to create state manager");

    // Create multiple records
    for version in &["1.1.0", "1.2.0", "1.3.0"] {
        let record = UpdateRecord::new(version.to_string());
        state_manager
            .upsert_record(version, record)
            .expect("Failed to save record");
    }

    // Verify all records exist
    let all_records = state_manager
        .get_all_records()
        .expect("Failed to get all records");

    assert_eq!(all_records.len(), 3);
}

#[test]
fn test_cleanup_old_backups_keeps_recent() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let binary_path = temp_dir.path().join("rusty");
    let backup_dir = temp_dir.path().join("backups");

    let backup_manager = BackupManager::with_directory(&backup_dir)
        .expect("Failed to create backup manager");

    // Create initial binary and multiple backups
    fs::write(&binary_path, b"v1.0.0").expect("Failed to write binary");

    for i in 1..=5 {
        if i > 1 {
            fs::write(&binary_path, format!("v1.{}.0", i).as_bytes())
                .expect("Failed to update binary");
        }

        backup_manager
            .backup_binary(&binary_path)
            .expect("Failed to create backup");

        // Delay to ensure different timestamps (backup uses HH:MM:SS granularity)
        std::thread::sleep(Duration::from_secs(2));
    }

    // List all backups
    let all_backups = backup_manager
        .list_backups()
        .expect("Failed to list backups");

    // Should have at least 2-5 backups (may be fewer if timestamps collide)
    assert!(all_backups.len() >= 2);
    let initial_count = all_backups.len();

    // Cleanup keeping only 1
    let deleted = backup_manager
        .cleanup_old_backups(1)
        .expect("Failed to cleanup");

    assert_eq!(deleted, initial_count - 1);

    // Verify only 1 remains
    let remaining = backup_manager
        .list_backups()
        .expect("Failed to list remaining");

    assert_eq!(remaining.len(), 1);
}

#[test]
fn test_version_comparison_and_update_detection() {
    let current = Version::new(1, 0, 0);
    let new = Version::new(1, 1, 0);
    let same = Version::new(1, 0, 0);

    assert!(new.is_update_available(&current));
    assert!(!current.is_update_available(&new));
    assert!(!same.is_update_available(&current));
}

#[test]
fn test_update_config_serialization() {
    let config = UpdateConfig::with_settings(true, 48);
    let json = serde_json::to_string(&config).expect("Failed to serialize");
    let deserialized: UpdateConfig =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(config.auto_check, deserialized.auto_check);
    assert_eq!(config.check_interval_hours, deserialized.check_interval_hours);
}

#[test]
fn test_scheduler_default_path_uses_home_directory() {
    let path = UpdateScheduler::default_config_path();

    // Should contain .rusty directory
    assert!(path.to_string_lossy().contains(".rusty"));
}
