//! Git tool - Native git operations using libgit2
//!
//! This tool provides native git operations without shelling out to git CLI.
//! It demonstrates:
//! - Using libgit2 (git2 crate) for git operations
//! - Status, diff, log, branch, add, commit operations
//! - Streaming results with progress updates

use crate::{ToolContext, ToolError, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use git2::{DiffOptions, Repository, Signature, StatusOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Git operation to perform
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GitOperation {
    /// Get repository status
    Status,
    /// Get diff of changes
    Diff {
        /// Show staged changes only
        #[serde(default)]
        staged: bool,
    },
    /// Get commit log
    Log {
        /// Maximum number of commits to show
        #[serde(default = "default_log_count")]
        count: usize,
    },
    /// List or create branches
    Branch {
        /// Name of new branch to create (if provided)
        #[serde(default)]
        create: Option<String>,
        /// List all branches (local and remote)
        #[serde(default)]
        all: bool,
    },
    /// Stage files for commit
    Add {
        /// Files to add (glob patterns supported)
        files: Vec<String>,
    },
    /// Create a commit
    Commit {
        /// Commit message
        message: String,
    },
}

fn default_log_count() -> usize {
    10
}

/// Parameters for the Git tool
#[derive(Debug, Deserialize)]
pub struct GitParams {
    /// The git operation to perform
    pub operation: GitOperation,

    /// Optional path to repository (defaults to cwd)
    #[serde(default)]
    pub repo_path: Option<String>,
}

/// Status entry for a file
#[derive(Debug, Serialize, Clone)]
pub struct StatusEntry {
    /// File path
    pub path: String,
    /// Status (modified, added, deleted, renamed, etc.)
    pub status: String,
    /// Whether the change is staged
    pub staged: bool,
}

/// Commit info
#[derive(Debug, Serialize, Clone)]
pub struct CommitInfo {
    /// Commit hash (short)
    pub id: String,
    /// Commit message (first line)
    pub message: String,
    /// Author name
    pub author: String,
    /// Commit timestamp (ISO 8601)
    pub timestamp: String,
}

/// Branch info
#[derive(Debug, Serialize, Clone)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Whether this is the current branch
    pub is_current: bool,
    /// Whether this is a remote branch
    pub is_remote: bool,
}

/// Output from Git tool
#[derive(Debug, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum GitOutput {
    /// Status output
    Status {
        /// Changed files
        files: Vec<StatusEntry>,
        /// Current branch name
        branch: Option<String>,
        /// Whether repo is clean
        clean: bool,
    },
    /// Diff output
    Diff {
        /// Diff text
        diff: String,
        /// Number of files changed
        files_changed: usize,
        /// Lines added
        insertions: usize,
        /// Lines deleted
        deletions: usize,
    },
    /// Log output
    Log {
        /// Commits
        commits: Vec<CommitInfo>,
    },
    /// Branch output
    Branch {
        /// Branches list
        branches: Vec<BranchInfo>,
        /// Current branch
        current: Option<String>,
        /// Newly created branch (if create was used)
        created: Option<String>,
    },
    /// Add output
    Add {
        /// Files staged
        staged: Vec<String>,
    },
    /// Commit output
    Commit {
        /// New commit hash
        id: String,
        /// Commit message
        message: String,
    },
}

/// The Git tool
pub struct GitTool;

