//! Session Index for Fast PR-to-Session Lookups
//!
//! Provides bidirectional fast lookups between GitHub PR numbers and session IDs.
//! Enables the `--from-pr` CLI flag to resume sessions linked to pull requests.
//!
//! # Philosophy
//!
//! - **Single Responsibility**: Only handles PR↔Session mappings
//! - **Self-contained**: Manages its own persistence
//! - **Standard library**: No external dependencies for core logic
//! - **Regeneratable**: Can be rebuilt from specification
//!
//! # Public API
//!
//! The "studs" (public interface):
//! - `SessionIndex::new()` - Create/load index
//! - `link_pr()` - Link session to PR
//! - `find_sessions_by_pr()` - Get sessions for PR number
//! - `find_pr_by_session()` - Get PR for session ID
//! - `remove_session()` - Remove session from index
//!
//! # Example
//!
//! ```rust,ignore
//! use rustyclawd_cli::session_index::SessionIndex;
//!
//! let mut index = SessionIndex::new()?;
//!
//! // Link a session to PR #123
//! index.link_pr("session-abc", 123)?;
//!
//! // Find all sessions for PR #123
//! if let Some(sessions) = index.find_sessions_by_pr(123) {
//!     for session_id in sessions {
//!         println!("Session: {}", session_id);
//!     }
//! }
//!
//! // Find PR for a session
//! if let Some(pr) = index.find_pr_by_session("session-abc") {
//!     println!("Session linked to PR #{}", pr);
//! }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Default location for session index
const INDEX_FILENAME: &str = "session_index.json";

/// Session index for fast PR-to-session lookups
///
/// Maintains bidirectional mappings:
/// - PR number → List of session IDs
/// - Session ID → PR number
///
/// Persisted to disk as JSON for durability across CLI invocations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIndex {
    /// Map from PR number to list of session IDs
    #[serde(default)]
    pr_to_session: HashMap<u64, Vec<String>>,

    /// Map from session ID to PR number
    #[serde(default)]
    session_to_pr: HashMap<String, u64>,

    /// Last update timestamp (ISO 8601 format)
    #[serde(default)]
    last_updated: Option<String>,

    /// Path where this index is persisted
    #[serde(skip)]
    storage_path: PathBuf,
}

impl SessionIndex {
    /// Create a new session index or load existing from default location
    ///
    /// Creates index file if it doesn't exist. Uses `~/.config/claude/session_index.json`.
    pub fn new() -> Result<Self> {
        let storage_path = Self::default_storage_path()?;
        Self::from_path(storage_path)
    }

    /// Create session index from specific path
    pub fn from_path(storage_path: PathBuf) -> Result<Self> {
        if storage_path.exists() {
            Self::load(&storage_path)
        } else {
            Self::create_empty(storage_path)
        }
    }

    /// Get default storage path for session index
    fn default_storage_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Failed to determine config directory")?
            .join("claude");

