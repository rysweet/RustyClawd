//! Memory entry data model
//!
//! Contains the `MemoryEntry` struct and its builder methods.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::{MemoryScope, MemoryType};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
