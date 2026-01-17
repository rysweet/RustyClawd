//! GitHub Tool - GitHub API operations for issues and pull requests
//!
//! This tool provides GitHub API operations including:
//! - Create/list issues
//! - Create/list pull requests
//! - Get PR comments
//!
//! # Philosophy
//! - One responsibility: GitHub API operations
//! - Uses GITHUB_TOKEN env var for authentication
//! - Direct reqwest HTTP client (no heavy SDK dependencies)

use crate::{ToolContext, ToolError, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// GitHub API client for making authenticated requests
struct GitHubClient {
    client: reqwest::Client,
    token: String,
}

impl GitHubClient {
    /// Create a new GitHub client from GITHUB_TOKEN env var
    fn from_env() -> ToolResult<Self> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            ToolError::Validation(
                "GITHUB_TOKEN environment variable not set. Please set it to authenticate with GitHub API.".to_string(),
            )
        })?;

        let client = reqwest::Client::builder()
            .user_agent("RustyClawd-GitHubTool/1.0")
            .build()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, token })
    }

    /// Make a GET request to GitHub API
    async fn get(&self, url: &str) -> ToolResult<serde_json::Value> {
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionFailed(format!(
                "GitHub API error {}: {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse JSON response: {}", e)))
    }

    /// Make a POST request to GitHub API
    async fn post(&self, url: &str, body: serde_json::Value) -> ToolResult<serde_json::Value> {
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ToolError::ExecutionFailed(format!(
                "GitHub API error {}: {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse JSON response: {}", e)))
    }
}

/// GitHub API operation to perform
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum GitHubOperation {
    /// Create a new issue
    CreateIssue {
        /// Repository in owner/repo format
        repo: String,
        /// Issue title
        title: String,
        /// Issue body (markdown)
        #[serde(default)]
        body: Option<String>,
        /// Labels to add
        #[serde(default)]
        labels: Vec<String>,
    },
    /// List issues in a repository
    ListIssues {
        /// Repository in owner/repo format
        repo: String,
        /// Filter by state: open, closed, all
        #[serde(default = "default_state")]
        state: String,
        /// Maximum number of issues to return
        #[serde(default = "default_per_page")]
        per_page: u32,
    },
    /// Create a new pull request
    CreatePr {
        /// Repository in owner/repo format
        repo: String,
        /// PR title
        title: String,
        /// Head branch (the branch with changes)
        head: String,
        /// Base branch (the branch to merge into)
        base: String,
        /// PR body (markdown)
        #[serde(default)]
        body: Option<String>,
        /// Whether the PR is a draft
        #[serde(default)]
        draft: bool,
    },
    /// List pull requests in a repository
    ListPrs {
        /// Repository in owner/repo format
        repo: String,
        /// Filter by state: open, closed, all
        #[serde(default = "default_state")]
        state: String,
        /// Maximum number of PRs to return
        #[serde(default = "default_per_page")]
        per_page: u32,
    },
    /// Get comments on a pull request
    GetPrComments {
        /// Repository in owner/repo format
        repo: String,
        /// Pull request number
        pr_number: u64,
    },
    /// Get a specific issue
    GetIssue {
        /// Repository in owner/repo format
        repo: String,
        /// Issue number
        issue_number: u64,
    },
    /// Get a specific pull request
    GetPr {
        /// Repository in owner/repo format
        repo: String,
        /// Pull request number
        pr_number: u64,
    },
}

fn default_state() -> String {
    "open".to_string()
}

fn default_per_page() -> u32 {
    30
}

/// Parameters for the GitHub tool
#[derive(Debug, Deserialize)]
pub struct GitHubParams {
    /// The operation to perform
    #[serde(flatten)]
    pub operation: GitHubOperation,
}

/// A GitHub issue
#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub html_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<GitHubUser>,
}

/// A GitHub pull request
#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubPullRequest {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub html_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub head: GitHubBranch,
    pub base: GitHubBranch,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<GitHubUser>,
    #[serde(default)]
    pub draft: bool,
}

