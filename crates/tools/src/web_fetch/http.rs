//! WebFetch HTTP operations - URL handling, fetching, and content conversion
//!
//! Implements HTTP fetching with manual redirect handling, domain approval checks,
//! HTTP-to-HTTPS upgrade, HTML-to-markdown conversion, and AI prompt formatting.

use super::types::{MAX_CONTENT_SIZE, MAX_MARKDOWN_LENGTH};
use super::WebFetchTool;
use crate::web_fetch_domains::PRE_APPROVED_DOMAINS;
use crate::ToolContext;
use bytes::Bytes;
use url::Url;

impl WebFetchTool {
    /// Check if domain is pre-approved
    pub(crate) fn is_domain_approved(url: &str) -> bool {
        if let Ok(parsed) = Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                return PRE_APPROVED_DOMAINS.contains(&host);
            }
        }
        false
    }

    /// Upgrade HTTP to HTTPS
    pub(crate) fn upgrade_to_https(url: &str) -> String {
        if url.starts_with("http://") {
            url.replacen("http://", "https://", 1)
        } else {
            url.to_string()
        }
    }

    /// Extract domain from URL
    pub(crate) fn get_domain(url: &str) -> Option<String> {
        Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
    }

    /// Check if redirect is same domain
    pub(crate) fn is_same_domain(original: &str, redirect: &str) -> bool {
        match (Self::get_domain(original), Self::get_domain(redirect)) {
            (Some(orig), Some(redir)) => orig == redir,
            _ => false,
        }
    }

    /// Fetch URL with manual redirect handling
    pub(crate) async fn fetch_url(
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

                    // SSRF protection: validate redirect target
                    if let Err(reason) = super::ssrf::validate_url(&redirect_url) {
                        return Err(format!(
                            "SSRF protection: redirect to blocked target: {}",
                            reason
                        ));
                    }

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
    pub(crate) fn to_markdown(bytes: &[u8], content_type: Option<&str>) -> Result<String, String> {
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
    pub(crate) async fn process_with_ai(
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
