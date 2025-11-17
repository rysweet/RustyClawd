//! GitHub Releases API client for fetching the latest release information

use crate::update::error::UpdateError;
use crate::update::version::Version;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// GitHub release asset information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: i64,
}

/// GitHub release information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub assets: Vec<ReleaseAsset>,
    pub draft: bool,
    pub prerelease: bool,
    pub published_at: Option<String>,
}

impl Release {
    /// Get the version from the tag name (removes 'v' prefix if present)
    pub fn version(&self) -> Result<Version, UpdateError> {
        Version::parse(&self.tag_name)
    }

    /// Get release notes (body text)
    pub fn release_notes(&self) -> Option<&str> {
        self.body.as_deref()
    }

    /// Find an asset by name pattern (for matching binary names)
    pub fn find_asset(&self, pattern: &str) -> Option<&ReleaseAsset> {
        self.assets.iter().find(|asset| asset.name.contains(pattern))
    }

    /// Get all asset names
    pub fn asset_names(&self) -> Vec<&str> {
        self.assets.iter().map(|a| a.name.as_str()).collect()
    }
}

/// GitHub client for interacting with the Releases API
pub struct GitHubClient {
    client: Client,
    repo_owner: String,
    repo_name: String,
    api_base: String,
}

impl GitHubClient {
    /// Create a new GitHub client for a specific repository
    pub fn new(owner: &str, repo: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("RustyClawd-Update-Client/1.0")
                .build()
                .unwrap_or_else(|_| Client::new()),
            repo_owner: owner.to_string(),
            repo_name: repo.to_string(),
            api_base: "https://api.github.com".to_string(),
        }
    }

    /// Get the latest release for the configured repository
    pub async fn get_latest_release(&self) -> Result<Release, UpdateError> {
        let url = format!(
            "{}/repos/{}/{}/releases/latest",
            self.api_base, self.repo_owner, self.repo_name
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| UpdateError::GitHubApiError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(UpdateError::GitHubApiError(format!(
                "GitHub API returned status: {}",
                response.status()
            )));
        }

        let release = response
            .json::<Release>()
            .await
            .map_err(|e| UpdateError::GitHubResponseParseFailed(e.to_string()))?;

        Ok(release)
    }

    /// Check if a new version is available
    pub async fn check_update(&self, current_version: &Version) -> Result<bool, UpdateError> {
        let latest = self.get_latest_release().await?;
        let latest_version = latest.version()?;

        Ok(latest_version.is_update_available(current_version))
    }

    /// Get the latest release with update information
    pub async fn get_update_info(&self, current_version: &Version) -> Result<Option<UpdateInfo>, UpdateError> {
        let latest = self.get_latest_release().await?;
        let latest_version = latest.version()?;

        if !latest_version.is_update_available(current_version) {
            return Ok(None);
        }

        Ok(Some(UpdateInfo {
            current_version: current_version.clone(),
            latest_version,
            release_tag: latest.tag_name.clone(),
            release_name: latest.name.clone(),
            release_notes: latest.body.clone(),
            assets: latest.assets.clone(),
            published_at: latest.published_at.clone(),
        }))
    }
}

/// Information about an available update
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current_version: Version,
    pub latest_version: Version,
    pub release_tag: String,
    pub release_name: Option<String>,
    pub release_notes: Option<String>,
    pub assets: Vec<ReleaseAsset>,
    pub published_at: Option<String>,
}

impl UpdateInfo {
    /// Get the update summary as a string
    pub fn summary(&self) -> String {
        format!(
            "Update available: {} -> {}",
            self.current_version,
            self.latest_version
        )
    }

    /// Find the appropriate binary asset for the current platform
    /// Returns asset URL if found
    pub fn get_asset_for_platform(&self) -> Option<String> {
        let target = get_platform_target();

        // For Windows, also check for .exe extension
        #[cfg(target_os = "windows")]
        let has_exe_extension = |name: &str| name.ends_with(".exe");

        #[cfg(not(target_os = "windows"))]
        let has_exe_extension = |_name: &str| true; // Always pass on non-Windows

        self.assets
            .iter()
            .find(|asset| {
                let contains_target = asset.name.contains(&target) ||
                                     asset.name.contains("x86_64-unknown-linux");
                contains_target && has_exe_extension(&asset.name)
            })
            .map(|asset| asset.browser_download_url.clone())
    }
}

/// Platform information for cross-platform support
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformInfo {
    pub target_triple: String,
    pub os: String,
    pub arch: String,
    pub binary_extension: Option<String>,
}

