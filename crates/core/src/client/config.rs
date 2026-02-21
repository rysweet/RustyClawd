//! Secure API configuration and key management
//!
//! This module handles API key loading with security features:
//! - Zeroization of sensitive data in memory
//! - Secret wrapper to prevent accidental logging
//! - Secure file permissions validation

use secrecy::{CloneableSecret, DebugSecret, Secret, Zeroize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
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

    /// Load from default location with priority chain
    ///
    /// Priority:
    /// 1. ANTHROPIC_API_KEY environment variable
    /// 2. .env file in current working directory
    /// 3. ~/.claude-msec-k (legacy, with deprecation warning)
    pub async fn from_default_location() -> ClientResult<Self> {
        // Try 1: Environment variable
        if let Some(key) = Self::try_from_env()? {
            return Ok(key);
        }

        // Try 2: .env file in current directory
        if let Some(key) = Self::try_from_dotenv().await? {
            return Ok(key);
        }

        // Try 3: Legacy file (with warning)
        if let Some(key) = Self::try_from_legacy_file().await? {
            Self::warn_legacy_usage();
            return Ok(key);
        }

        // None found
        Err(ClientError::ApiKeyNotFound)
    }

    /// Try loading from ANTHROPIC_API_KEY environment variable
    fn try_from_env() -> ClientResult<Option<Self>> {
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                return Ok(Some(Self::new(key)?));
            }
        }
        Ok(None)
    }

    /// Try loading from .env file in current directory
    async fn try_from_dotenv() -> ClientResult<Option<Self>> {
        if let Ok(cwd) = std::env::current_dir() {
            let dotenv_path = cwd.join(".env");
            if dotenv_path.exists() {
                // Validate file permissions (Unix-like systems)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = fs::metadata(&dotenv_path).await {
                        let mode = metadata.permissions().mode();
                        // Check if file is readable by others (should be 600 or similar)
                        if mode & 0o077 != 0 {
                            tracing::warn!(
                                ".env file {} has permissive permissions: {:o}. Consider: chmod 600 .env",
                                dotenv_path.display(),
                                mode & 0o777
                            );
                        }
                    }
                }

                // Read .env file and parse ANTHROPIC_API_KEY
                if let Ok(content) = fs::read_to_string(&dotenv_path).await {
                    for line in content.lines() {
                        let line = line.trim();
                        // Skip comments and empty lines
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }
                        // Parse KEY=value or KEY="value" format
                        if let Some((key, value)) = line.split_once('=') {
                            if key.trim() == "ANTHROPIC_API_KEY" {
                                let value = value.trim().trim_matches('"').trim_matches('\'');
                                if !value.is_empty() {
                                    return Ok(Some(Self::new(value.to_string())?));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Try loading from legacy file location
    async fn try_from_legacy_file() -> ClientResult<Option<Self>> {
        if let Ok(home) = std::env::var("HOME") {
            let legacy_path = PathBuf::from(home).join(".claude-msec-k");
            if legacy_path.exists() {
                return Ok(Some(Self::from_file(&legacy_path).await?));
            }
        }
        Ok(None)
    }

    /// Show deprecation warning once per process
    fn warn_legacy_usage() {
        static LEGACY_WARNING_SHOWN: OnceLock<()> = OnceLock::new();

        if LEGACY_WARNING_SHOWN.get().is_none() {
            tracing::warn!(
                "Using legacy API key location ~/.claude-msec-k. \
                 Consider setting ANTHROPIC_API_KEY environment variable instead."
            );
            let _ = LEGACY_WARNING_SHOWN.set(());
        }
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
    #[must_use]
    pub fn with_api_url(mut self, url: String) -> Self {
        self.api_url = url;
        self
    }

    /// Builder: Set custom API version
    #[must_use]
    pub fn with_api_version(mut self, version: String) -> Self {
        self.api_version = version;
        self
    }

    /// Builder: Set custom timeout
    #[must_use]
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
    use serial_test::serial;

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

    #[tokio::test]
    #[serial]
    async fn test_api_key_from_env() {
        // Clear any existing env var first to avoid test interference
        std::env::remove_var("ANTHROPIC_API_KEY");

        // Set env var and verify it's loaded
        let test_key = "sk-ant-test123456789";
        std::env::set_var("ANTHROPIC_API_KEY", test_key);

        let result = ApiKey::from_default_location().await;

        // Clean up
        std::env::remove_var("ANTHROPIC_API_KEY");

        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let key = result.unwrap();
        assert_eq!(key.expose(), test_key);
    }

    #[tokio::test]
    #[serial]
    async fn test_api_key_from_env_empty_string_ignored() {
        // Clear any existing env var first
        std::env::remove_var("ANTHROPIC_API_KEY");

        // Empty env var should be ignored
        std::env::set_var("ANTHROPIC_API_KEY", "");
        let result = ApiKey::try_from_env();
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_api_key_from_env_invalid_format() {
        // Clear any existing env var first
        std::env::remove_var("ANTHROPIC_API_KEY");

        // Invalid format should return error
        std::env::set_var("ANTHROPIC_API_KEY", "invalid-key");
        let result = ApiKey::from_default_location().await;
        std::env::remove_var("ANTHROPIC_API_KEY");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClientError::InvalidApiKey));
    }

    #[test]
    fn test_api_key_not_found_error_message() {
        // Verify helpful error message includes all 3 options
        let error = ClientError::ApiKeyNotFound;
        let message = error.to_string();
        assert!(message.contains("ANTHROPIC_API_KEY"));
        assert!(message.contains(".env"));
        assert!(message.contains("~/.claude-msec-k"));
        assert!(message.contains("https://console.anthropic.com/settings/keys"));
    }

    #[tokio::test]
    #[serial]
    async fn test_api_key_priority_chain_integration() {
        use std::env;
        use std::fs;

        // Clean up first
        env::remove_var("ANTHROPIC_API_KEY");
        let _ = fs::remove_file(".env");

        // Create .env file
        fs::write(".env", "ANTHROPIC_API_KEY=\"sk-ant-from-dotenv\"\n").unwrap();

        // Test 1: Environment variable should win
        env::set_var("ANTHROPIC_API_KEY", "sk-ant-from-env");
        let key1 = ApiKey::from_default_location().await.unwrap();
        assert_eq!(
            key1.expose(),
            "sk-ant-from-env",
            "Env var should have highest priority"
        );

        // Test 2: .env should win after env var removed
        env::remove_var("ANTHROPIC_API_KEY");
        let key2 = ApiKey::from_default_location().await.unwrap();
        assert_eq!(
            key2.expose(),
            "sk-ant-from-dotenv",
            ".env should be second priority"
        );

        // Cleanup
        fs::remove_file(".env").unwrap();
    }
}
