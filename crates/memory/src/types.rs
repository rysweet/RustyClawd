// Memory system types and data models
//
// Philosophy:
// - Zero-cost abstractions where possible
// - Clear ownership semantics
// - Type-safe memory operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

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

/// A single memory entry in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier for this memory
    pub id: String,

    /// Agent identifier that created this memory
    pub agent_id: String,

    /// Session identifier for session-scoped memories
    pub session_id: Option<String>,

    /// Memory type classification
    pub memory_type: MemoryType,

    /// Memory scope (local/project/user)
    pub scope: MemoryScope,

    /// Human-readable title
    pub title: String,

    /// Memory content (can be structured JSON or plain text)
    pub content: String,

    /// Importance rating (1-10, higher = more important)
    pub importance: u8,

    /// Tags for categorization and search
    pub tags: Vec<String>,

    /// Additional structured metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Optional parent memory ID for hierarchical organization
    pub parent_id: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Optional expiration time
    pub expires_at: Option<DateTime<Utc>>,
}

impl MemoryEntry {
    /// Create a new memory entry with required fields
    pub fn new(
        agent_id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        memory_type: MemoryType,
        scope: MemoryScope,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            agent_id: agent_id.into(),
            session_id: None,
            memory_type,
            scope,
            title: title.into(),
            content: content.into(),
            importance: 5, // Default middle importance
            tags: Vec::new(),
            metadata: HashMap::new(),
            parent_id: None,
            created_at: now,
            updated_at: now,
            expires_at: None,
        }
    }

    /// Builder method to set session ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Builder method to set importance
    pub fn with_importance(mut self, importance: u8) -> Self {
        self.importance = importance.min(10); // Cap at 10
        self
    }

    /// Builder method to set tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder method to add a tag
    pub fn add_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Builder method to set metadata
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Builder method to add metadata field
    pub fn add_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Builder method to set parent ID
    pub fn with_parent_id(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Builder method to set expiration
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if this memory has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| exp < Utc::now())
    }
}

/// Query filters for retrieving memories
#[derive(Debug, Clone, Default)]
pub struct MemoryQuery {
    /// Filter by agent ID
    pub agent_id: Option<String>,

    /// Filter by session ID
    pub session_id: Option<String>,

    /// Filter by memory type
    pub memory_type: Option<MemoryType>,

    /// Filter by scope
    pub scope: Option<MemoryScope>,

    /// Filter by minimum importance
    pub min_importance: Option<u8>,

    /// Filter by tags (all tags must match)
    pub tags: Vec<String>,

    /// Full-text search in title and content
    pub search: Option<String>,

    /// Filter by creation time range
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,

    /// Limit number of results
    pub limit: Option<usize>,

    /// Skip first N results (for pagination)
    pub offset: Option<usize>,
}

impl MemoryQuery {
    /// Create a new empty query
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by agent ID
    pub fn agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Filter by session ID
    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Filter by memory type
    pub fn memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = Some(memory_type);
        self
    }

    /// Filter by scope
    pub fn scope(mut self, scope: MemoryScope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Filter by minimum importance
    pub fn min_importance(mut self, min_importance: u8) -> Self {
        self.min_importance = Some(min_importance);
        self
    }

    /// Add a required tag
    pub fn add_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Full-text search
    pub fn search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }

    /// Filter by creation time
    pub fn created_after(mut self, created_after: DateTime<Utc>) -> Self {
        self.created_after = Some(created_after);
        self
    }

    /// Filter by creation time
    pub fn created_before(mut self, created_before: DateTime<Utc>) -> Self {
        self.created_before = Some(created_before);
        self
    }

    /// Limit results
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Skip results
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
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

    #[test]
    fn test_memory_entry_builder() {
        let entry = MemoryEntry::new(
            "architect",
            "Test Decision",
            "Use Rust for memory system",
            MemoryType::Decision,
            MemoryScope::Project,
        )
        .with_importance(9)
        .add_tag("architecture")
        .add_tag("rust");

        assert_eq!(entry.agent_id, "architect");
        assert_eq!(entry.importance, 9);
        assert_eq!(entry.tags.len(), 2);
    }

    #[test]
    fn test_memory_expiration() {
        let expired = MemoryEntry::new(
            "test",
            "Expired",
            "Content",
            MemoryType::Context,
            MemoryScope::Local,
        )
        .with_expiration(Utc::now() - chrono::Duration::hours(1));

        assert!(expired.is_expired());

        let valid = MemoryEntry::new(
            "test",
            "Valid",
            "Content",
            MemoryType::Context,
            MemoryScope::Local,
        )
        .with_expiration(Utc::now() + chrono::Duration::hours(1));

        assert!(!valid.is_expired());
    }

    #[test]
    fn test_memory_query_builder() {
        let query = MemoryQuery::new()
            .agent_id("architect")
            .memory_type(MemoryType::Decision)
            .min_importance(8)
            .add_tag("important")
            .limit(10);

        assert_eq!(query.agent_id.as_deref(), Some("architect"));
        assert_eq!(query.memory_type, Some(MemoryType::Decision));
        assert_eq!(query.min_importance, Some(8));
        assert_eq!(query.limit, Some(10));
    }
}
