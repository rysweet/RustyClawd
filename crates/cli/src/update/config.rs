//! Update configuration module for managing auto-check settings

use crate::update::error::UpdateError;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Update configuration settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateConfig {
    /// Enable automatic checking for updates
    pub auto_check: bool,

    /// Interval in hours between update checks
    pub check_interval_hours: u32,

    /// Last time an update check was performed (Unix timestamp)
    #[serde(default)]
    pub last_check_timestamp: u64,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            auto_check: true,
            check_interval_hours: 24,
            last_check_timestamp: 0,
        }
    }
}

impl UpdateConfig {
    /// Create a new update configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new configuration with specific values
    pub fn with_settings(auto_check: bool, check_interval_hours: u32) -> Self {
        Self {
            auto_check,
            check_interval_hours,
            last_check_timestamp: 0,
        }
    }

    /// Check if enough time has passed since the last check
    pub fn should_check_now(&self) -> bool {
        if !self.auto_check {
            return false;
        }

        if self.last_check_timestamp == 0 {
            return true;
        }

        let now = current_unix_timestamp();
        let hours_since_check = (now - self.last_check_timestamp) / 3600;

        hours_since_check >= self.check_interval_hours as u64
    }

    /// Update the last check timestamp to now
    pub fn update_last_check(&mut self) {
        self.last_check_timestamp = current_unix_timestamp();
    }

    /// Get human-readable description of the check interval
    pub fn interval_description(&self) -> String {
        match self.check_interval_hours {
            0 => "never".to_string(),
            1 => "hourly".to_string(),
            24 => "daily".to_string(),
            168 => "weekly".to_string(),
            720 => "monthly".to_string(),
            _ => format!("every {} hours", self.check_interval_hours),
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), UpdateError> {
        if self.check_interval_hours == 0 && self.auto_check {
            return Err(UpdateError::ConfigError(
                "auto_check cannot be true when check_interval_hours is 0".to_string(),
            ));
        }

        // Allow 0 interval only if auto_check is false
        if self.check_interval_hours == 0 && !self.auto_check {
            return Ok(());
        }

        // Reasonable bounds check (1 hour to 1 year)
        if self.check_interval_hours < 1 || self.check_interval_hours > 8760 {
            return Err(UpdateError::ConfigError(
                "check_interval_hours must be between 1 and 8760 (1 year)".to_string(),
            ));
        }

        Ok(())
    }

    /// Set auto-check enabled/disabled
    pub fn set_auto_check(&mut self, enabled: bool) {
        self.auto_check = enabled;
    }

    /// Set the check interval in hours
    pub fn set_check_interval(&mut self, hours: u32) -> Result<(), UpdateError> {
        if hours == 0 {
            return Err(UpdateError::ConfigError("Check interval must be at least 1 hour".to_string()));
        }
        if hours > 8760 {
            return Err(UpdateError::ConfigError("Check interval cannot exceed 1 year (8760 hours)".to_string()));
        }
        self.check_interval_hours = hours;
        Ok(())
    }
}

/// Get the current Unix timestamp
fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_config_default() {
        let config = UpdateConfig::new();
        assert!(config.auto_check);
        assert_eq!(config.check_interval_hours, 24);
        assert_eq!(config.last_check_timestamp, 0);
    }

    #[test]
    fn test_update_config_with_settings() {
        let config = UpdateConfig::with_settings(false, 48);
        assert!(!config.auto_check);
        assert_eq!(config.check_interval_hours, 48);
    }

    #[test]
    fn test_should_check_now_first_time() {
        let config = UpdateConfig::new();
        assert!(config.should_check_now());
    }

    #[test]
    fn test_should_check_now_disabled() {
        let mut config = UpdateConfig::new();
        config.auto_check = false;
        assert!(!config.should_check_now());
    }

    #[test]
    fn test_should_check_now_interval_not_reached() {
        let mut config = UpdateConfig::new();
        config.last_check_timestamp = current_unix_timestamp();
        config.check_interval_hours = 24;
        assert!(!config.should_check_now());
    }

    #[test]
    fn test_update_last_check() {
        let mut config = UpdateConfig::new();
        let before = current_unix_timestamp();
        config.update_last_check();
        let after = current_unix_timestamp();

        assert!(config.last_check_timestamp >= before);
        assert!(config.last_check_timestamp <= after + 1);
    }

    #[test]
    fn test_interval_description() {
        assert_eq!(UpdateConfig::with_settings(true, 1).interval_description(), "hourly");
        assert_eq!(UpdateConfig::with_settings(true, 24).interval_description(), "daily");
        assert_eq!(UpdateConfig::with_settings(true, 168).interval_description(), "weekly");
        assert_eq!(UpdateConfig::with_settings(true, 720).interval_description(), "monthly");
        assert_eq!(UpdateConfig::with_settings(true, 12).interval_description(), "every 12 hours");
    }

    #[test]
    fn test_validate_valid_config() {
        let config = UpdateConfig::new();
        assert!(config.validate().is_ok());

        let config = UpdateConfig::with_settings(false, 0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_config() {
        let config = UpdateConfig::with_settings(true, 0);
        assert!(config.validate().is_err());

        let config = UpdateConfig::with_settings(true, 10000);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_set_auto_check() {
        let mut config = UpdateConfig::new();
        config.set_auto_check(false);
        assert!(!config.auto_check);

        config.set_auto_check(true);
        assert!(config.auto_check);
    }

    #[test]
    fn test_set_check_interval() {
        let mut config = UpdateConfig::new();
        assert!(config.set_check_interval(12).is_ok());
        assert_eq!(config.check_interval_hours, 12);

        assert!(config.set_check_interval(0).is_err());
        assert!(config.set_check_interval(10000).is_err());
    }

    #[test]
    fn test_serialization() {
        let config = UpdateConfig::with_settings(true, 48);
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: UpdateConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }
}
