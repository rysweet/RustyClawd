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

/// Azure AI Foundry authentication using DefaultAzureCredential.
///
/// Acquires tokens from the Azure identity chain (CLI, managed identity,
/// environment variables, etc.) and caches them until near-expiry.
#[derive(Clone)]
pub struct AzureAuth {
    endpoint: String,
    deployment: String,
    api_version: String,
    cached_token: Arc<RwLock<Option<CachedToken>>>,
}

impl AzureAuth {
    /// Create a new Azure auth handle.
    ///
    /// Does NOT acquire a token yet — that happens lazily on first request.
    pub fn new(endpoint: &str, deployment: &str, api_version: &str) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            deployment: deployment.to_string(),
            api_version: api_version.to_string(),
            cached_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Get a valid bearer token, refreshing if expired or absent.
    pub async fn get_token(&self) -> ClientResult<String> {
        // Check cache first
        {
            let cache = self.cached_token.read().await;
            if let Some(ref tok) = *cache {
                // Refresh 60s before expiry to avoid edge-case failures
                if tok.expires_at > std::time::Instant::now() + std::time::Duration::from_secs(60) {
                    return Ok(tok.value.clone());
                }
            }
        }

        // Acquire a new token via az CLI (DefaultAzureCredential equivalent)
        let token = acquire_azure_token().await?;
        let expires_at = std::time::Instant::now() + std::time::Duration::from_secs(3000);

        {
            let mut cache = self.cached_token.write().await;
            *cache = Some(CachedToken {
                value: token.clone(),
                expires_at,
            });
        }

        Ok(token)
    }

    /// Build the chat completions URL for this deployment.
    pub fn chat_url(&self) -> String {
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint, self.deployment, self.api_version
        )
    }

    /// Get the deployment name (used as model name in requests).
    pub fn deployment(&self) -> &str {
        &self.deployment
    }
}

/// Acquire an Azure AD token using `az account get-access-token`.
///
/// This is the CLI-based equivalent of `DefaultAzureCredential` — it works
/// when the user has run `az login`. For managed identity or service principal
/// scenarios, the AZURE_CLIENT_ID / AZURE_TENANT_ID / AZURE_CLIENT_SECRET
/// environment variables can be used with the azure_identity crate (future).
async fn acquire_azure_token() -> ClientResult<String> {
    // Try az CLI first (most common for dev scenarios)
    let output = tokio::process::Command::new("az")
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
        .output()
        .await
        .map_err(|e| {
            ClientError::Unknown(format!(
                "Failed to run `az account get-access-token`: {e}. \
                 Ensure Azure CLI is installed and you have run `az login`."
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ClientError::Unknown(format!(
            "Azure CLI token acquisition failed: {stderr}"
        )));
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
    let token = auth.get_token().await?;
    let mut oai_request = to_oai_request(request);
    // Use the deployment name as the model
    oai_request.model = auth.deployment().to_string();

    let response = http_client
        .post(auth.chat_url())
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&oai_request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(ClientError::from_response(response).await);
    }

    let oai_response: OaiChatResponse = response.json().await?;
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
    let token = auth.get_token().await?;
    let mut oai_request = to_oai_request(request);
    oai_request.model = auth.deployment().to_string();
    oai_request.stream = true;

    let response = http_client
        .post(auth.chat_url())
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .json(&oai_request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(ClientError::from_response(response).await);
    }

    // Parse the SSE stream — same format as Copilot (OpenAI SSE)
    use super::copilot::OaiStreamChunk;

    let byte_stream = response.bytes_stream();
    let block_index: u32 = 0;

    Ok(futures::stream::unfold(
        (byte_stream, String::new(), block_index),
        move |(mut stream, mut buffer, mut bi)| async move {
            use futures::TryStreamExt;
            loop {
                // Try to parse any complete SSE events from the buffer
                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim_end_matches('\r').to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            return None;
                        }
                        if let Ok(chunk) = serde_json::from_str::<OaiStreamChunk>(data) {
                            let events = from_oai_stream_chunk(&chunk, &mut bi);
                            if let Some(event) = events.into_iter().next() {
                                return Some((Ok(event), (stream, buffer, bi)));
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
                            (stream, buffer, bi),
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
}
