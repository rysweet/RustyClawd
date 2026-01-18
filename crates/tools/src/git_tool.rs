//! Git tool - Native git operations without shelling out to git CLI
//!
//! Provides:
//! - Repository status (staged, unstaged, untracked files)
//! - Diff output for changes
//! - Commit log history
//! - Branch listing
//! - Commit information lookup
//!
//! Uses the git2 crate for native git operations.

use crate::{ToolContext, ToolError, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use git2::{DiffOptions, Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;

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

/// The Git tool
pub struct GitTool;

impl GitTool {
    fn open_repository(repo_path: Option<&str>, cwd: &Path) -> Result<Repository, ToolError> {
        let path = match repo_path {
            Some(p) => std::path::PathBuf::from(p),
            None => cwd.to_path_buf(),
        };

        Repository::discover(&path).map_err(|e| {
            ToolError::ExecutionFailed(format!(
                "Failed to open git repository at {:?}: {}",
                path, e
            ))
        })
    }

    fn get_status(repo: &Repository) -> Result<GitOutput, ToolError> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        let statuses = repo.statuses(Some(&mut opts)).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to get repository status: {}", e))
        })?;

        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("<invalid path>").to_string();
            let status = entry.status();

            // Check for staged changes (index changes)
            if status.is_index_new() {
                staged.push(FileStatus {
                    path: path.clone(),
                    status: "new file".to_string(),
                });
            } else if status.is_index_modified() {
                staged.push(FileStatus {
                    path: path.clone(),
                    status: "modified".to_string(),
                });
            } else if status.is_index_deleted() {
                staged.push(FileStatus {
                    path: path.clone(),
                    status: "deleted".to_string(),
                });
            } else if status.is_index_renamed() {
                staged.push(FileStatus {
                    path: path.clone(),
                    status: "renamed".to_string(),
                });
            } else if status.is_index_typechange() {
                staged.push(FileStatus {
                    path: path.clone(),
                    status: "typechange".to_string(),
                });
            }

            // Check for unstaged changes (worktree changes)
            if status.is_wt_modified() {
                unstaged.push(FileStatus {
                    path: path.clone(),
                    status: "modified".to_string(),
                });
            } else if status.is_wt_deleted() {
                unstaged.push(FileStatus {
                    path: path.clone(),
                    status: "deleted".to_string(),
                });
            } else if status.is_wt_renamed() {
                unstaged.push(FileStatus {
                    path: path.clone(),
                    status: "renamed".to_string(),
                });
            } else if status.is_wt_typechange() {
                unstaged.push(FileStatus {
                    path: path.clone(),
                    status: "typechange".to_string(),
                });
            }

            // Check for untracked files
            if status.is_wt_new() {
                untracked.push(FileStatus {
                    path,
                    status: "untracked".to_string(),
                });
            }
        }

        let is_clean = staged.is_empty() && unstaged.is_empty() && untracked.is_empty();

        Ok(GitOutput::Status {
            staged,
            unstaged,
            untracked,
            is_clean,
        })
    }

    fn get_diff(
        repo: &Repository,
        staged: bool,
        path: Option<&str>,
    ) -> Result<GitOutput, ToolError> {
        let mut opts = DiffOptions::new();

        if let Some(p) = path {
            opts.pathspec(p);
        }

        let diff = if staged {
            // Diff between HEAD and index (staged changes)
            let head = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
            repo.diff_tree_to_index(head.as_ref(), None, Some(&mut opts))
        } else {
            // Diff between index and working directory (unstaged changes)
            repo.diff_index_to_workdir(None, Some(&mut opts))
        }
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to compute diff: {}", e)))?;

        let stats = diff
            .stats()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get diff stats: {}", e)))?;

        let mut diff_output = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let origin = line.origin();
            if origin == '+' || origin == '-' || origin == ' ' {
                diff_output.push(origin);
            }
            diff_output.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        })
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to format diff: {}", e)))?;

        Ok(GitOutput::Diff {
            diff: diff_output,
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
        })
    }

    fn get_log(
        repo: &Repository,
        count: usize,
        from_ref: Option<&str>,
    ) -> Result<GitOutput, ToolError> {
        let start_oid = if let Some(ref_name) = from_ref {
            repo.revparse_single(ref_name)
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!(
                        "Failed to resolve ref '{}': {}",
                        ref_name, e
                    ))
                })?
                .id()
        } else {
            repo.head()
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get HEAD: {}", e)))?
                .target()
                .ok_or_else(|| ToolError::ExecutionFailed("HEAD has no target".to_string()))?
        };

        let mut revwalk = repo
            .revwalk()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create revwalk: {}", e)))?;

        revwalk.push(start_oid).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to start revwalk from commit: {}", e))
        })?;

        let mut commits = Vec::new();
        for (idx, oid_result) in revwalk.enumerate() {
            if idx >= count {
                break;
            }

            let oid = oid_result
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get commit oid: {}", e)))?;

            let commit = repo
                .find_commit(oid)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to find commit: {}", e)))?;

            let author = commit.author();
            let time = commit.time();
            let timestamp = chrono::DateTime::from_timestamp(time.seconds(), 0)
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                .unwrap_or_else(|| "unknown".to_string());

            commits.push(CommitEntry {
                hash: oid.to_string()[..7].to_string(),
                hash_full: oid.to_string(),
                message: commit
                    .message()
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string(),
                author: author.name().unwrap_or("Unknown").to_string(),
                email: author.email().unwrap_or("").to_string(),
                timestamp,
            });
        }

        let commit_count = commits.len();
        Ok(GitOutput::Log {
            commits,
            count: commit_count,
        })
    }

    fn get_branches(repo: &Repository, include_remote: bool) -> Result<GitOutput, ToolError> {
        let head = repo.head().ok();
        let head_name = head
            .as_ref()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));

        let mut branches_vec = Vec::new();

        let branch_filter = if include_remote {
            None
        } else {
            Some(git2::BranchType::Local)
        };

        let branches = repo
            .branches(branch_filter)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list branches: {}", e)))?;

        for branch_result in branches {
            let (branch, branch_type) = branch_result.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to get branch info: {}", e))
            })?;

            let name = branch
                .name()
                .ok()
                .flatten()
                .unwrap_or("<invalid>")
                .to_string();

            let is_head = head_name.as_ref().map(|h| h == &name).unwrap_or(false);
            let is_remote = branch_type == git2::BranchType::Remote;

            branches_vec.push(BranchEntry {
                name,
                is_head,
                is_remote,
            });
        }

        Ok(GitOutput::Branches {
            branches: branches_vec,
            current: head_name,
        })
    }

    fn get_commit_info(repo: &Repository, commit_ref: &str) -> Result<GitOutput, ToolError> {
        let obj = repo.revparse_single(commit_ref).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to resolve ref '{}': {}", commit_ref, e))
        })?;

        let commit = obj.peel_to_commit().map_err(|e| {
            ToolError::ExecutionFailed(format!("'{}' is not a commit: {}", commit_ref, e))
        })?;

        let author = commit.author();
        let time = commit.time();
        let timestamp = chrono::DateTime::from_timestamp(time.seconds(), 0)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let oid = commit.id();

        Ok(GitOutput::CommitInfo(CommitEntry {
            hash: oid.to_string()[..7].to_string(),
            hash_full: oid.to_string(),
            message: commit.message().unwrap_or("").to_string(),
            author: author.name().unwrap_or("Unknown").to_string(),
            email: author.email().unwrap_or("").to_string(),
            timestamp,
        }))
    }

    fn get_current_branch(repo: &Repository) -> Result<GitOutput, ToolError> {
        let head = repo
            .head()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get HEAD: {}", e)))?;

        let head_commit = head
            .target()
            .map(|oid| oid.to_string()[..7].to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let name = if head.is_branch() {
            head.shorthand().map(|s| s.to_string())
        } else {
            None // Detached HEAD
        };

        Ok(GitOutput::CurrentBranch { name, head_commit })
    }
}

