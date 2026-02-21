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
fn test_update() {
    let (db, _temp_dir) = create_test_db();

    let mut entry = MemoryEntry::new(
        "test_agent",
        "Original Title",
        "Original content",
        MemoryType::Decision,
        MemoryScope::Project,
    )
    .with_importance(5);

    let id = db.store(&entry).unwrap();

    // Modify and update
    entry.title = "Updated Title".to_string();
    entry.content = "Updated content".to_string();
    entry.importance = 9;

    let updated = db.update(&entry).unwrap();
    assert!(updated);

    let retrieved = db.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.title, "Updated Title");
    assert_eq!(retrieved.content, "Updated content");
    assert_eq!(retrieved.importance, 9);
    // updated_at should be newer than created_at (or at least equal, since time resolution)
    assert!(retrieved.updated_at >= retrieved.created_at);
}

#[test]
fn test_update_nonexistent_returns_false() {
    let (db, _temp_dir) = create_test_db();

    let entry = MemoryEntry::new(
        "test_agent",
        "Ghost",
        "Does not exist",
        MemoryType::Context,
        MemoryScope::Local,
    );

    let updated = db.update(&entry).unwrap();
    assert!(!updated);
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
fn test_search_escapes_like_wildcards() {
    let (db, _temp_dir) = create_test_db();

    // Store entries with special LIKE characters in content
    let entry1 = MemoryEntry::new(
        "test_agent",
        "Percent Entry",
        "This has 100% coverage",
        MemoryType::Context,
        MemoryScope::Local,
    );
    db.store(&entry1).unwrap();

    let entry2 = MemoryEntry::new(
        "test_agent",
        "Underscore Entry",
        "Use snake_case naming",
        MemoryType::Context,
        MemoryScope::Local,
    );
    db.store(&entry2).unwrap();

    let entry3 = MemoryEntry::new(
        "test_agent",
        "Normal Entry",
        "Just normal content here",
        MemoryType::Context,
        MemoryScope::Local,
    );
    db.store(&entry3).unwrap();

    // Searching for literal "%" should only match the percent entry
    let query = MemoryQuery::new().search("100%");
    let results = db.query(&query).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Percent Entry");

    // Searching for literal "_" should only match the underscore entry
    let query = MemoryQuery::new().search("snake_case");
    let results = db.query(&query).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Underscore Entry");
}

#[test]
fn test_tag_filtering_in_sql() {
    let (db, _temp_dir) = create_test_db();

    let entry1 = MemoryEntry::new(
        "test_agent",
        "Tagged Entry",
        "Content",
        MemoryType::Decision,
        MemoryScope::Project,
    )
    .add_tag("rust")
    .add_tag("architecture");
    db.store(&entry1).unwrap();

    let entry2 = MemoryEntry::new(
        "test_agent",
        "Other Entry",
        "Content",
        MemoryType::Decision,
        MemoryScope::Project,
    )
    .add_tag("python");
    db.store(&entry2).unwrap();

    // Query for "rust" tag
    let query = MemoryQuery::new().add_tag("rust");
    let results = db.query(&query).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Tagged Entry");

    // Query for both "rust" and "architecture" tags
    let query = MemoryQuery::new().add_tag("rust").add_tag("architecture");
    let results = db.query(&query).unwrap();
    assert_eq!(results.len(), 1);

    // Query for a tag that doesn't match any entry
    let query = MemoryQuery::new().add_tag("go");
    let results = db.query(&query).unwrap();
    assert_eq!(results.len(), 0);
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

#[test]
fn test_concurrent_access() {
    let (db, _temp_dir) = create_test_db();

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let db = db.clone();
            std::thread::spawn(move || {
                let entry = MemoryEntry::new(
                    format!("agent_{}", i),
                    format!("Thread {} Memory", i),
                    format!("Content from thread {}", i),
                    MemoryType::Context,
                    MemoryScope::Local,
                );
                db.store(&entry).unwrap();
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let stats = db.stats().unwrap();
    assert_eq!(stats.total_entries, 10);
}

#[test]
fn test_corrupted_database_file_returns_error() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("corrupt.db");
    std::fs::write(&db_path, b"this is not a sqlite database").unwrap();

    let result = Database::open(&db_path);
    // Should return an error, not panic
    assert!(result.is_err());
}

#[test]
fn test_empty_string_inputs() {
    let (db, _temp_dir) = create_test_db();

    // Empty title and content should succeed (no validation rejects empty)
    let entry = MemoryEntry::new("", "", "", MemoryType::Context, MemoryScope::Local);
    let result = db.store(&entry);
    assert!(result.is_ok());

    let id = result.unwrap();
    let retrieved = db.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.title, "");
    assert_eq!(retrieved.content, "");
    assert_eq!(retrieved.agent_id, "");
}

#[test]
fn test_concurrent_reads_while_writing() {
    use std::sync::Barrier;

    let (db, _temp_dir) = create_test_db();

    // Seed some data first
    for i in 0..5 {
        let entry = MemoryEntry::new(
            "agent",
            format!("Seed {}", i),
            "Content",
            MemoryType::Context,
            MemoryScope::Local,
        );
        db.store(&entry).unwrap();
    }

    let barrier = Arc::new(Barrier::new(12)); // 10 readers + 2 writers

    // Spawn 10 reader threads and 2 writer threads concurrently
    let mut handles = Vec::new();

    for i in 0..10 {
        let db = db.clone();
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            // Concurrent read
            let query = MemoryQuery::new().limit(10);
            let results = db.query(&query).unwrap();
            assert!(!results.is_empty(), "Reader {} got no results", i);
        }));
    }

    for i in 0..2 {
        let db = db.clone();
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            // Concurrent write
            let entry = MemoryEntry::new(
                "writer",
                format!("Written {}", i),
                "New content",
                MemoryType::Decision,
                MemoryScope::Project,
            );
            db.store(&entry).unwrap();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all writes completed
    let stats = db.stats().unwrap();
    assert_eq!(stats.total_entries, 7); // 5 seeded + 2 written
}

