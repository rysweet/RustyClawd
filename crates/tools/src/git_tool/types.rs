//! Git tool types - Data models for git operations
//!
//! Contains all serialization/deserialization types used by the git tool:
//! parameters, operation enums, and output structures.

use serde::{Deserialize, Serialize};

/// Git operation to perform
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GitOperation {
    /// Get repository status (staged, unstaged, untracked)
    Status,
    /// Get diff of changes (staged or unstaged)
    Diff {
        /// Show staged changes instead of unstaged
        #[serde(default)]
        staged: bool,
        /// Specific path to diff (optional)
        #[serde(default)]
        path: Option<String>,
    },
    /// Get commit log history
    Log {
        /// Number of commits to show (default: 10)
        #[serde(default = "default_log_count")]
        count: usize,
        /// Start from specific commit/ref (optional)
        #[serde(default)]
        from_ref: Option<String>,
    },
    /// List branches
    Branches {
        /// Include remote branches
        #[serde(default)]
        include_remote: bool,
    },
    /// Get information about a specific commit
    CommitInfo {
        /// Commit hash or reference (e.g., "HEAD", "abc1234")
        commit_ref: String,
    },
    /// Get the current branch name
    CurrentBranch,
}

fn default_log_count() -> usize {
    10
}

/// Parameters for the Git tool
#[derive(Debug, Deserialize)]
pub struct GitParams {
    /// The git operation to perform
    pub operation: GitOperation,
    /// Repository path (defaults to current directory)
    #[serde(default)]
    pub repo_path: Option<String>,
}

/// A file status entry
#[derive(Debug, Serialize, Clone)]
pub struct FileStatus {
    /// File path relative to repository root
    pub path: String,
    /// Status type
    pub status: String,
}

/// A commit entry
#[derive(Debug, Serialize, Clone)]
pub struct CommitEntry {
    /// Commit hash (short)
    pub hash: String,
    /// Full commit hash
    pub hash_full: String,
    /// Commit message (first line)
    pub message: String,
    /// Author name
    pub author: String,
    /// Author email
    pub email: String,
    /// Commit timestamp (ISO 8601)
    pub timestamp: String,
}

/// A branch entry
#[derive(Debug, Serialize, Clone)]
pub struct BranchEntry {
    /// Branch name
    pub name: String,
    /// Is this the current/HEAD branch?
    pub is_head: bool,
    /// Is this a remote branch?
    pub is_remote: bool,
}

/// Output from Git tool
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GitOutput {
    /// Status result
    Status {
        /// Staged files (ready for commit)
        staged: Vec<FileStatus>,
        /// Modified but not staged files
        unstaged: Vec<FileStatus>,
        /// Untracked files
        untracked: Vec<FileStatus>,
        /// Is the working directory clean?
        is_clean: bool,
    },
    /// Diff result
    Diff {
        /// The diff content
        diff: String,
        /// Number of files changed
        files_changed: usize,
        /// Lines added
        insertions: usize,
        /// Lines deleted
        deletions: usize,
    },
    /// Log result
    Log {
        /// List of commits
        commits: Vec<CommitEntry>,
        /// Total commits returned
        count: usize,
    },
    /// Branches result
    Branches {
        /// List of branches
        branches: Vec<BranchEntry>,
        /// Current branch name
        current: Option<String>,
    },
    /// Commit info result
    CommitInfo(CommitEntry),
    /// Current branch result
    CurrentBranch {
        /// Branch name (None if detached HEAD)
        name: Option<String>,
        /// HEAD commit hash
        head_commit: String,
    },
}
