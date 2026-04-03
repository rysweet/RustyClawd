//! Azure AI Foundry backend.
//!
//! Implements authentication via Azure `DefaultAzureCredential` and provides
//! request/response translation using the same OpenAI-compatible format as
//! the Copilot backend. Azure AI Foundry's inference API supports both
//! OpenAI models (GPT-5.x) and third-party models (Claude, Llama, etc.)
//! through the same endpoint.
//!
//! Auth flow:
//! 1. Acquire a bearer token via `DefaultAzureCredential` (az login, managed identity, etc.)
//! 2. Send requests to `{endpoint}/openai/deployments/{deployment}/chat/completions`
//! 3. Token is cached and refreshed automatically before expiry
//!
//! The Azure AI Foundry inference API uses the OpenAI chat completions
//! format, so we reuse the OAI translation functions from the copilot module.

use reqwest::Client as HttpClient;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::copilot::{from_oai_response, from_oai_stream_chunk, to_oai_request, OaiChatResponse};
use super::error::{ClientError, ClientResult};
use super::request::CreateMessageRequest;
use super::response::{MessageResponse, StreamEvent};

const AZURE_COGNITIVE_SCOPE: &str = "https://cognitiveservices.azure.com/.default";

// ---------------------------------------------------------------------------
// Azure token management
// ---------------------------------------------------------------------------

/// Cached Azure AD bearer token with expiry tracking.
#[derive(Clone)]
struct CachedToken {
    value: String,
    expires_at: std::time::Instant,
}

/// Azure AI Foundry authentication.
///
/// Supports two auth modes:
/// - **API key**: Set via `AZURE_OPENAI_API_KEY` env var or passed directly
/// - **Bearer token**: Acquired via `az account get-access-token` (DefaultAzureCredential equivalent)
///
/// Supports multiple deployments for load balancing: pass comma-separated
/// deployment names and requests will round-robin across them.
#[derive(Clone)]
pub struct AzureAuth {
    endpoint: String,
    deployments: Vec<String>,
    api_version: String,
    api_key: Option<String>,
    cached_token: Arc<RwLock<Option<CachedToken>>>,
    counter: Arc<std::sync::atomic::AtomicUsize>,
}

