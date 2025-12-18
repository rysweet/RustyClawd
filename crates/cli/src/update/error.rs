//! Error types for the update mechanism

use thiserror::Error;

/// Errors that can occur during version checking and update operations
#[derive(Error, Debug)]
pub enum UpdateError {
    /// Failed to parse the current version
    #[error("Failed to parse current version: {0}")]
    VersionParseFailed(String),

    /// Failed to parse a remote version
    #[error("Failed to parse remote version: {0}")]
    RemoteVersionParseFailed(String),

    /// GitHub API request failed
    #[error("GitHub API request failed: {0}")]
    GitHubApiError(String),

    /// Failed to parse GitHub API response
    #[error("Failed to parse GitHub response: {0}")]
    GitHubResponseParseFailed(String),

    /// Network error occurred
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Version comparison failed
    #[error("Version comparison failed: {0}")]
    ComparisonFailed(String),

    /// Asset not found for the platform
    #[error("Asset not found for current platform")]
    AssetNotFound,

    /// Generic I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// Timeout occurred
    #[error("Operation timed out")]
    Timeout,

    /// SHA256 verification failed
    #[error("SHA256 verification failed: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// Download failed
    #[error("Download failed: {0}")]
    DownloadFailed(String),

    /// Backup operation failed
    #[error("Backup operation failed: {0}")]
    BackupFailed(String),

    /// State persistence failed
    #[error("Failed to persist update state: {0}")]
    StatePersistenceFailed(String),

    /// Invalid state data
    #[error("Invalid state data: {0}")]
    InvalidStateData(String),

    /// No releases available (not an error, just informational)
    #[error("No releases available for repository")]
    NoReleasesAvailable,

    /// Private repository access error (401/403/404)
    #[error("Private repository access denied (HTTP {status})")]
    PrivateRepositoryAccess { status: u16 },
}

impl From<std::io::Error> for UpdateError {
    fn from(err: std::io::Error) -> Self {
        UpdateError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for UpdateError {
    fn from(err: serde_json::Error) -> Self {
        UpdateError::GitHubResponseParseFailed(err.to_string())
    }
}

impl From<reqwest::Error> for UpdateError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            UpdateError::Timeout
        } else if err.is_request() {
            UpdateError::NetworkError(format!("Request error: {}", err))
        } else {
            UpdateError::NetworkError(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = UpdateError::VersionParseFailed("1.0.0.0".to_string());
        assert!(err.to_string().contains("Failed to parse current version"));
    }

    #[test]
    fn test_error_conversion_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let update_err: UpdateError = io_err.into();
        assert!(matches!(update_err, UpdateError::IoError(_)));
    }

    #[test]
    fn test_error_conversion_from_json() {
        let json_str = r#"{"invalid json"#;
        let json_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let update_err: UpdateError = json_err.into();
        assert!(matches!(
            update_err,
            UpdateError::GitHubResponseParseFailed(_)
        ));
    }

    #[test]
    fn test_private_repository_access_error() {
        let err = UpdateError::PrivateRepositoryAccess { status: 403 };
        let msg = err.to_string();
        assert!(msg.contains("Private repository access denied"));
        assert!(msg.contains("403"));
    }

    #[test]
    fn test_private_repository_access_error_401() {
        let err = UpdateError::PrivateRepositoryAccess { status: 401 };
        assert!(matches!(
            err,
            UpdateError::PrivateRepositoryAccess { status: 401 }
        ));
    }

    #[test]
    fn test_private_repository_access_error_404() {
        let err = UpdateError::PrivateRepositoryAccess { status: 404 };
        let msg = err.to_string();
        assert!(msg.contains("404"));
    }
}
