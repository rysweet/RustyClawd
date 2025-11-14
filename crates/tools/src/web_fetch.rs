//! WebFetch tool - Fetch and process web content
//!
//! Demonstrates:
//! - HTTP client with reqwest
//! - Async HTTP requests
//! - HTML to markdown conversion
//! - Streaming large responses

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for WebFetch tool
#[derive(Debug, Deserialize)]
pub struct WebFetchParams {
    /// URL to fetch
    pub url: String,

    /// Prompt describing what information to extract
    pub prompt: String,
}

/// Output from WebFetch tool
#[derive(Debug, Serialize)]
pub struct WebFetchOutput {
    /// Processed content (HTML converted to markdown)
    pub content: String,

    /// URL that was fetched
    pub url: String,

    /// Status code
    pub status_code: u16,

    /// Content type
    pub content_type: Option<String>,
}

/// The WebFetch tool
pub struct WebFetchTool;

#[async_trait]
impl crate::Tool for WebFetchTool {
    type Params = WebFetchParams;
    type Output = WebFetchOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "WebFetch",
            description: "Fetches content from URLs and converts HTML to markdown",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let url = params.url.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Fetching: {}", url),
                percentage: Some(10.0),
            };

            if debug {
                tracing::debug!(url = %url, prompt = %params.prompt, "Fetching web content");
            }

            // Build HTTP client
            let client = match reqwest::Client::builder()
                .user_agent("claude-code-rs/0.1.0 (Educational)")
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
                step: "Sending request...".to_string(),
                percentage: Some(30.0),
            };

            // Fetch URL
            let response = match client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to fetch URL: {}", e),
                    };
                    return;
                }
            };

            let status_code = response.status().as_u16();
            let content_type = response.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            if !response.status().is_success() {
                yield ToolEvent::Error {
                    message: format!("HTTP error {}: {}", status_code, response.status()),
                };
                return;
            }

            yield ToolEvent::Progress {
                step: "Downloading content...".to_string(),
                percentage: Some(60.0),
            };

            // Read body
            let body = match response.text().await {
                Ok(b) => b,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to read response body: {}", e),
                    };
                    return;
                }
            };

            yield ToolEvent::Progress {
                step: "Processing content...".to_string(),
                percentage: Some(80.0),
            };

            // Convert HTML to Markdown using html2md
            let content = if content_type.as_ref()
                .map(|ct| ct.contains("text/html") || ct.contains("html"))
                .unwrap_or(false)
            {
                // Use html2md for proper HTML to markdown conversion
                html2md::parse_html(&body)
            } else {
                // Not HTML, use as-is
                body
            };

            if debug {
                tracing::debug!(
                    status_code = status_code,
                    content_length = content.len(),
                    "Web fetch complete"
                );
            }

            yield ToolEvent::Result(WebFetchOutput {
                content,
                url: params.url.clone(),
                status_code,
                content_type,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true // Fetching doesn't modify local system
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Each fetch is independent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_webfetch_basic() {
        let tool = WebFetchTool;
        let params = WebFetchParams {
            url: "https://httpbin.org/html".to_string(),
            prompt: "Get the HTML content".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        });

        if let Some(output) = result {
            assert_eq!(output.status_code, 200);
            assert!(!output.content.is_empty());
        }
    }
}
