//! Auto-check scheduler for periodic update checks
//!
//! Manages scheduling of automatic update checks on startup if the
//! configured interval has elapsed since the last check.

use crate::update::config::UpdateConfig;
use crate::update::error::UpdateError;
use crate::update::github_client::{GitHubClient, UpdateInfo};
use crate::update::version::Version;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, info, warn};

/// Configuration file for scheduler (persisted checks)
const SCHEDULER_CONFIG_FILE: &str = ".rusty/update_check.json";

/// Scheduler for automatic update checks
#[derive(Debug, Clone)]
pub struct UpdateScheduler {
    /// Configuration for update checking
    config: UpdateConfig,
    /// Path to configuration file for persistence
    config_path: PathBuf,
}

/// Result of a scheduled update check
#[derive(Debug, Clone)]
pub struct ScheduledCheckResult {
    /// Whether a check was performed
    pub check_performed: bool,
    /// Update information if available (stored as JSON string for persistence)
    pub update_available: Option<UpdateInfo>,
    /// Reason for performing or skipping the check
    pub reason: String,
    /// Timestamp of the check
    pub timestamp: u64,
}

impl UpdateScheduler {
    /// Create a new scheduler with default configuration
    pub fn new() -> Result<Self, UpdateError> {
        Self::with_config_path(Self::default_config_path())
    }

    /// Create a scheduler with a specific configuration path
    pub fn with_config_path(config_path: PathBuf) -> Result<Self, UpdateError> {
        // Load existing config or create new one
        let config = if config_path.exists() {
            debug!("Loading update config from: {:?}", config_path);
            Self::load_config(&config_path)?
        } else {
            debug!("Creating new update config at: {:?}", config_path);
            UpdateConfig::new()
        };

        // Validate the loaded/created config
        config.validate()?;

        Ok(Self {
            config,
            config_path,
        })
    }

    /// Get the default configuration file path
    pub fn default_config_path() -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            home.join(SCHEDULER_CONFIG_FILE)
        } else {
            PathBuf::from(SCHEDULER_CONFIG_FILE)
        }
    }

    /// Check if an update check should be performed on startup
    pub fn should_check_on_startup(&self) -> bool {
        self.config.should_check_now()
    }

    /// Perform a scheduled update check
    pub async fn perform_scheduled_check(
        &mut self,
        current_version: &Version,
        github_client: &GitHubClient,
    ) -> Result<ScheduledCheckResult, UpdateError> {
        // Check if we should perform the check
        let should_check = self.should_check_on_startup();

        if !should_check {
            let reason = if !self.config.auto_check {
                "Auto-check is disabled".to_string()
            } else {
                format!(
                    "Interval not elapsed (check interval: {})",
                    self.config.interval_description()
                )
            };

            return Ok(ScheduledCheckResult {
                check_performed: false,
                update_available: None,
                reason,
                timestamp: current_unix_timestamp(),
            });
        }

        info!("Performing scheduled update check");

        // Update the last check timestamp regardless of outcome so that
        // persistent network errors don't cause a retry on every startup.
        self.config.update_last_check();
        self.save_config()?;

        // Get update info from GitHub
        let update_info = match github_client.get_update_info(current_version).await {
            Ok(info) => info,
            Err(e) => {
                // Don't warn if there are simply no releases available yet
                use crate::update::error::UpdateError;
                if !matches!(e, UpdateError::NoReleasesAvailable) {
                    warn!("Failed to check for updates: {}", e);
                } else {
                    debug!("No releases available for update check");
                }
                return Ok(ScheduledCheckResult {
                    check_performed: true,
                    update_available: None,
                    reason: if matches!(e, UpdateError::NoReleasesAvailable) {
                        "No releases available".to_string()
                    } else {
                        format!("Failed to check: {}", e)
                    },
                    timestamp: current_unix_timestamp(),
                });
            }
        };

        let result = ScheduledCheckResult {
            check_performed: true,
            update_available: update_info.clone(),
            reason: if update_info.is_some() {
                "Update available".to_string()
            } else {
                "Already at latest version".to_string()
            },
            timestamp: current_unix_timestamp(),
        };

        Ok(result)
    }

    /// Load configuration from file
    fn load_config(path: &Path) -> Result<UpdateConfig, UpdateError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| UpdateError::ConfigError(format!("Failed to read config file: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| UpdateError::ConfigError(format!("Failed to parse config: {}", e)))
    }

    /// Save configuration to file
    pub fn save_config(&self) -> Result<(), UpdateError> {
        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                UpdateError::ConfigError(format!("Failed to create config directory: {}", e))
            })?;
        }

        let json = serde_json::to_string_pretty(&self.config)
            .map_err(|e| UpdateError::ConfigError(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(&self.config_path, json)
            .map_err(|e| UpdateError::ConfigError(format!("Failed to write config file: {}", e)))?;

        debug!("Config saved to: {:?}", self.config_path);
        Ok(())
    }

    /// Get current configuration
    pub fn config(&self) -> &UpdateConfig {
        &self.config
    }

    /// Get mutable reference to configuration
    pub fn config_mut(&mut self) -> &mut UpdateConfig {
        &mut self.config
    }

    /// Set auto-check enabled/disabled
    pub fn set_auto_check(&mut self, enabled: bool) -> Result<(), UpdateError> {
        self.config.set_auto_check(enabled);
        self.save_config()
    }

    /// Set check interval in hours
    pub fn set_check_interval(&mut self, hours: u32) -> Result<(), UpdateError> {
        self.config.set_check_interval(hours)?;
        self.save_config()
    }

    /// Get time until next check (in seconds)
    pub fn time_until_next_check(&self) -> u64 {
        let now = current_unix_timestamp();

        // If last check was 0, return 0 (should check immediately)
        if self.config.last_check_timestamp == 0 {
            return 0;
        }

        let seconds_since = now.saturating_sub(self.config.last_check_timestamp);
        let interval_seconds = (self.config.check_interval_hours as u64).saturating_mul(3600);

        if seconds_since >= interval_seconds {
            0
        } else {
            interval_seconds.saturating_sub(seconds_since)
        }
    }
}