/// A GitHub branch reference
#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubBranch {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

/// A GitHub user
#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub html_url: String,
}

/// A GitHub comment
#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubComment {
    pub id: u64,
    pub body: String,
    pub created_at: String,
    pub html_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<GitHubUser>,
}

/// Output from the GitHub tool
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum GitHubOutput {
    /// Single issue
    Issue(GitHubIssue),
    /// List of issues
    Issues(Vec<GitHubIssue>),
    /// Single pull request
    PullRequest(GitHubPullRequest),
    /// List of pull requests
    PullRequests(Vec<GitHubPullRequest>),
    /// List of comments
    Comments(Vec<GitHubComment>),
}

/// The GitHub tool
pub struct GitHubTool;

impl GitHubTool {
    /// Execute create_issue operation
    async fn create_issue(
        client: &GitHubClient,
        repo: &str,
        title: &str,
        body: Option<&str>,
        labels: &[String],
    ) -> ToolResult<GitHubIssue> {
        let url = format!("https://api.github.com/repos/{}/issues", repo);

        let mut request_body = serde_json::json!({
            "title": title,
        });

        if let Some(b) = body {
            request_body["body"] = serde_json::json!(b);
        }

        if !labels.is_empty() {
            request_body["labels"] = serde_json::json!(labels);
        }

        let response = client.post(&url, request_body).await?;
        parse_issue(&response)
    }

    /// Execute list_issues operation
    async fn list_issues(
        client: &GitHubClient,
        repo: &str,
        state: &str,
        per_page: u32,
    ) -> ToolResult<Vec<GitHubIssue>> {
        let url = format!(
            "https://api.github.com/repos/{}/issues?state={}&per_page={}",
            repo, state, per_page
        );

        let response = client.get(&url).await?;
        let array = response.as_array().ok_or_else(|| {
            ToolError::ExecutionFailed("Expected array response from GitHub API".to_string())
        })?;

        array
            .iter()
            .filter(|item| item.get("pull_request").is_none()) // Filter out PRs
            .map(parse_issue)
            .collect()
    }

    /// Execute get_issue operation
    async fn get_issue(
        client: &GitHubClient,
        repo: &str,
        issue_number: u64,
    ) -> ToolResult<GitHubIssue> {
        let url = format!("https://api.github.com/repos/{}/issues/{}", repo, issue_number);
        let response = client.get(&url).await?;
        parse_issue(&response)
    }

    /// Execute create_pr operation
    async fn create_pr(
        client: &GitHubClient,
        repo: &str,
        title: &str,
        head: &str,
        base: &str,
        body: Option<&str>,
        draft: bool,
    ) -> ToolResult<GitHubPullRequest> {
        let url = format!("https://api.github.com/repos/{}/pulls", repo);

        let mut request_body = serde_json::json!({
            "title": title,
            "head": head,
            "base": base,
            "draft": draft,
        });

        if let Some(b) = body {
            request_body["body"] = serde_json::json!(b);
        }

        let response = client.post(&url, request_body).await?;
        parse_pr(&response)
    }

    /// Execute list_prs operation
    async fn list_prs(
        client: &GitHubClient,
        repo: &str,
        state: &str,
        per_page: u32,
    ) -> ToolResult<Vec<GitHubPullRequest>> {
        let url = format!(
            "https://api.github.com/repos/{}/pulls?state={}&per_page={}",
            repo, state, per_page
        );

        let response = client.get(&url).await?;
        let array = response.as_array().ok_or_else(|| {
            ToolError::ExecutionFailed("Expected array response from GitHub API".to_string())
        })?;

        array.iter().map(parse_pr).collect()
    }

    /// Execute get_pr operation
    async fn get_pr(
        client: &GitHubClient,
        repo: &str,
        pr_number: u64,
    ) -> ToolResult<GitHubPullRequest> {
        let url = format!("https://api.github.com/repos/{}/pulls/{}", repo, pr_number);
        let response = client.get(&url).await?;
        parse_pr(&response)
    }

