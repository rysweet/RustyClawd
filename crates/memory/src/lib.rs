//! RustyClawd Memory System
//!
//! High-performance memory system for AI agents with:
//! - SQLite backend (<50ms operations)
//! - Scope hierarchy (Local > Project > User)
//! - Automatic session management
//! - Thread-safe operations
//!
//! # Philosophy
//!
//! - **Ruthless Simplicity**: Clean, focused API without unnecessary abstractions
//! - **Zero-BS Implementation**: No stubs, all functionality works
//! - **Performance First**: <50ms operations (typically 2-15ms)
//! - **Modular Design**: Self-contained brick with clear public interface
//!
//! # Quick Start
//!
//! ```no_run
//! use rustyclawd_memory::{MemoryManager, MemoryType, MemoryScope};
//!
//! # fn main() -> anyhow::Result<()> {
//! // Create a memory manager
//! let manager = MemoryManager::new()?;
//!
//! // Store a memory
//! let id = manager.store(
//!     "architect",
//!     "API Design Decision",
//!     "Use REST API with JSON responses",
//!     MemoryType::Decision,
//!     MemoryScope::Project,
//! )?;
//!
//! // Retrieve memories
//! let memories = manager.important(Some(10))?;
//! # Ok(())
//! # }
//! ```
//!
//! # Session Management
//!
//! ```no_run
//! use rustyclawd_memory::MemoryManager;
//!
//! # fn main() -> anyhow::Result<()> {
//! // Create manager with session
//! let manager = MemoryManager::with_session("session-123")?;
//!
//! // Session ID automatically attached to memories
//! let memories = manager.retrieve_session_memories(None, None)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Builder Pattern
//!
//! ```no_run
//! use rustyclawd_memory::{MemoryBuilder, MemoryManager, MemoryType, MemoryScope};
//!
//! # fn main() -> anyhow::Result<()> {
//! let manager = MemoryManager::new()?;
//!
//! let id = MemoryBuilder::new(
//!     "architect",
//!     "Architecture Decision",
//!     "Use microservices pattern",
//!     MemoryType::Decision,
//!     MemoryScope::Project,
//! )
//! .importance(9)
//! .tag("architecture")
//! .tag("design")
//! .store(&manager)?;
//! # Ok(())
//! # }
//! ```

mod database;
mod manager;
mod types;

// Re-export public API
pub use database::{Database, MemoryStats};
pub use manager::{MemoryBuilder, MemoryManager};
pub use types::{MemoryEntry, MemoryParseError, MemoryQuery, MemoryScope, MemoryType};

