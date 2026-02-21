// Dynamic SQL query construction for memory entries
//
// Builds parameterized WHERE clauses, ORDER BY, and LIMIT/OFFSET
// from a MemoryQuery. Separated from database.rs to isolate query
// construction logic from connection management and CRUD operations.

use crate::types::{MemoryEntry, MemoryQuery};
use anyhow::Result;
use rusqlite::Connection;
use tracing::debug;

/// Escape LIKE wildcard characters in a search string
fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Execute a dynamic query against memory_entries, returning matching rows.
///
/// Builds SQL with parameterized placeholders from the filters in `query`.
/// The `row_to_entry` function is passed in to avoid coupling to Database internals.
pub(crate) fn execute_query(
    conn: &Connection,
    query: &MemoryQuery,
    row_to_entry: fn(&rusqlite::Row) -> rusqlite::Result<MemoryEntry>,
) -> Result<Vec<MemoryEntry>> {
    // Build SQL query dynamically based on filters
    let mut sql = String::from("SELECT * FROM memory_entries WHERE 1=1");
    let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(agent_id) = &query.agent_id {
        sql.push_str(" AND agent_id = ?");
        param_values.push(Box::new(agent_id.clone()));
    }

    if let Some(session_id) = &query.session_id {
        sql.push_str(" AND session_id = ?");
        param_values.push(Box::new(session_id.clone()));
    }

    if let Some(memory_type) = &query.memory_type {
        sql.push_str(" AND memory_type = ?");
        param_values.push(Box::new(memory_type.as_str().to_string()));
    }

    if let Some(scope) = &query.scope {
        sql.push_str(" AND scope = ?");
        param_values.push(Box::new(scope.as_str().to_string()));
    }

    if let Some(min_importance) = query.min_importance {
        sql.push_str(" AND importance >= ?");
        param_values.push(Box::new(min_importance));
    }

    if let Some(search) = &query.search {
        sql.push_str(" AND (title LIKE ? ESCAPE '\\' OR content LIKE ? ESCAPE '\\')");
        let escaped = escape_like(search);
        let pattern = format!("%{}%", escaped);
        param_values.push(Box::new(pattern.clone()));
        param_values.push(Box::new(pattern));
    }

    if let Some(created_after) = &query.created_after {
        sql.push_str(" AND created_at >= ?");
        param_values.push(Box::new(created_after.to_rfc3339()));
    }

    if let Some(created_before) = &query.created_before {
        sql.push_str(" AND created_at <= ?");
        param_values.push(Box::new(created_before.to_rfc3339()));
    }

    // Tag filtering in SQL using json_each() to avoid loading all rows into memory
    if !query.tags.is_empty() {
        for tag in &query.tags {
            sql.push_str(
                " AND EXISTS (SELECT 1 FROM json_each(memory_entries.tags) WHERE json_each.value = ?)",
            );
            param_values.push(Box::new(tag.clone()));
        }
    }

    // Filter out expired entries
    let now = chrono::Utc::now().to_rfc3339();
    sql.push_str(" AND (expires_at IS NULL OR expires_at > ?)");
    param_values.push(Box::new(now));

    // Order by importance and creation time
    sql.push_str(" ORDER BY importance DESC, created_at DESC");

    // Use parameterized LIMIT/OFFSET instead of format! interpolation
    if let Some(limit) = query.limit {
        sql.push_str(" LIMIT ?");
        param_values.push(Box::new(limit as i64));
    }

    if let Some(offset) = query.offset {
        sql.push_str(" OFFSET ?");
        param_values.push(Box::new(offset as i64));
    }

    // Execute query
    let param_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let entries = stmt
        .query_map(param_refs.as_slice(), row_to_entry)?
        .collect::<Result<Vec<_>, _>>()?;

    debug!("Query returned {} entries", entries.len());
    Ok(entries)
}
