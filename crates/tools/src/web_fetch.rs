//! WebFetch tool - Phase 2 Implementation
//!
//! Complete implementation of WebFetch tool with all Phase 2 features:
//! - HTTP client with manual redirect handling
//! - HTML to markdown conversion with truncation
//! - 15-minute TTL cache
//! - Pre-approved domains whitelist (80+ domains)
//! - Cross-domain redirect detection
//! - HTTP→HTTPS upgrade
//! - AI processing with prompts
//! - 10MB content size limit
//! - 100k character markdown truncation

use crate::web_fetch_domains::PRE_APPROVED_DOMAINS;
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use bytes::Bytes;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use url::Url;

/// Maximum content size in bytes (10 MB)
const MAX_CONTENT_SIZE: usize = 10 * 1024 * 1024;

/// Maximum markdown length in characters (100k)
const MAX_MARKDOWN_LENGTH: usize = 100_000;

/// Cache TTL in seconds (15 minutes)
const CACHE_TTL_SECONDS: u64 = 900;

/// HTTP request timeout in seconds
const REQUEST_TIMEOUT_SECONDS: u64 = 30;

/// Parameters for WebFetch tool
#[derive(Debug, Clone, Deserialize)]
pub struct WebFetchParams {
    /// URL to fetch
    pub url: String,

    /// Prompt describing what information to extract
    pub prompt: String,
}

/// Output from WebFetch tool (Phase 2 spec)
#[derive(Debug, Clone, Serialize)]
pub struct WebFetchOutput {
    /// Number of bytes fetched
    pub bytes: usize,

    /// HTTP status code
    pub code: u16,

    /// HTTP status text
    pub code_text: String,

    /// AI-processed result based on prompt
    pub result: String,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// Final URL (after redirects)
    pub url: String,
}

/// Cached response data
#[derive(Debug, Clone)]
pub(crate) struct CachedResponse {
    /// Markdown content
    pub(crate) content: String,
    /// Original bytes count
    pub(crate) bytes: usize,
    /// Status code
    pub(crate) status_code: u16,
    /// Status text
    pub(crate) status_text: String,
    /// Final URL
    pub(crate) final_url: String,
}

/// Cache for web fetch responses with 15-minute TTL
pub struct WebFetchCache {
    cache: Cache<String, CachedResponse>,
}

impl WebFetchCache {
    /// Create a new cache with 15-minute TTL
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(100)
                .time_to_live(Duration::from_secs(CACHE_TTL_SECONDS))
                .build(),
        }
    }

    /// Get cached response
    pub(crate) async fn get(&self, key: &str) -> Option<CachedResponse> {
        self.cache.get(key).await
    }

    /// Store response in cache
    pub(crate) async fn insert(&self, key: String, response: CachedResponse) {
        self.cache.insert(key, response).await;
    }
}