/// Module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (MemoryManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("integration_test.db");
        let manager = MemoryManager::with_db_path(&db_path).unwrap();
        (manager, temp_dir)
    }

    #[test]
    fn test_full_workflow() {
        let (manager, _temp_dir) = create_test_manager();

        // Store different types of memories
        let decision_id = manager
            .store(
                "architect",
                "Database Choice",
                "Selected PostgreSQL for ACID compliance",
                MemoryType::Decision,
                MemoryScope::Project,
            )
            .unwrap();

        let pattern_id = MemoryBuilder::new(
            "architect",
            "Repository Pattern",
            "Use repository pattern for data access",
            MemoryType::Pattern,
            MemoryScope::Project,
        )
        .importance(8)
        .tag("patterns")
        .tag("architecture")
        .store(&manager)
        .unwrap();

        // Search memories
        let postgres_memories = manager.search("PostgreSQL", Some(10)).unwrap();
        assert_eq!(postgres_memories.len(), 1);

        // Get important memories
        let important = manager.important(None).unwrap();
        assert!(important.iter().any(|m| m.id == pattern_id));

        // Get by scope
        let project_memories = manager.by_scope(MemoryScope::Project).unwrap();
        assert_eq!(project_memories.len(), 2);

        // Delete memory
        assert!(manager.delete(&decision_id).unwrap());
        assert!(manager.get(&decision_id).unwrap().is_none());

        // Stats
        let stats = manager.stats().unwrap();
        assert_eq!(stats.total_entries, 1); // One deleted, one remains
    }

    #[test]
    fn test_session_isolation() {
        let (mut manager1, _temp_dir) = create_test_manager();

        // Create two sessions
        manager1.set_session_id("session1");

        manager1
            .store(
                "agent1",
                "Session 1 Memory",
                "Content for session 1",
                MemoryType::Context,
                MemoryScope::Local,
            )
            .unwrap();

        manager1.set_session_id("session2");

        manager1
            .store(
                "agent1",
                "Session 2 Memory",
                "Content for session 2",
                MemoryType::Context,
                MemoryScope::Local,
            )
            .unwrap();

        // Retrieve session 1 memories
        manager1.set_session_id("session1");
        let session1_memories = manager1.retrieve_session_memories(None, None).unwrap();
        assert_eq!(session1_memories.len(), 1);
        assert!(session1_memories[0].title.contains("Session 1"));

        // Retrieve session 2 memories
        manager1.set_session_id("session2");
        let session2_memories = manager1.retrieve_session_memories(None, None).unwrap();
        assert_eq!(session2_memories.len(), 1);
        assert!(session2_memories[0].title.contains("Session 2"));
    }

    #[test]
    fn test_scope_hierarchy() {
        let (manager, _temp_dir) = create_test_manager();

        // Store memories at different scopes
        manager
            .store(
                "agent1",
                "Local Memory",
                "Session-specific data",
                MemoryType::Context,
                MemoryScope::Local,
            )
            .unwrap();

        manager
            .store(
                "agent1",
                "Project Memory",
                "Project-wide knowledge",
                MemoryType::Learning,
                MemoryScope::Project,
            )
            .unwrap();

        manager
            .store(
                "agent1",
                "User Memory",
                "User preferences",
                MemoryType::Context,
                MemoryScope::User,
            )
            .unwrap();

        // Verify scope filtering (each scope returns its own entries)
        let local = manager.by_scope(MemoryScope::Local).unwrap();
        let project = manager.by_scope(MemoryScope::Project).unwrap();
        let user = manager.by_scope(MemoryScope::User).unwrap();

        assert_eq!(local.len(), 1);
        assert_eq!(project.len(), 1);
        assert_eq!(user.len(), 1);
    }

    #[test]
    fn test_expiration() {
        let (manager, _temp_dir) = create_test_manager();

        // Create memory that expires immediately
        let entry = MemoryEntry::new(
            "agent1",
            "Expired Memory",
            "This should expire",
            MemoryType::Context,
            MemoryScope::Local,
        )
        .with_expiration(chrono::Utc::now() - chrono::Duration::hours(1));

        manager.store_entry(entry).unwrap();

        // Cleanup should remove it
        let cleaned = manager.cleanup_expired().unwrap();
        assert_eq!(cleaned, 1);

        // Verify it's gone
        let stats = manager.stats().unwrap();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn test_complex_query() {
        let (mut manager, _temp_dir) = create_test_manager();

        manager.set_session_id("test-session");

        // Store various memories
        for i in 0..10 {
            let entry = MemoryEntry::new(
                "test_agent",
                format!("Memory {}", i),
                format!("Content for memory {}", i),
                if i % 2 == 0 {
                    MemoryType::Decision
                } else {
                    MemoryType::Context
                },
                MemoryScope::Project,
            )
            .with_importance((i % 10) as u8)
            .add_tag(format!("tag{}", i % 3));

            manager.store_entry(entry).unwrap();
        }

        // Complex query: decisions with importance >= 5, limited to 3
        let query = MemoryQuery::new()
            .memory_type(MemoryType::Decision)
            .min_importance(5)
            .limit(3);

        let results = manager.retrieve(&query).unwrap();
        assert!(results.len() <= 3);
        assert!(results.iter().all(|m| m.memory_type == MemoryType::Decision));
        assert!(results.iter().all(|m| m.importance >= 5));
    }
}
