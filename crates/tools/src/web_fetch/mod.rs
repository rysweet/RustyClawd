//! WebFetch tool - Phase 2 Implementation
//!
//! Complete implementation of WebFetch tool with all Phase 2 features:
//! - HTTP client with manual redirect handling
//! - HTML to markdown conversion with truncation
//! - 15-minute TTL cache
//! - Pre-approved domains whitelist (80+ domains)
//! - Cross-domain redirect detection
//! - HTTP->HTTPS upgrade
//! - AI processing with prompts
//! - 10MB content size limit
//! - 100k character markdown truncation

mod cache;
mod http;
mod ssrf;
mod types;

pub use cache::WebFetchCache;
pub use types::*;

// Re-export pub(crate) items needed by the test module via super::*
#[cfg(test)]
pub(crate) use types::{CachedResponse, MAX_MARKDOWN_LENGTH};

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The WebFetch tool with Phase 2 features
pub struct WebFetchTool {
    cache: Arc<WebFetchCache>,
    client: reqwest::Client,
}

impl WebFetchTool {
    /// Create a new WebFetch tool with caching and a shared HTTP client
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("RustyClawd/0.1.0 (Educational)")
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(types::REQUEST_TIMEOUT_SECONDS))
            .build()
            .expect("Failed to build reqwest::Client");

        Self {
            cache: Arc::new(WebFetchCache::new()),
            client,
        }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::Tool for WebFetchTool {
    type Params = WebFetchParams;
    type Output = WebFetchOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "WebFetch",
            description:
                "Fetches web content with caching, converts to markdown, and processes with AI",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let cache = self.cache.clone();
        let ctx_clone = ctx.clone();
        let client = self.client.clone();
        let debug = ctx.debug;
        let url_param = params.url.clone();
        let prompt_param = params.prompt.clone();

        Ok(Box::pin(stream! {
            let start_time = Instant::now();
            let original_url = url_param.clone();

            // Upgrade HTTP to HTTPS
            let url = Self::upgrade_to_https(&original_url);

            if url != original_url && debug {
                tracing::debug!(original = %original_url, upgraded = %url, "Upgraded HTTP to HTTPS");
            }

            // SSRF protection: block private/internal IPs and dangerous schemes
            if let Err(reason) = ssrf::validate_url(&url) {
                yield ToolEvent::Error {
                    message: format!("SSRF protection: {}", reason),
                };
                return;
            }

            yield ToolEvent::Progress {
                step: format!("Fetching: {}", url),
                percentage: Some(10.0),
            };

            // Check cache
            let cache_key = format!("{}|{}", url, prompt_param);
            if let Some(cached) = cache.get(&cache_key).await {
                if debug {
                    tracing::debug!("Cache hit for {}", url);
                }

                yield ToolEvent::Progress {
                    step: "Using cached response...".to_string(),
                    percentage: Some(50.0),
                };

                // Process with AI
                yield ToolEvent::Progress {
                    step: "Processing with AI...".to_string(),
                    percentage: Some(80.0),
                };

                let result = match Self::process_with_ai(&cached.content, &prompt_param, &url, &ctx_clone).await {
                    Ok(r) => r,
                    Err(e) => {
                        yield ToolEvent::Error {
                            message: format!("AI processing failed: {}", e),
                        };
                        return;
                    }
                };

                let duration_ms = start_time.elapsed().as_millis() as u64;

                yield ToolEvent::Result(WebFetchOutput {
                    bytes: cached.bytes,
                    code: cached.status_code,
                    code_text: cached.status_text,
                    result,
                    duration_ms,
                    url: cached.final_url,
                });
                return;
            }

            // Check if domain is approved
            if !Self::is_domain_approved(&url) && debug {
                tracing::warn!(url = %url, "URL not in pre-approved domains");
            }

            if debug {
                tracing::debug!(url = %url, prompt = %prompt_param, "Fetching web content");
            }

            yield ToolEvent::Progress {
                step: "Sending request...".to_string(),
                percentage: Some(30.0),
            };

            // Fetch URL with manual redirect handling
            let (bytes, status_code, status_text, final_url) = match Self::fetch_url(&client, &url, debug).await {
                Ok(result) => result,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: e,
                    };
                    return;
                }
            };

            yield ToolEvent::Progress {
                step: "Converting to markdown...".to_string(),
                percentage: Some(60.0),
            };

            // Convert to markdown
            let content = match Self::to_markdown(&bytes, None) {
                Ok(c) => c,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to convert to markdown: {}", e),
                    };
                    return;
                }
            };

            if debug {
                tracing::debug!(
                    bytes = bytes.len(),
                    markdown_len = content.len(),
                    "Conversion complete"
                );
            }

            // Cache the response
            cache.insert(
                cache_key,
                CachedResponse {
                    content: content.clone(),
                    bytes: bytes.len(),
                    status_code,
                    status_text: status_text.clone(),
                    final_url: final_url.clone(),
                },
            ).await;

            yield ToolEvent::Progress {
                step: "Processing with AI...".to_string(),
                percentage: Some(80.0),
            };

            // Process with AI
            let result = match Self::process_with_ai(&content, &prompt_param, &final_url, &ctx_clone).await {
                Ok(r) => r,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("AI processing failed: {}", e),
                    };
                    return;
                }
            };

            let duration_ms = start_time.elapsed().as_millis() as u64;

            if debug {
                tracing::debug!(
                    status_code = status_code,
                    bytes = bytes.len(),
                    duration_ms = duration_ms,
                    "Web fetch complete"
                );
            }

            yield ToolEvent::Result(WebFetchOutput {
                bytes: bytes.len(),
                code: status_code,
                code_text: status_text,
                result,
                duration_ms,
                url: final_url,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
}

#[cfg(test)]
#[path = "../web_fetch_tests.rs"]
mod tests;