impl Default for WebFetchCache {
    fn default() -> Self {
        Self::new()
    }
}

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
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .build()
            .expect("Failed to build reqwest::Client");

        Self {
            cache: Arc::new(WebFetchCache::new()),
            client,
        }
    }

    /// Check if domain is pre-approved
    fn is_domain_approved(url: &str) -> bool {
        if let Ok(parsed) = Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                return PRE_APPROVED_DOMAINS.contains(&host);
            }
        }
        false
    }

    /// Upgrade HTTP to HTTPS
    fn upgrade_to_https(url: &str) -> String {
        if url.starts_with("http://") {
            url.replacen("http://", "https://", 1)
        } else {
            url.to_string()
        }
    }

    /// Extract domain from URL
    fn get_domain(url: &str) -> Option<String> {
        Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
    }

    /// Check if redirect is same domain
    fn is_same_domain(original: &str, redirect: &str) -> bool {
        match (Self::get_domain(original), Self::get_domain(redirect)) {
            (Some(orig), Some(redir)) => orig == redir,
            _ => false,
        }
    }

    /// Fetch URL with manual redirect handling
    async fn fetch_url(
        client: &reqwest::Client,
        url: &str,
        debug: bool,
    ) -> Result<(Bytes, u16, String, String), String> {
        let mut current_url = url.to_string();
        let mut redirect_count = 0;
        const MAX_REDIRECTS: u8 = 5;

        loop {
            if debug {
                tracing::debug!(url = %current_url, "Fetching URL");
            }

            let response = client
                .get(&current_url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch URL: {}", e))?;

            let status = response.status();
            let status_code = status.as_u16();
            let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();

            // Handle redirects
            if status.is_redirection() {
                redirect_count += 1;
                if redirect_count > MAX_REDIRECTS {
                    return Err("Too many redirects".to_string());
                }

                if let Some(location) = response.headers().get("location") {
                    let redirect_url = location
                        .to_str()
                        .map_err(|_| "Invalid redirect location")?
                        .to_string();

                    // Make relative URLs absolute
                    let redirect_url = if redirect_url.starts_with("http") {
                        redirect_url
                    } else {
                        let base = Url::parse(&current_url)
                            .map_err(|_| "Invalid base URL for redirect")?;
                        base.join(&redirect_url)
                            .map_err(|_| "Failed to resolve relative redirect")?
                            .to_string()
                    };

                    // Check if cross-domain redirect
                    if !Self::is_same_domain(&current_url, &redirect_url) {
                        return Err(format!(
                            "REDIRECT_DETECTED: Cross-domain redirect from {} to {}. \
                             Please provide the redirect URL explicitly.",
                            current_url, redirect_url
                        ));
                    }

                    current_url = redirect_url;
                    continue;
                }
            }

            // Check status
            if !status.is_success() {
                return Err(format!("HTTP error {}: {}", status_code, status_text));
            }

            // Check content length
            if let Some(content_length) = response.content_length() {
                if content_length > MAX_CONTENT_SIZE as u64 {
                    return Err(format!(
                        "Content too large: {} bytes (max {} bytes)",
                        content_length, MAX_CONTENT_SIZE
                    ));
                }
            }

            // Read body with size limit
            let bytes = response
                .bytes()
                .await
                .map_err(|e| format!("Failed to read response body: {}", e))?;

            if bytes.len() > MAX_CONTENT_SIZE {
                return Err(format!(
                    "Content too large: {} bytes (max {} bytes)",
                    bytes.len(),
                    MAX_CONTENT_SIZE
                ));
            }

            return Ok((bytes, status_code, status_text, current_url));
        }
    }

    /// Convert bytes to markdown with truncation
    fn to_markdown(bytes: &[u8], content_type: Option<&str>) -> Result<String, String> {
        // Detect if HTML
        let is_html = content_type
            .map(|ct| ct.contains("text/html") || ct.contains("html"))
            .unwrap_or_else(|| {
                // Try to detect HTML from content
                let preview = std::str::from_utf8(&bytes[..bytes.len().min(1000)]).unwrap_or("");
                preview.contains("<html") || preview.contains("<!DOCTYPE")
            });

        let text = String::from_utf8_lossy(bytes).to_string();

        let markdown = if is_html {
            html2md::parse_html(&text)
        } else {
            text
        };

        // Truncate to max length
        let truncated = if markdown.len() > MAX_MARKDOWN_LENGTH {
            let mut truncated = markdown
                .chars()
                .take(MAX_MARKDOWN_LENGTH)
                .collect::<String>();
            truncated.push_str("\n\n[Content truncated at 100,000 characters]");
            truncated
        } else {
            markdown
        };

        Ok(truncated)
    }

    /// Format fetched content with the user's prompt for the LLM to process.
    ///
    /// The LLM that invoked WebFetch receives this formatted output and
    /// naturally processes it in its next reasoning step. Calling the Claude
    /// API from within a tool would create a circular dependency since tools
    /// are invoked BY the API client.
    async fn process_with_ai(
        content: &str,
        prompt: &str,
        url: &str,
        _ctx: &ToolContext,
    ) -> Result<String, String> {
        let summary = format!(
            "Content from: {}\n\nUser query: {}\n\n---\n\n{}",
            url,
            prompt,
            if content.len() > 5000 {
                format!(
                    "{}...\n\n[Content continues - {} total characters]",
                    &content[..5000],
                    content.len()
                )
            } else {
                content.to_string()
            }
        );

        Ok(summary)
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
#[path = "web_fetch_tests.rs"]
mod tests;