        // Ensure config directory exists
        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        // Set restrictive permissions (0700) on config directory
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o700);
            fs::set_permissions(&config_dir, perms).with_context(|| {
                format!("Failed to set permissions on {}", config_dir.display())
            })?;
        }

        Ok(config_dir.join(INDEX_FILENAME))
    }

    /// Create empty index at specified path
    fn create_empty(storage_path: PathBuf) -> Result<Self> {
        let index = Self {
            pr_to_session: HashMap::new(),
            session_to_pr: HashMap::new(),
            last_updated: None,
            storage_path,
        };

        index.save()?;
        Ok(index)
    }

    /// Load index from file
    fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read session index from {}", path.display()))?;

        let mut index: Self = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse session index from {}", path.display()))?;

        index.storage_path = path.to_path_buf();
        Ok(index)
    }

    /// Save index to file (atomic write via temp file + rename)
    fn save(&self) -> Result<()> {
        let contents =
            serde_json::to_string_pretty(self).context("Failed to serialize session index")?;

        let tmp_path = self.storage_path.with_extension("json.tmp");
        fs::write(&tmp_path, &contents).with_context(|| {
            format!(
                "Failed to write temp session index to {}",
                tmp_path.display()
            )
        })?;

        // Set restrictive permissions (0600) on temp file before rename
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&tmp_path, perms).with_context(|| {
                format!("Failed to set permissions on {}", tmp_path.display())
            })?;
        }

        fs::rename(&tmp_path, &self.storage_path).with_context(|| {
            format!(
                "Failed to rename temp file to {}",
                self.storage_path.display()
            )
        })?;

        Ok(())
    }

    /// Link a session to a PR number
    ///
    /// Creates bidirectional mapping. If session was previously linked to
    /// a different PR, the old link is removed.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Unique session identifier
    /// * `pr_number` - GitHub pull request number
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// index.link_pr("session-abc", 123)?;
    /// ```
    pub fn link_pr(&mut self, session_id: &str, pr_number: u64) -> Result<()> {
        // Remove any existing link for this session
        if let Some(old_pr) = self.session_to_pr.get(session_id).copied() {
            if old_pr == pr_number {
                // Already linked to the same PR - nothing to do
                return Ok(());
            }
            self.remove_session_from_pr(session_id, old_pr);
        }

        // Add to pr_to_session map (avoid duplicates)
        let sessions = self.pr_to_session.entry(pr_number).or_default();
        if !sessions.contains(&session_id.to_string()) {
            sessions.push(session_id.to_string());
        }

        // Add to session_to_pr map
        self.session_to_pr.insert(session_id.to_string(), pr_number);

        // Update timestamp
        self.last_updated = Some(chrono::Utc::now().to_rfc3339());

        // Persist to disk
        self.save()?;

        Ok(())
    }

    /// Find all sessions linked to a PR number
    ///
    /// Returns list of session IDs, or None if no sessions linked.
    /// Sessions are in the order they were linked (oldest first).
    pub fn find_sessions_by_pr(&self, pr_number: u64) -> Option<&[String]> {
        self.pr_to_session.get(&pr_number).map(|v| v.as_slice())
    }

    /// Find PR number linked to a session
    ///
    /// Returns PR number, or None if session not linked to any PR.
    pub fn find_pr_by_session(&self, session_id: &str) -> Option<u64> {
        self.session_to_pr.get(session_id).copied()
    }

    /// Remove a session from the index
    ///
    /// Removes session from both directions of the mapping.
    /// If this was the last session for a PR, the PR entry is also removed.
    pub fn remove_session(&mut self, session_id: &str) -> Result<()> {
        // Find and remove from session_to_pr
        if let Some(pr_number) = self.session_to_pr.remove(session_id) {
            // Remove from pr_to_session
            self.remove_session_from_pr(session_id, pr_number);

            // Update timestamp and save
            self.last_updated = Some(chrono::Utc::now().to_rfc3339());
            self.save()?;
        }

        Ok(())
    }

    /// Remove session from a specific PR's session list
    fn remove_session_from_pr(&mut self, session_id: &str, pr_number: u64) {
        if let Some(sessions) = self.pr_to_session.get_mut(&pr_number) {
            sessions.retain(|s| s != session_id);

            // If no sessions left for this PR, remove the PR entry
            if sessions.is_empty() {
                self.pr_to_session.remove(&pr_number);
            }
        }
    }

    /// Get most recent session for a PR
    ///
    /// Returns the last session ID linked to the PR (most recently linked).
    pub fn get_latest_session_for_pr(&self, pr_number: u64) -> Option<&str> {
        self.pr_to_session
            .get(&pr_number)
            .and_then(|sessions| sessions.last())
            .map(|s| s.as_str())
    }

    /// Get total number of unique PRs tracked
    pub fn pr_count(&self) -> usize {
        self.pr_to_session.len()
    }

    /// Get total number of sessions tracked
    pub fn session_count(&self) -> usize {
        self.session_to_pr.len()
    }

    /// Get all PR numbers tracked
    pub fn pr_numbers(&self) -> Vec<u64> {
        let mut prs: Vec<u64> = self.pr_to_session.keys().copied().collect();
        prs.sort_unstable();
        prs
    }
}

