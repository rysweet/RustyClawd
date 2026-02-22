//! Memory query filters
//!
//! Contains the `MemoryQuery` struct and its builder methods for filtering memories.

use chrono::{DateTime, Utc};

use super::types::{MemoryScope, MemoryType};

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