impl Default for UpdateScheduler {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            config: UpdateConfig::default(),
            config_path: Self::default_config_path(),
        })
    }
}

/// Get current Unix timestamp
fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scheduler_new() {
        let scheduler = UpdateScheduler::new();
        assert!(scheduler.is_ok());
    }

    #[test]
    fn test_scheduler_with_custom_path() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.json");

        let scheduler = UpdateScheduler::with_config_path(config_path.clone());
        assert!(scheduler.is_ok());

        let scheduler = scheduler.unwrap();
        assert_eq!(scheduler.config_path, config_path);
    }

    #[test]
    fn test_scheduler_should_check_on_startup() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.json");

        let scheduler =
            UpdateScheduler::with_config_path(config_path).expect("Failed to create scheduler");

        // First check should be true (no timestamp set)
        assert!(scheduler.should_check_on_startup());
    }

    #[test]
    fn test_scheduler_config_persistence() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.json");

        // Create scheduler and save config
        let mut scheduler = UpdateScheduler::with_config_path(config_path.clone())
            .expect("Failed to create scheduler");
        scheduler.config.update_last_check();
        scheduler.save_config().expect("Failed to save config");

        // Load scheduler again
        let scheduler2 =
            UpdateScheduler::with_config_path(config_path).expect("Failed to create scheduler");

        // Configs should match
        assert_eq!(scheduler.config.auto_check, scheduler2.config.auto_check);
        assert_eq!(
            scheduler.config.check_interval_hours,
            scheduler2.config.check_interval_hours
        );
    }

    #[test]
    fn test_scheduler_set_auto_check() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.json");

        let mut scheduler =
            UpdateScheduler::with_config_path(config_path).expect("Failed to create scheduler");

        assert!(scheduler.set_auto_check(false).is_ok());
        assert!(!scheduler.config.auto_check);
    }

    #[test]
    fn test_scheduler_set_check_interval() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.json");

        let mut scheduler =
            UpdateScheduler::with_config_path(config_path).expect("Failed to create scheduler");

        assert!(scheduler.set_check_interval(48).is_ok());
        assert_eq!(scheduler.config.check_interval_hours, 48);
    }

    #[test]
    fn test_scheduler_time_until_next_check() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.json");

        let mut scheduler =
            UpdateScheduler::with_config_path(config_path).expect("Failed to create scheduler");

        // First check: time until check should be 0 (should check immediately)
        assert_eq!(scheduler.time_until_next_check(), 0);

        // After updating last check
        scheduler.config.update_last_check();
        let time_until = scheduler.time_until_next_check();

        // Should be approximately 24 hours (86400 seconds)
        // Allow for some variance due to test execution time
        assert!(time_until > 86300);
        assert!(time_until <= 86400);
    }

    #[test]
    fn test_default_scheduler() {
        let _scheduler = UpdateScheduler::default();
        // Should not panic and create a valid scheduler
    }
}
