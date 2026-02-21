//! GitHub Releases API client for fetching the latest release information

use crate::update::error::UpdateError;
use crate::update::version::Version;
use crate::update::version_compare::UpdateInfo;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Command;
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
        self.assets
            .iter()
            .find(|asset| asset.name.contains(pattern))
    }

    /// Get all asset names
    pub fn asset_names(&self) -> Vec<&str> {
        self.assets.iter().map(|a| a.name.as_str()).collect()
    }
}

/// Minimal release information from fallback sources (gh CLI or git)
/// Contains only the tag name, which is sufficient for version checking
#[derive(Debug, Clone)]
struct MinimalRelease {
    tag_name: String,
}

impl MinimalRelease {
    /// Convert MinimalRelease to full Release struct with empty fields
    fn into_release(self) -> Release {
        Release {
            tag_name: self.tag_name,
            name: None,
            body: None,
            assets: vec![],
            draft: false,
            prerelease: false,
            published_at: None,
        }
    }
}

/// GitHub client for interacting with the Releases API
pub struct GitHubClient {
    client: Client,
    repo_owner: String,
    repo_name: String,
    api_base: String,
}

/// Check if a command is available in the system PATH
fn command_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Execute a command and return stdout as String if successful
fn execute_command(cmd: &str, args: &[&str]) -> Result<String, UpdateError> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| UpdateError::IoError(format!("Failed to execute {}: {}", cmd, e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(UpdateError::GitHubApiError(format!(
            "{} command failed: {}",
            cmd, stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
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

    /// Get the latest release from the GitHub API
    /// Returns PrivateRepositoryAccess error for 401/403/404 status codes
    async fn get_latest_release_from_api(&self) -> Result<Release, UpdateError> {
        let url = format!(
            "{}/repos/{}/{}/releases/latest",
            self.api_base, self.repo_owner, self.repo_name
        );

        tracing::debug!("Fetching latest release from GitHub API: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| UpdateError::GitHubApiError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();

            // Detect private repository access errors (401/403) or 404
            if status_code == 401 || status_code == 403 {
                tracing::warn!(
                    "Private repository access detected (HTTP {}), will try fallback methods",
                    status_code
                );
                return Err(UpdateError::PrivateRepositoryAccess {
                    status: status_code,
                });
            }

            // Handle 404 specifically - could be no releases or private repo
            if status_code == 404 {
                // Try to determine if this is a private repo or just no releases
                // For now, treat 404 as NoReleasesAvailable since fallback will handle private repos
                return Err(UpdateError::NoReleasesAvailable);
            }

            return Err(UpdateError::GitHubApiError(format!(
                "GitHub API returned status: {}",
                status
            )));
        }

        let release = response
            .json::<Release>()
            .await
            .map_err(|e| UpdateError::GitHubResponseParseFailed(e.to_string()))?;

        tracing::info!(
            "Successfully fetched release {} from GitHub API",
            release.tag_name
        );
        Ok(release)
    }

    /// Get the latest release, trying API first, then fallback methods
    pub async fn get_latest_release(&self) -> Result<Release, UpdateError> {
        // Try API first
        match self.get_latest_release_from_api().await {
            Ok(release) => Ok(release),
            Err(UpdateError::PrivateRepositoryAccess { .. }) => {
                tracing::info!("Attempting fallback methods for private repository");
                self.get_latest_release_fallback().await
            }
            Err(e) => Err(e),
        }
    }

    /// Check if a new version is available
    pub async fn check_update(&self, current_version: &Version) -> Result<bool, UpdateError> {
        let latest = self.get_latest_release().await?;
        let latest_version = latest.version()?;

        Ok(latest_version.is_update_available(current_version))
    }

    /// Get the latest release with update information
    pub async fn get_update_info(
        &self,
        current_version: &Version,
    ) -> Result<Option<UpdateInfo>, UpdateError> {
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

    /// Try fallback methods to get latest release for private repositories
    /// Returns Release with minimal information (tag_name only)
    async fn get_latest_release_fallback(&self) -> Result<Release, UpdateError> {
        // Try gh CLI first
        if let Ok(release) = self.try_gh_cli() {
            tracing::info!("Successfully fetched release from gh CLI");
            return Ok(release.into_release());
        }

        // Fall back to git ls-remote
        if let Ok(release) = self.try_git_ls_remote() {
            tracing::info!("Successfully fetched release from git ls-remote");
            return Ok(release.into_release());
        }

        Err(UpdateError::GitHubApiError(
            "All fallback methods failed to fetch release information".to_string(),
        ))
    }

    /// Try to get latest release using gh CLI
    fn try_gh_cli(&self) -> Result<MinimalRelease, UpdateError> {
        if !command_available("gh") {
            tracing::debug!("gh CLI not available");
            return Err(UpdateError::GitHubApiError(
                "gh CLI not available".to_string(),
            ));
        }

        tracing::info!("Trying gh CLI for release information");

        let repo = format!("{}/{}", self.repo_owner, self.repo_name);
        let output = execute_command(
            "gh",
            &[
                "release", "list", "--repo", &repo, "--limit", "1", "--json", "tagName",
            ],
        )?;

        // Parse JSON output: [{"tagName": "v1.2.3"}]
        let releases: Vec<serde_json::Value> = serde_json::from_str(&output)
            .map_err(|e| UpdateError::GitHubResponseParseFailed(e.to_string()))?;

        if releases.is_empty() {
            return Err(UpdateError::GitHubApiError(
                "No releases found via gh CLI".to_string(),
            ));
        }

        let tag_name = releases[0]["tagName"]
            .as_str()
            .ok_or_else(|| UpdateError::GitHubResponseParseFailed("Missing tagName".to_string()))?
            .to_string();

        Ok(MinimalRelease { tag_name })
    }

    /// Try to get latest release using git ls-remote
    fn try_git_ls_remote(&self) -> Result<MinimalRelease, UpdateError> {
        if !command_available("git") {
            tracing::debug!("git not available");
            return Err(UpdateError::GitHubApiError("git not available".to_string()));
        }

        tracing::info!("Trying git ls-remote for release information");

        let repo_url = format!(
            "https://github.com/{}/{}.git",
            self.repo_owner, self.repo_name
        );
        let output = execute_command(
            "git",
            &[
                "ls-remote",
                "--tags",
                "--refs",
                "--sort=-v:refname",
                &repo_url,
            ],
        )?;

        // Parse output: each line is "commit_hash\trefs/tags/tagname"
        // We want the first line (latest version)
        for line in output.lines() {
            if let Some(tag_ref) = line.split('\t').nth(1) {
                if let Some(tag_name) = tag_ref.strip_prefix("refs/tags/") {
                    // Validate it looks like a version tag
                    let normalized = tag_name.trim_start_matches('v');
                    if Version::parse(normalized).is_ok() {
                        return Ok(MinimalRelease {
                            tag_name: tag_name.to_string(),
                        });
                    }
                }
            }
        }

        Err(UpdateError::GitHubApiError(
            "No valid version tags found via git ls-remote".to_string(),
        ))
    }
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
    fn test_minimal_release_to_release_conversion() {
        let minimal = MinimalRelease {
            tag_name: "v1.2.3".to_string(),
        };

        let release = minimal.into_release();
        assert_eq!(release.tag_name, "v1.2.3");
        assert_eq!(release.name, None);
        assert_eq!(release.body, None);
        assert!(release.assets.is_empty());
        assert!(!release.draft);
        assert!(!release.prerelease);
        assert_eq!(release.published_at, None);
    }

    #[test]
    fn test_minimal_release_version_parsing() {
        let minimal = MinimalRelease {
            tag_name: "v2.5.10".to_string(),
        };

        let release = minimal.into_release();
        let version = release.version().unwrap();
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 5);
        assert_eq!(version.patch, 10);
    }

    #[test]
    fn test_command_available_nonexistent() {
        // A command that definitely doesn't exist
        let available = command_available("this-command-does-not-exist-xyz123");
        assert!(!available);
    }

    #[test]
    fn test_execute_command_success() {
        // Test with a simple command that should work on all platforms
        let result = execute_command("git", &["--version"]);
        assert!(result.is_ok());
        if let Ok(output) = result {
            assert!(output.contains("git"));
        }
    }

    #[test]
    fn test_execute_command_failure() {
        // Test with an invalid git command
        let result = execute_command("git", &["this-is-not-a-valid-command"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_command_nonexistent() {
        // Test with a command that doesn't exist
        let result = execute_command("nonexistent-command-xyz", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_git_ls_remote_tag_parsing() {
        // Simulate git ls-remote output
        let output = r#"abc123	refs/tags/v1.0.0
def456	refs/tags/v1.1.0
ghi789	refs/tags/v1.2.3
jkl012	refs/tags/invalid-tag
mno345	refs/tags/2.0.0"#;

        // Parse the first valid version tag
        let mut found_tag = None;
        for line in output.lines() {
            if let Some(tag_ref) = line.split('\t').nth(1) {
                if let Some(tag_name) = tag_ref.strip_prefix("refs/tags/") {
                    let normalized = tag_name.trim_start_matches('v');
                    if Version::parse(normalized).is_ok() {
                        found_tag = Some(tag_name.to_string());
                        break;
                    }
                }
            }
        }

        assert_eq!(found_tag, Some("v1.0.0".to_string()));
    }

    #[test]
    fn test_git_ls_remote_tag_parsing_without_v_prefix() {
        // Test parsing tags without 'v' prefix
        let output = "abc123\trefs/tags/2.5.3";

        let mut found_tag = None;
        for line in output.lines() {
            if let Some(tag_ref) = line.split('\t').nth(1) {
                if let Some(tag_name) = tag_ref.strip_prefix("refs/tags/") {
                    let normalized = tag_name.trim_start_matches('v');
                    if Version::parse(normalized).is_ok() {
                        found_tag = Some(tag_name.to_string());
                        break;
                    }
                }
            }
        }

        assert_eq!(found_tag, Some("2.5.3".to_string()));
    }

    #[test]
    fn test_gh_cli_json_parsing() {
        // Simulate gh CLI JSON output
        let json_output = r#"[{"tagName":"v1.2.3"}]"#;

        let releases: Vec<serde_json::Value> = serde_json::from_str(json_output).unwrap();
        assert!(!releases.is_empty());

        let tag_name = releases[0]["tagName"].as_str().unwrap();
        assert_eq!(tag_name, "v1.2.3");
    }

    #[test]
    fn test_gh_cli_json_parsing_empty() {
        // Simulate empty gh CLI output
        let json_output = "[]";

        let releases: Vec<serde_json::Value> = serde_json::from_str(json_output).unwrap();
        assert!(releases.is_empty());
    }

    #[test]
    fn test_gh_cli_json_parsing_multiple_releases() {
        // Simulate gh CLI with multiple releases (we only use the first)
        let json_output = r#"[{"tagName":"v2.0.0"},{"tagName":"v1.9.0"}]"#;

        let releases: Vec<serde_json::Value> = serde_json::from_str(json_output).unwrap();
        assert_eq!(releases.len(), 2);

        let tag_name = releases[0]["tagName"].as_str().unwrap();
        assert_eq!(tag_name, "v2.0.0");
    }
}
