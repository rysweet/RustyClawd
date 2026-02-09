// Memory Manager - High-level interface for memory operations
//
// Philosophy:
// - Simple, intuitive API
// - Automatic session management
// - Performance-optimized (<50ms operations)

use crate::database::{Database, MemoryStats};
use crate::types::{MemoryEntry, MemoryQuery, MemoryScope, MemoryType};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// High-level memory manager interface
///
/// Note: `Default` is intentionally not implemented because construction
/// touches the filesystem (creating directories and the SQLite database),
/// which can fail. Use `MemoryManager::new()` and handle the Result.
#[derive(Clone)]
pub struct MemoryManager {
    db: Arc<Database>,
    session_id: Option<String>,
}

impl MemoryManager {
    /// Create a new memory manager with the default database location
    pub fn new() -> Result<Self> {
        let db_path = Self::default_db_path();
        Self::with_db_path(db_path)
    }

    /// Create a memory manager with a specific database path
    pub fn with_db_path(path: impl Into<PathBuf>) -> Result<Self> {
        let db = Database::open(path.into())?;
        Ok(Self {
            db: Arc::new(db),
            session_id: None,
        })
    }

    /// Create a memory manager with a specific session ID
    pub fn with_session(session_id: impl Into<String>) -> Result<Self> {
        let mut manager = Self::new()?;
        manager.session_id = Some(session_id.into());
        Ok(manager)
    }

    /// Get the default database path (~/.rustyclawd/memory.db)
    fn default_db_path() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());

        PathBuf::from(home)
            .join(".rustyclawd")
            .join("memory.db")
    }

    /// Store a new memory
    pub fn store(
        &self,
        agent_id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        memory_type: MemoryType,
        scope: MemoryScope,
    ) -> Result<String> {
        let mut entry = MemoryEntry::new(agent_id, title, content, memory_type, scope);

        // Automatically set session ID if available
        if let Some(session_id) = &self.session_id {
            entry = entry.with_session_id(session_id.clone());
        }

        self.db.store(&entry)
    }

    /// Store a pre-built memory entry
    pub fn store_entry(&self, mut entry: MemoryEntry) -> Result<String> {
        // Automatically set session ID if available and not already set
        if entry.session_id.is_none() {
            if let Some(session_id) = &self.session_id {
                entry = entry.with_session_id(session_id.clone());
            }
        }

        self.db.store(&entry)
    }

    /// Update an existing memory entry
    ///
    /// Updates all mutable fields and sets updated_at to the current time.
    /// Returns true if the entry was found and updated, false if the ID
    /// does not exist.
    pub fn update(&self, entry: &MemoryEntry) -> Result<bool> {
        self.db.update(entry)
    }

    /// Retrieve memories matching the query
    pub fn retrieve(&self, query: &MemoryQuery) -> Result<Vec<MemoryEntry>> {
        self.db.query(query)
    }

    /// Retrieve memories for the current session
    pub fn retrieve_session_memories(
        &self,
        agent_id: Option<&str>,
        memory_type: Option<MemoryType>,
    ) -> Result<Vec<MemoryEntry>> {
        let mut query = MemoryQuery::new();

        if let Some(session_id) = &self.session_id {
            query = query.session_id(session_id);
        }

        if let Some(agent_id) = agent_id {
            query = query.agent_id(agent_id);
        }

        if let Some(memory_type) = memory_type {
            query = query.memory_type(memory_type);
        }

        self.db.query(&query)
    }

    /// Get a specific memory by ID
    pub fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        self.db.get(id)
    }

    /// Delete a memory
    pub fn delete(&self, id: &str) -> Result<bool> {
        self.db.delete(id)
    }

    /// Search memories by text
    pub fn search(&self, search_text: &str, limit: Option<usize>) -> Result<Vec<MemoryEntry>> {
        let mut query = MemoryQuery::new().search(search_text);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        self.db.query(&query)
    }

    /// Get recent memories
    pub fn recent(&self, limit: usize) -> Result<Vec<MemoryEntry>> {
        let query = MemoryQuery::new().limit(limit);
        self.db.query(&query)
    }

    /// Get important memories (importance >= 8)
    pub fn important(&self, limit: Option<usize>) -> Result<Vec<MemoryEntry>> {
        let mut query = MemoryQuery::new().min_importance(8);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        self.db.query(&query)
    }

    /// Get memories by scope (respecting hierarchy: Local > Project > User)
    pub fn by_scope(&self, scope: MemoryScope) -> Result<Vec<MemoryEntry>> {
        let query = MemoryQuery::new().scope(scope);
        self.db.query(&query)
    }

    /// Clean up expired memories
    pub fn cleanup_expired(&self) -> Result<usize> {
        self.db.cleanup_expired()
    }

    /// Get memory statistics
    pub fn stats(&self) -> Result<MemoryStats> {
        self.db.stats()
    }

    /// Get the current session ID
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Set the session ID
    pub fn set_session_id(&mut self, session_id: impl Into<String>) {
        self.session_id = Some(session_id.into());
    }

    /// Clear the session ID
    pub fn clear_session_id(&mut self) {
        self.session_id = None;
    }
}

/// Builder for creating memories with fluent API
pub struct MemoryBuilder {
    entry: MemoryEntry,
}

