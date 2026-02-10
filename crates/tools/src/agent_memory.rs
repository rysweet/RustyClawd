//! Agent memory system for multi-agent collaboration
//!
//! Provides shared memory across agents with three scopes:
//! - User scope: Shared across all agents for a user
//! - Project scope: Shared across agents within a project
//! - Local scope: Private to a single agent
//!
//! Philosophy:
//! - Ruthlessly simple: Just key-value storage with scopes
//! - Zero-BS: No complex indexing or query engines
//! - Modular: Self-contained with clear public API
//!
//! Performance:
//! - LRU eviction is O(log n) via BTreeMap keyed by (timestamp, key)
//! - Read-heavy workloads benefit from RwLock over Mutex

use crate::error::AgentMemoryError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::RwLock;

/// Memory scope determines sharing boundaries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum MemoryScope {
    /// Shared across all agents for a user (widest scope)
    User,
    /// Shared across agents within a project
    Project,
    /// Private to a single agent (narrowest scope)
    Local,
}

/// Memory entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Arbitrary JSON value
    pub value: serde_json::Value,
    /// When this entry was created (Unix timestamp in milliseconds)
    pub created_at: u64,
    /// When this entry was last updated (Unix timestamp in milliseconds)
    pub updated_at: u64,
    /// Which agent created this entry
    pub created_by: String,
}

impl MemoryEntry {
    /// Create a new memory entry.
    /// Timestamps are in milliseconds since Unix epoch for precise LRU ordering.
    pub fn new(value: serde_json::Value, agent_id: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        Self {
            value,
            created_at: now,
            updated_at: now,
            created_by: agent_id,
        }
    }

    /// Update the entry value
    pub fn update(&mut self, value: serde_json::Value) {
        self.value = value;
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }
}

/// Default maximum entries per scope (per agent for local, per project for project, total for user)
const DEFAULT_MAX_ENTRIES_PER_SCOPE: usize = 1000;

/// Default maximum size in bytes for a single entry value (64 KB)
const DEFAULT_MAX_ENTRY_SIZE_BYTES: usize = 64 * 1024;

/// A scoped memory partition that uses a BTreeMap ordered by (updated_at, key)
/// so the oldest entry can be evicted in O(log n) instead of O(n).
struct ScopedMemory {
    /// Primary lookup: key -> (entry, current_timestamp_in_order_index)
    entries: HashMap<String, MemoryEntry>,
    /// Order index: (updated_at, key) for O(log n) eviction of the oldest entry
    order: BTreeMap<(u64, String), ()>,
}

