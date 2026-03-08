//! Git worktree isolation for agent execution
//!
//! When an agent has `isolation: worktree` in its frontmatter, it runs in
//! an isolated git worktree. This module handles creating and cleaning up
//! those worktrees.
//!
//! Flow:
//! 1. Create a branch `agent/{agent_id}` from current HEAD
//! 2. Create a worktree at `/tmp/rustyclawd-worktree-{agent_id}`
//! 3. Agent runs with CWD set to the worktree path
//! 4. After completion, check if the agent made changes
//! 5. If no changes: prune worktree and delete branch
//! 6. If changes exist: prune worktree but keep the branch

use git2::{Repository, WorktreeAddOptions, WorktreePruneOptions};
use std::path::{Path, PathBuf};

/// Information about a created worktree for agent isolation.
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Filesystem path to the worktree directory
    pub worktree_path: PathBuf,
    /// Name of the git branch created for this worktree
    pub branch_name: String,
    /// Internal worktree name used by git (for find_worktree/prune)
    pub worktree_name: String,
    /// Path to the original repository (for cleanup)
    pub repo_path: PathBuf,
}

/// Create an isolated git worktree for agent execution.
///
/// Creates a new branch from HEAD and checks it out into a temporary directory.
/// Returns `WorktreeInfo` with paths needed for cleanup.
pub fn create_worktree(cwd: &Path, agent_id: &str) -> Result<WorktreeInfo, String> {
    let repo = Repository::discover(cwd)
        .map_err(|e| format!("Failed to open git repository at {}: {}", cwd.display(), e))?;

    let repo_workdir = repo
        .workdir()
        .ok_or_else(|| "Repository has no working directory (bare repo)".to_string())?
        .to_path_buf();

    // Sanitize agent_id for branch/worktree names (replace non-alphanumeric except - and _)
    let safe_id: String = agent_id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let branch_name = format!("agent/{}", safe_id);
    let worktree_name = format!("rustyclawd-{}", safe_id);
    let worktree_path = rustyclawd_core::tmpdir::get().join(format!("rustyclawd-worktree-{}", safe_id));

    // Clean up stale worktree directory if it exists from a previous failed run
    if worktree_path.exists() {
        std::fs::remove_dir_all(&worktree_path).map_err(|e| {
            format!(
                "Failed to remove stale worktree directory {}: {}",
                worktree_path.display(),
                e
            )
        })?;
    }

    // Get HEAD commit for the new branch
    let head = repo
        .head()
        .map_err(|e| format!("Failed to get HEAD: {}", e))?;
    let head_commit = head
        .peel_to_commit()
        .map_err(|e| format!("Failed to peel HEAD to commit: {}", e))?;

    // Create branch from HEAD. If it already exists from a prior run, force-recreate it.
    let branch = match repo.branch(&branch_name, &head_commit, false) {
        Ok(b) => b,
        Err(e) if e.code() == git2::ErrorCode::Exists => {
            // Branch exists - force recreate pointing to current HEAD
            repo.branch(&branch_name, &head_commit, true)
                .map_err(|e| format!("Failed to recreate branch '{}': {}", branch_name, e))?
        }
        Err(e) => {
            return Err(format!("Failed to create branch '{}': {}", branch_name, e));
        }
    };

    // Create worktree with the branch as its HEAD
    let branch_ref = branch.into_reference();
    let mut opts = WorktreeAddOptions::new();
    opts.reference(Some(&branch_ref));

    repo.worktree(&worktree_name, &worktree_path, Some(&opts))
        .map_err(|e| {
            // Clean up: try to remove directory if worktree creation partially failed
            let _ = std::fs::remove_dir_all(&worktree_path);
            format!(
                "Failed to create worktree at {}: {}",
                worktree_path.display(),
                e
            )
        })?;

    tracing::info!(
        worktree_path = %worktree_path.display(),
        branch = %branch_name,
        "Created isolated worktree for agent"
    );

    Ok(WorktreeInfo {
        worktree_path,
        branch_name,
        worktree_name,
        repo_path: repo_workdir,
    })
}