impl GitTool {
    /// Open repository at given path or discover from current directory
    fn open_repo(path: &Path) -> Result<Repository, ToolError> {
        Repository::discover(path).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to open git repository: {}", e))
        })
    }

    /// Convert git2 status flags to human-readable status
    fn status_to_string(status: git2::Status) -> &'static str {
        if status.is_index_new() || status.is_wt_new() {
            "added"
        } else if status.is_index_modified() || status.is_wt_modified() {
            "modified"
        } else if status.is_index_deleted() || status.is_wt_deleted() {
            "deleted"
        } else if status.is_index_renamed() || status.is_wt_renamed() {
            "renamed"
        } else if status.is_index_typechange() || status.is_wt_typechange() {
            "typechange"
        } else if status.is_conflicted() {
            "conflict"
        } else {
            "unknown"
        }
    }

    /// Check if status indicates staged change
    fn is_staged(status: git2::Status) -> bool {
        status.is_index_new()
            || status.is_index_modified()
            || status.is_index_deleted()
            || status.is_index_renamed()
            || status.is_index_typechange()
    }

    /// Get repository status
    fn get_status(repo: &Repository) -> Result<GitOutput, ToolError> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);

        let statuses = repo.statuses(Some(&mut opts)).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to get status: {}", e))
        })?;

        let mut files = Vec::new();
        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let status = entry.status();
                files.push(StatusEntry {
                    path: path.to_string(),
                    status: Self::status_to_string(status).to_string(),
                    staged: Self::is_staged(status),
                });
            }
        }

        let branch = repo.head().ok().and_then(|head| {
            head.shorthand().map(|s| s.to_string())
        });

        let clean = files.is_empty();

        Ok(GitOutput::Status {
            files,
            branch,
            clean,
        })
    }

    /// Get diff of changes
    fn get_diff(repo: &Repository, staged: bool) -> Result<GitOutput, ToolError> {
        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(true);

        let diff = if staged {
            // Diff between HEAD and index (staged changes)
            let head_tree = repo
                .head()
                .ok()
                .and_then(|h| h.peel_to_tree().ok());

            repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut diff_opts))
        } else {
            // Diff between index and workdir (unstaged changes)
            repo.diff_index_to_workdir(None, Some(&mut diff_opts))
        };

        let diff = diff.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to get diff: {}", e))
        })?;

        let stats = diff.stats().map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to get diff stats: {}", e))
        })?;

        // Format diff as string
        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let origin = line.origin();
            if origin == '+' || origin == '-' || origin == ' ' {
                diff_text.push(origin);
            }
            if let Ok(content) = std::str::from_utf8(line.content()) {
                diff_text.push_str(content);
            }
            true
        })
        .map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to format diff: {}", e))
        })?;

        Ok(GitOutput::Diff {
            diff: diff_text,
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
        })
    }

    /// Get commit log
    fn get_log(repo: &Repository, count: usize) -> Result<GitOutput, ToolError> {
        let mut revwalk = repo.revwalk().map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to create revwalk: {}", e))
        })?;

        revwalk.push_head().map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to push HEAD: {}", e))
        })?;

        let mut commits = Vec::new();
        for (idx, oid) in revwalk.enumerate() {
            if idx >= count {
                break;
            }

            let oid = oid.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to get commit oid: {}", e))
            })?;

            let commit = repo.find_commit(oid).map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to find commit: {}", e))
            })?;

            let author = commit.author();
            let time = commit.time();

            // Convert git time to ISO 8601
            let timestamp = chrono::DateTime::from_timestamp(time.seconds(), 0)
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                .unwrap_or_else(|| "unknown".to_string());

            commits.push(CommitInfo {
                id: oid.to_string()[..7].to_string(),
                message: commit
                    .message()
                    .unwrap_or("")
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string(),
                author: author.name().unwrap_or("unknown").to_string(),
                timestamp,
            });
        }

        Ok(GitOutput::Log { commits })
    }

    /// List or create branches
    fn handle_branch(
        repo: &Repository,
        create: Option<String>,
        all: bool,
    ) -> Result<GitOutput, ToolError> {
        let mut created = None;

        // Create new branch if requested
        if let Some(branch_name) = create {
            let head = repo.head().map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to get HEAD: {}", e))
            })?;

            let commit = head.peel_to_commit().map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to get HEAD commit: {}", e))
            })?;

            repo.branch(&branch_name, &commit, false).map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to create branch: {}", e))
            })?;

            created = Some(branch_name);
        }

        // List branches
        let mut branches_list = Vec::new();
        let current_branch = repo.head().ok().and_then(|h| h.shorthand().map(|s| s.to_string()));

        let filter = if all {
            None
        } else {
            Some(git2::BranchType::Local)
        };

        let branches = repo.branches(filter).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to list branches: {}", e))
        })?;

        for branch in branches {
            let (branch, branch_type) = branch.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to get branch: {}", e))
            })?;

            if let Some(name) = branch.name().ok().flatten() {
                let is_current = current_branch.as_deref() == Some(name);
                branches_list.push(BranchInfo {
                    name: name.to_string(),
                    is_current,
                    is_remote: branch_type == git2::BranchType::Remote,
                });
            }
        }

        Ok(GitOutput::Branch {
            branches: branches_list,
            current: current_branch,
            created,
        })
    }

    /// Stage files for commit
    fn add_files(repo: &Repository, files: &[String]) -> Result<GitOutput, ToolError> {
        let mut index = repo.index().map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to get index: {}", e))
        })?;

        let mut staged = Vec::new();

        for pattern in files {
            // Use glob pattern matching
            index
                .add_all([pattern].iter(), git2::IndexAddOption::DEFAULT, None)
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to add files: {}", e))
                })?;
            staged.push(pattern.clone());
        }

        index.write().map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to write index: {}", e))
        })?;

        Ok(GitOutput::Add { staged })
    }

    /// Create a commit
    fn create_commit(repo: &Repository, message: &str) -> Result<GitOutput, ToolError> {
        let mut index = repo.index().map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to get index: {}", e))
        })?;

        let tree_id = index.write_tree().map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to write tree: {}", e))
        })?;

        let tree = repo.find_tree(tree_id).map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to find tree: {}", e))
        })?;

        // Get signature from config or use defaults
        let signature = repo
            .signature()
            .or_else(|_| Signature::now("RustyClawd", "rustyclawd@example.com"))
            .map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to create signature: {}", e))
            })?;

        // Get parent commit (HEAD)
        let parent = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok());

        let parents: Vec<&git2::Commit> = parent.iter().collect();

        let commit_id = repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &parents,
            )
            .map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to create commit: {}", e))
            })?;

        Ok(GitOutput::Commit {
            id: commit_id.to_string()[..7].to_string(),
            message: message.to_string(),
        })
    }
}

