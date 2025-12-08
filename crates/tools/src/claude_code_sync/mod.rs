//! Claude Code Sync Monitor
//!
//! Monitors Claude Code releases and creates GitHub issues for feature gaps.
//!
//! # Architecture
//!
//! Three self-contained bricks:
//! - `feature_discovery`: Fetches Claude Code changelog and documentation
//! - `feature_mapping`: Compares with RustyClawd feature inventory
//! - `issue_management`: Creates GitHub issues and maintains ledger
//!
//! # Philosophy
//! - Ruthlessly simple: Each brick has one responsibility
//! - Working code only: No stubs or placeholders
//! - Self-contained: Each module is regeneratable from specification

pub mod feature_discovery;
pub mod feature_mapping;
pub mod issue_management;
pub mod types;

pub use feature_discovery::FeatureDiscovery;
pub use feature_mapping::FeatureMapper;
pub use issue_management::{IssueCreated, IssueManager};
pub use types::*;

use anyhow::Result;

/// Main entry point for the sync monitor
pub struct SyncMonitor {
    discovery: FeatureDiscovery,
    mapper: FeatureMapper,
    issue_manager: IssueManager,
}

impl SyncMonitor {
    /// Create a new sync monitor
    pub fn new(
        inventory_path: impl Into<String>,
        ledger_path: impl Into<String>,
        github_token: impl Into<String>,
        repo: impl Into<String>,
    ) -> Self {
        Self {
            discovery: FeatureDiscovery::new(),
            mapper: FeatureMapper::new(inventory_path),
            issue_manager: IssueManager::new(ledger_path, github_token, repo),
        }
    }

    /// Run the sync monitor workflow
    pub async fn run(&mut self) -> Result<SyncReport> {
        // 1. Discover Claude Code features
        let claude_features = self.discovery.fetch_features().await?;

        // 2. Map against RustyClawd inventory
        let gaps = self.mapper.find_gaps(&claude_features).await?;

        // 3. Create issues for new gaps
        let issues_created = self.issue_manager.create_issues(&gaps).await?;

        Ok(SyncReport {
            claude_features_found: claude_features.len(),
            gaps_identified: gaps.len(),
            issues_created: issues_created.len(),
            issues: issues_created,
        })
    }
}

/// Report of sync monitor execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncReport {
    pub claude_features_found: usize,
    pub gaps_identified: usize,
    pub issues_created: usize,
    pub issues: Vec<IssueCreated>,
}
