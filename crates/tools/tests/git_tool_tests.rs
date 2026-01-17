//! Integration tests for GitTool
//!
//! These tests verify the GitTool functionality using temporary git repositories.
//! All tests are self-contained and don't require any external git setup.

use futures::StreamExt;
use rustyclawd_tools::{
    git_tool::{GitOperation, GitOutput, GitParams, GitTool},
    Tool, ToolContext, ToolEvent,
};
use std::fs;
use tempfile::TempDir;

/// Helper to initialize a test git repository
fn init_test_repo(temp_dir: &TempDir) -> git2::Repository {
    let repo = git2::Repository::init(temp_dir.path()).unwrap();

    // Configure user for commits
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    repo
}

/// Helper to create an initial commit in the test repository
fn create_initial_commit(repo: &git2::Repository, temp_dir: &TempDir) {
    fs::write(temp_dir.path().join("README.md"), "# Test Repository").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();
}

/// Test that GitTool has correct metadata
#[test]
fn test_git_tool_metadata() {
    let tool = GitTool;
    let metadata = tool.metadata();

    assert_eq!(metadata.name, "Git");
    assert!(metadata.description.contains("status"));
    assert!(metadata.description.contains("diff"));
    assert!(metadata.description.contains("commit"));
}

/// Test that GitTool is NOT read-only (it can modify state)
#[test]
fn test_git_tool_is_not_read_only() {
    let tool = GitTool;
    assert!(!tool.is_read_only());
}

/// Test that GitTool is NOT concurrency safe (git operations should be serialized)
#[test]
fn test_git_tool_is_not_concurrency_safe() {
    let tool = GitTool;
    assert!(!tool.is_concurrency_safe());
}

/// Test status operation parameter deserialization
#[test]
fn test_status_params() {
    let json = r#"{
        "operation": "status"
    }"#;

    let params: GitParams = serde_json::from_str(json).unwrap();
    assert!(matches!(params.operation, GitOperation::Status));
}

/// Test diff operation parameter deserialization
#[test]
fn test_diff_params() {
    let json = r#"{
        "operation": { "diff": { "staged": true } }
    }"#;

    let params: GitParams = serde_json::from_str(json).unwrap();
    if let GitOperation::Diff { staged } = params.operation {
        assert!(staged);
    } else {
        panic!("Expected Diff operation");
    }
}

/// Test log operation parameter deserialization
#[test]
fn test_log_params() {
    let json = r#"{
        "operation": { "log": { "count": 5 } }
    }"#;

    let params: GitParams = serde_json::from_str(json).unwrap();
    if let GitOperation::Log { count } = params.operation {
        assert_eq!(count, 5);
    } else {
        panic!("Expected Log operation");
    }
}

/// Test log operation with default count
#[test]
fn test_log_params_default_count() {
    let json = r#"{
        "operation": { "log": {} }
    }"#;

    let params: GitParams = serde_json::from_str(json).unwrap();
    if let GitOperation::Log { count } = params.operation {
        assert_eq!(count, 10); // default
    } else {
        panic!("Expected Log operation");
    }
}

/// Test branch operation parameter deserialization
#[test]
fn test_branch_params() {
    let json = r#"{
        "operation": { "branch": { "create": "feature-x", "all": true } }
    }"#;

    let params: GitParams = serde_json::from_str(json).unwrap();
    if let GitOperation::Branch { create, all } = params.operation {
        assert_eq!(create, Some("feature-x".to_string()));
        assert!(all);
    } else {
        panic!("Expected Branch operation");
    }
}

/// Test add operation parameter deserialization
#[test]
fn test_add_params() {
    let json = r#"{
        "operation": { "add": { "files": ["*.rs", "Cargo.toml"] } }
    }"#;

    let params: GitParams = serde_json::from_str(json).unwrap();
    if let GitOperation::Add { files } = params.operation {
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"*.rs".to_string()));
        assert!(files.contains(&"Cargo.toml".to_string()));
    } else {
        panic!("Expected Add operation");
    }
}

/// Test commit operation parameter deserialization
#[test]
fn test_commit_params() {
    let json = r#"{
        "operation": { "commit": { "message": "Fix bug in parser" } }
    }"#;

    let params: GitParams = serde_json::from_str(json).unwrap();
    if let GitOperation::Commit { message } = params.operation {
        assert_eq!(message, "Fix bug in parser");
    } else {
        panic!("Expected Commit operation");
    }
}

