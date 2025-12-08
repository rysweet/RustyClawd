//! Types for Claude Code Sync Monitor
//!
//! Clear, self-contained data structures with serde support.

use serde::{Deserialize, Serialize};

/// A feature from Claude Code
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClaudeFeature {
    /// Feature name (e.g., "Bash tool", "Read tool")
    pub name: String,

    /// Category (e.g., "tools", "capabilities", "ui")
    pub category: String,

    /// Description from Claude Code docs
    pub description: String,

    /// Version when introduced (if known)
    pub since_version: Option<String>,
}

/// A feature from RustyClawd inventory
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RustyClawdFeature {
    /// Feature name
    pub name: String,

    /// Category
    pub category: String,

    /// Implementation status
    pub status: FeatureStatus,

    /// Notes about implementation
    pub notes: Option<String>,
}

/// Implementation status of a feature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeatureStatus {
    /// Fully implemented
    Complete,

    /// Partially implemented
    Partial,

    /// Not yet implemented
    Missing,

    /// Not applicable (e.g., Claude Code-specific UI features)
    NotApplicable,
}

/// A gap between Claude Code and RustyClawd
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureGap {
    /// The Claude Code feature that's missing/incomplete
    pub claude_feature: ClaudeFeature,

    /// Current RustyClawd status (if exists)
    pub rustyclawd_status: Option<RustyClawdFeature>,

    /// Gap type
    pub gap_type: GapType,
}

/// Type of gap identified
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GapType {
    /// Feature is completely missing
    Missing,

    /// Feature is partially implemented
    Incomplete,

    /// Feature exists but may have differences
    Drift,
}

/// Feature inventory loaded from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureInventory {
    /// List of RustyClawd features
    pub features: Vec<RustyClawdFeature>,

    /// Last updated timestamp
    pub last_updated: Option<String>,
}

/// Issue ledger to track created issues
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IssueLedger {
    /// Map of feature name -> issue number
    pub issues: std::collections::HashMap<String, u64>,

    /// Last sync timestamp
    pub last_sync: Option<String>,
}