impl ScopedMemory {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            order: BTreeMap::new(),
        }
    }

    fn get(&self, key: &str) -> Option<&MemoryEntry> {
        self.entries.get(key)
    }

    fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    fn insert(&mut self, key: String, entry: MemoryEntry) {
        // Remove old order entry if key already exists
        if let Some(old_entry) = self.entries.get(&key) {
            self.order.remove(&(old_entry.updated_at, key.clone()));
        }
        self.order.insert((entry.updated_at, key.clone()), ());
        self.entries.insert(key, entry);
    }

    fn remove(&mut self, key: &str) -> Option<MemoryEntry> {
        if let Some(entry) = self.entries.remove(key) {
            self.order.remove(&(entry.updated_at, key.to_string()));
            Some(entry)
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    /// Evict the oldest entry (smallest updated_at) if we exceed max_entries.
    /// O(log n) because we pop from the front of a BTreeMap.
    fn evict_oldest(&mut self, max_entries: usize) -> Option<String> {
        if self.entries.len() <= max_entries {
            return None;
        }
        // Pop the first (smallest) key from the BTreeMap
        if let Some(((_, key), ())) = self.order.pop_first() {
            self.entries.remove(&key);
            Some(key)
        } else {
            None
        }
    }
}

/// Agent memory storage with scoped access.
/// Each scope has a configurable max_entries limit. When the limit is reached,
/// the oldest entry (by updated_at timestamp) is evicted (LRU eviction) in O(log n).
///
/// Uses `RwLock` for read-heavy workloads: multiple readers can proceed concurrently,
/// only writes take an exclusive lock.
pub struct AgentMemory {
    /// User-scoped memory (shared across all agents for a user)
    user_memory: Arc<RwLock<ScopedMemory>>,
    /// Project-scoped memory (shared within a project)
    project_memory: Arc<RwLock<HashMap<String, ScopedMemory>>>,
    /// Local agent memory (private to each agent)
    local_memory: Arc<RwLock<HashMap<String, ScopedMemory>>>,
    /// Maximum entries per scope partition (per-agent for local, per-project for project, total for user)
    max_entries_per_scope: usize,
    /// Maximum size in bytes for a single entry value (serialized JSON)
    max_entry_size_bytes: usize,
}

impl AgentMemory {
    /// Create a new agent memory system with default limits (1000 entries per scope, 64KB per entry)
    pub fn new() -> Self {
        Self::with_limits(DEFAULT_MAX_ENTRIES_PER_SCOPE, DEFAULT_MAX_ENTRY_SIZE_BYTES)
    }

    /// Create a new agent memory system with a custom max entries limit per scope
    /// and default entry size limit (64KB)
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self::with_limits(max_entries, DEFAULT_MAX_ENTRY_SIZE_BYTES)
    }

    /// Create a new agent memory system with custom limits
    pub fn with_limits(max_entries: usize, max_entry_size_bytes: usize) -> Self {
        Self {
            user_memory: Arc::new(RwLock::new(ScopedMemory::new())),
            project_memory: Arc::new(RwLock::new(HashMap::new())),
            local_memory: Arc::new(RwLock::new(HashMap::new())),
            max_entries_per_scope: max_entries,
            max_entry_size_bytes,
        }
    }

    /// Get the configured max entries per scope
    pub fn max_entries_per_scope(&self) -> usize {
        self.max_entries_per_scope
    }

    /// Get the configured max entry size in bytes
    pub fn max_entry_size_bytes(&self) -> usize {
        self.max_entry_size_bytes
    }

    /// Validate that a value does not exceed the maximum entry size.
    fn validate_entry_size(&self, value: &serde_json::Value) -> Result<(), AgentMemoryError> {
        let serialized_size = serde_json::to_vec(value).map(|v| v.len()).unwrap_or(0);
        if serialized_size > self.max_entry_size_bytes {
            return Err(AgentMemoryError::EntrySizeLimitExceeded {
                max_bytes: self.max_entry_size_bytes,
                actual_bytes: serialized_size,
            });
        }
        Ok(())
    }

    /// Store a value in memory
    ///
    /// # Arguments
    ///
    /// * `scope` - Memory scope (user/project/local)
    /// * `key` - Unique key for the value
    /// * `value` - JSON value to store
    /// * `agent_id` - ID of the agent storing the value
    /// * `project_id` - Optional project ID (required for project scope)
    ///
    /// # Errors
    ///
    /// Returns `AgentMemoryError::ProjectIdRequired` if scope is Project and project_id is None.
    /// Returns `AgentMemoryError::EntrySizeLimitExceeded` if the serialized value exceeds the size limit.
    pub async fn set(
        &self,
        scope: MemoryScope,
        key: String,
        value: serde_json::Value,
        agent_id: String,
        project_id: Option<String>,
    ) -> Result<(), AgentMemoryError> {
        self.validate_entry_size(&value)?;
        let entry = MemoryEntry::new(value, agent_id.clone());

        let max = self.max_entries_per_scope;
        match scope {
            MemoryScope::User => {
                let mut memory = self.user_memory.write().await;
                memory.insert(key, entry);
                memory.evict_oldest(max);
                Ok(())
            }
            MemoryScope::Project => {
                let pid = project_id.ok_or(AgentMemoryError::ProjectIdRequired)?;
                let mut memory = self.project_memory.write().await;
                let project_map = memory.entry(pid).or_insert_with(ScopedMemory::new);
                project_map.insert(key, entry);
                project_map.evict_oldest(max);
                Ok(())
            }
            MemoryScope::Local => {
                let mut memory = self.local_memory.write().await;
                let agent_map = memory.entry(agent_id).or_insert_with(ScopedMemory::new);
                agent_map.insert(key, entry);
                agent_map.evict_oldest(max);
                Ok(())
            }
        }
    }

    /// Get a value from memory
    ///
    /// # Arguments
    ///
    /// * `scope` - Memory scope to query
    /// * `key` - Key to retrieve
    /// * `agent_id` - ID of the agent requesting the value
    /// * `project_id` - Optional project ID (required for project scope)
    pub async fn get(
        &self,
        scope: MemoryScope,
        key: &str,
        agent_id: &str,
        project_id: Option<&str>,
    ) -> Result<Option<MemoryEntry>, AgentMemoryError> {
        match scope {
            MemoryScope::User => {
                let memory = self.user_memory.read().await;
                Ok(memory.get(key).cloned())
            }
            MemoryScope::Project => {
                let pid = project_id.ok_or(AgentMemoryError::ProjectIdRequired)?;
                let memory = self.project_memory.read().await;
                Ok(memory.get(pid).and_then(|m| m.get(key).cloned()))
            }
            MemoryScope::Local => {
                let memory = self.local_memory.read().await;
                Ok(memory.get(agent_id).and_then(|m| m.get(key).cloned()))
            }
        }
    }

    /// Delete a value from memory
    pub async fn delete(
        &self,
        scope: MemoryScope,
        key: &str,
        agent_id: &str,
        project_id: Option<&str>,
    ) -> Result<bool, AgentMemoryError> {
        match scope {
            MemoryScope::User => {
                let mut memory = self.user_memory.write().await;
                Ok(memory.remove(key).is_some())
            }
            MemoryScope::Project => {
                let pid = project_id.ok_or(AgentMemoryError::ProjectIdRequired)?;
                let mut memory = self.project_memory.write().await;
                Ok(memory.get_mut(pid).and_then(|m| m.remove(key)).is_some())
            }
            MemoryScope::Local => {
                let mut memory = self.local_memory.write().await;
                Ok(memory
                    .get_mut(agent_id)
                    .and_then(|m| m.remove(key))
                    .is_some())
            }
        }
    }

    /// List all keys in a memory scope
    pub async fn list_keys(
        &self,
        scope: MemoryScope,
        agent_id: &str,
        project_id: Option<&str>,
    ) -> Result<Vec<String>, AgentMemoryError> {
        match scope {
            MemoryScope::User => {
                let memory = self.user_memory.read().await;
                Ok(memory.keys().cloned().collect())
            }
            MemoryScope::Project => {
                let pid = project_id.ok_or(AgentMemoryError::ProjectIdRequired)?;
                let memory = self.project_memory.read().await;
                Ok(memory
                    .get(pid)
                    .map(|m| m.keys().cloned().collect())
                    .unwrap_or_default())
            }
            MemoryScope::Local => {
                let memory = self.local_memory.read().await;
                Ok(memory
                    .get(agent_id)
                    .map(|m| m.keys().cloned().collect())
                    .unwrap_or_default())
            }
        }
    }

    /// Clear all memory for a specific scope
    pub async fn clear(
        &self,
        scope: MemoryScope,
        agent_id: &str,
        project_id: Option<&str>,
    ) -> Result<(), AgentMemoryError> {
        match scope {
            MemoryScope::User => {
                let mut memory = self.user_memory.write().await;
                memory.clear();
                Ok(())
            }
            MemoryScope::Project => {
                let pid = project_id.ok_or(AgentMemoryError::ProjectIdRequired)?;
                let mut memory = self.project_memory.write().await;
                if let Some(m) = memory.get_mut(pid) {
                    m.clear();
                }
                Ok(())
            }
            MemoryScope::Local => {
                let mut memory = self.local_memory.write().await;
                if let Some(m) = memory.get_mut(agent_id) {
                    m.clear();
                }
                Ok(())
            }
        }
    }
}