impl AzureAuth {
    /// Create a new Azure auth handle.
    ///
    /// `deployment` may be comma-separated for round-robin load balancing
    /// (e.g. "gpt-54-skwaq,gpt-54-skwaq-2,gpt-54-skwaq-3").
    ///
    /// # Panics
    /// Panics if `deployment` is empty or contains only whitespace/commas.
    pub fn new(endpoint: &str, deployment: &str, api_version: &str) -> Self {
        validate_azure_endpoint(endpoint);
        let api_key = std::env::var("AZURE_OPENAI_API_KEY").ok();
        let deployments: Vec<String> = deployment
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert!(
            !deployments.is_empty(),
            "Azure deployment name(s) must not be empty"
        );
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            deployments,
            api_version: api_version.to_string(),
            api_key,
            cached_token: Arc::new(RwLock::new(None)),
            counter: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Create with an explicit API key (skips bearer token auth entirely).
    ///
    /// # Panics
    /// Panics if `deployment` is empty or contains only whitespace/commas.
    pub fn with_api_key(endpoint: &str, deployment: &str, api_version: &str, key: &str) -> Self {
        let deployments: Vec<String> = deployment
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert!(
            !deployments.is_empty(),
            "Azure deployment name(s) must not be empty"
        );
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            deployments,
            api_version: api_version.to_string(),
            api_key: Some(key.to_string()),
            cached_token: Arc::new(RwLock::new(None)),
            counter: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Whether this auth uses API key (vs bearer token).
    pub fn uses_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    /// Get a valid bearer token, refreshing if expired or absent.
    ///
    /// Uses a read lock for the fast path (cached token still valid) and
    /// upgrades to a write lock only when a refresh is needed. This avoids
    /// serializing concurrent requests on the common case.
    pub async fn get_token(&self) -> ClientResult<String> {
        // Fast path: read lock to check cache
        {
            let cache = self.cached_token.read().await;
            if let Some(ref tok) = *cache {
                if tok.expires_at > std::time::Instant::now() + std::time::Duration::from_secs(60) {
                    return Ok(tok.value.clone());
                }
            }
        }

        // Slow path: write lock to refresh
        let mut cache = self.cached_token.write().await;

        // Double-check after acquiring write lock (another task may have refreshed)
        if let Some(ref tok) = *cache {
            if tok.expires_at > std::time::Instant::now() + std::time::Duration::from_secs(60) {
                return Ok(tok.value.clone());
            }
        }

        // Acquire a new token via az CLI (DefaultAzureCredential equivalent)
        let token = acquire_azure_token().await?;
        // Azure AD tokens typically expire in 3600s; we use a conservative 3540s
        // (60s buffer) since we don't parse the actual exp claim from the JWT.
        let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(3540);

        *cache = Some(CachedToken {
            value: token.clone(),
            expires_at,
        });

        Ok(token)
    }

    /// Pick the next deployment in round-robin order, returning (url, deployment_name).
    pub fn next_request_target(&self) -> (String, String) {
        let idx = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let deployment = &self.deployments[idx % self.deployments.len()];
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint, deployment, self.api_version
        );
        (url, deployment.clone())
    }

    /// Build the chat completions URL (uses round-robin).
    pub fn chat_url(&self) -> String {
        self.next_request_target().0
    }

    /// Get the first deployment name (for display/logging).
    pub fn deployment(&self) -> &str {
        &self.deployments[0]
    }

    /// Number of deployments configured.
    pub fn deployment_count(&self) -> usize {
        self.deployments.len()
    }

    /// Invalidate the cached bearer token so the next `get_token()` call
    /// acquires a fresh one.
    ///
    /// This should be called when a request receives a 401 Unauthorized
    /// response, indicating the cached token has expired or been revoked.
    pub async fn invalidate_cached_token(&self) {
        let mut cache = self.cached_token.write().await;
        *cache = None;
    }
}

/// Validate that an Azure endpoint looks like a reasonable HTTPS URL.
///
/// # Panics
/// Panics if the endpoint is empty, uses a non-HTTPS scheme, or has no host.
fn validate_azure_endpoint(endpoint: &str) {
    let trimmed = endpoint.trim().trim_end_matches('/');
    assert!(!trimmed.is_empty(), "Azure endpoint must not be empty");
    assert!(
        trimmed.starts_with("https://"),
        "Azure endpoint must use HTTPS (got: {trimmed})"
    );
    let host = trimmed.strip_prefix("https://").unwrap_or("");
    assert!(
        host.contains('.') && !host.starts_with('.'),
        "Azure endpoint has an invalid hostname"
    );
}

/// Acquire an Azure AD token using `az account get-access-token`.
///
/// This is the CLI-based equivalent of `DefaultAzureCredential` — it works
/// when the user has run `az login`. For managed identity or service principal
/// scenarios, the AZURE_CLIENT_ID / AZURE_TENANT_ID / AZURE_CLIENT_SECRET
/// environment variables can be used with the azure_identity crate (future).
async fn acquire_azure_token() -> ClientResult<String> {
    // Try az CLI first (most common for dev scenarios).
    // 30s timeout prevents indefinite hangs if Azure identity servers are unreachable.
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new("az")
            .args([
                "account",
                "get-access-token",
                "--resource",
                AZURE_COGNITIVE_SCOPE
                    .strip_suffix("/.default")
                    .unwrap_or(AZURE_COGNITIVE_SCOPE),
                "--query",
                "accessToken",
                "--output",
                "tsv",
            ])
            .output(),
    )
    .await
    .map_err(|_| {
        ClientError::Unknown(
            "Azure CLI token acquisition timed out after 30s. \
             Check network connectivity to Microsoft identity servers."
                .to_string(),
        )
    })?
    .map_err(|e| {
        ClientError::Unknown(format!(
            "Failed to run `az account get-access-token`: {e}. \
             Ensure Azure CLI is installed and you have run `az login`."
        ))
    })?;

