//! Feature Mapping Brick
//!
//! Maps Claude Code features against RustyClawd inventory to identify gaps.
//!
//! # Philosophy
//! - One responsibility: Compare feature sets and identify gaps
//! - File-based: Reads from feature_inventory.yaml
//! - Simple matching: Name-based with fuzzy tolerance

use crate::claude_code_sync::types::{
    ClaudeFeature, FeatureGap, FeatureInventory, FeatureStatus, GapType, RustyClawdFeature,
};
use anyhow::{Context, Result};
use std::path::Path;

/// Feature mapper for comparing Claude Code and RustyClawd
pub struct FeatureMapper {
    inventory_path: String,
}

impl FeatureMapper {
    /// Create a new feature mapper
    pub fn new(inventory_path: impl Into<String>) -> Self {
        Self {
            inventory_path: inventory_path.into(),
        }
    }

    /// Find gaps between Claude Code and RustyClawd
    pub async fn find_gaps(&self, claude_features: &[ClaudeFeature]) -> Result<Vec<FeatureGap>> {
        // Load RustyClawd inventory
        let inventory = self.load_inventory().await?;

        let mut gaps = Vec::new();

        for claude_feature in claude_features {
            // Try to find matching RustyClawd feature
            let rustyclawd_match = self.find_match(&inventory, claude_feature);

            // Determine gap type
            let gap_type = match &rustyclawd_match {
                None => GapType::Missing,
                Some(rc_feature) => match rc_feature.status {
                    FeatureStatus::Complete => continue, // No gap
                    FeatureStatus::Partial => GapType::Incomplete,
                    FeatureStatus::Missing => GapType::Missing,
                    FeatureStatus::NotApplicable => continue, // Skip
                },
            };

            gaps.push(FeatureGap {
                claude_feature: claude_feature.clone(),
                rustyclawd_status: rustyclawd_match,
                gap_type,
            });
        }

        Ok(gaps)
    }

    /// Load feature inventory from YAML file
    async fn load_inventory(&self) -> Result<FeatureInventory> {
        let path = Path::new(&self.inventory_path);

        if !path.exists() {
            // Return empty inventory if file doesn't exist yet
            return Ok(FeatureInventory {
                features: vec![],
                last_updated: None,
            });
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read inventory file")?;

        serde_yaml::from_str(&content).context("Failed to parse inventory YAML")
    }

    /// Find a matching RustyClawd feature for a Claude feature
    fn find_match(
        &self,
        inventory: &FeatureInventory,
        claude_feature: &ClaudeFeature,
    ) -> Option<RustyClawdFeature> {
        // Normalize names for comparison
        let claude_name = self.normalize_name(&claude_feature.name);

        for rc_feature in &inventory.features {
            let rc_name = self.normalize_name(&rc_feature.name);

            // Exact match
            if claude_name == rc_name {
                return Some(rc_feature.clone());
            }

            // Fuzzy match (e.g., "slash_command" vs "slashcommand")
            if self.fuzzy_match(&claude_name, &rc_name) {
                return Some(rc_feature.clone());
            }
        }

        None
    }

    /// Normalize a feature name for comparison
    fn normalize_name(&self, name: &str) -> String {
        name.to_lowercase()
            .replace("_", "")
            .replace("-", "")
            .replace(" ", "")
    }

    /// Fuzzy match between two normalized names
    fn fuzzy_match(&self, name1: &str, name2: &str) -> bool {
        // Check if one contains the other
        name1.contains(name2) || name2.contains(name1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_name() {
        let mapper = FeatureMapper::new("test.yaml");

        assert_eq!(mapper.normalize_name("Bash Tool"), "bashtool");
        assert_eq!(mapper.normalize_name("slash_command"), "slashcommand");
        assert_eq!(mapper.normalize_name("Web-Search"), "websearch");
    }

    #[test]
    fn test_fuzzy_match() {
        let mapper = FeatureMapper::new("test.yaml");

        assert!(mapper.fuzzy_match("bash", "bashtool"));
        assert!(mapper.fuzzy_match("bashtool", "bash"));
        assert!(!mapper.fuzzy_match("bash", "read"));
    }

    #[tokio::test]
    async fn test_empty_inventory() {
        let mapper = FeatureMapper::new("/nonexistent/path.yaml");
        let inventory = mapper.load_inventory().await.unwrap();

        assert_eq!(inventory.features.len(), 0);
    }

    #[tokio::test]
    async fn test_find_gaps_all_missing() {
        let mapper = FeatureMapper::new("/nonexistent/path.yaml");

        let claude_features = vec![ClaudeFeature {
            name: "NewTool".to_string(),
            category: "tools".to_string(),
            description: "A new tool".to_string(),
            since_version: None,
        }];

        let gaps = mapper.find_gaps(&claude_features).await.unwrap();

        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].gap_type, GapType::Missing);
    }
}