    /// Execute get_pr_comments operation
    async fn get_pr_comments(
        client: &GitHubClient,
        repo: &str,
        pr_number: u64,
    ) -> ToolResult<Vec<GitHubComment>> {
        // Get both review comments and issue comments for the PR
        let review_url = format!(
            "https://api.github.com/repos/{}/pulls/{}/comments",
            repo, pr_number
        );
        let issue_url = format!(
            "https://api.github.com/repos/{}/issues/{}/comments",
            repo, pr_number
        );

        let review_response = client.get(&review_url).await?;
        let issue_response = client.get(&issue_url).await?;

        let mut comments = Vec::new();

        // Parse review comments
        if let Some(array) = review_response.as_array() {
            for item in array {
                if let Ok(comment) = parse_comment(item) {
                    comments.push(comment);
                }
            }
        }

        // Parse issue comments
        if let Some(array) = issue_response.as_array() {
            for item in array {
                if let Ok(comment) = parse_comment(item) {
                    comments.push(comment);
                }
            }
        }

        // Sort by created_at
        comments.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        Ok(comments)
    }
}

/// Parse a GitHub issue from JSON
fn parse_issue(value: &serde_json::Value) -> ToolResult<GitHubIssue> {
    Ok(GitHubIssue {
        number: value["number"].as_u64().unwrap_or(0),
        title: value["title"].as_str().unwrap_or("").to_string(),
        state: value["state"].as_str().unwrap_or("").to_string(),
        html_url: value["html_url"].as_str().unwrap_or("").to_string(),
        body: value["body"].as_str().map(|s| s.to_string()),
        created_at: value["created_at"].as_str().unwrap_or("").to_string(),
        user: parse_user(&value["user"]),
    })
}

/// Parse a GitHub pull request from JSON
fn parse_pr(value: &serde_json::Value) -> ToolResult<GitHubPullRequest> {
    Ok(GitHubPullRequest {
        number: value["number"].as_u64().unwrap_or(0),
        title: value["title"].as_str().unwrap_or("").to_string(),
        state: value["state"].as_str().unwrap_or("").to_string(),
        html_url: value["html_url"].as_str().unwrap_or("").to_string(),
        body: value["body"].as_str().map(|s| s.to_string()),
        head: GitHubBranch {
            ref_name: value["head"]["ref"].as_str().unwrap_or("").to_string(),
            sha: value["head"]["sha"].as_str().unwrap_or("").to_string(),
        },
        base: GitHubBranch {
            ref_name: value["base"]["ref"].as_str().unwrap_or("").to_string(),
            sha: value["base"]["sha"].as_str().unwrap_or("").to_string(),
        },
        created_at: value["created_at"].as_str().unwrap_or("").to_string(),
        user: parse_user(&value["user"]),
        draft: value["draft"].as_bool().unwrap_or(false),
    })
}

/// Parse a GitHub comment from JSON
fn parse_comment(value: &serde_json::Value) -> ToolResult<GitHubComment> {
    Ok(GitHubComment {
        id: value["id"].as_u64().unwrap_or(0),
        body: value["body"].as_str().unwrap_or("").to_string(),
        created_at: value["created_at"].as_str().unwrap_or("").to_string(),
        html_url: value["html_url"].as_str().unwrap_or("").to_string(),
        user: parse_user(&value["user"]),
    })
}

/// Parse a GitHub user from JSON
fn parse_user(value: &serde_json::Value) -> Option<GitHubUser> {
    if value.is_null() {
        return None;
    }

    Some(GitHubUser {
        login: value["login"].as_str().unwrap_or("").to_string(),
        html_url: value["html_url"].as_str().unwrap_or("").to_string(),
    })
}

