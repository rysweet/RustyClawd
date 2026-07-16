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

mod operations;
mod types;

pub use types::*;

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;

/// The Git tool
pub struct GitTool;

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
    use serial_test::serial;
    use std::ffi::OsString;
    use std::process::Command;
    use tempfile::TempDir;

    struct GitEnvGuard {
        saved: Vec<(&'static str, Option<OsString>)>,
    }

    impl GitEnvGuard {
        fn new() -> Self {
            let vars = [
                "GIT_DIR",
                "GIT_WORK_TREE",
                "GIT_INDEX_FILE",
                "GIT_OBJECT_DIRECTORY",
                "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            ];
            let saved = vars
                .into_iter()
                .map(|var| {
                    let value = std::env::var_os(var);
                    std::env::remove_var(var);
                    (var, value)
                })
                .collect();
            Self { saved }
        }
    }

    impl Drop for GitEnvGuard {
        fn drop(&mut self) {
            for (var, value) in &self.saved {
                if let Some(value) = value {
                    std::env::set_var(var, value);
                } else {
                    std::env::remove_var(var);
                }
            }
        }
    }

    fn setup_git_repo() -> (TempDir, GitEnvGuard) {
        let git_env = GitEnvGuard::new();
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

        (temp_dir, git_env)
    }

    #[tokio::test]
    #[serial]
    async fn test_git_status_clean() {
        let (temp_dir, _git_env) = setup_git_repo();

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
    #[serial]
    async fn test_git_status_with_changes() {
        let (temp_dir, _git_env) = setup_git_repo();

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
    #[serial]
    async fn test_git_log() {
        let (temp_dir, _git_env) = setup_git_repo();

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
    #[serial]
    async fn test_git_branches() {
        let (temp_dir, _git_env) = setup_git_repo();

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
    #[serial]
    async fn test_git_current_branch() {
        let (temp_dir, _git_env) = setup_git_repo();

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
    #[serial]
    async fn test_git_commit_info() {
        let (temp_dir, _git_env) = setup_git_repo();

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
    #[serial]
    async fn test_git_diff() {
        let (temp_dir, _git_env) = setup_git_repo();

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
    #[serial]
    async fn test_git_not_a_repo() {
        let _git_env = GitEnvGuard::new();
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
