// Database schema initialization
//
// Owns the DDL statements (CREATE TABLE, CREATE INDEX) and version tracking.
// Separated from database.rs to isolate schema concerns from CRUD operations.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use tracing::{debug, info};

/// Database schema version for migrations
pub(crate) const SCHEMA_VERSION: i32 = 1;

/// Initialize database schema inside an already-locked connection.
///
/// Uses BEGIN EXCLUSIVE to serialize concurrent schema initialization.
/// The version check runs INSIDE the exclusive transaction to prevent
/// a TOCTOU race where another process could initialize the schema
/// between the version read and the lock acquisition.
pub(crate) fn initialize_schema(conn: &Connection) -> Result<()> {
    // Acquire exclusive lock FIRST, then check version inside the transaction
    // to eliminate the TOCTOU race condition (issue #378).
    conn.execute_batch("BEGIN EXCLUSIVE")?;

    // Check if schema exists (now safely inside the exclusive transaction)
    let version: i32 = conn
        .query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if version >= SCHEMA_VERSION {
        debug!("Schema version {} is current", version);
        conn.execute_batch("COMMIT")?;
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

    // Record schema version (INSERT OR IGNORE to handle concurrent init)
    conn.execute(
        "INSERT OR IGNORE INTO schema_version (version) VALUES (?1)",
        params![SCHEMA_VERSION],
    )?;

    conn.execute_batch("COMMIT")?;

    info!("Database schema initialized successfully");
    Ok(())
}