/// Clean up a worktree after agent execution.
///
/// Removes the worktree from git's tracking and deletes the directory.
/// If the agent made commits on the branch (branch tip differs from where it started),
/// the branch is kept so the user can review changes. Otherwise the branch is deleted.
///
/// Returns `true` if the agent made changes (branch kept), `false` otherwise.
pub fn cleanup_worktree(info: &WorktreeInfo) -> Result<bool, String> {
    let repo = Repository::discover(&info.repo_path)
        .map_err(|e| format!("Failed to open repository for cleanup: {}", e))?;

    // Check if agent made commits by comparing branch tip to HEAD
    let has_changes = check_for_changes(&repo, &info.branch_name)?;

    // Prune the worktree (removes git's internal reference)
    // We need to prune with valid=true and working_tree=true to actually remove it
    match repo.find_worktree(&info.worktree_name) {
        Ok(wt) => {
            let mut prune_opts = WorktreePruneOptions::new();
            prune_opts.valid(true);
            prune_opts.working_tree(true);
            wt.prune(Some(&mut prune_opts))
                .map_err(|e| format!("Failed to prune worktree '{}': {}", info.worktree_name, e))?;
        }
        Err(e) => {
            tracing::warn!(
                worktree_name = %info.worktree_name,
                error = %e,
                "Worktree not found during cleanup (may have been manually removed)"
            );
        }
    }

    // Remove the worktree directory if it still exists
    if info.worktree_path.exists() {
        std::fs::remove_dir_all(&info.worktree_path).map_err(|e| {
            format!(
                "Failed to remove worktree directory {}: {}",
                info.worktree_path.display(),
                e
            )
        })?;
    }

    // If no changes were made, also delete the branch
    if !has_changes {
        delete_branch(&repo, &info.branch_name)?;
        tracing::info!(
            branch = %info.branch_name,
            "Cleaned up worktree and branch (no changes)"
        );
    } else {
        tracing::info!(
            branch = %info.branch_name,
            "Cleaned up worktree, kept branch with agent changes"
        );
    }

    Ok(has_changes)
}

/// Check if the agent branch has diverged from where it was created (HEAD at creation time).
///
/// We compare the branch tip to the merge-base with HEAD. If they're the same commit,
/// no new commits were made. If the branch tip is different, changes exist.
fn check_for_changes(repo: &Repository, branch_name: &str) -> Result<bool, String> {
    let branch_ref_name = format!("refs/heads/{}", branch_name);

    let branch_ref = match repo.find_reference(&branch_ref_name) {
        Ok(r) => r,
        Err(_) => return Ok(false), // Branch doesn't exist, no changes
    };

    let branch_oid = branch_ref
        .target()
        .ok_or_else(|| format!("Branch '{}' has no target", branch_name))?;

    let head_ref = repo
        .head()
        .map_err(|e| format!("Failed to get HEAD: {}", e))?;
    let head_oid = head_ref
        .target()
        .ok_or_else(|| "HEAD has no target".to_string())?;

    // If the branch tip equals HEAD, no new commits were added
    Ok(branch_oid != head_oid)
}