#[async_trait]
impl crate::Tool for GitTool {
    type Params = GitParams;
    type Output = GitOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "Git",
            description: "Native git operations (status, diff, log, branch, add, commit)",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let operation = params.operation.clone();
        let repo_path = params
            .repo_path
            .clone()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| ctx.cwd.clone());
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Executing git operation: {:?}", operation),
                percentage: None,
            };

            if debug {
                tracing::debug!(
                    operation = ?operation,
                    repo_path = ?repo_path,
                    "Executing git operation"
                );
            }

            // Open repository
            let repo = match Self::open_repo(&repo_path) {
                Ok(repo) => repo,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: e.to_string(),
                    };
                    return;
                }
            };

            // Execute operation
            let result = match operation {
                GitOperation::Status => Self::get_status(&repo),
                GitOperation::Diff { staged } => Self::get_diff(&repo, staged),
                GitOperation::Log { count } => Self::get_log(&repo, count),
                GitOperation::Branch { create, all } => Self::handle_branch(&repo, create, all),
                GitOperation::Add { files } => Self::add_files(&repo, &files),
                GitOperation::Commit { message } => Self::create_commit(&repo, &message),
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
        false // Git can modify repository state (add, commit, branch)
    }

    fn is_concurrency_safe(&self) -> bool {
        false // Git operations should be serialized to avoid conflicts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;
    use std::fs;
    use tempfile::TempDir;

    fn init_test_repo(temp_dir: &TempDir) -> Repository {
        let repo = Repository::init(temp_dir.path()).unwrap();

        // Configure user for commits
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        repo
    }

    #[tokio::test]
    async fn test_git_status_empty_repo() {
        let temp_dir = TempDir::new().unwrap();
        let _repo = init_test_repo(&temp_dir);

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::Status,
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        if let GitOutput::Status { files, clean, .. } = result {
            assert!(clean);
            assert!(files.is_empty());
        } else {
            panic!("Expected Status output");
        }
    }

    #[tokio::test]
    async fn test_git_status_with_changes() {
        let temp_dir = TempDir::new().unwrap();
        let _repo = init_test_repo(&temp_dir);

        // Create a new file
        fs::write(temp_dir.path().join("test.txt"), "hello world").unwrap();

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::Status,
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        if let GitOutput::Status { files, clean, .. } = result {
            assert!(!clean);
            assert!(!files.is_empty());
            assert!(files.iter().any(|f| f.path == "test.txt"));
        } else {
            panic!("Expected Status output");
        }
    }

    #[tokio::test]
    async fn test_git_add_and_commit() {
        let temp_dir = TempDir::new().unwrap();
        let _repo = init_test_repo(&temp_dir);

        // Create a new file
        fs::write(temp_dir.path().join("test.txt"), "hello world").unwrap();

        let tool = GitTool;
        let ctx = ToolContext::default();
        let repo_path = Some(temp_dir.path().to_str().unwrap().to_string());

        // Add file
        let add_params = GitParams {
            operation: GitOperation::Add {
                files: vec!["test.txt".to_string()],
            },
            repo_path: repo_path.clone(),
        };

        let stream = tool.execute(add_params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        if let GitOutput::Add { staged } = result {
            assert!(staged.contains(&"test.txt".to_string()));
        } else {
            panic!("Expected Add output");
        }

        // Commit
        let commit_params = GitParams {
            operation: GitOperation::Commit {
                message: "Initial commit".to_string(),
            },
            repo_path: repo_path.clone(),
        };

        let stream = tool.execute(commit_params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        if let GitOutput::Commit { message, id } = result {
            assert_eq!(message, "Initial commit");
            assert_eq!(id.len(), 7); // Short hash
        } else {
            panic!("Expected Commit output");
        }
    }

    #[tokio::test]
    async fn test_git_log() {
        let temp_dir = TempDir::new().unwrap();
        let repo = init_test_repo(&temp_dir);

        // Create initial commit
        fs::write(temp_dir.path().join("test.txt"), "hello").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::Log { count: 5 },
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        if let GitOutput::Log { commits } = result {
            assert_eq!(commits.len(), 1);
            assert_eq!(commits[0].message, "Initial commit");
        } else {
            panic!("Expected Log output");
        }
    }

    #[tokio::test]
    async fn test_git_branch() {
        let temp_dir = TempDir::new().unwrap();
        let repo = init_test_repo(&temp_dir);

        // Create initial commit (required for branching)
        fs::write(temp_dir.path().join("test.txt"), "hello").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let tool = GitTool;
        let ctx = ToolContext::default();
        let repo_path = Some(temp_dir.path().to_str().unwrap().to_string());

        // Create new branch
        let params = GitParams {
            operation: GitOperation::Branch {
                create: Some("feature-test".to_string()),
                all: false,
            },
            repo_path: repo_path.clone(),
        };

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        if let GitOutput::Branch {
            branches,
            created,
            current,
        } = result
        {
            assert!(created.as_ref().map(|c| c == "feature-test").unwrap_or(false));
            assert!(branches.iter().any(|b| b.name == "feature-test"));
            assert!(current.is_some());
        } else {
            panic!("Expected Branch output");
        }
    }

    #[tokio::test]
    async fn test_git_diff() {
        let temp_dir = TempDir::new().unwrap();
        let repo = init_test_repo(&temp_dir);

        // Create initial commit
        fs::write(temp_dir.path().join("test.txt"), "hello").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Modify file
        fs::write(temp_dir.path().join("test.txt"), "hello world").unwrap();

        let tool = GitTool;
        let params = GitParams {
            operation: GitOperation::Diff { staged: false },
            repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        if let GitOutput::Diff {
            diff,
            files_changed,
            insertions,
            deletions,
        } = result
        {
            assert!(!diff.is_empty());
            assert_eq!(*files_changed, 1);
            assert!(*insertions > 0 || *deletions > 0);
        } else {
            panic!("Expected Diff output");
        }
    }
}
