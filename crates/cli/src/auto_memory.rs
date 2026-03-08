//! Auto-Memory - persistent memory across sessions
//!
//! Provides automatic storage of useful context in CLAUDE.md files,
//! supporting both user-scoped (~/.claude/CLAUDE.md) and project-scoped
//! (.claude/CLAUDE.md) memory.

use std::fs;
use std::path::{Path, PathBuf};

/// Scope for memory storage
pub enum MemoryScope {
    /// User-level memory at ~/.claude/CLAUDE.md
    User,
    /// Project-level memory at .claude/CLAUDE.md
    Project,
}

/// Auto-memory system for persisting context across sessions
pub struct AutoMemory;

impl AutoMemory {
    /// Record a memory entry with timestamp, appending to the appropriate CLAUDE.md file.
    ///
    /// Uses atomic write (temp file + rename) to avoid corruption.
    pub fn record_memory(content: &str, scope: MemoryScope) -> anyhow::Result<PathBuf> {
        let path = Self::resolve_path(&scope)?;
        Self::record_memory_to_path(content, &path)
    }

    /// Record a memory entry to an explicit path (for testability).
    pub fn record_memory_to_path(content: &str, path: &Path) -> anyhow::Result<PathBuf> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let existing = if path.exists() {
            match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Memory file corrupted, starting fresh: {}", e);
                    "# Auto-Memory\n".to_string()
                }
            }
        } else {
            "# Auto-Memory\n".to_string()
        };

        let timestamp = chrono::Utc::now().to_rfc3339();
        let updated = format!("{}\n## Memory - {}\n\n{}\n", existing, timestamp, content);

        // Atomic write: temp file + rename, clean up on failure
        let tmp = path.with_extension("tmp_memory");
        fs::write(&tmp, &updated)?;
        if let Err(e) = fs::rename(&tmp, path) {
            let _ = fs::remove_file(&tmp); // Clean up temp on failure
            return Err(e.into());
        }

        Ok(path.to_path_buf())
    }

    /// Read existing memory for the given scope.
    ///
    /// Returns `None` if no memory file exists yet.
    pub fn read_memory(scope: &MemoryScope) -> anyhow::Result<Option<String>> {
        let path = Self::resolve_path(scope)?;
        Self::read_memory_from_path(&path)
    }

    /// Read memory from an explicit path (for testability).
    pub fn read_memory_from_path(path: &Path) -> anyhow::Result<Option<String>> {
        if path.exists() {
            Ok(Some(fs::read_to_string(path)?))
        } else {
            Ok(None)
        }
    }

    /// Resolve the file path for a given memory scope.
    fn resolve_path(scope: &MemoryScope) -> anyhow::Result<PathBuf> {
        match scope {
            MemoryScope::User => {
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
                Ok(PathBuf::from(home).join(".claude").join("CLAUDE.md"))
            }
            MemoryScope::Project => Ok(PathBuf::from(".claude").join("CLAUDE.md")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_memory_path() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".claude").join("CLAUDE.md");
        (tmp, path)
    }

    #[test]
    fn test_record_then_read_roundtrip() {
        let (_tmp, path) = temp_memory_path();

        AutoMemory::record_memory_to_path("test entry one", &path).unwrap();

        let content = AutoMemory::read_memory_from_path(&path).unwrap();
        assert!(content.is_some());
        let text = content.unwrap();
        assert!(text.contains("# Auto-Memory"));
        assert!(text.contains("test entry one"));
        assert!(text.contains("## Memory - "));
    }

    #[test]
    fn test_read_missing_file_returns_none() {
        let (_tmp, path) = temp_memory_path();

        let result = AutoMemory::read_memory_from_path(&path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_record_creates_parent_dirs() {
        let (_tmp, path) = temp_memory_path();
        let claude_dir = path.parent().unwrap();
        assert!(!claude_dir.exists());

        AutoMemory::record_memory_to_path("creates dirs", &path).unwrap();

        assert!(claude_dir.exists());
        assert!(path.exists());
    }

    #[test]
    fn test_multiple_appends_accumulate() {
        let (_tmp, path) = temp_memory_path();

        AutoMemory::record_memory_to_path("first entry", &path).unwrap();
        AutoMemory::record_memory_to_path("second entry", &path).unwrap();
        AutoMemory::record_memory_to_path("third entry", &path).unwrap();

        let content = AutoMemory::read_memory_from_path(&path).unwrap().unwrap();
        assert!(content.contains("first entry"));
        assert!(content.contains("second entry"));
        assert!(content.contains("third entry"));

        // Should have 3 Memory headers
        let count = content.matches("## Memory - ").count();
        assert_eq!(count, 3);
    }
}
