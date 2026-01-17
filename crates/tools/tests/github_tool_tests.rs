//! Integration tests for GitHubTool
//!
//! These tests verify the GitHubTool functionality.
//! Most tests run without a real GitHub token (testing serialization/parsing).
//! Live API tests are marked with #[ignore] and require GITHUB_TOKEN.

use rustyclawd_tools::{
    GitHubBranch, GitHubComment, GitHubIssue, GitHubOperation, GitHubOutput, GitHubParams,
    GitHubPullRequest, GitHubTool, Tool, ToolContext, ToolEvent,
};
use futures::StreamExt;

/// Test that GitHubTool has correct metadata
#[test]
fn test_github_tool_metadata() {
    let tool = GitHubTool;
    let metadata = tool.metadata();

    assert_eq!(metadata.name, "GitHub");
    assert!(metadata.description.contains("GitHub API"));
    assert!(metadata.description.contains("issues"));
    assert!(metadata.description.contains("PRs"));
}

/// Test that GitHubTool correctly reports read-only status
#[test]
fn test_github_tool_is_not_read_only() {
    let tool = GitHubTool;
    assert!(!tool.is_read_only());
}

/// Test that GitHubTool is concurrency safe
#[test]
fn test_github_tool_is_concurrency_safe() {
    let tool = GitHubTool;
    assert!(tool.is_concurrency_safe());
}

/// Test create_issue parameter deserialization
#[test]
fn test_create_issue_params() {
    let json = r#"{
        "operation": "create_issue",
        "repo": "microsoft/rustyclawd",
        "title": "Bug: Something is broken",
        "body": "Description - Something went wrong.",
        "labels": ["bug", "priority-high"]
    }"#;

    let params: GitHubParams = serde_json::from_str(json).unwrap();
    match params.operation {
        GitHubOperation::CreateIssue {
            repo,
            title,
            body,
            labels,
        } => {
            assert_eq!(repo, "microsoft/rustyclawd");
            assert_eq!(title, "Bug: Something is broken");
            assert_eq!(body.unwrap(), "Description - Something went wrong.");
            assert_eq!(labels, vec!["bug", "priority-high"]);
        }
        _ => panic!("Expected CreateIssue operation"),
    }
}

/// Test create_issue with minimal params (no body, no labels)
#[test]
fn test_create_issue_minimal_params() {
    let json = r#"{
        "operation": "create_issue",
        "repo": "owner/repo",
        "title": "Simple issue"
    }"#;

    let params: GitHubParams = serde_json::from_str(json).unwrap();
    match params.operation {
        GitHubOperation::CreateIssue {
            repo,
            title,
            body,
            labels,
        } => {
            assert_eq!(repo, "owner/repo");
            assert_eq!(title, "Simple issue");
            assert!(body.is_none());
            assert!(labels.is_empty());
        }
        _ => panic!("Expected CreateIssue operation"),
    }
}

/// Test list_issues parameter deserialization with defaults
#[test]
fn test_list_issues_defaults() {
    let json = r#"{
        "operation": "list_issues",
        "repo": "owner/repo"
    }"#;

    let params: GitHubParams = serde_json::from_str(json).unwrap();
    match params.operation {
        GitHubOperation::ListIssues {
            repo,
            state,
            per_page,
        } => {
            assert_eq!(repo, "owner/repo");
            assert_eq!(state, "open"); // default
            assert_eq!(per_page, 30); // default
        }
        _ => panic!("Expected ListIssues operation"),
    }
}

/// Test list_issues with custom values
#[test]
fn test_list_issues_custom_values() {
    let json = r#"{
        "operation": "list_issues",
        "repo": "owner/repo",
        "state": "closed",
        "per_page": 100
    }"#;

    let params: GitHubParams = serde_json::from_str(json).unwrap();
    match params.operation {
        GitHubOperation::ListIssues {
            state, per_page, ..
        } => {
            assert_eq!(state, "closed");
            assert_eq!(per_page, 100);
        }
        _ => panic!("Expected ListIssues operation"),
    }
}

