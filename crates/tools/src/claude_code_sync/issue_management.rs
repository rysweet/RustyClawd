//! Issue Management Brick
//!
//! Creates GitHub issues for feature gaps and maintains a ledger to prevent duplicates.
//!
//! # Philosophy
//! - One responsibility: Create GitHub issues with deduplication
//! - Ledger-based: Tracks created issues in ledger.json
//! - Real GitHub API: No mocks, uses actual GitHub API

use crate::claude_code_sync::types::{FeatureGap, GapType, IssueLedger};
use anyhow::{Context, Result};
use serde_json::json;
use std::path::Path;

/// Issue manager for creating GitHub issues
pub struct IssueManager {
    ledger_path: String,
    github_token: String,
    repo: String,
    client: reqwest::Client,
}

impl IssueManager {
    /// Create a new issue manager
    pub fn new(
        ledger_path: impl Into<String>,
        github_token: impl Into<String>,
        repo: impl Into<String>,
    ) -> Self {
        Self {
            ledger_path: ledger_path.into(),
            github_token: github_token.into(),
            repo: repo.into(),
            client: reqwest::Client::builder()
                .user_agent("RustyClawd-SyncMonitor/1.0")
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create issues for gaps
    pub async fn create_issues(&mut self, gaps: &[FeatureGap]) -> Result<Vec<IssueCreated>> {
        // Load ledger
        let mut ledger = self.load_ledger().await?;

        let mut created_issues = Vec::new();

        for gap in gaps {
            // Check if issue already exists in ledger
            let feature_key = self.make_feature_key(&gap.claude_feature.name);

            if ledger.issues.contains_key(&feature_key) {
                continue; // Skip, already tracked
            }

            // Create GitHub issue
            match self.create_github_issue(gap).await {
                Ok(issue) => {
                    // Record in ledger
                    ledger.issues.insert(feature_key, issue.number);
                    created_issues.push(issue);
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to create issue for {}: {}",
                        gap.claude_feature.name, e
                    );
                }
            }
        }

        // Update ledger timestamp
        ledger.last_sync = Some(chrono::Utc::now().to_rfc3339());

        // Save ledger
        self.save_ledger(&ledger).await?;

        Ok(created_issues)
    }

    /// Load ledger from file
    async fn load_ledger(&self) -> Result<IssueLedger> {
        let path = Path::new(&self.ledger_path);

        if !path.exists() {
            return Ok(IssueLedger::default());
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read ledger file")?;

        serde_json::from_str(&content).context("Failed to parse ledger JSON")
    }

    /// Save ledger to file
    async fn save_ledger(&self, ledger: &IssueLedger) -> Result<()> {
        let content = serde_json::to_string_pretty(ledger)?;

        // Ensure parent directory exists
        if let Some(parent) = Path::new(&self.ledger_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&self.ledger_path, content)
            .await
            .context("Failed to write ledger file")
    }

    /// Create a GitHub issue for a gap
    async fn create_github_issue(&self, gap: &FeatureGap) -> Result<IssueCreated> {
        let title = format!("Feature Gap: {}", gap.claude_feature.name);
        let body = self.format_issue_body(gap);

        let url = format!("https://api.github.com/repos/{}/issues", self.repo);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.github_token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&json!({
                "title": title,
                "body": body,
                "labels": ["feature-gap", "claude-code-sync"]
            }))
            .send()
            .await
            .context("Failed to create GitHub issue")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error {}: {}", status, error_text);
        }

        let issue_data: serde_json::Value = response.json().await?;

        Ok(IssueCreated {
            number: issue_data["number"].as_u64().unwrap_or(0),
            title,
            url: issue_data["html_url"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Format issue body
    fn format_issue_body(&self, gap: &FeatureGap) -> String {
        let gap_type_str = match gap.gap_type {
            GapType::Missing => "completely missing",
            GapType::Incomplete => "partially implemented",
            GapType::Drift => "may have diverged",
        };

        let mut body = format!(
            "## Feature Gap Detected\n\n\
            **Claude Code Feature**: {}\n\
            **Category**: {}\n\
            **Status**: This feature is {}\n\n",
            gap.claude_feature.name, gap.claude_feature.category, gap_type_str
        );

        body.push_str(&format!(
            "### Description\n\n{}\n\n",
            gap.claude_feature.description
        ));

        if let Some(rc_feature) = &gap.rustyclawd_status {
            body.push_str(&format!(
                "### Current RustyClawd Status\n\n\
                - **Status**: {:?}\n",
                rc_feature.status
            ));

            if let Some(notes) = &rc_feature.notes {
                body.push_str(&format!("- **Notes**: {}\n", notes));
            }
            body.push('\n');
        }

        body.push_str(
            "---\n\
            *This issue was automatically created by the Claude Code Sync Monitor*\n",
        );

        body
    }

    /// Make a unique key for a feature
    fn make_feature_key(&self, name: &str) -> String {
        name.to_lowercase().replace(" ", "_")
    }
}

/// Issue created in GitHub
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IssueCreated {
    pub number: u64,
    pub title: String,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude_code_sync::types::{ClaudeFeature, FeatureStatus, RustyClawdFeature};

    #[test]
    fn test_make_feature_key() {
        let manager = IssueManager::new("test.json", "token", "owner/repo");

        assert_eq!(manager.make_feature_key("Bash Tool"), "bash_tool");
        assert_eq!(manager.make_feature_key("SlashCommand"), "slashcommand");
    }

    #[test]
    fn test_format_issue_body_missing() {
        let manager = IssueManager::new("test.json", "token", "owner/repo");

        let gap = FeatureGap {
            claude_feature: ClaudeFeature {
                name: "TestTool".to_string(),
                category: "tools".to_string(),
                description: "A test tool".to_string(),
                since_version: None,
            },
            rustyclawd_status: None,
            gap_type: GapType::Missing,
        };

        let body = manager.format_issue_body(&gap);

        assert!(body.contains("TestTool"));
        assert!(body.contains("completely missing"));
        assert!(body.contains("A test tool"));
    }

    #[test]
    fn test_format_issue_body_incomplete() {
        let manager = IssueManager::new("test.json", "token", "owner/repo");

        let gap = FeatureGap {
            claude_feature: ClaudeFeature {
                name: "TestTool".to_string(),
                category: "tools".to_string(),
                description: "A test tool".to_string(),
                since_version: None,
            },
            rustyclawd_status: Some(RustyClawdFeature {
                name: "TestTool".to_string(),
                category: "tools".to_string(),
                status: FeatureStatus::Partial,
                notes: Some("Work in progress".to_string()),
            }),
            gap_type: GapType::Incomplete,
        };

        let body = manager.format_issue_body(&gap);

        assert!(body.contains("partially implemented"));
        assert!(body.contains("Work in progress"));
    }

    #[tokio::test]
    async fn test_ledger_roundtrip() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let ledger_path = dir.path().join("ledger.json");

        let manager = IssueManager::new(
            ledger_path.to_string_lossy().to_string(),
            "token",
            "owner/repo",
        );

        // Save ledger
        let mut ledger = IssueLedger::default();
        ledger.issues.insert("test_feature".to_string(), 123);
        manager.save_ledger(&ledger).await.unwrap();

        // Load ledger
        let loaded = manager.load_ledger().await.unwrap();
        assert_eq!(loaded.issues.get("test_feature"), Some(&123));
    }
}
