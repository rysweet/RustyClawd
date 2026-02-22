//! Git tool operations - Core git logic using git2
//!
//! Implements repository operations: status, diff, log, branches,
//! commit info, and current branch detection.

use super::types::*;
use super::GitTool;
use crate::ToolError;
use git2::{DiffOptions, Repository, StatusOptions};
use std::path::Path;

impl GitTool {
    pub(crate) fn open_repository(
        repo_path: Option<&str>,
        cwd: &Path,
    ) -> Result<Repository, ToolError> {
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

    pub(crate) fn get_status(repo: &Repository) -> Result<GitOutput, ToolError> {
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

    pub(crate) fn get_diff(
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

    pub(crate) fn get_log(
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

            let oid = oid_result.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to get commit oid: {}", e))
            })?;

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

    pub(crate) fn get_branches(
        repo: &Repository,
        include_remote: bool,
    ) -> Result<GitOutput, ToolError> {
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

    pub(crate) fn get_commit_info(
        repo: &Repository,
        commit_ref: &str,
    ) -> Result<GitOutput, ToolError> {
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

    pub(crate) fn get_current_branch(repo: &Repository) -> Result<GitOutput, ToolError> {
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
