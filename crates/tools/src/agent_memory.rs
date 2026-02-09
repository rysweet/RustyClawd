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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::Mutex;

/// Memory scope determines sharing boundaries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    /// When this entry was created (Unix timestamp)
    pub created_at: u64,
    /// When this entry was last updated (Unix timestamp)
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

/// Agent memory storage with scoped access.
/// Each scope has a configurable max_entries limit. When the limit is reached,
/// the oldest entry (by updated_at timestamp) is evicted (LRU eviction).
pub struct AgentMemory {
    /// User-scoped memory (shared across all agents for a user)
    user_memory: Arc<Mutex<HashMap<String, MemoryEntry>>>,
    /// Project-scoped memory (shared within a project)
    project_memory: Arc<Mutex<HashMap<String, HashMap<String, MemoryEntry>>>>,
    /// Local agent memory (private to each agent)
    local_memory: Arc<Mutex<HashMap<String, HashMap<String, MemoryEntry>>>>,
    /// Maximum entries per scope partition (per-agent for local, per-project for project, total for user)
    max_entries_per_scope: usize,
}

impl AgentMemory {
    /// Create a new agent memory system with default max entries (1000 per scope)
    pub fn new() -> Self {
        Self::with_max_entries(DEFAULT_MAX_ENTRIES_PER_SCOPE)
    }

    /// Create a new agent memory system with a custom max entries limit per scope
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            user_memory: Arc::new(Mutex::new(HashMap::new())),
            project_memory: Arc::new(Mutex::new(HashMap::new())),
            local_memory: Arc::new(Mutex::new(HashMap::new())),
            max_entries_per_scope: max_entries,
        }
    }

    /// Get the configured max entries per scope
    pub fn max_entries_per_scope(&self) -> usize {
        self.max_entries_per_scope
    }

    /// Evict the oldest entry (by updated_at) from a HashMap if it exceeds max_entries.
    /// Returns the evicted key, if any.
    fn evict_oldest(map: &mut HashMap<String, MemoryEntry>, max_entries: usize) -> Option<String> {
        if map.len() <= max_entries {
            return None;
        }
        // Find the key with the smallest updated_at (LRU)
        let oldest_key = map
            .iter()
            .min_by_key(|(_, entry)| entry.updated_at)
            .map(|(k, _)| k.clone());
        if let Some(ref key) = oldest_key {
            map.remove(key);
        }
        oldest_key
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
    pub async fn set(
        &self,
        scope: MemoryScope,
        key: String,
        value: serde_json::Value,
        agent_id: String,
        project_id: Option<String>,
    ) -> Result<(), String> {
        let entry = MemoryEntry::new(value, agent_id.clone());

        let max = self.max_entries_per_scope;
        match scope {
            MemoryScope::User => {
                let mut memory = self.user_memory.lock().await;
                memory.insert(key, entry);
                Self::evict_oldest(&mut memory, max);
                Ok(())
            }
            MemoryScope::Project => {
                let pid = project_id.ok_or("Project ID required for project scope")?;
                let mut memory = self.project_memory.lock().await;
                let project_map = memory.entry(pid).or_insert_with(HashMap::new);
                project_map.insert(key, entry);
                Self::evict_oldest(project_map, max);
                Ok(())
            }
            MemoryScope::Local => {
                let mut memory = self.local_memory.lock().await;
                let agent_map = memory.entry(agent_id).or_insert_with(HashMap::new);
                agent_map.insert(key, entry);
                Self::evict_oldest(agent_map, max);
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
    ) -> Result<Option<MemoryEntry>, String> {
        match scope {
            MemoryScope::User => {
                let memory = self.user_memory.lock().await;
                Ok(memory.get(key).cloned())
            }
            MemoryScope::Project => {
                let pid = project_id.ok_or("Project ID required for project scope")?;
                let memory = self.project_memory.lock().await;
                Ok(memory.get(pid).and_then(|m| m.get(key).cloned()))
            }
            MemoryScope::Local => {
                let memory = self.local_memory.lock().await;
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
    ) -> Result<bool, String> {
        match scope {
            MemoryScope::User => {
                let mut memory = self.user_memory.lock().await;
                Ok(memory.remove(key).is_some())
            }
            MemoryScope::Project => {
                let pid = project_id.ok_or("Project ID required for project scope")?;
                let mut memory = self.project_memory.lock().await;
                Ok(memory.get_mut(pid).and_then(|m| m.remove(key)).is_some())
            }
            MemoryScope::Local => {
                let mut memory = self.local_memory.lock().await;
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
    ) -> Result<Vec<String>, String> {
        match scope {
            MemoryScope::User => {
                let memory = self.user_memory.lock().await;
                Ok(memory.keys().cloned().collect())
            }
            MemoryScope::Project => {
                let pid = project_id.ok_or("Project ID required for project scope")?;
                let memory = self.project_memory.lock().await;
                Ok(memory
                    .get(pid)
                    .map(|m| m.keys().cloned().collect())
                    .unwrap_or_default())
            }
            MemoryScope::Local => {
                let memory = self.local_memory.lock().await;
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
    ) -> Result<(), String> {
        match scope {
            MemoryScope::User => {
                let mut memory = self.user_memory.lock().await;
                memory.clear();
                Ok(())
            }
            MemoryScope::Project => {
                let pid = project_id.ok_or("Project ID required for project scope")?;
                let mut memory = self.project_memory.lock().await;
                if let Some(m) = memory.get_mut(pid) {
                    m.clear();
                }
                Ok(())
            }
            MemoryScope::Local => {
                let mut memory = self.local_memory.lock().await;
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
        assert!(result.unwrap_err().contains("Project ID required"));

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
}
