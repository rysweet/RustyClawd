//! GitHub tool - Native GitHub API operations
//!
//! Provides GitHub API operations as a native tool:
//! - List issues and pull requests
//! - Create issues
//! - Get PR information
//! - Repository information
//!
//! Authentication is handled via GITHUB_TOKEN environment variable.

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};

/// Supported GitHub operations
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GitHubOperation {
    /// List issues in a repository
    ListIssues,
    /// Create a new issue
    CreateIssue,
    /// Get a specific issue by number
    GetIssue,
    /// List pull requests in a repository
    ListPrs,
    /// Get a specific pull request by number
    GetPr,
    /// Get repository information
    GetRepo,
}

/// Parameters for GitHub tool
#[derive(Debug, Deserialize)]
pub struct GitHubParams {
    /// Operation to perform
    pub operation: GitHubOperation,

    /// Repository owner (required for all operations)
    pub owner: String,

    /// Repository name (required for all operations)
    pub repo: String,

    /// Issue or PR number (required for get_issue, get_pr)
    #[serde(default)]
    pub number: Option<u64>,

    /// Issue title (required for create_issue)
    #[serde(default)]
    pub title: Option<String>,

    /// Issue body (optional for create_issue)
    #[serde(default)]
    pub body: Option<String>,

    /// Labels for issue (optional for create_issue)
    #[serde(default)]
    pub labels: Option<Vec<String>>,

    /// State filter for list operations ("open", "closed", "all")
    #[serde(default = "default_state")]
    pub state: String,

    /// Maximum number of items to return for list operations
    #[serde(default = "default_per_page")]
    pub per_page: u8,
}

fn default_state() -> String {
    "open".to_string()
}

fn default_per_page() -> u8 {
    30
}

/// A simplified issue representation
#[derive(Debug, Serialize, Clone)]
pub struct IssueInfo {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub html_url: String,
    pub user: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub labels: Vec<String>,
    pub body: Option<String>,
}

/// A simplified pull request representation
#[derive(Debug, Serialize, Clone)]
pub struct PullRequestInfo {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub html_url: String,
    pub user: Option<String>,
    pub head: String,
    pub base: String,
    pub created_at: String,
    pub updated_at: String,
    pub merged: bool,
    pub draft: bool,
}

/// Repository information
#[derive(Debug, Serialize, Clone)]
pub struct RepoInfo {
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub default_branch: String,
    pub stars: u32,
    pub forks: u32,
    pub open_issues: u32,
    pub private: bool,
}

/// Output from GitHub tool
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GitHubOutput {
    /// List of issues
    Issues { issues: Vec<IssueInfo> },

    /// Single issue
    Issue { issue: IssueInfo },

    /// List of pull requests
    PullRequests { pull_requests: Vec<PullRequestInfo> },

    /// Single pull request
    PullRequest { pull_request: PullRequestInfo },

    /// Repository information
    Repository { repository: RepoInfo },

    /// Issue created successfully
    IssueCreated { number: u64, html_url: String },
}

/// The GitHub tool
pub struct GitHubTool;

impl GitHubTool {
    /// Create an authenticated Octocrab client
    fn create_client() -> Result<Octocrab, String> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
            "GITHUB_TOKEN environment variable not set. Please set it to authenticate with GitHub."
                .to_string()
        })?;

        Octocrab::builder()
            .personal_token(token)
            .build()
            .map_err(|e| format!("Failed to create GitHub client: {}", e))
    }

    /// Convert IssueState to string
    fn issue_state_to_string(state: octocrab::models::IssueState) -> String {
        match state {
            octocrab::models::IssueState::Open => "open".to_string(),
            octocrab::models::IssueState::Closed => "closed".to_string(),
            _ => "unknown".to_string(),
        }
    }
}