/// Global agent memory instance (singleton pattern)
static GLOBAL_AGENT_MEMORY: OnceLock<Arc<AgentMemory>> = OnceLock::new();

/// Get or create the global agent memory instance
pub fn global_agent_memory() -> Arc<AgentMemory> {
    Arc::clone(GLOBAL_AGENT_MEMORY.get_or_init(|| Arc::new(AgentMemory::new())))
}

impl Clone for AgentMemory {
    fn clone(&self) -> Self {
        Self {
            user_memory: Arc::clone(&self.user_memory),
            project_memory: Arc::clone(&self.project_memory),
            local_memory: Arc::clone(&self.local_memory),
            max_entries_per_scope: self.max_entries_per_scope,
            max_entry_size_bytes: self.max_entry_size_bytes,
        }
    }
}

impl Default for AgentMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_scope_memory() {
        let memory = AgentMemory::new();
        let agent1 = "agent1".to_string();
        let agent2 = "agent2".to_string();

        // Agent1 stores a value in user scope
        memory
            .set(
                MemoryScope::User,
                "shared_key".to_string(),
                serde_json::json!({"message": "Hello from agent1"}),
                agent1.clone(),
                None,
            )
            .await
            .unwrap();

        // Agent2 can read it
        let entry = memory
            .get(MemoryScope::User, "shared_key", &agent2, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.value["message"], "Hello from agent1");
        assert_eq!(entry.created_by, "agent1");
    }

    #[tokio::test]
    async fn test_project_scope_memory() {
        let memory = AgentMemory::new();
        let agent1 = "agent1".to_string();
        let agent2 = "agent2".to_string();
        let project = "project1".to_string();

        // Agent1 stores in project scope
        memory
            .set(
                MemoryScope::Project,
                "project_key".to_string(),
                serde_json::json!({"status": "in_progress"}),
                agent1.clone(),
                Some(project.clone()),
            )
            .await
            .unwrap();

        // Agent2 in same project can read it
        let entry = memory
            .get(MemoryScope::Project, "project_key", &agent2, Some(&project))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.value["status"], "in_progress");
    }

    #[tokio::test]
    async fn test_local_scope_memory() {
        let memory = AgentMemory::new();
        let agent1 = "agent1".to_string();
        let agent2 = "agent2".to_string();

        // Agent1 stores in local scope
        memory
            .set(
                MemoryScope::Local,
                "private_key".to_string(),
                serde_json::json!({"secret": "value"}),
                agent1.clone(),
                None,
            )
            .await
            .unwrap();

        // Agent1 can read it
        let entry = memory
            .get(MemoryScope::Local, "private_key", &agent1, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.value["secret"], "value");

        // Agent2 cannot read it
        let not_found = memory
            .get(MemoryScope::Local, "private_key", &agent2, None)
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_delete_memory() {
        let memory = AgentMemory::new();
        let agent = "agent1".to_string();

        memory
            .set(
                MemoryScope::User,
                "temp_key".to_string(),
                serde_json::json!({"temp": true}),
                agent.clone(),
                None,
            )
            .await
            .unwrap();

        // Verify it exists
        assert!(memory
            .get(MemoryScope::User, "temp_key", &agent, None)
            .await
            .unwrap()
            .is_some());

        // Delete it
        let deleted = memory
            .delete(MemoryScope::User, "temp_key", &agent, None)
            .await
            .unwrap();
        assert!(deleted);

        // Verify it's gone
        assert!(memory
            .get(MemoryScope::User, "temp_key", &agent, None)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_list_keys() {
        let memory = AgentMemory::new();
        let agent = "agent1".to_string();

        memory
            .set(
                MemoryScope::User,
                "key1".to_string(),
                serde_json::json!(1),
                agent.clone(),
                None,
            )
            .await
            .unwrap();
        memory
            .set(
                MemoryScope::User,
                "key2".to_string(),
                serde_json::json!(2),
                agent.clone(),
                None,
            )
            .await
            .unwrap();

        let keys = memory
            .list_keys(MemoryScope::User, &agent, None)
            .await
            .unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }

    #[tokio::test]
    async fn test_clear_memory() {
        let memory = AgentMemory::new();
        let agent = "agent1".to_string();

        memory
            .set(
                MemoryScope::Local,
                "key1".to_string(),
                serde_json::json!(1),
                agent.clone(),
                None,
            )
            .await
            .unwrap();

        // Verify it exists
        assert_eq!(
            memory
                .list_keys(MemoryScope::Local, &agent, None)
                .await
                .unwrap()
                .len(),
            1
        );

        // Clear local memory
        memory
            .clear(MemoryScope::Local, &agent, None)
            .await
            .unwrap();

        // Verify it's empty
        assert_eq!(
            memory
                .list_keys(MemoryScope::Local, &agent, None)
                .await
                .unwrap()
                .len(),
            0
        );
    }

    #[tokio::test]
    async fn test_memory_entry_update() {
        let mut entry = MemoryEntry::new(serde_json::json!({"count": 1}), "agent1".to_string());

        let original_updated_at = entry.updated_at;

        // Sleep to ensure different millisecond timestamp
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        entry.update(serde_json::json!({"count": 2}));

        assert_eq!(entry.value["count"], 2);
        assert!(entry.updated_at > original_updated_at);
        assert_eq!(entry.created_by, "agent1");
    }

    #[tokio::test]
    async fn test_project_scope_requires_project_id() {
        let memory = AgentMemory::new();
        let agent = "agent1".to_string();

        // Set without project_id should fail
        let result = memory
            .set(
                MemoryScope::Project,
                "key".to_string(),
                serde_json::json!({}),
                agent.clone(),
                None,
            )
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), AgentMemoryError::ProjectIdRequired);

        // Get without project_id should fail
        let result = memory.get(MemoryScope::Project, "key", &agent, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_global_memory_singleton() {
        let memory1 = global_agent_memory();
        let memory2 = global_agent_memory();

        // Should be the same instance
        assert!(Arc::ptr_eq(&memory1, &memory2));
    }

    #[tokio::test]
    async fn test_max_entries_per_scope_default() {
        let memory = AgentMemory::new();
        assert_eq!(memory.max_entries_per_scope(), 1000);
    }

    #[tokio::test]
    async fn test_max_entries_per_scope_custom() {
        let memory = AgentMemory::with_max_entries(5);
        assert_eq!(memory.max_entries_per_scope(), 5);
    }

    #[tokio::test]
    async fn test_user_scope_eviction() {
        // Use a small limit to test eviction
        let memory = AgentMemory::with_max_entries(3);
        let agent = "agent1".to_string();

        // Insert 3 entries (at limit)
        for i in 0..3 {
            memory
                .set(
                    MemoryScope::User,
                    format!("key{}", i),
                    serde_json::json!(i),
                    agent.clone(),
                    None,
                )
                .await
                .unwrap();
            // Small delay to ensure different timestamps for LRU ordering
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let keys = memory
            .list_keys(MemoryScope::User, &agent, None)
            .await
            .unwrap();
        assert_eq!(keys.len(), 3);

        // Insert a 4th entry - should evict the oldest (key0)
        memory
            .set(
                MemoryScope::User,
                "key3".to_string(),
                serde_json::json!(3),
                agent.clone(),
                None,
            )
            .await
            .unwrap();

        let keys = memory
            .list_keys(MemoryScope::User, &agent, None)
            .await
            .unwrap();
        assert_eq!(keys.len(), 3);
        // key0 should have been evicted (oldest updated_at)
        assert!(!keys.contains(&"key0".to_string()));
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
        assert!(keys.contains(&"key3".to_string()));
    }

    #[tokio::test]
    async fn test_local_scope_eviction() {
        let memory = AgentMemory::with_max_entries(2);
        let agent = "agent1".to_string();

        memory
            .set(
                MemoryScope::Local,
                "a".to_string(),
                serde_json::json!("first"),
                agent.clone(),
                None,
            )
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        memory
            .set(
                MemoryScope::Local,
                "b".to_string(),
                serde_json::json!("second"),
                agent.clone(),
                None,
            )
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // At limit (2). Insert a 3rd - should evict "a" (oldest)
        memory
            .set(
                MemoryScope::Local,
                "c".to_string(),
                serde_json::json!("third"),
                agent.clone(),
                None,
            )
            .await
            .unwrap();

        let keys = memory
            .list_keys(MemoryScope::Local, &agent, None)
            .await
            .unwrap();
        assert_eq!(keys.len(), 2);
        assert!(!keys.contains(&"a".to_string()));
        assert!(keys.contains(&"b".to_string()));
        assert!(keys.contains(&"c".to_string()));
    }

    #[tokio::test]
    async fn test_project_scope_eviction() {
        let memory = AgentMemory::with_max_entries(2);
        let agent = "agent1".to_string();
        let project = "proj1".to_string();

        memory
            .set(
                MemoryScope::Project,
                "x".to_string(),
                serde_json::json!(1),
                agent.clone(),
                Some(project.clone()),
            )
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        memory
            .set(
                MemoryScope::Project,
                "y".to_string(),
                serde_json::json!(2),
                agent.clone(),
                Some(project.clone()),
            )
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Insert 3rd entry - evicts "x"
        memory
            .set(
                MemoryScope::Project,
                "z".to_string(),
                serde_json::json!(3),
                agent.clone(),
                Some(project.clone()),
            )
            .await
            .unwrap();

        let keys = memory
            .list_keys(MemoryScope::Project, &agent, Some(&project))
            .await
            .unwrap();
        assert_eq!(keys.len(), 2);
        assert!(!keys.contains(&"x".to_string()));
        assert!(keys.contains(&"y".to_string()));
        assert!(keys.contains(&"z".to_string()));
    }

    #[tokio::test]
    async fn test_cross_project_memory_isolation() {
        let memory = AgentMemory::new();
        let agent = "agent1".to_string();

        // Store data in project A
        memory
            .set(
                MemoryScope::Project,
                "secret".to_string(),
                serde_json::json!({"data": "project_a_only"}),
                agent.clone(),
                Some("project_a".to_string()),
            )
            .await
            .unwrap();

        // Project B should not see project A's data
        let result = memory
            .get(MemoryScope::Project, "secret", &agent, Some("project_b"))
            .await
            .unwrap();
        assert!(result.is_none());

        // Project A can see its own data
        let result = memory
            .get(MemoryScope::Project, "secret", &agent, Some("project_a"))
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().value["data"], "project_a_only");
    }

    #[tokio::test]
    async fn test_entry_size_limit_enforced() {
        // Use a small size limit for testing (256 bytes)
        let memory = AgentMemory::with_limits(1000, 256);
        let agent = "agent1".to_string();

        // A small value should succeed
        let result = memory
            .set(
                MemoryScope::User,
                "small".to_string(),
                serde_json::json!({"msg": "hello"}),
                agent.clone(),
                None,
            )
            .await;
        assert!(result.is_ok());

        // A large value should fail
        let large_string = "x".repeat(512);
        let result = memory
            .set(
                MemoryScope::User,
                "large".to_string(),
                serde_json::json!({"data": large_string}),
                agent.clone(),
                None,
            )
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AgentMemoryError::EntrySizeLimitExceeded {
                max_bytes,
                actual_bytes,
            } => {
                assert_eq!(max_bytes, 256);
                assert!(actual_bytes > 256);
            }
            other => panic!("Expected EntrySizeLimitExceeded, got {:?}", other),
        }

        // The large entry should not have been stored
        let entry = memory
            .get(MemoryScope::User, "large", &agent, None)
            .await
            .unwrap();
        assert!(entry.is_none());
    }

    #[tokio::test]
    async fn test_default_entry_size_limit() {
        let memory = AgentMemory::new();
        assert_eq!(memory.max_entry_size_bytes(), 64 * 1024);
    }

    #[tokio::test]
    async fn test_concurrent_multi_agent_access() {
        // Spawn multiple tasks that read and write simultaneously to verify
        // RwLock correctness under contention.
        let memory = Arc::new(AgentMemory::new());
        let num_agents = 5;
        let ops_per_agent = 10;

        let mut handles = Vec::new();

        for agent_idx in 0..num_agents {
            let mem = Arc::clone(&memory);
            let handle = tokio::spawn(async move {
                let agent_id = format!("agent_{}", agent_idx);
                for op in 0..ops_per_agent {
                    let key = format!("key_{}_{}", agent_idx, op);
                    // Write
                    mem.set(
                        MemoryScope::User,
                        key.clone(),
                        serde_json::json!({"agent": agent_idx, "op": op}),
                        agent_id.clone(),
                        None,
                    )
                    .await
                    .unwrap();

                    // Read back (may or may not find it if evicted, but should not panic)
                    let _ = mem
                        .get(MemoryScope::User, &key, &agent_id, None)
                        .await
                        .unwrap();

                    // Also write to local scope (no contention across agents)
                    mem.set(
                        MemoryScope::Local,
                        key.clone(),
                        serde_json::json!({"private": true}),
                        agent_id.clone(),
                        None,
                    )
                    .await
                    .unwrap();

                    // Read from local scope
                    let entry = mem
                        .get(MemoryScope::Local, &key, &agent_id, None)
                        .await
                        .unwrap();
                    assert!(entry.is_some(), "Local scope entry should always be found");
                }
            });
            handles.push(handle);
        }

        // All tasks should complete without panics or errors
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify the memory is in a consistent state
        let user_keys = memory
            .list_keys(MemoryScope::User, "any", None)
            .await
            .unwrap();
        // We inserted num_agents * ops_per_agent = 50 keys, but max is 1000, so all should be present
        assert_eq!(user_keys.len(), num_agents * ops_per_agent);
    }

    #[tokio::test]
    async fn test_update_existing_key_refreshes_order() {
        // Verify that updating an existing key moves it to the end of the eviction order
        let memory = AgentMemory::with_max_entries(3);
        let agent = "agent1".to_string();

        // Insert key0, key1, key2
        for i in 0..3 {
            memory
                .set(
                    MemoryScope::User,
                    format!("key{}", i),
                    serde_json::json!(i),
                    agent.clone(),
                    None,
                )
                .await
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Update key0 so it becomes the newest
        memory
            .set(
                MemoryScope::User,
                "key0".to_string(),
                serde_json::json!("updated"),
                agent.clone(),
                None,
            )
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Insert key3 - should evict key1 (now the oldest), NOT key0
        memory
            .set(
                MemoryScope::User,
                "key3".to_string(),
                serde_json::json!(3),
                agent.clone(),
                None,
            )
            .await
            .unwrap();

        let keys = memory
            .list_keys(MemoryScope::User, &agent, None)
            .await
            .unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"key0".to_string()), "key0 was refreshed, should survive");
        assert!(!keys.contains(&"key1".to_string()), "key1 should be evicted as oldest");
        assert!(keys.contains(&"key2".to_string()));
        assert!(keys.contains(&"key3".to_string()));
    }
}