/// Delete a local branch by name.
fn delete_branch(repo: &Repository, branch_name: &str) -> Result<(), String> {
    match repo.find_branch(branch_name, git2::BranchType::Local) {
        Ok(mut branch) => {
            branch
                .delete()
                .map_err(|e| format!("Failed to delete branch '{}': {}", branch_name, e))?;
        }
        Err(e) => {
            tracing::warn!(
                branch = %branch_name,
                error = %e,
                "Branch not found during cleanup"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a minimal git repo with an initial commit so worktree operations work.
    fn init_test_repo() -> (TempDir, Repository) {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();

        // Need an initial commit for HEAD to exist
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            // Create a dummy file so we have something to commit
            let file_path = td.path().join("README.md");
            fs::write(&file_path, "# Test Repo").unwrap();
            index.add_path(Path::new("README.md")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        // Scope the tree borrow so repo can be moved after
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        (td, repo)
    }

    #[test]
    fn test_create_worktree_success() {
        let (td, _repo) = init_test_repo();

        let info = create_worktree(td.path(), "test_agent_123").unwrap();

        assert!(info.worktree_path.exists());
        assert_eq!(info.branch_name, "agent/test_agent_123");
        assert_eq!(info.worktree_name, "rustyclawd-test_agent_123");

        // The worktree should contain the same files as the original repo
        assert!(info.worktree_path.join("README.md").exists());

        // Cleanup
        let _ = cleanup_worktree(&info);
    }

    #[test]
    fn test_create_worktree_sanitizes_agent_id() {
        let (td, _repo) = init_test_repo();

        let info = create_worktree(td.path(), "agent.with/special:chars").unwrap();

        assert_eq!(info.branch_name, "agent/agent_with_special_chars");
        assert_eq!(info.worktree_name, "rustyclawd-agent_with_special_chars");
        assert!(info.worktree_path.exists());

        let _ = cleanup_worktree(&info);
    }

    #[test]
    fn test_cleanup_no_changes_removes_branch() {
        let (td, repo) = init_test_repo();

        let info = create_worktree(td.path(), "cleanup_test").unwrap();
        assert!(info.worktree_path.exists());

        let has_changes = cleanup_worktree(&info).unwrap();

        assert!(!has_changes);
        assert!(!info.worktree_path.exists());

        // Branch should be deleted
        assert!(repo
            .find_branch("agent/cleanup_test", git2::BranchType::Local)
            .is_err());
    }

    #[test]
    fn test_cleanup_with_changes_keeps_branch() {
        let (td, repo) = init_test_repo();

        let info = create_worktree(td.path(), "changes_test").unwrap();

        // Make a commit in the worktree
        {
            let wt_repo = Repository::open(&info.worktree_path).unwrap();
            let file_path = info.worktree_path.join("agent_output.txt");
            fs::write(&file_path, "Agent did work here").unwrap();

            let sig = git2::Signature::now("Agent", "agent@test.com").unwrap();
            let mut index = wt_repo.index().unwrap();
            index.add_path(Path::new("agent_output.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = wt_repo.find_tree(tree_id).unwrap();
            let parent = wt_repo.head().unwrap().peel_to_commit().unwrap();
            wt_repo
                .commit(Some("HEAD"), &sig, &sig, "Agent work", &tree, &[&parent])
                .unwrap();
        }

        let has_changes = cleanup_worktree(&info).unwrap();

        assert!(has_changes);
        assert!(!info.worktree_path.exists()); // Directory removed

        // Branch should still exist since there were changes
        assert!(repo
            .find_branch("agent/changes_test", git2::BranchType::Local)
            .is_ok());
    }

    #[test]
    fn test_create_worktree_not_a_repo() {
        let td = TempDir::new().unwrap();
        // td is NOT a git repo

        let result = create_worktree(td.path(), "test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Failed to open git repository"));
    }

    #[test]
    fn test_create_worktree_idempotent_stale_cleanup() {
        let (td, _repo) = init_test_repo();

        // First creation
        let info1 = create_worktree(td.path(), "idempotent_test").unwrap();
        assert!(info1.worktree_path.exists());

        // Clean up the worktree from git but leave the directory (simulating a crash)
        let repo = Repository::discover(td.path()).unwrap();
        if let Ok(wt) = repo.find_worktree(&info1.worktree_name) {
            let mut prune_opts = WorktreePruneOptions::new();
            prune_opts.valid(true).working_tree(true);
            wt.prune(Some(&mut prune_opts)).ok();
        }
        // Recreate the directory to simulate stale state
        fs::create_dir_all(&info1.worktree_path).unwrap();

        // Second creation should succeed (cleans up stale directory)
        let info2 = create_worktree(td.path(), "idempotent_test").unwrap();
        assert!(info2.worktree_path.exists());

        let _ = cleanup_worktree(&info2);
    }

    #[test]
    fn test_frontmatter_isolation_worktree() {
        use crate::agent::AgentFrontmatter;
        use crate::agent::AgentIsolation;

        let content = "---\nisolation: worktree\n---\n# Agent\nYou do things.";
        let (fm, prompt) = AgentFrontmatter::parse(content);
        assert_eq!(fm.isolation, Some(AgentIsolation::Worktree));
        assert!(prompt.contains("# Agent"));
    }

    #[test]
    fn test_frontmatter_isolation_unknown_value() {
        use crate::agent::AgentFrontmatter;

        let content = "---\nisolation: docker\n---\n# Agent";
        let (fm, _) = AgentFrontmatter::parse(content);
        assert!(fm.isolation.is_none());
    }

    #[test]
    fn test_frontmatter_isolation_combined_with_others() {
        use crate::agent::AgentFrontmatter;
        use crate::agent::AgentIsolation;
        use crate::agent_memory::MemoryScope;

        let content =
            "---\nbackground: true\nmemory: project\nisolation: worktree\n---\n# Isolated Agent";
        let (fm, prompt) = AgentFrontmatter::parse(content);
        assert!(fm.background);
        assert_eq!(fm.memory_scope, Some(MemoryScope::Project));
        assert_eq!(fm.isolation, Some(AgentIsolation::Worktree));
        assert!(prompt.contains("Isolated Agent"));
    }
}
