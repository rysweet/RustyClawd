//! WebSearch tool - Search the web using Claude's server-side tool
//!
//! Phase 2 implementation:
//! - Server-side web_search via API extraToolSchemas
//! - Streaming results with progressive disclosure
//! - Domain filtering (allowlist/blocklist, mutually exclusive)
//! - Max 8 searches per invocation
//! - Real-time progress tracking

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use rustyclawd_core::client::types::{ContentBlockStart, ContentDelta, StreamEvent};
use rustyclawd_core::client::{Client, CreateMessageRequest, ExtraToolSchema, Message};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use validator::Validate;

/// Parameters for WebSearch tool
#[derive(Debug, Deserialize, Validate)]
pub struct WebSearchParams {
    /// Search query (minimum 2 characters)
    #[validate(length(min = 2, message = "Query must be at least 2 characters"))]
    pub query: String,

    /// Domains to allow (mutually exclusive with blocked_domains)
    #[serde(default)]
    pub allowed_domains: Vec<String>,

    /// Domains to block (mutually exclusive with allowed_domains)
    #[serde(default)]
    pub blocked_domains: Vec<String>,
}

impl WebSearchParams {
    /// Validate that allowed_domains and blocked_domains are mutually exclusive
    pub fn validate_domain_exclusivity(&self) -> Result<(), String> {
        if !self.allowed_domains.is_empty() && !self.blocked_domains.is_empty() {
            return Err(
                "Cannot specify both allowed_domains and blocked_domains - they are mutually exclusive"
                    .to_string(),
            );
        }
        Ok(())
    }
}

/// A single search result
#[derive(Debug, Serialize, Clone)]
pub struct SearchHit {
    /// Result title
    pub title: String,

    /// Result URL
    pub url: String,
}

/// Search result block from Claude API
#[derive(Debug, Serialize, Clone)]
pub struct SearchResultBlock {
    /// Tool use ID
    pub tool_use_id: String,

    /// Search results
    pub content: Vec<SearchHit>,
}

/// Output from WebSearch tool
#[derive(Debug, Serialize)]
pub struct WebSearchOutput {
    /// Search results (can be blocks or error strings)
    pub results: Vec<SearchResultBlock>,

    /// Query that was searched
    pub query: String,

    /// Number of results found
    pub count: usize,

    /// Time taken for search (in seconds)
    pub duration_seconds: f64,
}

/// The WebSearch tool (Phase 2)
pub struct WebSearchTool {
    /// Optional client for testing
    #[allow(dead_code)]
    client: Option<Client>,
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSearchTool {
    /// Create a new WebSearch tool
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Create a WebSearch tool with a custom client (for testing)
    #[allow(dead_code)]
    pub fn with_client(client: Client) -> Self {
        Self {
            client: Some(client),
        }
    }
}

#[async_trait]
impl crate::Tool for WebSearchTool {
    type Params = WebSearchParams;
    type Output = WebSearchOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "WebSearch",
            description: "Searches the web using Claude's server-side tool and returns ranked results with domain filtering support",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        // Validate parameters
        params.validate().map_err(|e| {
            crate::ToolError::Validation(format!("Parameter validation failed: {}", e))
        })?;

        params
            .validate_domain_exclusivity()
            .map_err(crate::ToolError::Validation)?;

        let query = params.query.clone();
        let allowed = params.allowed_domains.clone();
        let blocked = params.blocked_domains.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            let start_time = Instant::now();

            yield ToolEvent::Progress {
                step: format!("Initiating web search for: {}", query),
                percentage: Some(10.0),
            };

            if debug {
                tracing::debug!(
                    query = %query,
                    allowed_domains = ?allowed,
                    blocked_domains = ?blocked,
                    "Web search initiated (Phase 2 - server-side)"
                );
            }

