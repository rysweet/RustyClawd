//! Binary downloader with SHA256 verification and progress tracking

use crate::update::error::UpdateError;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use tracing::{debug, info};

/// Configuration for the binary downloader
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// Timeout for download operations in seconds
    pub timeout_secs: u64,
    /// Enable progress reporting callbacks
    pub report_progress: bool,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 300,    // 5 minutes
            report_progress: true,
        }
    }
}

/// Represents a binary download with its checksum
#[derive(Debug, Clone)]
pub struct BinaryDownload {
    /// URL to download from
    pub url: String,
    /// Expected SHA256 checksum (hex string)
    pub expected_sha256: String,
    /// Downloaded file path
    pub file_path: PathBuf,
    /// Actual SHA256 checksum after download (hex string)
    pub actual_sha256: String,
    /// File size in bytes
    pub file_size: u64,
    /// Whether verification passed
    pub verified: bool,
}

impl BinaryDownload {
    /// Create a new binary download record
    pub fn new(url: String, expected_sha256: String, file_path: PathBuf) -> Self {
        Self {
            url,
            expected_sha256,
            file_path,
            actual_sha256: String::new(),
            file_size: 0,
            verified: false,
        }
    }
}

/// Binary downloader with progress tracking
pub struct BinaryDownloader {
    client: Client,
    config: DownloadConfig,
}

impl BinaryDownloader {
    /// Create a new binary downloader with default configuration
    pub fn new() -> Result<Self, UpdateError> {
        Self::with_config(DownloadConfig::default())
    }

    /// Create a new binary downloader with custom configuration
    pub fn with_config(config: DownloadConfig) -> Result<Self, UpdateError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .user_agent("RustyClawd-Binary-Downloader/1.0")
            .build()
            .map_err(|e| UpdateError::DownloadFailed(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, config })
    }

    /// Download a binary from the given URL to a temporary file
    ///
    /// # Arguments
    /// * `url` - The URL to download from (must be a valid GitHub asset URL)
    /// * `progress_callback` - Optional callback for progress updates (current_bytes, total_bytes)
    ///
    /// # Returns
    /// Path to the temporary file containing the downloaded binary
    pub async fn download_to_temp(
        &self,
        url: &str,
        progress_callback: Option<Box<dyn Fn(u64, u64) + Send>>,
    ) -> Result<PathBuf, UpdateError> {
        info!("Starting binary download from: {}", url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| UpdateError::DownloadFailed(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            return Err(UpdateError::DownloadFailed(format!(
                "HTTP error: {} ({})",
                response.status(),
                response.status().canonical_reason().unwrap_or("unknown")
            )));
        }

        let total_size = response
            .content_length()
            .ok_or_else(|| UpdateError::DownloadFailed("Server did not provide content length".to_string()))?;

        debug!("Downloading {} bytes", total_size);

        // Create a temporary file for the download
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| UpdateError::DownloadFailed(format!("Failed to create temp file: {}", e)))?;

        let mut bytes_downloaded = 0u64;
        let mut stream = response.bytes_stream();

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| UpdateError::DownloadFailed(format!("Download interrupted: {}", e)))?;

            temp_file
                .write_all(&chunk)
                .map_err(|e| UpdateError::DownloadFailed(format!("Failed to write to temp file: {}", e)))?;

            bytes_downloaded += chunk.len() as u64;

            if let Some(ref callback) = progress_callback {
                if self.config.report_progress {
                    callback(bytes_downloaded, total_size);
                }
            }
        }

        if bytes_downloaded != total_size {
            return Err(UpdateError::DownloadFailed(format!(
                "Incomplete download: expected {} bytes, got {}",
                total_size, bytes_downloaded
            )));
        }

        let temp_path = temp_file.path().to_path_buf();
        debug!("Download completed, saved to: {:?}", temp_path);

        Ok(temp_path)
    }

    /// Compute SHA256 checksum of a file
    ///
    /// # Arguments
    /// * `file_path` - Path to the file
    /// * `progress_callback` - Optional callback for progress updates
    ///
    /// # Returns
    /// Hex string representation of the SHA256 checksum
    pub fn compute_checksum<P: AsRef<Path>>(
        file_path: P,
        progress_callback: Option<Box<dyn Fn(u64) + Send>>,
    ) -> Result<String, UpdateError> {
        let file_path = file_path.as_ref();
        let mut file = std::fs::File::open(file_path)
            .map_err(|e| UpdateError::IoError(format!("Failed to open file for checksum: {}", e)))?;

        let file_size = file
            .metadata()
            .map_err(|e| UpdateError::IoError(format!("Failed to get file metadata: {}", e)))?
            .len();

        debug!("Computing SHA256 checksum for {} bytes", file_size);

        let mut hasher = Sha256::new();
        let mut buffer = vec![0; 8192];
        let mut bytes_processed = 0u64;

        loop {
            let bytes_read = std::io::Read::read(&mut file, &mut buffer)
                .map_err(|e| UpdateError::IoError(format!("Failed to read file: {}", e)))?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
            bytes_processed += bytes_read as u64;

            if let Some(ref callback) = progress_callback {
                callback(bytes_processed);
            }
        }

        let result = hasher.finalize();
        let checksum = format!("{:x}", result);

        debug!("Computed checksum: {}", checksum);
        Ok(checksum)
    }

    /// Download and verify a binary with SHA256 checksum
    ///
    /// # Arguments
    /// * `url` - The URL to download from
    /// * `expected_sha256` - Expected SHA256 checksum (hex string)
    /// * `output_path` - Where to save the verified binary
    ///
    /// # Returns
    /// BinaryDownload record with verification status
    pub async fn download_and_verify(
        &self,
        url: &str,
        expected_sha256: &str,
        output_path: &Path,
    ) -> Result<BinaryDownload, UpdateError> {
        // Download to temporary file
        let temp_path = self.download_to_temp(url, None).await?;

        // Compute checksum of downloaded file
        let actual_sha256 = Self::compute_checksum(&temp_path, None)?;

        // Verify checksum
        let verified = actual_sha256.to_lowercase() == expected_sha256.to_lowercase();

        if !verified {
            // Clean up temp file on verification failure
            let _ = std::fs::remove_file(&temp_path);
            return Err(UpdateError::ChecksumMismatch {
                expected: expected_sha256.to_string(),
                actual: actual_sha256,
            });
        }

        // Get file size
        let file_size = std::fs::metadata(&temp_path)
            .map_err(|e| UpdateError::IoError(format!("Failed to get file metadata: {}", e)))?
            .len();

        // Move temp file to output path (create parent directories if needed)
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| UpdateError::IoError(format!("Failed to create output directory: {}", e)))?;
            }
        }

        std::fs::rename(&temp_path, output_path)
            .map_err(|e| UpdateError::IoError(format!("Failed to move downloaded file: {}", e)))?;

        info!("Binary verified and saved to: {:?}", output_path);

        Ok(BinaryDownload {
            url: url.to_string(),
            expected_sha256: expected_sha256.to_string(),
            file_path: output_path.to_path_buf(),
            actual_sha256,
            file_size,
            verified: true,
        })
    }
}

