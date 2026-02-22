//! WebFetch types - Data models and constants for web fetching
//!
//! Contains parameters, output structures, cache types, and configuration constants.

use serde::{Deserialize, Serialize};

/// Maximum content size in bytes (10 MB)
pub(crate) const MAX_CONTENT_SIZE: usize = 10 * 1024 * 1024;

/// Maximum markdown length in characters (100k)
pub(crate) const MAX_MARKDOWN_LENGTH: usize = 100_000;

/// Cache TTL in seconds (15 minutes)
pub(crate) const CACHE_TTL_SECONDS: u64 = 900;

/// HTTP request timeout in seconds
pub(crate) const REQUEST_TIMEOUT_SECONDS: u64 = 30;

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