/// Test repo_path parameter
#[test]
fn test_repo_path_param() {
    let json = r#"{
        "operation": "status",
        "repo_path": "/some/path"
    }"#;

    let params: GitParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.repo_path, Some("/some/path".to_string()));
}

/// Integration test: Status on clean repository
#[tokio::test]
async fn test_status_clean_repo() {
    let temp_dir = TempDir::new().unwrap();
    let repo = init_test_repo(&temp_dir);
    create_initial_commit(&repo, &temp_dir);

    let tool = GitTool;
    let params = GitParams {
        operation: GitOperation::Status,
        repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
    };
    let ctx = ToolContext::default();

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    let mut got_result = false;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Result(GitOutput::Status { clean, branch, .. }) = event {
            assert!(clean, "Repository should be clean");
            assert!(branch.is_some(), "Should have a branch");
            got_result = true;
        }
    }
    assert!(got_result, "Expected Status result");
}

/// Integration test: Status with modified files
#[tokio::test]
async fn test_status_with_modified_files() {
    let temp_dir = TempDir::new().unwrap();
    let repo = init_test_repo(&temp_dir);
    create_initial_commit(&repo, &temp_dir);

    // Modify a file
    fs::write(temp_dir.path().join("README.md"), "# Modified").unwrap();
    // Add a new file
    fs::write(temp_dir.path().join("new_file.txt"), "New content").unwrap();

    let tool = GitTool;
    let params = GitParams {
        operation: GitOperation::Status,
        repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
    };
    let ctx = ToolContext::default();

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    let mut got_result = false;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Result(GitOutput::Status { files, clean, .. }) = event {
            assert!(!clean, "Repository should not be clean");
            assert!(!files.is_empty(), "Should have changed files");
            // Check for README.md modification
            assert!(
                files.iter().any(|f| f.path == "README.md"),
                "README.md should be modified"
            );
            // Check for new_file.txt
            assert!(
                files.iter().any(|f| f.path == "new_file.txt"),
                "new_file.txt should be listed"
            );
            got_result = true;
        }
    }
    assert!(got_result, "Expected Status result");
}

/// Integration test: Complete add-commit workflow
#[tokio::test]
async fn test_full_add_commit_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let repo = init_test_repo(&temp_dir);
    create_initial_commit(&repo, &temp_dir);

    // Create a new file
    fs::write(temp_dir.path().join("feature.rs"), "fn new_feature() {}").unwrap();

    let tool = GitTool;
    let ctx = ToolContext::default();
    let repo_path = Some(temp_dir.path().to_str().unwrap().to_string());

    // Stage the file
    let add_params = GitParams {
        operation: GitOperation::Add {
            files: vec!["feature.rs".to_string()],
        },
        repo_path: repo_path.clone(),
    };

    let mut stream = tool.execute(add_params, &ctx).await.unwrap();
    let mut add_succeeded = false;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Result(GitOutput::Add { staged }) = event {
            assert!(staged.contains(&"feature.rs".to_string()));
            add_succeeded = true;
        }
    }
    assert!(add_succeeded, "Add should succeed");

    // Commit the file
    let commit_params = GitParams {
        operation: GitOperation::Commit {
            message: "Add new feature".to_string(),
        },
        repo_path: repo_path.clone(),
    };

    let mut stream = tool.execute(commit_params, &ctx).await.unwrap();
    let mut commit_succeeded = false;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Result(GitOutput::Commit { id, message }) = event {
            assert_eq!(message, "Add new feature");
            assert_eq!(id.len(), 7); // Short hash
            commit_succeeded = true;
        }
    }
    assert!(commit_succeeded, "Commit should succeed");

    // Verify with status
    let status_params = GitParams {
        operation: GitOperation::Status,
        repo_path: repo_path.clone(),
    };

    let mut stream = tool.execute(status_params, &ctx).await.unwrap();
    while let Some(event) = stream.next().await {
        if let ToolEvent::Result(GitOutput::Status { clean, .. }) = event {
            assert!(clean, "Repository should be clean after commit");
        }
    }
}

