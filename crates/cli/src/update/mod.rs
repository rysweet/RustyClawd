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
pub mod version_compare;

// Re-export public API
pub use backup::{BackupEntry, BackupManager};
pub use config::UpdateConfig;
pub use downloader::{BinaryDownload, BinaryDownloader, DownloadConfig};
pub use error::UpdateError;
pub use github_client::{GitHubClient, Release, ReleaseAsset};
pub use handler::{
    format_update_message, handle_check_updates, handle_install_update, handle_rollback,
    UpdateOperationResult,
};
pub use installer::{BinaryInstaller, InstallResult, InstallerConfig};
pub use scheduler::{ScheduledCheckResult, UpdateScheduler};
pub use state::{UpdateRecord, UpdateStateManager, UpdateStatus};
pub use version::Version;
pub use version_compare::{PlatformInfo, UpdateInfo};

#[cfg(test)]
#[path = "update_tests.rs"]
mod integration_tests;
