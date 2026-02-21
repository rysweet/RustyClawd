//! Update information, platform detection, and version comparison utilities
//!
//! Extracted from github_client.rs to separate version/platform concerns
//! from HTTP client logic.

use crate::update::github_client::ReleaseAsset;
use crate::update::version::Version;
use serde::Serialize;

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
            self.current_version, self.latest_version
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
            .find(|asset| asset.name.contains(&target) && has_exe_extension(&asset.name))
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
    pub(crate) fn parse_target(target: &str) -> (&str, &str) {
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
pub fn get_platform_target() -> String {
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
        assert!(
            !target.contains("unknown-platform")
                || cfg!(not(any(target_arch = "x86_64", target_arch = "aarch64")))
        );
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
