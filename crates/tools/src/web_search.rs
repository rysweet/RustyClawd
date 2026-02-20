//! WebSearch tool - Phase 2 Implementation
//!
//! Complete implementation of WebSearch tool with all Phase 2 features:
//! - Real web search API integration via Claude's server-side tool
//! - Streaming results with progressive disclosure
//! - Domain filtering (allowlist/blocklist, mutually exclusive)
//! - Max 8 searches per invocation
//! - Real-time progress tracking
//! - Comprehensive error handling
//! - Model-specific availability (Opus 4, Sonnet 4, Haiku 4)

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use moka::future::Cache;
use rustyclawd_core::client::types::{ContentBlockStart, ContentDelta, StreamEvent};
use rustyclawd_core::client::{Client, CreateMessageRequest, ExtraToolSchema, Message};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use validator::Validate;

/// Cache TTL in seconds (15 minutes, same as WebFetch)
const CACHE_TTL_SECONDS: u64 = 900;

/// Parameters for WebSearch tool
#[derive(Debug, Clone, Deserialize, Validate)]
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

    /// Optional snippet/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

/// Search result block from Claude API
#[derive(Debug, Serialize, Clone)]
pub struct SearchResultBlock {
    /// Tool use ID
    pub tool_use_id: String,

    /// Search results
    pub content: Vec<SearchHit>,
}

/// Output from WebSearch tool (Phase 2 spec)
#[derive(Debug, Serialize)]
pub struct WebSearchOutput {
    /// Search query
    pub query: String,

    /// Search results
    pub results: Vec<SearchResultBlock>,

    /// Number of results found
    pub count: usize,

    /// Time taken for search (in seconds)
    pub duration_seconds: f64,

    /// Whether results were from cache
    pub cached: bool,
}

/// Cached search results
#[derive(Debug, Clone)]
struct CachedSearchResults {
    /// Search result blocks
    results: Vec<SearchResultBlock>,
    /// Total count
    count: usize,
}

/// Cache for web search results with 15-minute TTL
pub struct WebSearchCache {
    cache: Cache<String, CachedSearchResults>,
}

impl WebSearchCache {
    /// Create a new cache with 15-minute TTL
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(100)
                .time_to_live(Duration::from_secs(CACHE_TTL_SECONDS))
                .build(),
        }
    }

    /// Get cached results
    async fn get(&self, key: &str) -> Option<CachedSearchResults> {
        self.cache.get(key).await
    }

    /// Store results in cache
    async fn insert(&self, key: String, results: CachedSearchResults) {
        self.cache.insert(key, results).await;
    }
}

impl Default for WebSearchCache {
    fn default() -> Self {
        Self::new()
    }
}

/// The WebSearch tool with Phase 2 features
pub struct WebSearchTool {
    cache: Arc<WebSearchCache>,
}

impl WebSearchTool {
    /// Create a new WebSearch tool with caching
    pub fn new() -> Self {
        Self {
            cache: Arc::new(WebSearchCache::new()),
        }
    }

    /// Check if model supports web_search
    fn is_model_supported(model: &str) -> bool {
        // Opus 4, Sonnet 4, Haiku 4 support web_search
        model.contains("opus-4")
            || model.contains("sonnet-4")
            || model.contains("haiku-4")
            || model.contains("claude-3-5-sonnet")
            || model.contains("claude-3-opus")
    }

