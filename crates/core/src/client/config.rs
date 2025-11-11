//! Secure API configuration and key management
//!
//! This module handles API key loading with security features:
//! - Zeroization of sensitive data in memory
//! - Secret wrapper to prevent accidental logging
//! - Secure file permissions validation

use secrecy::{CloneableSecret, DebugSecret, Secret, Zeroize};
use std::fmt;
use std::path::{Path, PathBuf};
use tokio::fs;

use super::error::{ClientError, ClientResult};

/// API key with automatic zeroization on drop
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct ApiKey(String);

// Implement CloneableSecret to allow Secret<ApiKey>
impl CloneableSecret for ApiKey {}

// Implement DebugSecret to prevent leaking in debug output
impl DebugSecret for ApiKey {}

impl ApiKey {
    /// Create a new API key (validates format)
    pub fn new(key: String) -> ClientResult<Self> {
        if !key.starts_with("sk-ant-") {
            return Err(ClientError::InvalidApiKey);
        }
        Ok(Self(key))
    }

    /// Get the raw key value (use sparingly!)
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Load API key from file with security checks
    pub async fn from_file<P: AsRef<Path>>(path: P) -> ClientResult<Self> {
        let path = path.as_ref();

        // Validate file permissions (Unix-like systems)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(path).await.map_err(|e| {
                ClientError::ApiKeyRead(format!("Cannot read {}: {}", path.display(), e))
            })?;
            let mode = metadata.permissions().mode();
            // Check if file is readable by others (should be 600 or similar)
            if mode & 0o077 != 0 {
                tracing::warn!(
                    "API key file {} has permissive permissions: {:o}. Should be 600.",
                    path.display(),
                    mode & 0o777
                );
            }
        }

        // Read and parse the key
        let content = fs::read_to_string(path).await.map_err(|e| {
            ClientError::ApiKeyRead(format!("Failed to read {}: {}", path.display(), e))
        })?;

        let key = content.trim().to_string();
        Self::new(key)
    }

    /// Load from default location (~/.claude-msec-k)
    pub async fn from_default_location() -> ClientResult<Self> {
        let home = std::env::var("HOME").map_err(|_| {
            ClientError::ApiKeyRead("HOME environment variable not set".to_string())
        })?;
        let path = PathBuf::from(home).join(".claude-msec-k");
        Self::from_file(path).await
    }
}

// Prevent accidental logging of API key
impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey([REDACTED])")
    }
}

impl fmt::Display for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// Client configuration
#[derive(Clone)]
pub struct Config {
    /// API key wrapped in Secret for additional protection
    pub api_key: Secret<ApiKey>,
    /// API endpoint URL
    pub api_url: String,
    /// API version
    pub api_version: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Config {
    /// Default Anthropic API endpoint
    pub const DEFAULT_API_URL: &'static str = "https://api.anthropic.com";
    /// Default API version
    pub const DEFAULT_API_VERSION: &'static str = "2023-06-01";
    /// Default timeout (2 minutes)
    pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

    /// Create a new configuration with the given API key
    pub fn new(api_key: ApiKey) -> Self {
        Self {
            api_key: Secret::new(api_key),
            api_url: Self::DEFAULT_API_URL.to_string(),
            api_version: Self::DEFAULT_API_VERSION.to_string(),
            timeout_secs: Self::DEFAULT_TIMEOUT_SECS,
        }
    }

    /// Load configuration from default API key location
    pub async fn from_default_location() -> ClientResult<Self> {
        let api_key = ApiKey::from_default_location().await?;
        Ok(Self::new(api_key))
    }

    /// Load configuration from specific file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> ClientResult<Self> {
        let api_key = ApiKey::from_file(path).await?;
        Ok(Self::new(api_key))
    }

    /// Builder: Set custom API URL
    pub fn with_api_url(mut self, url: String) -> Self {
        self.api_url = url;
        self
    }

    /// Builder: Set custom API version
    pub fn with_api_version(mut self, version: String) -> Self {
        self.api_version = version;
        self
    }

    /// Builder: Set custom timeout
    pub fn with_timeout_secs(mut self, timeout: u64) -> Self {
        self.timeout_secs = timeout;
        self
    }
}

// Prevent accidental logging of config with API key
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("api_key", &"[REDACTED]")
            .field("api_url", &self.api_url)
            .field("api_version", &self.api_version)
            .field("timeout_secs", &self.timeout_secs)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_format_validation() {
        // Valid key
        let valid = ApiKey::new("sk-ant-test123".to_string());
        assert!(valid.is_ok());

        // Invalid key
        let invalid = ApiKey::new("invalid-key".to_string());
        assert!(invalid.is_err());
    }

    #[test]
    fn test_api_key_no_leak_in_debug() {
        let key = ApiKey::new("sk-ant-secret123".to_string()).unwrap();
        let debug_str = format!("{:?}", key);
        assert!(!debug_str.contains("secret123"));
        assert!(debug_str.contains("REDACTED"));
    }

    #[test]
    fn test_config_no_leak_in_debug() {
        let key = ApiKey::new("sk-ant-secret123".to_string()).unwrap();
        let config = Config::new(key);
        let debug_str = format!("{:?}", config);
        assert!(!debug_str.contains("secret123"));
        assert!(debug_str.contains("REDACTED"));
    }
}