#[test]
fn test_content_length_limit_enforced() {
    let (db, _temp_dir) = create_test_db();

    // Content exceeding 1MB should be rejected
    let huge_content = "x".repeat(MAX_CONTENT_LENGTH + 1);
    let entry = MemoryEntry::new(
        "agent",
        "Big Entry",
        huge_content,
        MemoryType::Context,
        MemoryScope::Local,
    );
    let result = db.store(&entry);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Content exceeds maximum length"));
}

#[test]
fn test_title_length_limit_enforced() {
    let (db, _temp_dir) = create_test_db();

    let huge_title = "t".repeat(MAX_TITLE_LENGTH + 1);
    let entry = MemoryEntry::new(
        "agent",
        huge_title,
        "Normal content",
        MemoryType::Context,
        MemoryScope::Local,
    );
    let result = db.store(&entry);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Title exceeds maximum length"));
}

#[cfg(unix)]
#[test]
fn test_database_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("perms_test").join("test.db");
    let _db = Database::open(&db_path).unwrap();

    // Verify file permissions are 0600 (owner read/write only)
    let file_perms = std::fs::metadata(&db_path).unwrap().permissions();
    assert_eq!(
        file_perms.mode() & 0o777,
        0o600,
        "Database file should have 0600 permissions"
    );

    // Verify parent directory permissions are 0700 (owner only)
    let dir_perms = std::fs::metadata(db_path.parent().unwrap())
        .unwrap()
        .permissions();
    assert_eq!(
        dir_perms.mode() & 0o777,
        0o700,
        "Parent directory should have 0700 permissions"
    );
}

#[test]
fn test_database_recovers_from_poisoned_lock() {
    use std::panic;

    let (db, _temp_dir) = create_test_db();
    let db_clone = db.clone();

    // Deliberately poison the mutex by panicking while holding the lock
    let handle = std::thread::spawn(move || {
        let _ = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let _guard = db_clone.lock_conn().unwrap();
            panic!("Intentional panic to poison the lock");
        }));
    });

    let _ = handle.join();

    // The mutex is now poisoned, but lock_conn() should recover
    // by unwrapping the PoisonError and returning the inner guard
    let entry = MemoryEntry::new(
        "test_agent",
        "After Poison",
        "Content after recovering from poisoned lock",
        MemoryType::Context,
        MemoryScope::Local,
    );

    // This should succeed because lock_conn() recovers from poisoning
    let result = db.store(&entry);
    assert!(
        result.is_ok(),
        "Database should recover from poisoned lock and allow operations"
    );

    // Verify the entry was actually stored
    let id = result.unwrap();
    let retrieved = db.get(&id).unwrap();
    assert!(
        retrieved.is_some(),
        "Entry should be retrievable after lock recovery"
    );
}