#[async_trait]
impl crate::Tool for GitHubTool {
    type Params = GitHubParams;
    type Output = GitHubOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "GitHub",
            description: "Performs GitHub API operations: create/list issues, create/list PRs, get PR comments",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let operation = params.operation;
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            // Progress: Starting operation
            let op_name = match &operation {
                GitHubOperation::CreateIssue { repo, .. } => format!("Creating issue in {}", repo),
                GitHubOperation::ListIssues { repo, .. } => format!("Listing issues in {}", repo),
                GitHubOperation::CreatePr { repo, .. } => format!("Creating PR in {}", repo),
                GitHubOperation::ListPrs { repo, .. } => format!("Listing PRs in {}", repo),
                GitHubOperation::GetPrComments { repo, pr_number } => format!("Getting comments for PR #{} in {}", pr_number, repo),
                GitHubOperation::GetIssue { repo, issue_number } => format!("Getting issue #{} in {}", issue_number, repo),
                GitHubOperation::GetPr { repo, pr_number } => format!("Getting PR #{} in {}", pr_number, repo),
            };

            yield ToolEvent::Progress {
                step: op_name.clone(),
                percentage: None,
            };

            if debug {
                tracing::debug!(operation = ?operation, "Executing GitHub operation");
            }

            // Create client
            let client = match GitHubClient::from_env() {
                Ok(c) => c,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("{}", e),
                    };
                    return;
                }
            };

            // Execute operation
            let result = match operation {
                GitHubOperation::CreateIssue { repo, title, body, labels } => {
                    match GitHubTool::create_issue(&client, &repo, &title, body.as_deref(), &labels).await {
                        Ok(issue) => Ok(GitHubOutput::Issue(issue)),
                        Err(e) => Err(e),
                    }
                }
                GitHubOperation::ListIssues { repo, state, per_page } => {
                    match GitHubTool::list_issues(&client, &repo, &state, per_page).await {
                        Ok(issues) => Ok(GitHubOutput::Issues(issues)),
                        Err(e) => Err(e),
                    }
                }
                GitHubOperation::GetIssue { repo, issue_number } => {
                    match GitHubTool::get_issue(&client, &repo, issue_number).await {
                        Ok(issue) => Ok(GitHubOutput::Issue(issue)),
                        Err(e) => Err(e),
                    }
                }
                GitHubOperation::CreatePr { repo, title, head, base, body, draft } => {
                    match GitHubTool::create_pr(&client, &repo, &title, &head, &base, body.as_deref(), draft).await {
                        Ok(pr) => Ok(GitHubOutput::PullRequest(pr)),
                        Err(e) => Err(e),
                    }
                }
                GitHubOperation::ListPrs { repo, state, per_page } => {
                    match GitHubTool::list_prs(&client, &repo, &state, per_page).await {
                        Ok(prs) => Ok(GitHubOutput::PullRequests(prs)),
                        Err(e) => Err(e),
                    }
                }
                GitHubOperation::GetPr { repo, pr_number } => {
                    match GitHubTool::get_pr(&client, &repo, pr_number).await {
                        Ok(pr) => Ok(GitHubOutput::PullRequest(pr)),
                        Err(e) => Err(e),
                    }
                }
                GitHubOperation::GetPrComments { repo, pr_number } => {
                    match GitHubTool::get_pr_comments(&client, &repo, pr_number).await {
                        Ok(comments) => Ok(GitHubOutput::Comments(comments)),
                        Err(e) => Err(e),
                    }
                }
            };

            match result {
                Ok(output) => {
                    if debug {
                        tracing::debug!("GitHub operation completed successfully");
                    }
                    yield ToolEvent::Result(output);
                }
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("{}", e),
                    };
                }
            }
        }))
    }

    fn is_read_only(&self) -> bool {
        false // Can create issues and PRs
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Each API call is independent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;

    #[test]
    fn test_operation_deserialize_create_issue() {
        let json = r#"{
            "operation": "create_issue",
            "repo": "owner/repo",
            "title": "Test Issue",
            "body": "Test body",
            "labels": ["bug", "urgent"]
        }"#;

        let params: GitHubParams = serde_json::from_str(json).unwrap();
        match params.operation {
            GitHubOperation::CreateIssue { repo, title, body, labels } => {
                assert_eq!(repo, "owner/repo");
                assert_eq!(title, "Test Issue");
                assert_eq!(body, Some("Test body".to_string()));
                assert_eq!(labels, vec!["bug", "urgent"]);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_operation_deserialize_list_issues() {
        let json = r#"{
            "operation": "list_issues",
            "repo": "owner/repo",
            "state": "closed",
            "per_page": 10
        }"#;

        let params: GitHubParams = serde_json::from_str(json).unwrap();
        match params.operation {
            GitHubOperation::ListIssues { repo, state, per_page } => {
                assert_eq!(repo, "owner/repo");
                assert_eq!(state, "closed");
                assert_eq!(per_page, 10);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_operation_deserialize_create_pr() {
        let json = r#"{
            "operation": "create_pr",
            "repo": "owner/repo",
            "title": "Test PR",
            "head": "feature-branch",
            "base": "main",
            "draft": true
        }"#;

        let params: GitHubParams = serde_json::from_str(json).unwrap();
        match params.operation {
            GitHubOperation::CreatePr { repo, title, head, base, body, draft } => {
                assert_eq!(repo, "owner/repo");
                assert_eq!(title, "Test PR");
                assert_eq!(head, "feature-branch");
                assert_eq!(base, "main");
                assert!(body.is_none());
                assert!(draft);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_operation_deserialize_get_pr_comments() {
        let json = r#"{
            "operation": "get_pr_comments",
            "repo": "owner/repo",
            "pr_number": 123
        }"#;

        let params: GitHubParams = serde_json::from_str(json).unwrap();
        match params.operation {
            GitHubOperation::GetPrComments { repo, pr_number } => {
                assert_eq!(repo, "owner/repo");
                assert_eq!(pr_number, 123);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[test]
    fn test_parse_issue() {
        let json = serde_json::json!({
            "number": 42,
            "title": "Test Issue",
            "state": "open",
            "html_url": "https://github.com/owner/repo/issues/42",
            "body": "Issue body",
            "created_at": "2024-01-01T00:00:00Z",
            "user": {
                "login": "testuser",
                "html_url": "https://github.com/testuser"
            }
        });

        let issue = parse_issue(&json).unwrap();
        assert_eq!(issue.number, 42);
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.state, "open");
        assert!(issue.user.is_some());
        assert_eq!(issue.user.unwrap().login, "testuser");
    }

    #[test]
    fn test_parse_pr() {
        let json = serde_json::json!({
            "number": 100,
            "title": "Test PR",
            "state": "open",
            "html_url": "https://github.com/owner/repo/pull/100",
            "body": "PR body",
            "head": {
                "ref": "feature-branch",
                "sha": "abc123"
            },
            "base": {
                "ref": "main",
                "sha": "def456"
            },
            "created_at": "2024-01-01T00:00:00Z",
            "user": {
                "login": "testuser",
                "html_url": "https://github.com/testuser"
            },
            "draft": false
        });

        let pr = parse_pr(&json).unwrap();
        assert_eq!(pr.number, 100);
        assert_eq!(pr.title, "Test PR");
        assert_eq!(pr.head.ref_name, "feature-branch");
        assert_eq!(pr.base.ref_name, "main");
        assert!(!pr.draft);
    }

    #[test]
    fn test_metadata() {
        let tool = GitHubTool;
        let metadata = tool.metadata();
        assert_eq!(metadata.name, "GitHub");
        assert!(metadata.description.contains("GitHub API"));
    }

    #[test]
    fn test_default_values() {
        // Test list_issues with defaults
        let json = r#"{
            "operation": "list_issues",
            "repo": "owner/repo"
        }"#;

        let params: GitHubParams = serde_json::from_str(json).unwrap();
        match params.operation {
            GitHubOperation::ListIssues { state, per_page, .. } => {
                assert_eq!(state, "open");
                assert_eq!(per_page, 30);
            }
            _ => panic!("Wrong operation type"),
        }
    }
}
