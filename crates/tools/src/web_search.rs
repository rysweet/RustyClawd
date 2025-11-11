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

            // In full implementation, would:
            // 1. Call search API (DuckDuckGo, Brave Search, etc.)
            // 2. Parse results
            // 3. Filter by domain whitelist/blacklist
            // 4. Rank and return top results

            // Simplified: Return mock results for educational purposes
            let results = vec![
                SearchResult {
                    title: format!("Result for: {}", query),
                    url: "https://example.com/1".to_string(),
                    snippet: "This is a sample search result snippet...".to_string(),
                },
            ];

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
    async fn test_web_search() {
        let tool = WebSearchTool;
        let params = WebSearchParams {
            query: "Rust programming".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec![],
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        }).unwrap();

        assert_eq!(result.query, "Rust programming");
        assert!(result.count > 0);
    }
}
