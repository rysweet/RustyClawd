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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::types::{MemoryQuery, MemoryScope, MemoryType};
    use tempfile::TempDir;

    fn setup_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        (db, temp_dir)
    }

    #[test]
    fn test_escape_like_special_chars() {
        assert_eq!(escape_like("hello%world"), "hello\\%world");
        assert_eq!(escape_like("under_score"), "under\\_score");
        assert_eq!(escape_like("back\\slash"), "back\\\\slash");
        assert_eq!(escape_like("normal"), "normal");
    }

    #[test]
    fn test_query_empty_filters() {
        let (db, _dir) = setup_test_db();
        // Store an entry
        let entry = MemoryEntry::new(
            "test_agent",
            "test",
            "content",
            MemoryType::Decision,
            MemoryScope::Local,
        );
        db.store(&entry).unwrap();

        let query = MemoryQuery::default();
        let results = db.query(&query).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_query_with_search_filter() {
        let (db, _dir) = setup_test_db();
        db.store(&MemoryEntry::new(
            "agent",
            "findme",
            "search content",
            MemoryType::Decision,
            MemoryScope::Local,
        ))
        .unwrap();
        db.store(&MemoryEntry::new(
            "agent",
            "other",
            "different stuff",
            MemoryType::Decision,
            MemoryScope::Local,
        ))
        .unwrap();

        let query = MemoryQuery::default().search("findme");
        let results = db.query(&query).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "findme");
    }

    #[test]
    fn test_query_limit_and_offset() {
        let (db, _dir) = setup_test_db();
        for i in 0..5 {
            db.store(&MemoryEntry::new(
                "agent",
                format!("entry{}", i),
                "content",
                MemoryType::Decision,
                MemoryScope::Local,
            ))
            .unwrap();
        }

        let query = MemoryQuery::default().limit(2);
        let results = db.query(&query).unwrap();
        assert_eq!(results.len(), 2);

        let query = MemoryQuery::default().limit(2).offset(3);
        let results = db.query(&query).unwrap();
        assert!(results.len() <= 2);
    }
}