    if !output.status.success() {
        return Err(ClientError::Unknown(
            "Azure CLI token acquisition failed. Ensure Azure CLI is installed \
             and you have run `az login`."
                .to_string(),
        ));
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        return Err(ClientError::Unknown(
            "Azure CLI returned an empty token. Run `az login` to authenticate.".to_string(),
        ));
    }

    Ok(token)
}

// ---------------------------------------------------------------------------
// Non-streaming request
// ---------------------------------------------------------------------------

/// Execute a non-streaming chat completion against Azure AI Foundry.
pub async fn create_message(
    http_client: &HttpClient,
    auth: &AzureAuth,
    request: &CreateMessageRequest,
) -> ClientResult<MessageResponse> {
    let (url, deployment) = auth.next_request_target();
    let mut oai_request = to_oai_request(request);
    oai_request.model = deployment;
    oai_request.max_completion_tokens = oai_request.max_tokens.take();

    let mut req_builder = http_client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json");

    if let Some(ref key) = auth.api_key {
        req_builder = req_builder.header("api-key", key.as_str());
    } else {
        let token = auth.get_token().await?;
        req_builder = req_builder.header("Authorization", format!("Bearer {token}"));
    }

    let response = req_builder.json(&oai_request).send().await?;

    if !response.status().is_success() {
        return Err(ClientError::from_response(response).await);
    }

    let oai_response: OaiChatResponse = response.json().await?;
    if oai_response.choices.is_empty() {
        return Err(ClientError::Unknown(
            "Azure API returned a response with no choices".to_string(),
        ));
    }
    Ok(from_oai_response(oai_response))
}

// ---------------------------------------------------------------------------
// Streaming request
// ---------------------------------------------------------------------------