/// Integration test: Log with multiple commits
#[tokio::test]
async fn test_log_multiple_commits() {
    let temp_dir = TempDir::new().unwrap();
    let repo = init_test_repo(&temp_dir);

    // Create multiple commits
    for i in 1..=3 {
        let filename = format!("file{}.txt", i);
        fs::write(temp_dir.path().join(&filename), format!("Content {}", i)).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(&filename)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();

        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("Commit {}", i),
            &tree,
            &parents,
        )
        .unwrap();
    }

    let tool = GitTool;
    let params = GitParams {
        operation: GitOperation::Log { count: 5 },
        repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
    };
    let ctx = ToolContext::default();

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    let mut got_result = false;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Result(GitOutput::Log { commits }) = event {
            assert_eq!(commits.len(), 3, "Should have 3 commits");
            // Commits should be in reverse chronological order
            assert_eq!(commits[0].message, "Commit 3");
            assert_eq!(commits[1].message, "Commit 2");
            assert_eq!(commits[2].message, "Commit 1");
            got_result = true;
        }
    }
    assert!(got_result, "Expected Log result");
}

/// Integration test: Branch creation
#[tokio::test]
async fn test_branch_creation() {
    let temp_dir = TempDir::new().unwrap();
    let repo = init_test_repo(&temp_dir);
    create_initial_commit(&repo, &temp_dir);

    let tool = GitTool;
    let params = GitParams {
        operation: GitOperation::Branch {
            create: Some("feature/new-branch".to_string()),
            all: false,
        },
        repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
    };
    let ctx = ToolContext::default();

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    let mut got_result = false;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Result(GitOutput::Branch {
            branches,
            created,
            current,
        }) = event
        {
            assert_eq!(created, Some("feature/new-branch".to_string()));
            assert!(branches.iter().any(|b| b.name == "feature/new-branch"));
            assert!(current.is_some());
            got_result = true;
        }
    }
    assert!(got_result, "Expected Branch result");
}

/// Integration test: Diff of unstaged changes
#[tokio::test]
async fn test_diff_unstaged() {
    let temp_dir = TempDir::new().unwrap();
    let repo = init_test_repo(&temp_dir);
    create_initial_commit(&repo, &temp_dir);

    // Modify README.md
    fs::write(temp_dir.path().join("README.md"), "# Modified Header\n\nNew content here.").unwrap();

    let tool = GitTool;
    let params = GitParams {
        operation: GitOperation::Diff { staged: false },
        repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
    };
    let ctx = ToolContext::default();

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    let mut got_result = false;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Result(GitOutput::Diff {
            diff,
            files_changed,
            insertions,
            deletions,
        }) = event
        {
            assert!(!diff.is_empty(), "Diff should not be empty");
            assert_eq!(files_changed, 1, "One file should be changed");
            assert!(insertions > 0 || deletions > 0, "Should have some changes");
            got_result = true;
        }
    }
    assert!(got_result, "Expected Diff result");
}

/// Integration test: Diff of staged changes
#[tokio::test]
async fn test_diff_staged() {
    let temp_dir = TempDir::new().unwrap();
    let repo = init_test_repo(&temp_dir);
    create_initial_commit(&repo, &temp_dir);

    // Create and stage a new file
    fs::write(temp_dir.path().join("staged.txt"), "Staged content").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("staged.txt")).unwrap();
    index.write().unwrap();

    let tool = GitTool;
    let params = GitParams {
        operation: GitOperation::Diff { staged: true },
        repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
    };
    let ctx = ToolContext::default();

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    let mut got_result = false;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Result(GitOutput::Diff { files_changed, .. }) = event {
            assert_eq!(files_changed, 1, "One file should be staged");
            got_result = true;
        }
    }
    assert!(got_result, "Expected Diff result");
}

/// Test error handling: Operation on non-git directory
#[tokio::test]
async fn test_error_on_non_git_directory() {
    let temp_dir = TempDir::new().unwrap();
    // Don't initialize git repo

    let tool = GitTool;
    let params = GitParams {
        operation: GitOperation::Status,
        repo_path: Some(temp_dir.path().to_str().unwrap().to_string()),
    };
    let ctx = ToolContext::default();

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    let mut got_error = false;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Error { message } = event {
            assert!(
                message.contains("git repository") || message.contains("repository"),
                "Should mention git repository in error: {}",
                message
            );
            got_error = true;
        }
    }
    assert!(got_error, "Expected error for non-git directory");
}