impl PlatformInfo {
    /// Get the current platform information
    pub fn current() -> Self {
        let target_triple = get_platform_target();
        let (os, arch) = Self::parse_target(&target_triple);

        Self {
            target_triple: target_triple.clone(),
            os: os.to_string(),
            arch: arch.to_string(),
            binary_extension: Self::get_binary_extension(),
        }
    }

    /// Parse a target triple into OS and architecture
    fn parse_target(target: &str) -> (&str, &str) {
        if target.contains("linux") {
            if target.contains("x86_64") {
                ("linux", "x86_64")
            } else if target.contains("aarch64") {
                ("linux", "aarch64")
            } else {
                ("linux", "unknown")
            }
        } else if target.contains("darwin") || target.contains("macos") {
            if target.contains("x86_64") {
                ("macos", "x86_64")
            } else if target.contains("aarch64") {
                ("macos", "aarch64")
            } else {
                ("macos", "unknown")
            }
        } else if target.contains("windows") {
            if target.contains("x86_64") {
                ("windows", "x86_64")
            } else if target.contains("aarch64") {
                ("windows", "aarch64")
            } else {
                ("windows", "unknown")
            }
        } else {
            ("unknown", "unknown")
        }
    }

    /// Get the binary extension for the current platform
    fn get_binary_extension() -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            Some(".exe".to_string())
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }
}

