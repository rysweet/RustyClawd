//! Update command handler for CLI integration
//!
//! Provides high-level interface for update operations triggered from CLI.

use crate::update::backup::BackupManager;
use crate::update::config::UpdateConfig;
use crate::update::downloader::BinaryDownloader;
use crate::update::error::UpdateError;
use crate::update::github_client::GitHubClient;
use crate::update::installer::{BinaryInstaller, InstallerConfig};
use crate::update::scheduler::UpdateScheduler;
use crate::update::state::{UpdateRecord, UpdateStateManager, UpdateStatus};
use crate::update::version::Version;
use std::env;
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Overall result of an update operation
#[derive(Debug, Clone)]
pub struct UpdateOperationResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Message to display to user
    pub message: String,
    /// Optional version that was installed/checked
    pub version: Option<String>,
    /// Whether a restart is required
    pub restart_required: bool,
}

/// Handle checking for updates (rusty update --check)
pub async fn handle_check_updates(force: bool) -> Result<UpdateOperationResult, UpdateError> {
    info!("Checking for updates (force: {})", force);

    let current_version = Version::current();
    let client = GitHubClient::new("rysweet", "RustyClawd");

    // If not forcing, check if we should perform the check
    if !force {
        let scheduler = UpdateScheduler::new()?;
        if !scheduler.should_check_on_startup() {
            let next_check_secs = scheduler.time_until_next_check();
            let hours = next_check_secs / 3600;
            let minutes = (next_check_secs % 3600) / 60;

            return Ok(UpdateOperationResult {
                success: true,
                message: format!(
                    "No update check needed. Next check in {} hour{} {} minute{}.",
                    hours,
                    if hours == 1 { "" } else { "s" },
                    minutes,
                    if minutes == 1 { "" } else { "s" }
                ),
                version: None,
                restart_required: false,
            });
        }
    }

    // Perform the check
    match client.get_update_info(&current_version).await {
        Ok(Some(update_info)) => {
            info!("Update available: {}", update_info.summary());

            Ok(UpdateOperationResult {
                success: true,
                message: format!(
                    "Update available: {} -> {}\n{}",
                    update_info.current_version,
                    update_info.latest_version,
                    update_info.summary()
                ),
                version: Some(update_info.latest_version.to_string()),
                restart_required: false,
            })
        }
        Ok(None) => {
            info!("Already at latest version");

            Ok(UpdateOperationResult {
                success: true,
                message: format!("You are already at the latest version: {}", current_version),
                version: None,
                restart_required: false,
            })
        }
        Err(e) => {
            error!("Failed to check for updates: {}", e);

            Err(UpdateError::NetworkError(format!(
                "Failed to check for updates: {}",
                e
            )))
        }
    }
}

/// Handle installing an update (rusty update)
pub async fn handle_install_update() -> Result<UpdateOperationResult, UpdateError> {
    info!("Starting update installation process");

    let current_version = Version::current();
    let client = GitHubClient::new("rysweet", "RustyClawd");

    // Check for updates
    let update_info = match client.get_update_info(&current_version).await? {
        Some(info) => {
            info!("Update found: {}", info.latest_version);
            info
        }
        None => {
            return Ok(UpdateOperationResult {
                success: true,
                message: "Already at the latest version".to_string(),
                version: None,
                restart_required: false,
            });
        }
    };

    // Get the download URL for the platform
    let download_url = update_info
        .get_asset_for_platform()
        .ok_or(UpdateError::AssetNotFound)?;

    // Get binary path
    let binary_path = env::current_exe()
        .map_err(|e| UpdateError::IoError(format!("Failed to get current binary path: {}", e)))?;

    info!("Current binary: {:?}", binary_path);

    // Download the new binary
    info!("Downloading update...");
    let downloader = BinaryDownloader::new()?;
    let download_path = downloader.download_to_temp(&download_url, None).await?;

    info!("Downloaded to: {:?}", download_path);

    // Get backup directory
    let backup_dir = if let Some(home) = dirs::home_dir() {
        home.join(".rusty/backups")
    } else {
        PathBuf::from(".rusty/backups")
    };

    // Install the update (with backup)
    info!("Installing update with backup...");
    let installer = BinaryInstaller::with_backup_dir(
        InstallerConfig {
            verify_before_install: false, // Already verified
            create_backup: true,
            keep_backup: true,
        },
        &backup_dir,
    )?;

    match installer.install_update(&download_path, &binary_path) {
        Ok(result) => {
            info!("Update installed successfully");

            Ok(UpdateOperationResult {
                success: true,
                message: format!(
                    "Successfully updated to version {}\n\
                    Backup saved at: {:?}\n\
                    Please restart the application to use the new version.",
                    update_info.latest_version, result.backup_path
                ),
                version: Some(update_info.latest_version.to_string()),
                restart_required: true,
            })
        }
        Err(e) => {
            error!("Failed to install update: {}", e);
            Err(e)
        }
    }
}

/// Handle rollback to previous version (rusty update --rollback)
pub async fn handle_rollback() -> Result<UpdateOperationResult, UpdateError> {
    info!("Starting rollback process");

    let backup_dir = if let Some(home) = dirs::home_dir() {
        home.join(".rusty/backups")
    } else {
        PathBuf::from(".rusty/backups")
    };

    let backup_manager = BackupManager::with_directory(&backup_dir)?;

    // Get the most recent backup
    let backups = backup_manager.list_backups()?;

    if backups.is_empty() {
        return Ok(UpdateOperationResult {
            success: false,
            message: "No backups available for rollback".to_string(),
            version: None,
            restart_required: false,
        });
    }

    // Use the most recent backup
    let latest_backup = &backups[0];
    info!("Rolling back to: {:?}", latest_backup.backup_path);

    let binary_path = env::current_exe()
        .map_err(|e| UpdateError::IoError(format!("Failed to get current binary path: {}", e)))?;

    // Rollback using installer
    let installer = BinaryInstaller::with_backup_dir(InstallerConfig::default(), &backup_dir)?;

    match installer.rollback_to_backup(&latest_backup.backup_path, &binary_path) {
        Ok(_) => {
            info!("Rollback completed successfully");

            Ok(UpdateOperationResult {
                success: true,
                message: "Successfully rolled back to previous version.\n\
                    Please restart the application to complete the rollback."
                    .to_string(),
                version: Some(Version::current().to_string()),
                restart_required: true,
            })
        }
        Err(e) => {
            error!("Failed to rollback: {}", e);
            Err(e)
        }
    }
}

/// Display update information in user-friendly format
pub fn format_update_message(result: &UpdateOperationResult) -> String {
    format!(
        "{}\n{}",
        result.message,
        if result.restart_required {
            "\nNote: Restart required to complete the update."
        } else {
            ""
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_update_message_with_restart() {
        let result = UpdateOperationResult {
            success: true,
            message: "Update installed".to_string(),
            version: Some("1.1.0".to_string()),
            restart_required: true,
        };

        let formatted = format_update_message(&result);
        assert!(formatted.contains("Update installed"));
        assert!(formatted.contains("Restart required"));
    }

    #[test]
    fn test_format_update_message_without_restart() {
        let result = UpdateOperationResult {
            success: true,
            message: "Check complete".to_string(),
            version: None,
            restart_required: false,
        };

        let formatted = format_update_message(&result);
        assert!(formatted.contains("Check complete"));
        assert!(!formatted.contains("Restart required"));
    }
}