#[async_trait]
impl crate::Tool for GitHubTool {
    type Params = GitHubParams;
    type Output = GitHubOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "GitHub",
            description: "GitHub API operations: list/create issues, get PRs, repo info",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let owner = params.owner.clone();
        let repo = params.repo.clone();
        let operation = params.operation.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Executing {:?} on {}/{}", operation, owner, repo),
                percentage: Some(10.0),
            };

            // Create authenticated client
            let client = match Self::create_client() {
                Ok(c) => c,
                Err(e) => {
                    yield ToolEvent::Error { message: e };
                    return;
                }
            };

            if debug {
                tracing::debug!(
                    operation = ?operation,
                    owner = %owner,
                    repo = %repo,
                    "Executing GitHub operation"
                );
            }

            yield ToolEvent::Progress {
                step: "Calling GitHub API...".to_string(),
                percentage: Some(30.0),
            };

            match operation {
                GitHubOperation::ListIssues => {
                    let state = match params.state.as_str() {
                        "closed" => octocrab::params::State::Closed,
                        "all" => octocrab::params::State::All,
                        _ => octocrab::params::State::Open,
                    };

                    let result = client
                        .issues(&owner, &repo)
                        .list()
                        .state(state)
                        .per_page(params.per_page)
                        .send()
                        .await;

                    match result {
                        Ok(page) => {
                            let issues: Vec<IssueInfo> = page
                                .items
                                .into_iter()
                                .filter(|i| i.pull_request.is_none()) // Filter out PRs
                                .map(|i| IssueInfo {
                                    number: i.number,
                                    title: i.title,
                                    state: Self::issue_state_to_string(i.state),
                                    html_url: i.html_url.to_string(),
                                    user: Some(i.user.login),
                                    created_at: i.created_at.to_rfc3339(),
                                    updated_at: i.updated_at.to_rfc3339(),
                                    labels: i.labels.into_iter().map(|l| l.name).collect(),
                                    body: i.body,
                                })
                                .collect();

                            yield ToolEvent::Result(GitHubOutput::Issues { issues });
                        }
                        Err(e) => {
                            yield ToolEvent::Error {
                                message: format!("Failed to list issues: {}", e),
                            };
                        }
                    }
                }

                GitHubOperation::CreateIssue => {
                    let title = match &params.title {
                        Some(t) => t.clone(),
                        None => {
                            yield ToolEvent::Error {
                                message: "Title is required for create_issue operation".to_string(),
                            };
                            return;
                        }
                    };

                    let issues_handler = client.issues(&owner, &repo);
                    let mut builder = issues_handler.create(&title);

                    if let Some(body) = &params.body {
                        builder = builder.body(body);
                    }

                    if let Some(labels) = &params.labels {
                        builder = builder.labels(labels.clone());
                    }

                    match builder.send().await {
                        Ok(issue) => {
                            yield ToolEvent::Result(GitHubOutput::IssueCreated {
                                number: issue.number,
                                html_url: issue.html_url.to_string(),
                            });
                        }
                        Err(e) => {
                            yield ToolEvent::Error {
                                message: format!("Failed to create issue: {}", e),
                            };
                        }
                    }
                }

                GitHubOperation::GetIssue => {
                    let number = match params.number {
                        Some(n) => n,
                        None => {
                            yield ToolEvent::Error {
                                message: "Number is required for get_issue operation".to_string(),
                            };
                            return;
                        }
                    };

                    match client.issues(&owner, &repo).get(number).await {
                        Ok(issue) => {
                            yield ToolEvent::Result(GitHubOutput::Issue {
                                issue: IssueInfo {
                                    number: issue.number,
                                    title: issue.title,
                                    state: Self::issue_state_to_string(issue.state),
                                    html_url: issue.html_url.to_string(),
                                    user: Some(issue.user.login),
                                    created_at: issue.created_at.to_rfc3339(),
                                    updated_at: issue.updated_at.to_rfc3339(),
                                    labels: issue.labels.into_iter().map(|l| l.name).collect(),
                                    body: issue.body,
                                },
                            });
                        }
                        Err(e) => {
                            yield ToolEvent::Error {
                                message: format!("Failed to get issue #{}: {}", number, e),
                            };
                        }
                    }
                }

                GitHubOperation::ListPrs => {
                    let state = match params.state.as_str() {
                        "closed" => octocrab::params::State::Closed,
                        "all" => octocrab::params::State::All,
                        _ => octocrab::params::State::Open,
                    };

                    let result = client
                        .pulls(&owner, &repo)
                        .list()
                        .state(state)
                        .per_page(params.per_page)
                        .send()
                        .await;

                    match result {
                        Ok(page) => {
                            let pull_requests: Vec<PullRequestInfo> = page
                                .items
                                .into_iter()
                                .map(|pr| PullRequestInfo {
                                    number: pr.number,
                                    title: pr.title.unwrap_or_default(),
                                    state: pr.state.map(|s| format!("{:?}", s).to_lowercase()).unwrap_or_default(),
                                    html_url: pr.html_url.map(|u| u.to_string()).unwrap_or_default(),
                                    user: pr.user.map(|u| u.login),
                                    head: pr.head.ref_field,
                                    base: pr.base.ref_field,
                                    created_at: pr.created_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                                    updated_at: pr.updated_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                                    merged: pr.merged_at.is_some(),
                                    draft: pr.draft.unwrap_or(false),
                                })
                                .collect();

                            yield ToolEvent::Result(GitHubOutput::PullRequests { pull_requests });
                        }
                        Err(e) => {
                            yield ToolEvent::Error {
                                message: format!("Failed to list PRs: {}", e),
                            };
                        }
                    }
                }

                GitHubOperation::GetPr => {
                    let number = match params.number {
                        Some(n) => n,
                        None => {
                            yield ToolEvent::Error {
                                message: "Number is required for get_pr operation".to_string(),
                            };
                            return;
                        }
                    };

                    match client.pulls(&owner, &repo).get(number).await {
                        Ok(pr) => {
                            yield ToolEvent::Result(GitHubOutput::PullRequest {
                                pull_request: PullRequestInfo {
                                    number: pr.number,
                                    title: pr.title.unwrap_or_default(),
                                    state: pr.state.map(|s| format!("{:?}", s).to_lowercase()).unwrap_or_default(),
                                    html_url: pr.html_url.map(|u| u.to_string()).unwrap_or_default(),
                                    user: pr.user.map(|u| u.login),
                                    head: pr.head.ref_field,
                                    base: pr.base.ref_field,
                                    created_at: pr.created_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                                    updated_at: pr.updated_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                                    merged: pr.merged_at.is_some(),
                                    draft: pr.draft.unwrap_or(false),
                                },
                            });
                        }
                        Err(e) => {
                            yield ToolEvent::Error {
                                message: format!("Failed to get PR #{}: {}", number, e),
                            };
                        }
                    }
                }

                GitHubOperation::GetRepo => {
                    match client.repos(&owner, &repo).get().await {
                        Ok(repository) => {
                            yield ToolEvent::Result(GitHubOutput::Repository {
                                repository: RepoInfo {
                                    full_name: repository.full_name.unwrap_or_default(),
                                    description: repository.description,
                                    html_url: repository.html_url.map(|u| u.to_string()).unwrap_or_default(),
                                    default_branch: repository.default_branch.unwrap_or_else(|| "main".to_string()),
                                    stars: repository.stargazers_count.unwrap_or(0),
                                    forks: repository.forks_count.unwrap_or(0),
                                    open_issues: repository.open_issues_count.unwrap_or(0),
                                    private: repository.private.unwrap_or(false),
                                },
                            });
                        }
                        Err(e) => {
                            yield ToolEvent::Error {
                                message: format!("Failed to get repository: {}", e),
                            };
                        }
                    }
                }
            }
        }))
    }

    fn is_read_only(&self) -> bool {
        false // create_issue modifies state
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Each API call is independent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_deserialization() {
        let json = r#"{
            "operation": "list_issues",
            "owner": "rust-lang",
            "repo": "rust"
        }"#;

        let params: GitHubParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.operation, GitHubOperation::ListIssues);
        assert_eq!(params.owner, "rust-lang");
        assert_eq!(params.repo, "rust");
        assert_eq!(params.state, "open");
        assert_eq!(params.per_page, 30);
    }

    #[test]
    fn test_params_with_options() {
        let json = r#"{
            "operation": "create_issue",
            "owner": "test-owner",
            "repo": "test-repo",
            "title": "Test issue",
            "body": "This is the body",
            "labels": ["bug", "help wanted"]
        }"#;

        let params: GitHubParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.operation, GitHubOperation::CreateIssue);
        assert_eq!(params.title, Some("Test issue".to_string()));
        assert_eq!(params.body, Some("This is the body".to_string()));
        assert_eq!(
            params.labels,
            Some(vec!["bug".to_string(), "help wanted".to_string()])
        );
    }

    #[test]
    fn test_params_get_pr() {
        let json = r#"{
            "operation": "get_pr",
            "owner": "test-owner",
            "repo": "test-repo",
            "number": 123
        }"#;

        let params: GitHubParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.operation, GitHubOperation::GetPr);
        assert_eq!(params.number, Some(123));
    }

    #[test]
    fn test_output_serialization() {
        let output = GitHubOutput::IssueCreated {
            number: 42,
            html_url: "https://github.com/owner/repo/issues/42".to_string(),
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"type\":\"issue_created\""));
        assert!(json.contains("\"number\":42"));
    }

    #[test]
    fn test_issue_info_serialization() {
        let issue = IssueInfo {
            number: 1,
            title: "Test".to_string(),
            state: "open".to_string(),
            html_url: "https://example.com".to_string(),
            user: Some("testuser".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-02T00:00:00Z".to_string(),
            labels: vec!["bug".to_string()],
            body: Some("Description".to_string()),
        };

        let json = serde_json::to_string(&issue).unwrap();
        assert!(json.contains("\"number\":1"));
        assert!(json.contains("\"title\":\"Test\""));
    }

    #[test]
    fn test_all_operations_deserialize() {
        let operations = [
            ("list_issues", GitHubOperation::ListIssues),
            ("create_issue", GitHubOperation::CreateIssue),
            ("get_issue", GitHubOperation::GetIssue),
            ("list_prs", GitHubOperation::ListPrs),
            ("get_pr", GitHubOperation::GetPr),
            ("get_repo", GitHubOperation::GetRepo),
        ];

        for (name, expected) in operations {
            let json = format!(
                r#"{{"operation": "{}", "owner": "o", "repo": "r"}}"#,
                name
            );
            let params: GitHubParams = serde_json::from_str(&json).unwrap();
            assert_eq!(params.operation, expected, "Failed for operation: {}", name);
        }
    }

    #[test]
    fn test_metadata() {
        let tool = GitHubTool;
        let metadata = crate::Tool::metadata(&tool);
        assert_eq!(metadata.name, "GitHub");
        assert!(!metadata.description.is_empty());
    }
}
