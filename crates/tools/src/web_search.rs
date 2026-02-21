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

use crate::web_search_parse::parse_search_results;
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use moka::future::Cache;
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
        model.contains("opus-4") || model.contains("sonnet-4") || model.contains("haiku-4")
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

            yield ToolEvent::Progress {
                step: "Processing search results...".to_string(),
                percentage: Some(60.0),
            };

            // Parse streaming events via extracted helper
            let pinned_stream = Box::pin(stream);
            let results = match parse_search_results(pinned_stream, debug).await {
                Ok(r) => r,
                Err(e) => {
                    yield ToolEvent::Error { message: e };
                    return;
                }
            };

            let count: usize = results.iter().map(|r| r.content.len()).sum();
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
#[path = "web_search_tests.rs"]
mod tests;
