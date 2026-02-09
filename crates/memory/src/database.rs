// SQLite database backend for memory system
//
// Philosophy:
// - <50ms operations (target: 2-15ms)
// - Thread-safe via connection pool
// - ACID compliance with WAL mode
// - Efficient indexing for fast queries

use crate::types::{MemoryEntry, MemoryQuery, MemoryScope, MemoryType};
use anyhow::{Context, Result};
use rusqlite::{params, Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

/// Database schema version for migrations
const SCHEMA_VERSION: i32 = 1;

/// Thread-safe database handle
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl Database {
    /// Open or create a database at the specified path
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Open database with flags for performance
        let conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX, // We'll handle thread safety
        )
        .with_context(|| format!("Failed to open database: {}", path.display()))?;

        // Enable WAL mode for better concurrency
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-64000;
             PRAGMA temp_store=MEMORY;",
        )
        .context("Failed to configure database")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: path.clone(),
        };

        // Initialize schema
        db.initialize_schema()
            .context("Failed to initialize database schema")?;

        info!("Opened memory database at {}", path.display());
        Ok(db)
    }

    /// Initialize database schema
    fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Check if schema exists
        let version: i32 = conn
            .query_row(
                "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if version >= SCHEMA_VERSION {
            debug!("Schema version {} is current", version);
            return Ok(());
        }

        info!("Initializing database schema version {}", SCHEMA_VERSION);

        // Create schema_version table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )
        .context("Failed to create schema_version table")?;

        // Create main memory_entries table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_entries (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                session_id TEXT,
                memory_type TEXT NOT NULL,
                scope TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                importance INTEGER NOT NULL DEFAULT 5,
                tags TEXT, -- JSON array
                metadata TEXT, -- JSON object
                parent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                expires_at TEXT,
                FOREIGN KEY (parent_id) REFERENCES memory_entries(id) ON DELETE SET NULL
            )",
            [],
        )
        .context("Failed to create memory_entries table")?;

        // Create indexes for fast queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_agent_id ON memory_entries(agent_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_session_id ON memory_entries(session_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memory_type ON memory_entries(memory_type)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_scope ON memory_entries(scope)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_importance ON memory_entries(importance)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_created_at ON memory_entries(created_at)",
            [],
        )?;

        // Record schema version
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            params![SCHEMA_VERSION],
        )?;

        info!("Database schema initialized successfully");
        Ok(())
    }

    /// Store a memory entry in the database
    pub fn store(&self, entry: &MemoryEntry) -> Result<String> {
        let conn = self.conn.lock().unwrap();

        let tags_json = serde_json::to_string(&entry.tags)
            .context("Failed to serialize tags")?;
        let metadata_json = serde_json::to_string(&entry.metadata)
            .context("Failed to serialize metadata")?;

        conn.execute(
            "INSERT INTO memory_entries (
                id, agent_id, session_id, memory_type, scope,
                title, content, importance, tags, metadata,
                parent_id, created_at, updated_at, expires_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                &entry.id,
                &entry.agent_id,
                &entry.session_id,
                entry.memory_type.as_str(),
                entry.scope.as_str(),
                &entry.title,
                &entry.content,
                entry.importance,
                tags_json,
                metadata_json,
                &entry.parent_id,
                entry.created_at.to_rfc3339(),
                entry.updated_at.to_rfc3339(),
                entry.expires_at.map(|dt| dt.to_rfc3339()),
            ],
        )
        .with_context(|| format!("Failed to store memory entry: {}", entry.id))?;

        debug!("Stored memory entry: {}", entry.id);
        Ok(entry.id.clone())
    }

    /// Retrieve memory entries matching the query
    pub fn query(&self, query: &MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock().unwrap();

        // Build SQL query dynamically based on filters
        let mut sql = String::from("SELECT * FROM memory_entries WHERE 1=1");
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(agent_id) = &query.agent_id {
            sql.push_str(" AND agent_id = ?");
            params.push(Box::new(agent_id.clone()));
        }

        if let Some(session_id) = &query.session_id {
            sql.push_str(" AND session_id = ?");
            params.push(Box::new(session_id.clone()));
        }

        if let Some(memory_type) = &query.memory_type {
            sql.push_str(" AND memory_type = ?");
            params.push(Box::new(memory_type.as_str().to_string()));
        }

        if let Some(scope) = &query.scope {
            sql.push_str(" AND scope = ?");
            params.push(Box::new(scope.as_str().to_string()));
        }

        if let Some(min_importance) = query.min_importance {
            sql.push_str(" AND importance >= ?");
            params.push(Box::new(min_importance));
        }

        if let Some(search) = &query.search {
            sql.push_str(" AND (title LIKE ? OR content LIKE ?)");
            let pattern = format!("%{}%", search);
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern));
        }

        if let Some(created_after) = &query.created_after {
            sql.push_str(" AND created_at >= ?");
            params.push(Box::new(created_after.to_rfc3339()));
        }

        if let Some(created_before) = &query.created_before {
            sql.push_str(" AND created_at <= ?");
            params.push(Box::new(created_before.to_rfc3339()));
        }

        // Filter out expired entries
        let now = chrono::Utc::now().to_rfc3339();
        sql.push_str(" AND (expires_at IS NULL OR expires_at > ?)");
        params.push(Box::new(now));

        // Order by importance and creation time
        sql.push_str(" ORDER BY importance DESC, created_at DESC");

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = query.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        // Execute query
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt
            .query_map(param_refs.as_slice(), |row| self.row_to_entry(row))?
            .collect::<Result<Vec<_>, _>>()?;

        // Filter by tags if specified (SQLite doesn't have good JSON array support)
        let filtered_entries = if query.tags.is_empty() {
            entries
        } else {
            entries
                .into_iter()
                .filter(|entry| {
                    query
                        .tags
                        .iter()
                        .all(|tag| entry.tags.contains(tag))
                })
                .collect()
        };

        debug!("Query returned {} entries", filtered_entries.len());
        Ok(filtered_entries)
    }

    /// Get a specific memory entry by ID
    pub fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let conn = self.conn.lock().unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT * FROM memory_entries WHERE id = ? AND (expires_at IS NULL OR expires_at > ?)"
        )?;

        match stmt.query_row(params![id, now], |row| self.row_to_entry(row)) {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a memory entry
    pub fn delete(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let rows_affected = conn.execute("DELETE FROM memory_entries WHERE id = ?", params![id])?;

        if rows_affected > 0 {
            debug!("Deleted memory entry: {}", id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Delete all expired memory entries
    pub fn cleanup_expired(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();

        let now = chrono::Utc::now().to_rfc3339();
        let count = conn.execute(
            "DELETE FROM memory_entries WHERE expires_at IS NOT NULL AND expires_at <= ?",
            params![now],
        )?;

        if count > 0 {
            info!("Cleaned up {} expired memory entries", count);
        }

        Ok(count)
    }

    /// Get memory statistics
    pub fn stats(&self) -> Result<MemoryStats> {
        let conn = self.conn.lock().unwrap();

        let total_entries: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memory_entries",
            [],
            |row| row.get(0),
        )?;

        let total_size: i64 = conn.query_row(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )?;

        Ok(MemoryStats {
            total_entries: total_entries as usize,
            database_size_bytes: total_size as u64,
        })
    }

    /// Get the database file path
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Convert a database row to a MemoryEntry
    fn row_to_entry(&self, row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
        use chrono::DateTime;

        let tags_json: String = row.get("tags")?;
        let metadata_json: String = row.get("metadata")?;

        let tags: Vec<String> = serde_json::from_str(&tags_json)
            .unwrap_or_default();
        let metadata: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_str(&metadata_json)
                .unwrap_or_default();

        let memory_type_str: String = row.get("memory_type")?;
        let scope_str: String = row.get("scope")?;

        let created_at_str: String = row.get("created_at")?;
        let updated_at_str: String = row.get("updated_at")?;
        let expires_at_str: Option<String> = row.get("expires_at")?;

        Ok(MemoryEntry {
            id: row.get("id")?,
            agent_id: row.get("agent_id")?,
            session_id: row.get("session_id")?,
            memory_type: MemoryType::from_str(&memory_type_str).unwrap_or(MemoryType::Context),
            scope: MemoryScope::from_str(&scope_str).unwrap_or(MemoryScope::Local),
            title: row.get("title")?,
            content: row.get("content")?,
            importance: row.get("importance")?,
            tags,
            metadata,
            parent_id: row.get("parent_id")?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .unwrap()
                .with_timezone(&chrono::Utc),
            expires_at: expires_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            }),
        })
    }
}