impl MemoryBuilder {
    /// Create a new memory builder
    pub fn new(
        agent_id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        memory_type: MemoryType,
        scope: MemoryScope,
    ) -> Self {
        Self {
            entry: MemoryEntry::new(agent_id, title, content, memory_type, scope),
        }
    }

    /// Set session ID
    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.entry = self.entry.with_session_id(session_id);
        self
    }

    /// Set importance
    pub fn importance(mut self, importance: u8) -> Self {
        self.entry = self.entry.with_importance(importance);
        self
    }

    /// Add a tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.entry = self.entry.add_tag(tag);
        self
    }

    /// Set tags
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.entry = self.entry.with_tags(tags);
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.entry = self.entry.add_metadata(key, value);
        self
    }

    /// Set parent ID
    pub fn parent_id(mut self, parent_id: impl Into<String>) -> Self {
        self.entry = self.entry.with_parent_id(parent_id);
        self
    }

    /// Set expiration
    pub fn expires_at(mut self, expires_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.entry = self.entry.with_expiration(expires_at);
        self
    }

    /// Build the memory entry
    pub fn build(self) -> MemoryEntry {
        self.entry
    }

    /// Build and store the memory
    pub fn store(self, manager: &MemoryManager) -> Result<String> {
        manager.store_entry(self.entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (MemoryManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let manager = MemoryManager::with_db_path(&db_path).unwrap();
        (manager, temp_dir)
    }

    #[test]
    fn test_manager_creation() {
        let (_manager, _temp_dir) = create_test_manager();
    }

    #[test]
    fn test_store_and_retrieve() {
        let (manager, _temp_dir) = create_test_manager();

        let id = manager
            .store(
                "test_agent",
                "Test Memory",
                "Test content",
                MemoryType::Decision,
                MemoryScope::Project,
            )
            .unwrap();

        let entry = manager.get(&id).unwrap().unwrap();
        assert_eq!(entry.title, "Test Memory");
    }

    #[test]
    fn test_update_via_manager() {
        let (manager, _temp_dir) = create_test_manager();

        let entry = MemoryEntry::new(
            "test_agent",
            "Original",
            "Original content",
            MemoryType::Decision,
            MemoryScope::Project,
        );
        let id = manager.store_entry(entry.clone()).unwrap();

        // Retrieve, modify, update
        let mut retrieved = manager.get(&id).unwrap().unwrap();
        retrieved.title = "Modified".to_string();
        retrieved.content = "Modified content".to_string();

        let updated = manager.update(&retrieved).unwrap();
        assert!(updated);

        let final_entry = manager.get(&id).unwrap().unwrap();
        assert_eq!(final_entry.title, "Modified");
        assert_eq!(final_entry.content, "Modified content");
    }

    #[test]
    fn test_session_management() {
        let (mut manager, _temp_dir) = create_test_manager();

        manager.set_session_id("session123");

        let id = manager
            .store(
                "test_agent",
                "Session Memory",
                "Content",
                MemoryType::Context,
                MemoryScope::Local,
            )
            .unwrap();

        let entry = manager.get(&id).unwrap().unwrap();
        assert_eq!(entry.session_id.as_deref(), Some("session123"));
    }

    #[test]
    fn test_search() {
        let (manager, _temp_dir) = create_test_manager();

        manager
            .store(
                "test_agent",
                "Database Design",
                "Using PostgreSQL",
                MemoryType::Decision,
                MemoryScope::Project,
            )
            .unwrap();

        let results = manager.search("database", Some(10)).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("Database"));
    }

    #[test]
    fn test_builder_pattern() {
        let (manager, _temp_dir) = create_test_manager();

        let id = MemoryBuilder::new(
            "architect",
            "API Design",
            "REST API with JSON",
            MemoryType::Decision,
            MemoryScope::Project,
        )
        .importance(9)
        .tag("architecture")
        .tag("api")
        .store(&manager)
        .unwrap();

        let entry = manager.get(&id).unwrap().unwrap();
        assert_eq!(entry.importance, 9);
        assert_eq!(entry.tags.len(), 2);
    }

    #[test]
    fn test_scope_filtering() {
        let (manager, _temp_dir) = create_test_manager();

        manager
            .store(
                "test_agent",
                "Local Memory",
                "Content",
                MemoryType::Context,
                MemoryScope::Local,
            )
            .unwrap();

        manager
            .store(
                "test_agent",
                "Project Memory",
                "Content",
                MemoryType::Context,
                MemoryScope::Project,
            )
            .unwrap();

        let local_memories = manager.by_scope(MemoryScope::Local).unwrap();
        assert_eq!(local_memories.len(), 1);

        let project_memories = manager.by_scope(MemoryScope::Project).unwrap();
        assert_eq!(project_memories.len(), 1);
    }

    #[test]
    fn test_important_memories() {
        let (manager, _temp_dir) = create_test_manager();

        for i in 5..=10 {
            let entry = MemoryEntry::new(
                "test_agent",
                format!("Memory {}", i),
                "Content",
                MemoryType::Decision,
                MemoryScope::Project,
            )
            .with_importance(i);
            manager.store_entry(entry).unwrap();
        }

        let important = manager.important(None).unwrap();
        assert_eq!(important.len(), 3); // importance 8, 9, 10
    }
}