/// Test create_pr parameter deserialization
#[test]
fn test_create_pr_params() {
    let json = r#"{
        "operation": "create_pr",
        "repo": "owner/repo",
        "title": "Add new feature",
        "head": "feature-branch",
        "base": "main",
        "body": "Changes: Added X and Fixed Y",
        "draft": true
    }"#;

    let params: GitHubParams = serde_json::from_str(json).unwrap();
    match params.operation {
        GitHubOperation::CreatePr {
            repo,
            title,
            head,
            base,
            body,
            draft,
        } => {
            assert_eq!(repo, "owner/repo");
            assert_eq!(title, "Add new feature");
            assert_eq!(head, "feature-branch");
            assert_eq!(base, "main");
            assert_eq!(body.unwrap(), "Changes: Added X and Fixed Y");
            assert!(draft);
        }
        _ => panic!("Expected CreatePr operation"),
    }
}

/// Test create_pr with minimal params (draft defaults to false)
#[test]
fn test_create_pr_minimal_params() {
    let json = r#"{
        "operation": "create_pr",
        "repo": "owner/repo",
        "title": "Quick fix",
        "head": "fix-branch",
        "base": "main"
    }"#;

    let params: GitHubParams = serde_json::from_str(json).unwrap();
    match params.operation {
        GitHubOperation::CreatePr { body, draft, .. } => {
            assert!(body.is_none());
            assert!(!draft); // default
        }
        _ => panic!("Expected CreatePr operation"),
    }
}

/// Test list_prs parameter deserialization
#[test]
fn test_list_prs_params() {
    let json = r#"{
        "operation": "list_prs",
        "repo": "owner/repo",
        "state": "all",
        "per_page": 50
    }"#;

    let params: GitHubParams = serde_json::from_str(json).unwrap();
    match params.operation {
        GitHubOperation::ListPrs {
            repo,
            state,
            per_page,
        } => {
            assert_eq!(repo, "owner/repo");
            assert_eq!(state, "all");
            assert_eq!(per_page, 50);
        }
        _ => panic!("Expected ListPrs operation"),
    }
}

/// Test get_pr_comments parameter deserialization
#[test]
fn test_get_pr_comments_params() {
    let json = r#"{
        "operation": "get_pr_comments",
        "repo": "owner/repo",
        "pr_number": 42
    }"#;

    let params: GitHubParams = serde_json::from_str(json).unwrap();
    match params.operation {
        GitHubOperation::GetPrComments { repo, pr_number } => {
            assert_eq!(repo, "owner/repo");
            assert_eq!(pr_number, 42);
        }
        _ => panic!("Expected GetPrComments operation"),
    }
}

/// Test get_issue parameter deserialization
#[test]
fn test_get_issue_params() {
    let json = r#"{
        "operation": "get_issue",
        "repo": "owner/repo",
        "issue_number": 123
    }"#;

    let params: GitHubParams = serde_json::from_str(json).unwrap();
    match params.operation {
        GitHubOperation::GetIssue { repo, issue_number } => {
            assert_eq!(repo, "owner/repo");
            assert_eq!(issue_number, 123);
        }
        _ => panic!("Expected GetIssue operation"),
    }
}

/// Test get_pr parameter deserialization
#[test]
fn test_get_pr_params() {
    let json = r#"{
        "operation": "get_pr",
        "repo": "owner/repo",
        "pr_number": 456
    }"#;

    let params: GitHubParams = serde_json::from_str(json).unwrap();
    match params.operation {
        GitHubOperation::GetPr { repo, pr_number } => {
            assert_eq!(repo, "owner/repo");
            assert_eq!(pr_number, 456);
        }
        _ => panic!("Expected GetPr operation"),
    }
}

/// Test issue output serialization
#[test]
fn test_issue_output_serialization() {
    let issue = GitHubIssue {
        number: 42,
        title: "Test Issue".to_string(),
        state: "open".to_string(),
        html_url: "https://github.com/owner/repo/issues/42".to_string(),
        body: Some("Issue body".to_string()),
        created_at: "2024-01-01T00:00:00Z".to_string(),
        user: None,
    };

    let output = GitHubOutput::Issue(issue);
    let json = serde_json::to_string(&output).unwrap();

    assert!(json.contains("42"));
    assert!(json.contains("Test Issue"));
    assert!(json.contains("open"));
}