#[async_trait]
impl crate::Tool for GitTool {
    type Params = GitParams;
    type Output = GitOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "Git",
            description: "Native git operations (status, diff, log, branches, commit info)",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let cwd = ctx.cwd.clone();
        let repo_path = params.repo_path.clone();
        let operation = params.operation.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Performing git {:?} operation", operation),
                percentage: None,
            };

            // Open repository
            let repo = match Self::open_repository(repo_path.as_deref(), &cwd) {
                Ok(r) => r,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: e.to_string(),
                    };
                    return;
                }
            };

            if debug {
                tracing::debug!(?operation, "Executing git operation");
            }

            let result = match operation {
                GitOperation::Status => Self::get_status(&repo),
                GitOperation::Diff { staged, path } => {
                    Self::get_diff(&repo, staged, path.as_deref())
                }
                GitOperation::Log { count, from_ref } => {
                    Self::get_log(&repo, count, from_ref.as_deref())
                }
                GitOperation::Branches { include_remote } => {
                    Self::get_branches(&repo, include_remote)
                }
                GitOperation::CommitInfo { commit_ref } => {
                    Self::get_commit_info(&repo, &commit_ref)
                }
                GitOperation::CurrentBranch => Self::get_current_branch(&repo),
            };

            match result {
                Ok(output) => {
                    if debug {
                        tracing::debug!("Git operation completed successfully");
                    }
                    yield ToolEvent::Result(output);
                }
                Err(e) => {
                    yield ToolEvent::Error {
                        message: e.to_string(),
                    };
                }
            }
        }))
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_git_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to init git repo");

        // Configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to configure git email");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to configure git name");

        // Create initial commit
        std::fs::write(temp_dir.path().join("README.md"), "# Test\n").unwrap();

        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to add file");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp_dir.path())
            .output()
            .expect("Failed to commit");

        temp_dir
    }

    #[tokio::test]
    async fn test_git_status_clean() {
        let temp_dir = setup_git_repo();

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::Status,
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext {
            cwd: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        assert!(result.is_some());
        if let Some(GitOutput::Status { is_clean, .. }) = result {
            assert!(is_clean);
        } else {
            panic!("Expected Status output");
        }
    }

    #[tokio::test]
    async fn test_git_status_with_changes() {
        let temp_dir = setup_git_repo();

        // Add a new untracked file
        std::fs::write(temp_dir.path().join("new_file.txt"), "new content").unwrap();

        // Modify the existing file
        std::fs::write(temp_dir.path().join("README.md"), "# Modified\n").unwrap();

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::Status,
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext {
            cwd: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        assert!(result.is_some());
        if let Some(GitOutput::Status {
            unstaged,
            untracked,
            is_clean,
            ..
        }) = result
        {
            assert!(!is_clean);
            assert_eq!(unstaged.len(), 1);
            assert_eq!(untracked.len(), 1);
        } else {
            panic!("Expected Status output");
        }
    }

    #[tokio::test]
    async fn test_git_log() {
        let temp_dir = setup_git_repo();

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::Log {
                count: 5,
                from_ref: None,
            },
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext {
            cwd: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        assert!(result.is_some());
        if let Some(GitOutput::Log { commits, count }) = result {
            assert_eq!(*count, 1);
            assert_eq!(commits.len(), 1);
            assert_eq!(commits[0].message, "Initial commit");
        } else {
            panic!("Expected Log output");
        }
    }

    #[tokio::test]
    async fn test_git_branches() {
        let temp_dir = setup_git_repo();

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::Branches {
                include_remote: false,
            },
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext {
            cwd: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        assert!(result.is_some());
        if let Some(GitOutput::Branches { branches, current }) = result {
            assert!(!branches.is_empty());
            // Modern git uses "main" or "master" as default
            assert!(current.is_some());
        } else {
            panic!("Expected Branches output");
        }
    }

    #[tokio::test]
    async fn test_git_current_branch() {
        let temp_dir = setup_git_repo();

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::CurrentBranch,
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext {
            cwd: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        assert!(result.is_some());
        if let Some(GitOutput::CurrentBranch { name, head_commit }) = result {
            assert!(name.is_some());
            assert!(!head_commit.is_empty());
        } else {
            panic!("Expected CurrentBranch output");
        }
    }

    #[tokio::test]
    async fn test_git_commit_info() {
        let temp_dir = setup_git_repo();

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::CommitInfo {
                commit_ref: "HEAD".to_string(),
            },
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext {
            cwd: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        assert!(result.is_some());
        if let Some(GitOutput::CommitInfo(commit)) = result {
            // Commit message may include trailing newline
            assert!(commit.message.starts_with("Initial commit"));
            assert_eq!(commit.author, "Test User");
            assert_eq!(commit.email, "test@example.com");
        } else {
            panic!("Expected CommitInfo output");
        }
    }

    #[tokio::test]
    async fn test_git_diff() {
        let temp_dir = setup_git_repo();

        // Modify the existing file
        std::fs::write(temp_dir.path().join("README.md"), "# Modified\nNew line\n").unwrap();

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::Diff {
                staged: false,
                path: None,
            },
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext {
            cwd: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        assert!(result.is_some());
        if let Some(GitOutput::Diff {
            diff,
            files_changed,
            insertions,
            deletions,
        }) = result
        {
            assert_eq!(*files_changed, 1);
            assert!(*insertions > 0);
            assert!(*deletions > 0);
            assert!(!diff.is_empty());
        } else {
            panic!("Expected Diff output");
        }
    }

    #[tokio::test]
    async fn test_git_not_a_repo() {
        let temp_dir = TempDir::new().unwrap();
        // Don't initialize git repo

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::Status,
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext {
            cwd: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
    }
}