            // Load client configuration
            let client = match rustyclawd_core::client::Config::from_default_location().await {
                Ok(config) => match Client::new(config) {
                    Ok(c) => c,
                    Err(e) => {
                        yield ToolEvent::Error {
                            message: format!("Failed to build HTTP client: {}", e),
                        };
                        return;
                    }
                },
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to load API configuration: {}", e),
                    };
                    return;
                }
            };

            yield ToolEvent::Progress {
                step: "Configuring search parameters...".to_string(),
                percentage: Some(20.0),
            };

            // Build extra tool schema for web_search
            let allowed_opt = if allowed.is_empty() { None } else { Some(allowed.clone()) };
            let blocked_opt = if blocked.is_empty() { None } else { Some(blocked.clone()) };

            let web_search_schema = ExtraToolSchema::web_search(
                allowed_opt,
                blocked_opt,
                Some(8), // Max 8 searches per invocation
            );

            // Create a request that will trigger web search
            let request = CreateMessageRequest::new(
                "claude-sonnet-4-5-20250929",
                vec![Message::user(format!(
                    "Please search the web for: {}",
                    query
                ))],
                4096,
            )
            .with_stream(true)
            .with_extra_tool_schemas(vec![web_search_schema]);

            yield ToolEvent::Progress {
                step: "Executing server-side search...".to_string(),
                percentage: Some(40.0),
            };

            // Execute streaming request
            let stream = match client.create_message_stream(request).await {
                Ok(s) => s,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to create search stream: {}", e),
                    };
                    return;
                }
            };

            // Parse streaming events for search results
            let mut results = Vec::new();
            let mut current_tool_id: Option<String> = None;
            let mut current_tool_name: Option<String> = None;
            let mut accumulated_json = String::new();
            let mut query_detected = false;

            tokio::pin!(stream);

            yield ToolEvent::Progress {
                step: "Processing search results...".to_string(),
                percentage: Some(60.0),
            };

            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(StreamEvent::ContentBlockStart { content_block, .. }) => {
                        if debug {
                            tracing::debug!("ContentBlockStart: {:?}", content_block);
                        }
                        if let ContentBlockStart::ToolUse { id, name } = content_block {
                            if name == "web_search" {
                                current_tool_id = Some(id.clone());
                                current_tool_name = Some(name.clone());
                                accumulated_json.clear();

                                if !query_detected {
                                    query_detected = true;
                                    yield ToolEvent::Progress {
                                        step: format!("Query parsed: {}", query),
                                        percentage: Some(70.0),
                                    };
                                }
                            }
                        }
                    }
                    Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                        if debug {
                            tracing::debug!("ContentBlockDelta: {:?}", delta);
                        }
                        if let ContentDelta::InputJsonDelta { partial_json } = delta {
                            if current_tool_name.as_deref() == Some("web_search") {
                                accumulated_json.push_str(&partial_json);
                            }
                        }
                    }
                    Ok(StreamEvent::ContentBlockStop { .. }) => {
                        if debug {
                            tracing::debug!("ContentBlockStop - accumulated JSON: {}", accumulated_json);
                        }

                        // Parse the complete JSON input
                        if let (Some(tool_id), Some(tool_name)) = (&current_tool_id, &current_tool_name) {
                            if tool_name == "web_search" && !accumulated_json.is_empty() {
                                // Try to parse search results from the accumulated JSON
                                match serde_json::from_str::<serde_json::Value>(&accumulated_json) {
                                    Ok(json) => {
                                        if debug {
                                            tracing::debug!("Parsed search input JSON: {:?}", json);
                                        }

                                        // Extract search results if available
                                        // The format may vary, but typically includes a results array
                                        let search_hits = if let Some(results_array) = json.get("results").and_then(|v| v.as_array()) {
                                            results_array.iter().filter_map(|item| {
                                                let title = item.get("title")?.as_str()?.to_string();
                                                let url = item.get("url")?.as_str()?.to_string();
                                                Some(SearchHit { title, url })
                                            }).collect()
                                        } else {
                                            Vec::new()
                                        };

                                        if !search_hits.is_empty() {
                                            results.push(SearchResultBlock {
                                                tool_use_id: tool_id.clone(),
                                                content: search_hits,
                                            });

                                            yield ToolEvent::Progress {
                                                step: format!("Received {} search results", results.iter().map(|r| r.content.len()).sum::<usize>()),
                                                percentage: Some(80.0),
                                            };
                                        }
                                    }
                                    Err(e) => {
                                        if debug {
                                            tracing::warn!("Failed to parse search input JSON: {} - JSON: {}", e, accumulated_json);
                                        }
                                    }
                                }

                                accumulated_json.clear();
                            }
                        }

                        current_tool_id = None;
                        current_tool_name = None;
                    }
                    Ok(StreamEvent::MessageStop) => {
                        if debug {
                            tracing::debug!("MessageStop - search complete");
                        }
                        break;
                    }
                    Ok(StreamEvent::Error { error }) => {
                        yield ToolEvent::Error {
                            message: format!("API error: {}", error.message),
                        };
                        return;
                    }
                    Err(e) => {
                        yield ToolEvent::Error {
                            message: format!("Stream error: {}", e),
                        };
                        return;
                    }
                    _ => {
                        // Ignore other event types
                    }
                }
            }

            let count = results.iter().map(|r| r.content.len()).sum();
            let duration = start_time.elapsed().as_secs_f64();

            yield ToolEvent::Progress {
                step: format!("Search complete - found {} results", count),
                percentage: Some(100.0),
            };

            yield ToolEvent::Result(WebSearchOutput {
                results,
                query: query.clone(),
                count,
                duration_seconds: duration,
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

    #[test]
    fn test_params_validation() {
        // Valid query
        let params = WebSearchParams {
            query: "Rust programming".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec![],
        };
        assert!(params.validate().is_ok());
        assert!(params.validate_domain_exclusivity().is_ok());

        // Query too short
        let params = WebSearchParams {
            query: "R".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec![],
        };
        assert!(params.validate().is_err());

        // Both allowed and blocked specified
        let params = WebSearchParams {
            query: "Rust".to_string(),
            allowed_domains: vec!["rust-lang.org".to_string()],
            blocked_domains: vec!["example.com".to_string()],
        };
        assert!(params.validate_domain_exclusivity().is_err());
    }

    #[tokio::test]
    async fn test_metadata() {
        let tool = WebSearchTool::new();
        let metadata = tool.metadata();
        assert_eq!(metadata.name, "WebSearch");
        assert!(metadata.description.contains("server-side"));
    }

    #[tokio::test]
    async fn test_query_too_short_error() {
        let tool = WebSearchTool::new();
        let params = WebSearchParams {
            query: "R".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec![],
        };
        let ctx = ToolContext::default();

        let result = tool.execute(params, &ctx).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("at least 2 characters"));
        }
    }

    #[tokio::test]
    async fn test_domain_exclusivity_error() {
        let tool = WebSearchTool::new();
        let params = WebSearchParams {
            query: "Test query".to_string(),
            allowed_domains: vec!["example.com".to_string()],
            blocked_domains: vec!["blocked.com".to_string()],
        };
        let ctx = ToolContext::default();

        let result = tool.execute(params, &ctx).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("mutually exclusive"));
        }
    }

    #[tokio::test]
    #[ignore] // Requires API key and network
    async fn test_web_search_integration() {
        // This test makes real API calls and requires:
        // 1. Valid API key in ~/.claude-msec-k
        // 2. Network connectivity
        // 3. Anthropic API access
        // Run with: cargo test test_web_search_integration -- --ignored --nocapture

        let tool = WebSearchTool::new();
        let params = WebSearchParams {
            query: "Rust programming language".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec![],
        };
        let ctx = ToolContext {
            debug: true,
            ..Default::default()
        };

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should have progress events and a result
        assert!(events.len() > 1);

        // Find the result event
        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        if let Some(output) = result {
            assert_eq!(output.query, "Rust programming language");
            println!("Search returned {} results", output.count);
            println!("Duration: {:.2}s", output.duration_seconds);

            for (idx, block) in output.results.iter().enumerate() {
                println!("\nResult block {}: {} hits", idx + 1, block.content.len());
                for (i, hit) in block.content.iter().enumerate() {
                    println!("  {}. {} - {}", i + 1, hit.title, hit.url);
                }
            }
        } else {
            panic!("No result event found in stream");
        }
    }

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_web_search_with_allowed_domains() {
        let tool = WebSearchTool::new();
        let params = WebSearchParams {
            query: "Rust".to_string(),
            allowed_domains: vec!["rust-lang.org".to_string()],
            blocked_domains: vec![],
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        if let Some(output) = result {
            // Verify all results are from allowed domain
            for block in &output.results {
                for hit in &block.content {
                    assert!(hit.url.contains("rust-lang.org"));
                }
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_web_search_with_blocked_domains() {
        let tool = WebSearchTool::new();
        let params = WebSearchParams {
            query: "Rust".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec!["example.com".to_string()],
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        if let Some(output) = result {
            // Verify no results are from blocked domain
            for block in &output.results {
                for hit in &block.content {
                    assert!(!hit.url.contains("example.com"));
                }
            }
        }
    }

    #[test]
    fn test_search_hit_serialization() {
        let hit = SearchHit {
            title: "Test Result".to_string(),
            url: "https://example.com".to_string(),
        };

        let json = serde_json::to_string(&hit).unwrap();
        assert!(json.contains("Test Result"));
        assert!(json.contains("https://example.com"));
    }

    #[test]
    fn test_search_result_block_serialization() {
        let block = SearchResultBlock {
            tool_use_id: "tool_123".to_string(),
            content: vec![
                SearchHit {
                    title: "Result 1".to_string(),
                    url: "https://example.com/1".to_string(),
                },
                SearchHit {
                    title: "Result 2".to_string(),
                    url: "https://example.com/2".to_string(),
                },
            ],
        };

        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("tool_123"));
        assert!(json.contains("Result 1"));
        assert!(json.contains("Result 2"));
    }

    #[test]
    fn test_web_search_output_serialization() {
        let output = WebSearchOutput {
            results: vec![SearchResultBlock {
                tool_use_id: "tool_123".to_string(),
                content: vec![SearchHit {
                    title: "Test".to_string(),
                    url: "https://example.com".to_string(),
                }],
            }],
            query: "test query".to_string(),
            count: 1,
            duration_seconds: 1.23,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("test query"));
        assert!(json.contains("1.23"));
    }

    #[test]
    fn test_tool_is_read_only() {
        let tool = WebSearchTool::new();
        assert!(tool.is_read_only());
    }

    #[test]
    fn test_tool_is_concurrency_safe() {
        let tool = WebSearchTool::new();
        assert!(tool.is_concurrency_safe());
    }
}
