// SQLite database backend for memory system
//
// Philosophy:
// - <50ms operations (target: 2-15ms)
// - Thread-safe via Mutex-guarded connection with poisoning recovery
// - ACID compliance with WAL mode
// - Efficient indexing for fast queries
//
// Thread safety note:
// The Connection is wrapped in Arc<Mutex<>> which serializes all access.
// rusqlite::Connection does not implement Sync, so RwLock cannot be used.
// SQLITE_OPEN_FULL_MUTEX is used as defense-in-depth.
// Lock poisoning is handled gracefully by recovering the inner guard.

use crate::query_builder;
use crate::schema;
use crate::types::{MemoryEntry, MemoryQuery, MemoryScope, MemoryType};
use anyhow::{Context, Result};
use chrono::DateTime;
use rusqlite::{params, Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// Maximum allowed content length (1 MB)
const MAX_CONTENT_LENGTH: usize = 1_048_576;
/// Maximum allowed title length (1000 characters)
const MAX_TITLE_LENGTH: usize = 1_000;
/// Maximum allowed agent_id length (256 characters)
const MAX_AGENT_ID_LENGTH: usize = 256;

/// Thread-safe database handle
///
/// Uses `Mutex` to serialize all access (rusqlite::Connection is not Sync).
/// Lock poisoning is handled gracefully by recovering the guard on panic.
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

            // Set restrictive permissions on parent directory (owner-only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
                    .with_context(|| {
                        format!("Failed to set directory permissions: {}", parent.display())
                    })?;
            }
        }

        // Open database with FULL_MUTEX for defense-in-depth thread safety
        let conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
        )
        .with_context(|| format!("Failed to open database: {}", path.display()))?;

        // Set restrictive permissions on database file (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .with_context(|| format!("Failed to set file permissions: {}", path.display()))?;
        }

        // Enable WAL mode for better concurrency
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-64000;
             PRAGMA temp_store=MEMORY;",
        )
        .context("Failed to configure database")?;

        info!("Opened memory database at {}", path.display());

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        };

        // Initialize schema
        {
            let conn = db.lock_conn()?;
            schema::initialize_schema(&conn).context("Failed to initialize database schema")?;
        }

        Ok(db)
    }

    /// Acquire lock on the database connection
    ///
    /// Recovers from poisoned mutex by unwrapping the guard, allowing
    /// continued operation even if a previous thread panicked while holding the lock.
    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        match self.conn.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                warn!("Database lock was poisoned, recovering...");
                Ok(poisoned.into_inner())
            }
        }
    }

    /// Validate content length limits on a memory entry
    fn validate_entry(entry: &MemoryEntry) -> Result<()> {
        if entry.content.len() > MAX_CONTENT_LENGTH {
            anyhow::bail!(
                "Content exceeds maximum length of {} bytes (got {})",
                MAX_CONTENT_LENGTH,
                entry.content.len()
            );
        }
        if entry.title.len() > MAX_TITLE_LENGTH {
            anyhow::bail!(
                "Title exceeds maximum length of {} characters (got {})",
                MAX_TITLE_LENGTH,
                entry.title.len()
            );
        }
        if entry.agent_id.len() > MAX_AGENT_ID_LENGTH {
            anyhow::bail!(
                "Agent ID exceeds maximum length of {} characters (got {})",
                MAX_AGENT_ID_LENGTH,
                entry.agent_id.len()
            );
        }
        Ok(())
    }

    /// Store a memory entry in the database
    pub fn store(&self, entry: &MemoryEntry) -> Result<String> {
        Self::validate_entry(entry)?;
        let conn = self.lock_conn()?;

        let tags_json = serde_json::to_string(&entry.tags).context("Failed to serialize tags")?;
        let metadata_json =
            serde_json::to_string(&entry.metadata).context("Failed to serialize metadata")?;

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

    /// Update an existing memory entry
    ///
    /// Updates all mutable fields (title, content, importance, tags, metadata,
    /// scope, memory_type, parent_id, expires_at) and sets updated_at to now.
    /// Returns true if the entry existed and was updated, false if not found.
    pub fn update(&self, entry: &MemoryEntry) -> Result<bool> {
        Self::validate_entry(entry)?;
        let conn = self.lock_conn()?;

        let tags_json = serde_json::to_string(&entry.tags).context("Failed to serialize tags")?;
        let metadata_json =
            serde_json::to_string(&entry.metadata).context("Failed to serialize metadata")?;

        let now = chrono::Utc::now().to_rfc3339();

        let rows_affected = conn
            .execute(
                "UPDATE memory_entries SET
                memory_type = ?1,
                scope = ?2,
                title = ?3,
                content = ?4,
                importance = ?5,
                tags = ?6,
                metadata = ?7,
                parent_id = ?8,
                updated_at = ?9,
                expires_at = ?10
            WHERE id = ?11",
                params![
                    entry.memory_type.as_str(),
                    entry.scope.as_str(),
                    &entry.title,
                    &entry.content,
                    entry.importance,
                    tags_json,
                    metadata_json,
                    &entry.parent_id,
                    now,
                    entry.expires_at.map(|dt| dt.to_rfc3339()),
                    &entry.id,
                ],
            )
            .with_context(|| format!("Failed to update memory entry: {}", entry.id))?;

        if rows_affected > 0 {
            debug!("Updated memory entry: {}", entry.id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Retrieve memory entries matching the query
    pub fn query(&self, query: &MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let conn = self.lock_conn()?;
        query_builder::execute_query(&conn, query, Self::row_to_entry)
    }

    /// Get a specific memory entry by ID
    pub fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let conn = self.lock_conn()?;

        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT * FROM memory_entries WHERE id = ? AND (expires_at IS NULL OR expires_at > ?)",
        )?;

        match stmt.query_row(params![id, now], Self::row_to_entry) {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a memory entry
    pub fn delete(&self, id: &str) -> Result<bool> {
        let conn = self.lock_conn()?;

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
        let conn = self.lock_conn()?;

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
        let conn = self.lock_conn()?;

        let total_entries: i64 =
            conn.query_row("SELECT COUNT(*) FROM memory_entries", [], |row| row.get(0))?;

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
    ///
    /// Handles malformed datetime values gracefully by falling back to Utc::now()
    /// with a warning log, rather than panicking and poisoning the mutex.
    fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
        let tags_json: String = row.get("tags")?;
        let metadata_json: String = row.get("metadata")?;

        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let metadata: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_str(&metadata_json).unwrap_or_default();

        let memory_type_str: String = row.get("memory_type")?;
        let scope_str: String = row.get("scope")?;

        let created_at_str: String = row.get("created_at")?;
        let updated_at_str: String = row.get("updated_at")?;
        let expires_at_str: Option<String> = row.get("expires_at")?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|e| {
                warn!(
                    "Malformed created_at '{}' in database, using current time: {}",
                    created_at_str, e
                );
                chrono::Utc::now()
            });

        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|e| {
                warn!(
                    "Malformed updated_at '{}' in database, using current time: {}",
                    updated_at_str, e
                );
                chrono::Utc::now()
            });

        let expires_at = expires_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        Ok(MemoryEntry {
            id: row.get("id")?,
            agent_id: row.get("agent_id")?,
            session_id: row.get("session_id")?,
            memory_type: memory_type_str.parse().unwrap_or(MemoryType::Context),
            scope: scope_str.parse().unwrap_or(MemoryScope::Local),
            title: row.get("title")?,
            content: row.get("content")?,
            importance: row.get("importance")?,
            tags,
            metadata,
            parent_id: row.get("parent_id")?,
            created_at,
            updated_at,
            expires_at,
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
#[path = "database_tests.rs"]
mod tests;
