//! WebFetch cache - TTL-based response caching using moka
//!
//! Provides a 15-minute TTL cache for web fetch responses to avoid
//! redundant network requests.

use super::types::{CachedResponse, CACHE_TTL_SECONDS};
use moka::future::Cache;
use std::time::Duration;

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
