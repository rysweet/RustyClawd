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
pub mod state;
pub mod version;

// Re-export public API
pub use backup::{BackupEntry, BackupManager};
pub use config::UpdateConfig;
pub use downloader::{BinaryDownload, BinaryDownloader, DownloadConfig};
pub use error::UpdateError;
pub use github_client::{GitHubClient, Release, ReleaseAsset, UpdateInfo};
pub use state::{UpdateRecord, UpdateStateManager, UpdateStatus};
pub use version::Version;

#[cfg(test)]
mod integration_tests {
    use super::*;
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
}
