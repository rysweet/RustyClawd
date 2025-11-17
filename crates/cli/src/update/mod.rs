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

pub mod config;
pub mod error;
pub mod github_client;
pub mod version;

// Re-export public API
pub use config::UpdateConfig;
pub use error::UpdateError;
pub use github_client::{GitHubClient, Release, ReleaseAsset, UpdateInfo};
pub use version::Version;

#[cfg(test)]
mod integration_tests {
    use super::*;

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
}
