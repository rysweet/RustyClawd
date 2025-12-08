//! Feature Discovery Brick
//!
//! Fetches Claude Code changelog and documentation to discover features.
//!
//! # Philosophy
//! - One responsibility: Fetch and parse Claude Code features
//! - Working implementation: Uses real HTTP requests
//! - Simple parsing: Markdown-based feature extraction

use crate::claude_code_sync::types::ClaudeFeature;
use anyhow::{Context, Result};
use reqwest::Client;

/// Feature discovery for Claude Code
pub struct FeatureDiscovery {
    client: Client,
}

impl FeatureDiscovery {
    /// Create a new feature discovery instance
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("RustyClawd-SyncMonitor/1.0")
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Fetch features from Claude Code sources
    pub async fn fetch_features(&self) -> Result<Vec<ClaudeFeature>> {
        let mut features = Vec::new();

        // Fetch from Claude Code changelog
        let changelog_features = self.fetch_from_changelog().await?;
        features.extend(changelog_features);

        // Fetch from Claude Code documentation
        let docs_features = self.fetch_from_docs().await?;
        features.extend(docs_features);

        // Deduplicate by name
        features.sort_by(|a, b| a.name.cmp(&b.name));
        features.dedup_by(|a, b| a.name == b.name);

        Ok(features)
    }

    /// Fetch features from Claude Code changelog
    async fn fetch_from_changelog(&self) -> Result<Vec<ClaudeFeature>> {
        // Claude Code changelog is typically in their docs or release notes
        let url = "https://docs.anthropic.com/en/docs/changelog";

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch Claude Code changelog")?;

        let html = response
            .text()
            .await
            .context("Failed to read changelog response")?;

        // Parse features from changelog
        self.parse_changelog(&html)
    }

    /// Fetch features from Claude Code documentation
    async fn fetch_from_docs(&self) -> Result<Vec<ClaudeFeature>> {
        // Claude Code tools documentation
        let url = "https://docs.anthropic.com/en/docs/build-with-claude/tool-use";

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to fetch Claude Code docs")?;

        let html = response
            .text()
            .await
            .context("Failed to read docs response")?;

        // Parse features from documentation
        self.parse_docs(&html)
    }

    /// Parse features from changelog HTML
    fn parse_changelog(&self, html: &str) -> Result<Vec<ClaudeFeature>> {
        let mut features = Vec::new();

        // Convert HTML to markdown for easier parsing
        let markdown = html2md::parse_html(html);

        // Look for tool/feature mentions in changelog
        for line in markdown.lines() {
            // Look for tool mentions (e.g., "Bash tool", "Read tool")
            if let Some(feature) = self.extract_feature_from_line(line, "tools") {
                features.push(feature);
            }
        }

        Ok(features)
    }

    /// Parse features from documentation HTML
    fn parse_docs(&self, html: &str) -> Result<Vec<ClaudeFeature>> {
        let mut features = Vec::new();

        // Convert HTML to markdown
        let markdown = html2md::parse_html(html);

        // Look for tool definitions
        for line in markdown.lines() {
            if let Some(feature) = self.extract_feature_from_line(line, "tools") {
                features.push(feature);
            }
        }

        Ok(features)
    }

    /// Extract a feature from a line of text
    fn extract_feature_from_line(&self, line: &str, category: &str) -> Option<ClaudeFeature> {
        // Look for common patterns in Claude Code documentation
        let patterns = [
            r"(?i)(bash|read|write|edit|glob|grep|task|skill|slash_?command|web_?fetch|web_?search|todo_?write|notebook_?edit|kill_?shell|ask_?user|bash_?output|agent)\s+(tool|capability|feature)",
            r"(?i)##\s+(.*?)\s+(tool|capability)",
            r"(?i)`(.*?)`.*?tool",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(line) {
                    let name = captures.get(1).map(|m| m.as_str().trim().to_string())?;

                    return Some(ClaudeFeature {
                        name,
                        category: category.to_string(),
                        description: line.trim().to_string(),
                        since_version: None,
                    });
                }
            }
        }

        None
    }
}

impl Default for FeatureDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bash_tool() {
        let discovery = FeatureDiscovery::new();
        let line = "The Bash tool allows executing shell commands";

        let feature = discovery.extract_feature_from_line(line, "tools");
        assert!(feature.is_some());

        let feature = feature.unwrap();
        assert_eq!(feature.name.to_lowercase(), "bash");
        assert_eq!(feature.category, "tools");
    }

    #[test]
    fn test_extract_from_header() {
        let discovery = FeatureDiscovery::new();
        let line = "## Read Tool";

        let feature = discovery.extract_feature_from_line(line, "tools");
        assert!(feature.is_some());

        let feature = feature.unwrap();
        assert_eq!(feature.name.to_lowercase(), "read");
    }

    #[test]
    fn test_no_match() {
        let discovery = FeatureDiscovery::new();
        let line = "This is just a regular sentence";

        let feature = discovery.extract_feature_from_line(line, "tools");
        assert!(feature.is_none());
    }
}
