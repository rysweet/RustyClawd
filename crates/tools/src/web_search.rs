//! WebSearch tool - Search the web
//!
//! Demonstrates:
//! - Search API integration
//! - Domain filtering
//! - Result ranking

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for WebSearch tool
#[derive(Debug, Deserialize)]
pub struct WebSearchParams {
    /// Search query
    pub query: String,

    /// Domains to allow
    #[serde(default)]
    pub allowed_domains: Vec<String>,

    /// Domains to block
    #[serde(default)]
    pub blocked_domains: Vec<String>,
}

/// A single search result
#[derive(Debug, Serialize, Clone)]
pub struct SearchResult {
    /// Result title
    pub title: String,

    /// Result URL
    pub url: String,

    /// Snippet/description
    pub snippet: String,
}

/// Output from WebSearch tool
#[derive(Debug, Serialize)]
pub struct WebSearchOutput {
    /// Search results
    pub results: Vec<SearchResult>,

    /// Query that was searched
    pub query: String,

    /// Number of results found
    pub count: usize,
}

/// The WebSearch tool
pub struct WebSearchTool;

#[async_trait]
impl crate::Tool for WebSearchTool {
    type Params = WebSearchParams;
    type Output = WebSearchOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "WebSearch",
            description: "Searches the web and returns ranked results",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let query = params.query.clone();
        let allowed = params.allowed_domains.clone();
        let blocked = params.blocked_domains.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Searching for: {}", query),
                percentage: None,
            };

            if debug {
                tracing::debug!(
                    query = %query,
                    allowed_domains = ?allowed,
                    blocked_domains = ?blocked,
                    "Web search initiated"
                );
            }

            // Build HTTP client
            let client = match reqwest::Client::builder()
                .user_agent("claude-code-rs/0.1.0")
                .timeout(std::time::Duration::from_secs(30))
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to create HTTP client: {}", e),
                    };
                    return;
                }
            };

            yield ToolEvent::Progress {
                step: "Querying DuckDuckGo...".to_string(),
                percentage: Some(50.0),
            };

            // Use DuckDuckGo Instant Answer API (free, no auth required)
            let api_url = format!(
                "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
                urlencoding::encode(&query)
            );

            let response = match client.get(&api_url).send().await {
                Ok(r) => r,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to query search API: {}", e),
                    };
                    return;
                }
            };

            let json: serde_json::Value = match response.json().await {
                Ok(j) => j,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to parse search results: {}", e),
                    };
                    return;
                }
            };

            // Parse results from DuckDuckGo response
            let mut results = Vec::new();

            // Add abstract/instant answer if available
            if let Some(abstract_text) = json.get("AbstractText").and_then(|v| v.as_str()) {
                if !abstract_text.is_empty() {
                    if let Some(abstract_url) = json.get("AbstractURL").and_then(|v| v.as_str()) {
                        results.push(SearchResult {
                            title: json.get("Heading").and_then(|v| v.as_str()).unwrap_or(&query).to_string(),
                            url: abstract_url.to_string(),
                            snippet: abstract_text.to_string(),
                        });
                    }
                }
            }

            // Add related topics
            if let Some(related) = json.get("RelatedTopics").and_then(|v| v.as_array()) {
                for topic in related.iter().take(10) {
                    if let Some(text) = topic.get("Text").and_then(|v| v.as_str()) {
                        if let Some(url) = topic.get("FirstURL").and_then(|v| v.as_str()) {
                            results.push(SearchResult {
                                title: text.split(" - ").next().unwrap_or(text).to_string(),
                                url: url.to_string(),
                                snippet: text.to_string(),
                            });
                        }
                    }
                }
            }

            // Filter by allowed/blocked domains
            if !allowed.is_empty() || !blocked.is_empty() {
                results.retain(|result| {
                    if let Ok(url) = url::Url::parse(&result.url) {
                        if let Some(domain) = url.domain() {
                            // Check blocked list first
                            if !blocked.is_empty() && blocked.iter().any(|b| domain.contains(b)) {
                                return false;
                            }
                            // Check allowed list if specified
                            if !allowed.is_empty() && !allowed.iter().any(|a| domain.contains(a)) {
                                return false;
                            }
                        }
                    }
                    true
                });
            }

            let count = results.len();

            yield ToolEvent::Result(WebSearchOutput {
                results,
                query: params.query.clone(),
                count,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true // Search doesn't modify local state
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Searches are independent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;

    #[tokio::test]
    #[ignore]
    async fn test_web_search() {
        // This test makes real HTTP calls to the DuckDuckGo API and requires network connectivity.
        // It is ignored by default to avoid flaky tests in CI/CD environments.
        // To run this test manually: cargo test test_web_search -- --ignored --nocapture
        let tool = WebSearchTool;
        let params = WebSearchParams {
            query: "Rust programming".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec![],
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

        assert_eq!(result.query, "Rust programming");
        assert!(result.count > 0);
    }
}