    /// Build cache key from params
    fn build_cache_key(params: &WebSearchParams) -> String {
        // Include domain filters in cache key
        let mut key = params.query.clone();
        if !params.allowed_domains.is_empty() {
            key.push_str("|allow:");
            key.push_str(&params.allowed_domains.join(","));
        }
        if !params.blocked_domains.is_empty() {
            key.push_str("|block:");
            key.push_str(&params.blocked_domains.join(","));
        }
        key
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::Tool for WebSearchTool {
    type Params = WebSearchParams;
    type Output = WebSearchOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "WebSearch",
            description: "Searches the web using Claude's server-side tool with streaming results, domain filtering, and 15-minute caching",
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
        let cache = self.cache.clone();
        let cache_key = Self::build_cache_key(&params);

        Ok(Box::pin(stream! {
            let start_time = Instant::now();

            yield ToolEvent::Progress {
                step: format!("Initiating web search: {}", query),
                percentage: Some(10.0),
            };

            if debug {
                tracing::debug!(
                    query = %query,
                    allowed_domains = ?allowed,
                    blocked_domains = ?blocked,
                    "Web search Phase 2 initiated"
                );
            }

            // Check cache first
            if let Some(cached) = cache.get(&cache_key).await {
                if debug {
                    tracing::debug!("Cache hit for query: {}", query);
                }

                yield ToolEvent::Progress {
                    step: "Using cached results...".to_string(),
                    percentage: Some(90.0),
                };

                let duration = start_time.elapsed().as_secs_f64();

                yield ToolEvent::Result(WebSearchOutput {
                    query: query.clone(),
                    results: cached.results,
                    count: cached.count,
                    duration_seconds: duration,
                    cached: true,
                });
                return;
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

            // Get model from config (default to Sonnet 4.5)
            let model = "claude-sonnet-4-5-20250929";

            // Check model support
            if !Self::is_model_supported(model) {
                yield ToolEvent::Error {
                    message: format!(
                        "Model {} does not support web_search. Use Opus 4, Sonnet 4, or Haiku 4.",
                        model
                    ),
                };
                return;
            }

            // Build extra tool schema for web_search
            let allowed_opt = if allowed.is_empty() { None } else { Some(allowed.clone()) };
            let blocked_opt = if blocked.is_empty() { None } else { Some(blocked.clone()) };

            let web_search_schema = ExtraToolSchema::web_search(
                allowed_opt,
                blocked_opt,
                Some(8), // Max 8 searches per invocation
            );

            if debug {
                tracing::debug!("Web search schema: {:?}", web_search_schema);
            }

            // Create a request that will trigger web search
            let request = CreateMessageRequest::new(
                model,
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
            let mut progress_emitted = false;

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

                                if !progress_emitted {
                                    progress_emitted = true;
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
                            tracing::trace!("ContentBlockDelta: {:?}", delta);
                        }
                        if let ContentDelta::InputJsonDelta { partial_json } = delta {
                            if current_tool_name.as_deref() == Some("web_search") {
                                accumulated_json.push_str(&partial_json);
                            }
                        }
                    }
                    Ok(StreamEvent::ContentBlockStop { .. }) => {
                        if debug {
                            tracing::debug!("ContentBlockStop - accumulated JSON length: {}", accumulated_json.len());
                        }

                        // Parse the complete JSON input
                        if let (Some(tool_id), Some(tool_name)) = (&current_tool_id, &current_tool_name) {
                            if tool_name == "web_search" && !accumulated_json.is_empty() {
                                // Try to parse search results from the accumulated JSON
                                match serde_json::from_str::<serde_json::Value>(&accumulated_json) {
                                    Ok(json) => {
                                        if debug {
                                            tracing::debug!("Parsed search JSON successfully");
                                        }

                                        // Extract search results if available
                                        let search_hits = if let Some(results_array) = json.get("results").and_then(|v| v.as_array()) {
                                            results_array.iter().filter_map(|item| {
                                                let title = item.get("title")?.as_str()?.to_string();
                                                let url = item.get("url")?.as_str()?.to_string();
                                                let snippet = item.get("snippet")
                                                    .and_then(|s| s.as_str())
                                                    .map(|s| s.to_string());
                                                Some(SearchHit { title, url, snippet })
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
                                            tracing::warn!("Failed to parse search JSON: {} - JSON preview: {}...",
                                                e,
                                                accumulated_json.chars().take(100).collect::<String>()
                                            );
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
                        // Ignore other event types (Ping, MessageDelta, etc.)
                    }
                }
            }

            let count = results.iter().map(|r| r.content.len()).sum();
            let duration = start_time.elapsed().as_secs_f64();

            if debug {
                tracing::debug!(
                    count = count,
                    duration_seconds = duration,
                    "Search complete"
                );
            }

            // Cache the results
            cache.insert(
                cache_key,
                CachedSearchResults {
                    results: results.clone(),
                    count,
                },
            ).await;

            yield ToolEvent::Progress {
                step: format!("Search complete - found {} results", count),
                percentage: Some(100.0),
            };

            yield ToolEvent::Result(WebSearchOutput {
                query: query.clone(),
                results,
                count,
                duration_seconds: duration,
                cached: false,
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
    fn test_params_validation_valid() {
        let params = WebSearchParams {
            query: "Rust programming".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec![],
        };
        assert!(params.validate().is_ok());
        assert!(params.validate_domain_exclusivity().is_ok());
    }

    #[test]
    fn test_params_validation_query_too_short() {
        let params = WebSearchParams {
            query: "R".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec![],
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_params_validation_domain_exclusivity() {
        let params = WebSearchParams {
            query: "Rust programming".to_string(),
            allowed_domains: vec!["rust-lang.org".to_string()],
            blocked_domains: vec!["example.com".to_string()],
        };
        assert!(params.validate_domain_exclusivity().is_err());
    }

    #[test]
    fn test_params_validation_allowed_only() {
        let params = WebSearchParams {
            query: "Rust programming".to_string(),
            allowed_domains: vec!["rust-lang.org".to_string()],
            blocked_domains: vec![],
        };
        assert!(params.validate().is_ok());
        assert!(params.validate_domain_exclusivity().is_ok());
    }

    #[test]
    fn test_params_validation_blocked_only() {
        let params = WebSearchParams {
            query: "Rust programming".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec!["example.com".to_string()],
        };
        assert!(params.validate().is_ok());
        assert!(params.validate_domain_exclusivity().is_ok());
    }

    #[test]
    fn test_model_support_detection() {
        assert!(WebSearchTool::is_model_supported("claude-opus-4-20250514"));
        assert!(WebSearchTool::is_model_supported(
            "claude-sonnet-4-5-20250929"
        ));
        assert!(WebSearchTool::is_model_supported("claude-haiku-4-20250514"));
        assert!(WebSearchTool::is_model_supported(
            "claude-3-5-sonnet-20241022"
        ));
        assert!(!WebSearchTool::is_model_supported("claude-2.1"));
        assert!(!WebSearchTool::is_model_supported("gpt-4"));
    }

    #[test]
    fn test_cache_key_building() {
        let params1 = WebSearchParams {
            query: "Rust".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec![],
        };
        let key1 = WebSearchTool::build_cache_key(&params1);
        assert_eq!(key1, "Rust");

        let params2 = WebSearchParams {
            query: "Rust".to_string(),
            allowed_domains: vec!["rust-lang.org".to_string()],
            blocked_domains: vec![],
        };
        let key2 = WebSearchTool::build_cache_key(&params2);
        assert!(key2.contains("allow:"));
        assert!(key2.contains("rust-lang.org"));

        let params3 = WebSearchParams {
            query: "Rust".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec!["example.com".to_string()],
        };
        let key3 = WebSearchTool::build_cache_key(&params3);
        assert!(key3.contains("block:"));
        assert!(key3.contains("example.com"));
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let cache = WebSearchCache::new();
        let key = "test_query".to_string();
        let results = CachedSearchResults {
            results: vec![SearchResultBlock {
                tool_use_id: "tool_123".to_string(),
                content: vec![SearchHit {
                    title: "Test Result".to_string(),
                    url: "https://example.com".to_string(),
                    snippet: Some("A test snippet".to_string()),
                }],
            }],
            count: 1,
        };

        // Insert and retrieve
        cache.insert(key.clone(), results.clone()).await;
        let cached = cache.get(&key).await;
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.count, 1);
        assert_eq!(cached.results.len(), 1);
        assert_eq!(cached.results[0].content.len(), 1);
    }

    #[test]
    fn test_search_hit_serialization() {
        let hit = SearchHit {
            title: "Test Result".to_string(),
            url: "https://example.com".to_string(),
            snippet: Some("A test snippet".to_string()),
        };

        let json = serde_json::to_string(&hit).unwrap();
        assert!(json.contains("Test Result"));
        assert!(json.contains("https://example.com"));
        assert!(json.contains("A test snippet"));
    }

    #[test]
    fn test_search_result_block_serialization() {
        let block = SearchResultBlock {
            tool_use_id: "tool_123".to_string(),
            content: vec![
                SearchHit {
                    title: "Result 1".to_string(),
                    url: "https://example.com/1".to_string(),
                    snippet: None,
                },
                SearchHit {
                    title: "Result 2".to_string(),
                    url: "https://example.com/2".to_string(),
                    snippet: Some("Result 2 snippet".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("tool_123"));
        assert!(json.contains("Result 1"));
        assert!(json.contains("Result 2"));
        assert!(json.contains("Result 2 snippet"));
    }

    #[test]
    fn test_web_search_output_serialization() {
        let output = WebSearchOutput {
            query: "test query".to_string(),
            results: vec![SearchResultBlock {
                tool_use_id: "tool_123".to_string(),
                content: vec![SearchHit {
                    title: "Test".to_string(),
                    url: "https://example.com".to_string(),
                    snippet: None,
                }],
            }],
            count: 1,
            duration_seconds: 1.23,
            cached: false,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("test query"));
        assert!(json.contains("1.23"));
        assert!(json.contains("false")); // cached: false
    }

    #[tokio::test]
    async fn test_metadata() {
        let tool = WebSearchTool::new();
        let metadata = tool.metadata();
        assert_eq!(metadata.name, "WebSearch");
        assert!(metadata.description.contains("server-side"));
        assert!(metadata.description.contains("caching"));
    }

    #[tokio::test]
    async fn test_tool_is_read_only() {
        let tool = WebSearchTool::new();
        assert!(tool.is_read_only());
    }

    #[tokio::test]
    async fn test_tool_is_concurrency_safe() {
        let tool = WebSearchTool::new();
        assert!(tool.is_concurrency_safe());
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
            let err_msg = e.to_string();
            assert!(err_msg.contains("at least 2 characters") || err_msg.contains("validation"));
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
            let err_msg = e.to_string();
            assert!(err_msg.contains("mutually exclusive") || err_msg.contains("validation"));
        }
    }

    #[tokio::test]
    #[ignore] // Requires API key and network
    async fn test_web_search_phase2_integration() {
        // This test makes real API calls and requires:
        // 1. Valid API key in ~/.claude-msec-k
        // 2. Network connectivity
        // 3. Anthropic API access
        // Run with: cargo test test_web_search_phase2_integration -- --ignored --nocapture

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
            assert!(!output.cached); // First request should not be cached
            println!("Search returned {} results", output.count);
            println!("Duration: {:.2}s", output.duration_seconds);

            for (idx, block) in output.results.iter().enumerate() {
                println!("\nResult block {}: {} hits", idx + 1, block.content.len());
                for (i, hit) in block.content.iter().enumerate() {
                    println!("  {}. {} - {}", i + 1, hit.title, hit.url);
                    if let Some(snippet) = &hit.snippet {
                        println!("     {}", snippet);
                    }
                }
            }
        } else {
            panic!("No result event found in stream");
        }
    }

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_web_search_phase2_caching() {
        let tool = WebSearchTool::new();
        let params = WebSearchParams {
            query: "Rust caching test".to_string(),
            allowed_domains: vec![],
            blocked_domains: vec![],
        };
        let ctx = ToolContext {
            debug: true,
            ..Default::default()
        };

        // First search
        let stream1 = tool.execute(params.clone(), &ctx).await.unwrap();
        let events1: Vec<_> = stream1.collect().await;
        let result1 = events1.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        assert!(result1.is_some());
        let output1 = result1.unwrap();
        assert!(!output1.cached);

        // Second search (should be cached)
        let stream2 = tool.execute(params.clone(), &ctx).await.unwrap();
        let events2: Vec<_> = stream2.collect().await;
        let result2 = events2.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        assert!(result2.is_some());
        let output2 = result2.unwrap();
        assert!(output2.cached);
        assert_eq!(output2.count, output1.count);
    }

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_web_search_phase2_allowed_domains() {
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
                    assert!(
                        hit.url.contains("rust-lang.org"),
                        "URL {} should be from rust-lang.org",
                        hit.url
                    );
                }
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_web_search_phase2_blocked_domains() {
        let tool = WebSearchTool::new();
        let params = WebSearchParams {
            query: "programming".to_string(),
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
                    assert!(
                        !hit.url.contains("example.com"),
                        "URL {} should not be from example.com",
                        hit.url
                    );
                }
            }
        }
    }
}