impl Default for SessionIndex {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback to a temp directory if config dir is unavailable
            let tmp_path = std::env::temp_dir().join(INDEX_FILENAME);
            Self {
                pr_to_session: HashMap::new(),
                session_to_pr: HashMap::new(),
                last_updated: None,
                storage_path: tmp_path,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_index() -> (SessionIndex, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index.json");
        let index = SessionIndex::from_path(index_path).unwrap();
        (index, temp_dir)
    }

    #[test]
    fn test_link_pr_basic() {
        let (mut index, _temp) = create_test_index();

        index.link_pr("session-1", 123).unwrap();

        assert_eq!(index.find_pr_by_session("session-1"), Some(123));
        assert_eq!(
            index.find_sessions_by_pr(123),
            Some(&["session-1".to_string()][..])
        );
    }

    #[test]
    fn test_multiple_sessions_per_pr() {
        let (mut index, _temp) = create_test_index();

        index.link_pr("session-1", 123).unwrap();
        index.link_pr("session-2", 123).unwrap();
        index.link_pr("session-3", 123).unwrap();

        let sessions = index.find_sessions_by_pr(123).unwrap();
        assert_eq!(sessions.len(), 3);
        assert!(sessions.contains(&"session-1".to_string()));
        assert!(sessions.contains(&"session-2".to_string()));
        assert!(sessions.contains(&"session-3".to_string()));
    }

    #[test]
    fn test_relink_session_to_different_pr() {
        let (mut index, _temp) = create_test_index();

        // Link to PR 123
        index.link_pr("session-1", 123).unwrap();
        assert_eq!(index.find_pr_by_session("session-1"), Some(123));

        // Relink to PR 456
        index.link_pr("session-1", 456).unwrap();
        assert_eq!(index.find_pr_by_session("session-1"), Some(456));

        // Should no longer be in PR 123
        assert!(index.find_sessions_by_pr(123).is_none());

        // Should be in PR 456
        assert_eq!(
            index.find_sessions_by_pr(456),
            Some(&["session-1".to_string()][..])
        );
    }

    #[test]
    fn test_remove_session() {
        let (mut index, _temp) = create_test_index();

        index.link_pr("session-1", 123).unwrap();
        index.link_pr("session-2", 123).unwrap();

        index.remove_session("session-1").unwrap();

        assert!(index.find_pr_by_session("session-1").is_none());
        assert_eq!(
            index.find_sessions_by_pr(123),
            Some(&["session-2".to_string()][..])
        );
    }

    #[test]
    fn test_remove_last_session_removes_pr() {
        let (mut index, _temp) = create_test_index();

        index.link_pr("session-1", 123).unwrap();
        index.remove_session("session-1").unwrap();

        // PR entry should be completely removed
        assert!(index.find_sessions_by_pr(123).is_none());
        assert_eq!(index.pr_count(), 0);
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index.json");

        // Create and populate index
        {
            let mut index = SessionIndex::from_path(index_path.clone()).unwrap();
            index.link_pr("session-1", 123).unwrap();
            index.link_pr("session-2", 456).unwrap();
        }

        // Load and verify
        {
            let index = SessionIndex::from_path(index_path).unwrap();
            assert_eq!(index.find_pr_by_session("session-1"), Some(123));
            assert_eq!(index.find_pr_by_session("session-2"), Some(456));
            assert_eq!(index.session_count(), 2);
            assert_eq!(index.pr_count(), 2);
        }
    }

    #[test]
    fn test_get_latest_session_for_pr() {
        let (mut index, _temp) = create_test_index();

        index.link_pr("session-1", 123).unwrap();
        index.link_pr("session-2", 123).unwrap();
        index.link_pr("session-3", 123).unwrap();

        assert_eq!(index.get_latest_session_for_pr(123), Some("session-3"));
    }

    #[test]
    fn test_pr_numbers_sorted() {
        let (mut index, _temp) = create_test_index();

        index.link_pr("session-1", 456).unwrap();
        index.link_pr("session-2", 123).unwrap();
        index.link_pr("session-3", 789).unwrap();

        assert_eq!(index.pr_numbers(), vec![123, 456, 789]);
    }

    #[test]
    fn test_counts() {
        let (mut index, _temp) = create_test_index();

        assert_eq!(index.session_count(), 0);
        assert_eq!(index.pr_count(), 0);

        index.link_pr("session-1", 123).unwrap();
        assert_eq!(index.session_count(), 1);
        assert_eq!(index.pr_count(), 1);

        index.link_pr("session-2", 123).unwrap();
        assert_eq!(index.session_count(), 2);
        assert_eq!(index.pr_count(), 1);

        index.link_pr("session-3", 456).unwrap();
        assert_eq!(index.session_count(), 3);
        assert_eq!(index.pr_count(), 2);
    }

    #[test]
    fn test_idempotent_link_pr() {
        let (mut index, _temp) = create_test_index();

        // Link same session to same PR twice
        index.link_pr("session-1", 123).unwrap();
        index.link_pr("session-1", 123).unwrap();

        // Should have exactly one entry, not a duplicate
        let sessions = index.find_sessions_by_pr(123).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0], "session-1");
        assert_eq!(index.session_count(), 1);
    }

    #[test]
    fn test_corrupted_json_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index.json");

        // Write invalid JSON to the file
        fs::write(&index_path, "{ this is not valid json !!!").unwrap();

        // Loading should return an error, not panic
        let result = SessionIndex::from_path(index_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to parse"),
            "Error should mention parsing failure, got: {}",
            err_msg
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_file_permissions_unix() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("test_index.json");

        let mut index = SessionIndex::from_path(index_path.clone()).unwrap();

        // Link a session to trigger save
        index.link_pr("test-session", 123).unwrap();

        // Check file permissions
        let metadata = fs::metadata(&index_path).unwrap();
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        // Verify file has 0600 permissions (user read+write only)
        assert_eq!(
            mode & 0o777,
            0o600,
            "Session index file should have 0600 permissions, got {:o}",
            mode & 0o777
        );
    }
}