/// Execute a streaming chat completion against Azure AI Foundry.
pub async fn create_message_stream(
    http_client: &HttpClient,
    auth: &AzureAuth,
    request: &CreateMessageRequest,
) -> ClientResult<impl futures::Stream<Item = ClientResult<StreamEvent>>> {
    let (url, deployment) = auth.next_request_target();
    let mut oai_request = to_oai_request(request);
    oai_request.model = deployment;
    oai_request.max_completion_tokens = oai_request.max_tokens.take();
    oai_request.stream = true;

    let mut req_builder = http_client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream");

    if let Some(ref key) = auth.api_key {
        req_builder = req_builder.header("api-key", key.as_str());
    } else {
        let token = auth.get_token().await?;
        req_builder = req_builder.header("Authorization", format!("Bearer {token}"));
    }

    let response = req_builder.json(&oai_request).send().await?;

    if !response.status().is_success() {
        return Err(ClientError::from_response(response).await);
    }

    // Parse the SSE stream — same format as Copilot (OpenAI SSE)
    use super::copilot::OaiStreamChunk;

    let byte_stream = response.bytes_stream();
    let block_index: u32 = 0;
    let pending_events: Vec<StreamEvent> = Vec::new();

    Ok(futures::stream::unfold(
        (byte_stream, String::new(), block_index, pending_events),
        move |(mut stream, mut buffer, mut bi, mut pending)| async move {
            use futures::TryStreamExt;

            // Yield any buffered events from a previous chunk first
            if let Some(event) = pending.pop() {
                return Some((Ok(event), (stream, buffer, bi, pending)));
            }

            loop {
                // Try to parse any complete SSE events from the buffer
                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim_end_matches('\r').to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            return None;
                        }
                        match serde_json::from_str::<OaiStreamChunk>(data) {
                            Ok(chunk) => {
                                let mut events = from_oai_stream_chunk(&chunk, &mut bi);
                                if let Some(first) = events.first().cloned() {
                                    // Buffer remaining events for subsequent yields
                                    if events.len() > 1 {
                                        events.remove(0);
                                        events.reverse();
                                        pending = events;
                                    }
                                    return Some((Ok(first), (stream, buffer, bi, pending)));
                                }
                            }
                            Err(e) => {
                                tracing::debug!("Failed to parse Azure SSE chunk: {e}");
                            }
                        }
                    }
                }

                // Read more data from the stream
                match stream.try_next().await {
                    Ok(Some(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Ok(None) => return None,
                    Err(e) => {
                        return Some((
                            Err(ClientError::Unknown(format!("Stream error: {e}"))),
                            (stream, buffer, bi, pending),
                        ))
                    }
                }
            }
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_azure_auth_url_construction() {
        let auth = AzureAuth::new(
            "https://myresource.cognitiveservices.azure.com/",
            "gpt-51-skwaq",
            "2024-10-21",
        );
        assert_eq!(
            auth.chat_url(),
            "https://myresource.cognitiveservices.azure.com/openai/deployments/gpt-51-skwaq/chat/completions?api-version=2024-10-21"
        );
    }

    #[test]
    fn test_azure_auth_trailing_slash_normalized() {
        let auth = AzureAuth::new(
            "https://test.cognitiveservices.azure.com",
            "gpt-54",
            "2024-10-21",
        );
        // No double slash
        assert!(!auth.chat_url().contains("azure.com//"));
    }

    #[test]
    fn test_deployment_accessor() {
        let auth = AzureAuth::new("https://x.azure.com", "my-deploy", "2024-10-21");
        assert_eq!(auth.deployment(), "my-deploy");
    }

    #[test]
    fn test_multi_deployment_round_robin() {
        let auth = AzureAuth::new("https://x.azure.com", "d1,d2,d3", "2024-10-21");
        assert_eq!(auth.deployment_count(), 3);
        let (url1, dep1) = auth.next_request_target();
        let (url2, dep2) = auth.next_request_target();
        let (url3, dep3) = auth.next_request_target();
        let (url4, dep4) = auth.next_request_target();
        assert_eq!(dep1, "d1");
        assert_eq!(dep2, "d2");
        assert_eq!(dep3, "d3");
        assert_eq!(dep4, "d1"); // wraps around
        assert!(url1.contains("/d1/"));
        assert!(url2.contains("/d2/"));
        assert!(url3.contains("/d3/"));
        assert!(url4.contains("/d1/")); // wraps around
    }

    #[test]
    #[should_panic(expected = "must not be empty")]
    fn test_empty_deployment_panics() {
        AzureAuth::new("https://x.azure.com", "", "2024-10-21");
    }

    #[test]
    #[should_panic(expected = "must not be empty")]
    fn test_whitespace_only_deployment_panics() {
        AzureAuth::new("https://x.azure.com", " , , ", "2024-10-21");
    }

    #[tokio::test]
    async fn test_invalidate_cached_token() {
        let auth = AzureAuth::new("https://x.azure.com", "deploy", "2024-10-21");
        // Manually inject a cached token
        {
            let mut cache = auth.cached_token.write().await;
            *cache = Some(CachedToken {
                value: "old-token".to_string(),
                expires_at: std::time::Instant::now() + std::time::Duration::from_secs(3600),
            });
        }
        // Verify token is cached
        assert!(auth.cached_token.read().await.is_some());

        // Invalidate and verify it's cleared
        auth.invalidate_cached_token().await;
        assert!(auth.cached_token.read().await.is_none());
    }

    #[test]
    #[should_panic(expected = "must use HTTPS")]
    fn test_http_endpoint_panics() {
        AzureAuth::new("http://x.azure.com", "deploy", "2024-10-21");
    }

    #[test]
    #[should_panic(expected = "must not be empty")]
    fn test_empty_endpoint_panics() {
        AzureAuth::new("", "deploy", "2024-10-21");
    }
}