/// Test pull request output serialization
#[test]
fn test_pr_output_serialization() {
    let pr = GitHubPullRequest {
        number: 100,
        title: "Test PR".to_string(),
        state: "open".to_string(),
        html_url: "https://github.com/owner/repo/pull/100".to_string(),
        body: None,
        head: GitHubBranch {
            ref_name: "feature".to_string(),
            sha: "abc123".to_string(),
        },
        base: GitHubBranch {
            ref_name: "main".to_string(),
            sha: "def456".to_string(),
        },
        created_at: "2024-01-01T00:00:00Z".to_string(),
        user: None,
        draft: false,
    };

    let output = GitHubOutput::PullRequest(pr);
    let json = serde_json::to_string(&output).unwrap();

    assert!(json.contains("100"));
    assert!(json.contains("Test PR"));
    assert!(json.contains("feature"));
    assert!(json.contains("main"));
}

/// Test comments output serialization
#[test]
fn test_comments_output_serialization() {
    let comments = vec![
        GitHubComment {
            id: 1,
            body: "First comment".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            html_url: "https://github.com/owner/repo/issues/1#issuecomment-1".to_string(),
            user: None,
        },
        GitHubComment {
            id: 2,
            body: "Second comment".to_string(),
            created_at: "2024-01-01T01:00:00Z".to_string(),
            html_url: "https://github.com/owner/repo/issues/1#issuecomment-2".to_string(),
            user: None,
        },
    ];

    let output = GitHubOutput::Comments(comments);
    let json = serde_json::to_string(&output).unwrap();

    assert!(json.contains("First comment"));
    assert!(json.contains("Second comment"));
}

/// Test that tool returns error when GITHUB_TOKEN is not set
#[tokio::test]
async fn test_github_tool_without_token() {
    // Temporarily unset GITHUB_TOKEN if it exists
    let original_token = std::env::var("GITHUB_TOKEN").ok();
    std::env::remove_var("GITHUB_TOKEN");

    let tool = GitHubTool;
    let params = GitHubParams {
        operation: GitHubOperation::ListIssues {
            repo: "owner/repo".to_string(),
            state: "open".to_string(),
            per_page: 10,
        },
    };
    let ctx = ToolContext::default();

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    // Should get progress then error
    let mut got_error = false;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Error { message } = event {
            assert!(message.contains("GITHUB_TOKEN"));
            got_error = true;
        }
    }
    assert!(got_error, "Expected error about missing GITHUB_TOKEN");

    // Restore original token if it existed
    if let Some(token) = original_token {
        std::env::set_var("GITHUB_TOKEN", token);
    }
}

/// Live test: List issues from a public repo (requires GITHUB_TOKEN)
/// Run with: cargo test test_live_list_issues -- --ignored
#[tokio::test]
#[ignore]
async fn test_live_list_issues() {
    let tool = GitHubTool;
    let params = GitHubParams {
        operation: GitHubOperation::ListIssues {
            repo: "rust-lang/rust".to_string(), // Large public repo
            state: "open".to_string(),
            per_page: 5,
        },
    };
    let ctx = ToolContext::default();

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    let mut got_result = false;
    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(GitHubOutput::Issues(issues)) => {
                assert!(!issues.is_empty(), "Should return at least one issue");
                for issue in &issues {
                    assert!(!issue.title.is_empty());
                    assert!(!issue.html_url.is_empty());
                }
                got_result = true;
            }
            ToolEvent::Error { message } => {
                panic!("Unexpected error: {}", message);
            }
            _ => {}
        }
    }
    assert!(got_result, "Expected issues result");
}

/// Live test: List PRs from a public repo (requires GITHUB_TOKEN)
/// Run with: cargo test test_live_list_prs -- --ignored
#[tokio::test]
#[ignore]
async fn test_live_list_prs() {
    let tool = GitHubTool;
    let params = GitHubParams {
        operation: GitHubOperation::ListPrs {
            repo: "rust-lang/rust".to_string(),
            state: "open".to_string(),
            per_page: 5,
        },
    };
    let ctx = ToolContext::default();

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    let mut got_result = false;
    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Result(GitHubOutput::PullRequests(prs)) => {
                assert!(!prs.is_empty(), "Should return at least one PR");
                for pr in &prs {
                    assert!(!pr.title.is_empty());
                    assert!(!pr.head.ref_name.is_empty());
                    assert!(!pr.base.ref_name.is_empty());
                }
                got_result = true;
            }
            ToolEvent::Error { message } => {
                panic!("Unexpected error: {}", message);
            }
            _ => {}
        }
    }
    assert!(got_result, "Expected PRs result");
}