impl Default for BinaryDownloader {
    fn default() -> Self {
        Self::new().expect("Failed to create default BinaryDownloader")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_config_default() {
        let config = DownloadConfig::default();
        assert_eq!(config.timeout_secs, 300);
        assert!(config.report_progress);
    }

    #[test]
    fn test_download_config_custom() {
        let config = DownloadConfig {
            timeout_secs: 60,
            report_progress: false,
        };
        assert_eq!(config.timeout_secs, 60);
        assert!(!config.report_progress);
    }

    #[test]
    fn test_binary_download_creation() {
        let download = BinaryDownload::new(
            "https://example.com/binary".to_string(),
            "abc123".to_string(),
            PathBuf::from("/tmp/binary"),
        );

        assert_eq!(download.url, "https://example.com/binary");
        assert_eq!(download.expected_sha256, "abc123");
        assert!(!download.verified);
    }

    #[test]
    fn test_compute_checksum_simple_file() {
        // Create a temporary file with known content
        let content = b"test content";
        let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let temp_path = temp_file.path().to_path_buf();

        std::fs::write(&temp_path, content).expect("Failed to write test file");

        let checksum = BinaryDownloader::compute_checksum(&temp_path, None).expect("Failed to compute checksum");

        // Verify the checksum is a valid hex string
        assert!(checksum.len() > 0);
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));

        // Clean up
        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_compute_checksum_empty_file() {
        let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let temp_path = temp_file.path().to_path_buf();

        let checksum = BinaryDownloader::compute_checksum(&temp_path, None).expect("Failed to compute checksum");

        // SHA256 of empty file is well-known
        assert_eq!(checksum, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");

        // Clean up
        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_compute_checksum_nonexistent_file() {
        let result = BinaryDownloader::compute_checksum("/nonexistent/file", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_binary_downloader_creation() {
        let downloader = BinaryDownloader::new();
        assert!(downloader.is_ok());
    }

    #[test]
    fn test_binary_downloader_with_config() {
        let config = DownloadConfig {
            timeout_secs: 60,
            report_progress: false,
        };
        let downloader = BinaryDownloader::with_config(config);
        assert!(downloader.is_ok());
    }

    #[tokio::test]
    async fn test_download_to_temp_invalid_url() {
        let downloader = BinaryDownloader::new().expect("Failed to create downloader");
        let result = downloader.download_to_temp("https://invalid.example.com/nonexistent", None).await;

        // Should fail with download error
        assert!(result.is_err());
    }
}