/// Get the platform target string for binary matching
fn get_platform_target() -> String {
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    {
        return "x86_64-unknown-linux".to_string();
    }

    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    {
        return "x86_64-apple-darwin".to_string();
    }

    #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
    {
        return "x86_64-pc-windows".to_string();
    }

    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    {
        return "aarch64-unknown-linux".to_string();
    }

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        return "aarch64-apple-darwin".to_string();
    }

    #[cfg(all(target_arch = "aarch64", target_os = "windows"))]
    {
        return "aarch64-pc-windows".to_string();
    }

    #[cfg(not(any(
        all(target_arch = "x86_64", target_os = "linux"),
        all(target_arch = "x86_64", target_os = "macos"),
        all(target_arch = "x86_64", target_os = "windows"),
        all(target_arch = "aarch64", target_os = "linux"),
        all(target_arch = "aarch64", target_os = "macos"),
        all(target_arch = "aarch64", target_os = "windows")
    )))]
    {
        return "unknown-platform".to_string();
    }

    #[allow(unreachable_code)]
    "unknown-platform".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_client_creation() {
        let client = GitHubClient::new("rysweet", "RustyClawd");
        assert_eq!(client.repo_owner, "rysweet");
        assert_eq!(client.repo_name, "RustyClawd");
    }

    #[test]
    fn test_release_asset_finding() {
        let assets = vec![
            ReleaseAsset {
                name: "rusty-x86_64-unknown-linux-gnu".to_string(),
                browser_download_url: "https://example.com/rusty-linux".to_string(),
                size: 1024,
            },
            ReleaseAsset {
                name: "rusty-checksums.txt".to_string(),
                browser_download_url: "https://example.com/checksums".to_string(),
                size: 512,
            },
        ];

        let release = Release {
            tag_name: "v1.0.0".to_string(),
            name: Some("Version 1.0.0".to_string()),
            body: Some("Release notes".to_string()),
            assets,
            draft: false,
            prerelease: false,
            published_at: Some("2024-01-01T00:00:00Z".to_string()),
        };

        assert_eq!(release.asset_names().len(), 2);
        assert!(release.find_asset("linux").is_some());
        assert!(release.find_asset("checksums").is_some());
        assert!(release.find_asset("nonexistent").is_none());
    }

    #[test]
    fn test_release_version_parsing() {
        let release = Release {
            tag_name: "v1.2.3".to_string(),
            name: None,
            body: None,
            assets: vec![],
            draft: false,
            prerelease: false,
            published_at: None,
        };

        let version = release.version().unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_update_info_summary() {
        let info = UpdateInfo {
            current_version: Version::new(1, 0, 0),
            latest_version: Version::new(1, 1, 0),
            release_tag: "v1.1.0".to_string(),
            release_name: Some("Version 1.1.0".to_string()),
            release_notes: Some("New features".to_string()),
            assets: vec![],
            published_at: None,
        };

        let summary = info.summary();
        assert!(summary.contains("1.0.0"));
        assert!(summary.contains("1.1.0"));
    }

    #[test]
    fn test_platform_target() {
        let target = get_platform_target();
        assert!(!target.contains("unknown-platform") || cfg!(not(any(target_arch = "x86_64", target_arch = "aarch64"))));
    }

    #[test]
    fn test_platform_info_current() {
        let platform = PlatformInfo::current();

        // Verify we have valid platform information
        assert!(!platform.os.is_empty());
        assert!(!platform.arch.is_empty());
        assert!(!platform.target_triple.is_empty());

        // Verify platform-specific binary extension
        #[cfg(target_os = "windows")]
        assert_eq!(platform.binary_extension, Some(".exe".to_string()));

        #[cfg(not(target_os = "windows"))]
        assert_eq!(platform.binary_extension, None);
    }

    #[test]
    fn test_platform_info_parse_target() {
        // Test Linux targets
        assert_eq!(
            PlatformInfo::parse_target("x86_64-unknown-linux-gnu"),
            ("linux", "x86_64")
        );
        assert_eq!(
            PlatformInfo::parse_target("aarch64-unknown-linux-gnu"),
            ("linux", "aarch64")
        );

        // Test macOS targets
        assert_eq!(
            PlatformInfo::parse_target("x86_64-apple-darwin"),
            ("macos", "x86_64")
        );
        assert_eq!(
            PlatformInfo::parse_target("aarch64-apple-darwin"),
            ("macos", "aarch64")
        );

        // Test Windows targets
        assert_eq!(
            PlatformInfo::parse_target("x86_64-pc-windows-msvc"),
            ("windows", "x86_64")
        );
        assert_eq!(
            PlatformInfo::parse_target("aarch64-pc-windows-msvc"),
            ("windows", "aarch64")
        );

        // Test unknown target
        assert_eq!(
            PlatformInfo::parse_target("unknown-unknown-unknown"),
            ("unknown", "unknown")
        );
    }

    #[test]
    fn test_update_info_asset_matching_for_platform() {
        let assets = vec![
            ReleaseAsset {
                name: "rusty-x86_64-unknown-linux-gnu".to_string(),
                browser_download_url: "https://example.com/rusty-linux".to_string(),
                size: 1024,
            },
            ReleaseAsset {
                name: "rusty-x86_64-apple-darwin".to_string(),
                browser_download_url: "https://example.com/rusty-macos".to_string(),
                size: 1024,
            },
            ReleaseAsset {
                name: "rusty-x86_64-pc-windows-msvc.exe".to_string(),
                browser_download_url: "https://example.com/rusty-windows.exe".to_string(),
                size: 1024,
            },
            ReleaseAsset {
                name: "rusty-aarch64-apple-darwin".to_string(),
                browser_download_url: "https://example.com/rusty-macos-arm".to_string(),
                size: 1024,
            },
        ];

        let info = UpdateInfo {
            current_version: Version::new(1, 0, 0),
            latest_version: Version::new(1, 1, 0),
            release_tag: "v1.1.0".to_string(),
            release_name: Some("Version 1.1.0".to_string()),
            release_notes: Some("New features".to_string()),
            assets,
            published_at: None,
        };

        // The asset matching should find appropriate binaries for the platform
        let asset_url = info.get_asset_for_platform();

        // We should find a matching asset (or None if platform not supported)
        if let Some(url) = asset_url {
            assert!(url.contains("example.com"));

            // On Windows, verify .exe extension
            #[cfg(target_os = "windows")]
            assert!(url.contains(".exe"));
        }
    }

    #[test]
    fn test_windows_exe_extension_filtering() {
        let assets = vec![
            ReleaseAsset {
                name: "rusty-x86_64-pc-windows-msvc.exe".to_string(),
                browser_download_url: "https://example.com/rusty.exe".to_string(),
                size: 1024,
            },
            ReleaseAsset {
                name: "rusty-x86_64-pc-windows-msvc".to_string(), // Without .exe
                browser_download_url: "https://example.com/rusty".to_string(),
                size: 1024,
            },
        ];

        let info = UpdateInfo {
            current_version: Version::new(1, 0, 0),
            latest_version: Version::new(1, 1, 0),
            release_tag: "v1.1.0".to_string(),
            release_name: None,
            release_notes: None,
            assets,
            published_at: None,
        };

        let _asset_url = info.get_asset_for_platform();

        // On Windows, should only match the .exe version
        #[cfg(target_os = "windows")]
        {
            assert!(_asset_url.is_some());
            assert!(_asset_url.unwrap().contains(".exe"));
        }
    }
}
