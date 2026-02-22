// Memory system core types
//
// This module contains the foundational enum types shared across the memory system:
// `MemoryType`, `MemoryScope`, and `MemoryParseError`.
//
// Struct types live in their own modules:
// - `entry.rs` for `MemoryEntry`
// - `query.rs` for `MemoryQuery`

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

// Re-export structs so existing `use crate::types::*` paths continue to work
pub use super::entry::MemoryEntry;
pub use super::query::MemoryQuery;

/// Errors that can occur when parsing memory types
#[derive(Debug, Error)]
pub enum MemoryParseError {
    #[error("Invalid memory type: {0}")]
    InvalidType(String),
    #[error("Invalid memory scope: {0}")]
    InvalidScope(String),
}

/// Memory type classification for organizational purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Conversation history and chat context
    Conversation,
    /// Architectural and design decisions
    Decision,
    /// Recognized code patterns and solutions
    Pattern,
    /// Session context and state information
    Context,
    /// Accumulated knowledge and learnings
    Learning,
    /// Generated artifacts (code, docs, etc.)
    Artifact,
}

impl MemoryType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Conversation => "conversation",
            Self::Decision => "decision",
            Self::Pattern => "pattern",
            Self::Context => "context",
            Self::Learning => "learning",
            Self::Artifact => "artifact",
        }
    }
}

impl fmt::Display for MemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MemoryType {
    type Err = MemoryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "conversation" => Ok(Self::Conversation),
            "decision" => Ok(Self::Decision),
            "pattern" => Ok(Self::Pattern),
            "context" => Ok(Self::Context),
            "learning" => Ok(Self::Learning),
            "artifact" => Ok(Self::Artifact),
            _ => Err(MemoryParseError::InvalidType(s.to_string())),
        }
    }
}

/// Memory scope hierarchy: Local > Project > User
///
/// Enum variants are ordered so that derived Ord matches documented priority:
/// User (lowest priority) < Project < Local (highest priority).
/// This means `MemoryScope::Local > MemoryScope::Project > MemoryScope::User`
/// which matches the documented "Local > Project > User" hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    /// User-level memory (shared across projects, lowest priority)
    User,
    /// Project-wide memory (shared across sessions)
    Project,
    /// Session-local memory (highest priority)
    Local,
}

impl MemoryScope {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Project => "project",
            Self::User => "user",
        }
    }
}

impl fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MemoryScope {
    type Err = MemoryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" => Ok(Self::Local),
            "project" => Ok(Self::Project),
            "user" => Ok(Self::User),
            _ => Err(MemoryParseError::InvalidScope(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type_conversions() {
        assert_eq!(
            "decision".parse::<MemoryType>().ok(),
            Some(MemoryType::Decision)
        );
        assert_eq!(MemoryType::Decision.as_str(), "decision");
        assert!("invalid".parse::<MemoryType>().is_err());
    }

    #[test]
    fn test_memory_type_display() {
        assert_eq!(format!("{}", MemoryType::Decision), "decision");
        assert_eq!(format!("{}", MemoryType::Conversation), "conversation");
    }

    #[test]
    fn test_memory_scope_display() {
        assert_eq!(format!("{}", MemoryScope::Local), "local");
        assert_eq!(format!("{}", MemoryScope::Project), "project");
        assert_eq!(format!("{}", MemoryScope::User), "user");
    }

    #[test]
    fn test_memory_scope_ordering_matches_documented_priority() {
        // Documented: Local > Project > User (Local is highest priority)
        assert!(MemoryScope::Local > MemoryScope::Project);
        assert!(MemoryScope::Project > MemoryScope::User);
        assert!(MemoryScope::Local > MemoryScope::User);
    }

    #[test]
    fn test_memory_parse_error_is_structured() {
        let err = "invalid".parse::<MemoryType>().unwrap_err();
        assert!(matches!(err, MemoryParseError::InvalidType(_)));
        // Verify Display impl from thiserror
        assert!(err.to_string().contains("Invalid memory type: invalid"));

        let err = "invalid".parse::<MemoryScope>().unwrap_err();
        assert!(matches!(err, MemoryParseError::InvalidScope(_)));
    }
}