/// Memory database statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub database_size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        (db, temp_dir)
    }

    #[test]
    fn test_database_creation() {
        let (_db, _temp_dir) = create_test_db();
        // Database should be created successfully
    }

    #[test]
    fn test_store_and_retrieve() {
        let (db, _temp_dir) = create_test_db();

        let entry = MemoryEntry::new(
            "test_agent",
            "Test Memory",
            "Test content",
            MemoryType::Decision,
            MemoryScope::Project,
        )
        .with_importance(8);

        let id = db.store(&entry).unwrap();
        assert_eq!(id, entry.id);

        let retrieved = db.get(&id).unwrap().unwrap();
        assert_eq!(retrieved.title, "Test Memory");
        assert_eq!(retrieved.importance, 8);
    }

    #[test]
    fn test_query_filtering() {
        let (db, _temp_dir) = create_test_db();

        // Store multiple entries
        for i in 0..5 {
            let entry = MemoryEntry::new(
                "test_agent",
                format!("Memory {}", i),
                format!("Content {}", i),
                MemoryType::Decision,
                MemoryScope::Project,
            )
            .with_importance((i + 5) as u8);
            db.store(&entry).unwrap();
        }

        // Query with importance filter
        let query = MemoryQuery::new().min_importance(7);
        let results = db.query(&query).unwrap();
        assert_eq!(results.len(), 3); // Entries with importance 7, 8, 9

        // Query with limit
        let query = MemoryQuery::new().limit(2);
        let results = db.query(&query).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_delete() {
        let (db, _temp_dir) = create_test_db();

        let entry = MemoryEntry::new(
            "test_agent",
            "To Delete",
            "Content",
            MemoryType::Context,
            MemoryScope::Local,
        );

        let id = db.store(&entry).unwrap();
        assert!(db.delete(&id).unwrap());
        assert!(db.get(&id).unwrap().is_none());
    }

    #[test]
    fn test_expiration_cleanup() {
        let (db, _temp_dir) = create_test_db();

        // Store expired entry
        let expired = MemoryEntry::new(
            "test_agent",
            "Expired",
            "Content",
            MemoryType::Context,
            MemoryScope::Local,
        )
        .with_expiration(chrono::Utc::now() - chrono::Duration::hours(1));

        db.store(&expired).unwrap();

        // Cleanup should remove it
        let count = db.cleanup_expired().unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_stats() {
        let (db, _temp_dir) = create_test_db();

        let stats = db.stats().unwrap();
        assert_eq!(stats.total_entries, 0);

        let entry = MemoryEntry::new(
            "test_agent",
            "Test",
            "Content",
            MemoryType::Context,
            MemoryScope::Local,
        );
        db.store(&entry).unwrap();

        let stats = db.stats().unwrap();
        assert_eq!(stats.total_entries, 1);
        assert!(stats.database_size_bytes > 0);
    }
}
