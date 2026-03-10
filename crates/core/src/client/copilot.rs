//! GitHub Copilot API backend (with GitHub Models API fallback)
//!
//! Implements authentication via GitHub token and provides request/response
//! translation between our internal Anthropic-native types and the
//! OpenAI-compatible Chat Completions API.
//!
//! Auth flow:
//! 1. Get GitHub token from `gh auth token`, GITHUB_TOKEN env, or config files
//! 2. Try the Copilot API at `api.githubcopilot.com` with the raw token
//! 3. If that fails, fall back to GitHub Models API at `models.github.ai`
//!    which accepts the same token (no exchange needed)
//!
//! The GitHub Models fallback is useful when a GitHub token has Models API
//! access but not Copilot-specific access.
//!
//! Reference: <https://docs.rs/copilot-client/latest/copilot_client/>

use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use zeroize::Zeroize;

use super::error::{ClientError, ClientResult};
use super::request::CreateMessageRequest;
use super::response::{
    ContentBlockStart, ContentDelta, MessageResponse, StreamEvent, Usage as AnthropicUsage,
};
use super::types::{ContentBlock, MessageContent, Role};

// ---------------------------------------------------------------------------
// Copilot API endpoints
// ---------------------------------------------------------------------------

const COPILOT_CHAT_URL: &str = "https://api.githubcopilot.com/chat/completions";
const COPILOT_MODELS_URL: &str = "https://api.githubcopilot.com/models";
const COPILOT_USER_AGENT: &str = concat!(
    "RustyClawd/",
    env!("CARGO_PKG_VERSION"),
    " (GitHub Copilot Integration)"
);

// GitHub Models API fallback endpoints
const GITHUB_MODELS_CHAT_URL: &str = "https://models.github.ai/inference/chat/completions";
const GITHUB_MODELS_LIST_URL: &str = "https://models.github.ai/inference/models";
/// Model name prefix required by the GitHub Models API (e.g., "openai/gpt-4o").
const GITHUB_MODELS_PREFIX: &str = "openai/";

// ---------------------------------------------------------------------------
// GitHub token acquisition
// ---------------------------------------------------------------------------

/// Get a GitHub token for Copilot authentication.
///
/// Priority chain:
/// 1. GITHUB_TOKEN environment variable
/// 2. `gh auth token` CLI command
/// 3. Config files (~/.config/github-copilot/hosts.json, apps.json)
pub async fn get_github_token() -> ClientResult<String> {
    // Try 1: Environment variable
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            tracing::debug!("Using GitHub token from GITHUB_TOKEN env var");
            return Ok(token);
        }
    }

    // Try 2: gh auth token
    match tokio::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .await
    {
        Ok(output) if output.status.success() => {
            let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !token.is_empty() {
                tracing::debug!("Using GitHub token from `gh auth token`");
                return Ok(token);
            }
        }
        Ok(output) => {
            tracing::debug!(
                "gh auth token failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => {
            tracing::debug!("gh command not found or failed: {}", e);
        }
    }

    // Try 3: Config files
    if let Some(token) = try_copilot_config_files().await {
        tracing::debug!("Using GitHub token from Copilot config files");
        return Ok(token);
    }

    Err(ClientError::Unknown(
        "GitHub token not found. To use the Copilot backend, authenticate via one of:\n  \
         1. Run: gh auth login\n  \
         2. Run: gh auth refresh --hostname github.com --scopes copilot\n  \
         3. Set GITHUB_TOKEN environment variable"
            .to_string(),
    ))
}

/// Try to read GitHub token from Copilot config files.
async fn try_copilot_config_files() -> Option<String> {
    let config_dir = dirs_config_path()?;

    for filename in &["hosts.json", "apps.json"] {
        let path = config_dir.join(filename);
        if let Ok(content) = tokio::fs::read_to_string(&path).await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // hosts.json format: {"github.com": {"oauth_token": "..."}}
                if let Some(token) = json
                    .get("github.com")
                    .and_then(|v| v.get("oauth_token"))
                    .and_then(|v| v.as_str())
                {
                    if !token.is_empty() {
                        return Some(token.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Get the github-copilot config directory path.
fn dirs_config_path() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".config/github-copilot"))
    }
    #[cfg(windows)]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|h| PathBuf::from(h).join("github-copilot"))
    }
}

// ---------------------------------------------------------------------------
// Copilot token exchange
// ---------------------------------------------------------------------------

/// Which API endpoint is being used for chat completions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopilotEndpoint {
    /// Standard Copilot API at api.githubcopilot.com
    Copilot,
    /// GitHub Models API at models.github.ai (fallback)
    GitHubModels,
}

/// Manages GitHub Copilot API authentication.
///
/// The GitHub token is wrapped for zeroization on drop, consistent
/// with how the Anthropic API key is handled.
#[derive(Clone)]
pub struct CopilotAuth {
    github_token: Arc<secrecy::SecretBox<GhToken>>,
    http_client: HttpClient,
    /// Which endpoint was successfully validated during connect.
    endpoint: CopilotEndpoint,
}

/// GitHub token with automatic zeroization on drop.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
struct GhToken(String);

impl secrecy::CloneableSecret for GhToken {}

impl GhToken {
    fn expose(&self) -> &str {
        &self.0
    }
}

impl CopilotAuth {
    /// Create a CopilotAuth and eagerly validate credentials by hitting the
    /// models endpoint. Surfaces auth errors at startup rather than deferring
    /// them to the first API call.
    ///
    /// Tries the standard Copilot API first. If that returns 404 (some GitHub
    /// token types lack Copilot access), falls back to the GitHub Models API
    /// at models.github.ai which accepts the same raw GitHub token.
    pub async fn connect(github_token: String, http_client: HttpClient) -> ClientResult<Self> {
        // Try 1: Standard Copilot API
        tracing::debug!("Validating Copilot credentials against models endpoint");
        let response = http_client
            .get(COPILOT_MODELS_URL)
            .header("Authorization", format!("Bearer {}", github_token))
            .header("Accept", "application/json")
            .header("User-Agent", COPILOT_USER_AGENT)
            .send()
            .await
            .map_err(|e| {
                ClientError::NetworkError(format!("Failed to validate Copilot credentials: {}", e))
            })?;

        if response.status().is_success() {
            // Discard the response body (we only needed to validate auth)
            let _ = response.bytes().await;
            tracing::info!("Copilot credentials validated successfully");
            return Ok(Self {
                github_token: Arc::new(secrecy::SecretBox::new(Box::new(GhToken(
                    github_token,
                )))),
                http_client,
                endpoint: CopilotEndpoint::Copilot,
            });
        }

        let copilot_status = response.status();
        let copilot_body = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        tracing::warn!(
            "Copilot models endpoint returned HTTP {} — trying GitHub Models API fallback",
            copilot_status
        );

        // Try 2: GitHub Models API fallback
        let fallback_response = http_client
            .get(GITHUB_MODELS_LIST_URL)
            .header("Authorization", format!("Bearer {}", github_token))
            .header("Accept", "application/json")
            .header("User-Agent", COPILOT_USER_AGENT)
            .send()
            .await
            .map_err(|e| {
                ClientError::NetworkError(format!(
                    "Failed to validate GitHub Models credentials: {}",
                    e
                ))
            })?;

        if fallback_response.status().is_success() {
            let _ = fallback_response.bytes().await;
            tracing::info!(
                "GitHub Models API credentials validated — using models.github.ai as backend"
            );
            return Ok(Self {
                github_token: Arc::new(secrecy::SecretBox::new(Box::new(GhToken(
                    github_token,
                )))),
                http_client,
                endpoint: CopilotEndpoint::GitHubModels,
            });
        }

        // Both failed — report the original Copilot error
        let sanitized = super::error::sanitize_error(&copilot_body);
        Err(ClientError::Unauthorized(format!(
            "Copilot authentication failed (HTTP {}): {}. \
             GitHub Models fallback also failed. \
             Ensure you have GitHub Copilot or Models API access and run: \
             gh auth refresh --hostname github.com --scopes copilot",
            copilot_status, sanitized
        )))
    }

    /// Get the Bearer token for API requests.
    pub fn get_token(&self) -> &str {
        use secrecy::ExposeSecret;
        self.github_token.expose_secret().expose()
    }

    /// Which endpoint this auth instance is using.
    pub fn endpoint(&self) -> CopilotEndpoint {
        self.endpoint
    }

    /// Get the chat completions URL for the active endpoint.
    pub fn chat_url(&self) -> &str {
        match self.endpoint {
            CopilotEndpoint::Copilot => COPILOT_CHAT_URL,
            CopilotEndpoint::GitHubModels => GITHUB_MODELS_CHAT_URL,
        }
    }

    /// Get the models list URL for the active endpoint.
    pub fn models_url(&self) -> &str {
        match self.endpoint {
            CopilotEndpoint::Copilot => COPILOT_MODELS_URL,
            CopilotEndpoint::GitHubModels => GITHUB_MODELS_LIST_URL,
        }
    }

    /// Ensure the model name has the correct prefix for the active endpoint.
    ///
    /// The GitHub Models API requires an `openai/` prefix on model names
    /// (e.g., `openai/gpt-4o`). This method adds the prefix if needed when
    /// using the GitHub Models backend, and leaves names unchanged for Copilot.
    pub fn qualify_model(&self, model: &str) -> String {
        match self.endpoint {
            CopilotEndpoint::Copilot => model.to_string(),
            CopilotEndpoint::GitHubModels => {
                if model.contains('/') {
                    model.to_string()
                } else {
                    format!("{}{}", GITHUB_MODELS_PREFIX, model)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Copilot model listing
// ---------------------------------------------------------------------------

/// A model available via the Copilot API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CopilotModel {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub tokenizer: Option<String>,
    #[serde(default)]
    pub max_input_tokens: Option<u32>,
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
}

impl std::fmt::Display for CopilotModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Show ID first (what you pass to --model), then friendly name
        write!(f, "{:<30}", self.id)?;
        if !self.name.is_empty() && self.name != self.id {
            write!(f, " {}", self.name)?;
        }
        Ok(())
    }
}

/// Response wrapper for models endpoint (may be a bare array or an object).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ModelsResponse {
    Array(Vec<CopilotModel>),
    Object { data: Vec<CopilotModel> },
}

/// Fetch available models from the active API endpoint.
pub async fn list_models(auth: &CopilotAuth) -> ClientResult<Vec<CopilotModel>> {
    let token = auth.get_token();
    let models_url = auth.models_url();

    let response = auth
        .http_client
        .get(models_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("User-Agent", COPILOT_USER_AGENT)
        .send()
        .await
        .map_err(|e| ClientError::NetworkError(format!("Failed to list models: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        return Err(ClientError::Unknown(format!(
            "Failed to list Copilot models (HTTP {}): {}",
            status, body
        )));
    }

    let models_resp: ModelsResponse = response.json().await.map_err(|e| {
        ClientError::Unknown(format!("Failed to parse Copilot models response: {}", e))
    })?;

    Ok(match models_resp {
        ModelsResponse::Array(models) => models,
        ModelsResponse::Object { data } => data,
    })
}

// ---------------------------------------------------------------------------
// OpenAI-compatible request/response types (for Copilot API)
// ---------------------------------------------------------------------------

/// OpenAI-compatible chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OaiMessage {
    pub role: String,
    pub content: OaiContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OaiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Content can be a string or structured parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OaiContent {
    Text(String),
    Null,
}

/// OpenAI tool call in assistant response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OaiToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub function: OaiFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OaiFunction {
    pub name: String,
    pub arguments: String,
}

/// OpenAI-compatible tool definition.
#[derive(Debug, Clone, Serialize)]
struct OaiToolDef {
    #[serde(rename = "type")]
    type_field: String,
    function: OaiFunctionDef,
}

#[derive(Debug, Clone, Serialize)]
struct OaiFunctionDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// OpenAI-compatible chat completion request.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct OaiChatRequest {
    model: String,
    messages: Vec<OaiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OaiToolDef>>,
    n: u32,
}

/// OpenAI-compatible chat completion response.
#[derive(Debug, Clone, Deserialize)]
pub struct OaiChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<OaiChoice>,
    #[serde(default)]
    pub usage: Option<OaiUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OaiChoice {
    #[serde(default)]
    pub index: u32,
    pub message: OaiResponseMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OaiResponseMessage {
    pub role: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<OaiToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OaiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI-compatible SSE chunk for streaming.
#[derive(Debug, Clone, Deserialize)]
pub struct OaiStreamChunk {
    pub id: String,
    pub model: String,
    pub choices: Vec<OaiStreamChoice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OaiStreamChoice {
    pub index: u32,
    pub delta: OaiStreamDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OaiStreamDelta {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<OaiStreamToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OaiStreamToolCall {
    pub index: u32,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type", default)]
    pub type_field: Option<String>,
    #[serde(default)]
    pub function: Option<OaiStreamFunction>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OaiStreamFunction {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}

// ---------------------------------------------------------------------------
// Translation: Anthropic-native ↔ OpenAI-compatible
// ---------------------------------------------------------------------------

/// Convert our internal CreateMessageRequest to an OpenAI-compatible request.
pub(crate) fn to_oai_request(request: &CreateMessageRequest) -> OaiChatRequest {
    let mut messages = Vec::new();

    // Add system message if present
    if let Some(ref system) = request.system {
        messages.push(OaiMessage {
            role: "system".to_string(),
            content: OaiContent::Text(system.clone()),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // Convert messages
    for msg in &request.messages {
        match &msg.content {
            MessageContent::Text(text) => {
                let role = match msg.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                };
                messages.push(OaiMessage {
                    role: role.to_string(),
                    content: OaiContent::Text(text.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
            MessageContent::Blocks(blocks) => {
                // Handle structured content (tool use/result)
                let role = match msg.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                };

                // Check if this is a tool result message
                let mut tool_results = Vec::new();
                let mut text_parts = Vec::new();
                let mut assistant_tool_calls = Vec::new();

                for block in blocks {
                    match block {
                        ContentBlock::Text { text } => {
                            text_parts.push(text.clone());
                        }
                        ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } => {
                            let mut result_text = content
                                .iter()
                                .filter_map(|b| {
                                    if let ContentBlock::Text { text } = b {
                                        Some(text.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            // Signal errors to the model since OpenAI format
                            // has no is_error field
                            if *is_error == Some(true) && !result_text.starts_with("Error:") {
                                result_text = format!("Error: {}", result_text);
                            }
                            tool_results.push((tool_use_id.clone(), result_text));
                        }
                        ContentBlock::ToolUse { id, name, input } => {
                            assistant_tool_calls.push(OaiToolCall {
                                id: id.clone(),
                                type_field: "function".to_string(),
                                function: OaiFunction {
                                    name: name.clone(),
                                    arguments: input.to_string(),
                                },
                            });
                        }
                        ContentBlock::Thinking { .. } => {
                            // Skip thinking blocks for OpenAI format
                        }
                    }
                }

                if !assistant_tool_calls.is_empty() {
                    // Assistant message with tool calls
                    let content_text = if text_parts.is_empty() {
                        OaiContent::Null
                    } else {
                        OaiContent::Text(text_parts.join("\n"))
                    };
                    messages.push(OaiMessage {
                        role: "assistant".to_string(),
                        content: content_text,
                        tool_calls: Some(assistant_tool_calls),
                        tool_call_id: None,
                    });
                } else if !tool_results.is_empty() {
                    // Tool result messages (one per result in OpenAI format)
                    for (tool_use_id, result_text) in tool_results {
                        messages.push(OaiMessage {
                            role: "tool".to_string(),
                            content: OaiContent::Text(result_text),
                            tool_calls: None,
                            tool_call_id: Some(tool_use_id),
                        });
                    }
                } else if !text_parts.is_empty() {
                    messages.push(OaiMessage {
                        role: role.to_string(),
                        content: OaiContent::Text(text_parts.join("\n")),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
            }
        }
    }

    // Convert tool definitions
    let tools = request.tools.as_ref().map(|tools| {
        tools
            .iter()
            .map(|t| OaiToolDef {
                type_field: "function".to_string(),
                function: OaiFunctionDef {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.input_schema.clone(),
                },
            })
            .collect()
    });

    OaiChatRequest {
        model: request.model.clone(),
        messages,
        max_tokens: Some(request.max_tokens),
        temperature: request.temperature,
        top_p: request.top_p,
        stream: request.stream,
        tools,
        n: 1,
    }
}

/// Convert an OpenAI chat response back to our internal MessageResponse.
pub fn from_oai_response(oai: OaiChatResponse) -> MessageResponse {
    let choice = oai.choices.into_iter().next();

    let mut content = Vec::new();
    let mut stop_reason = None;

    if let Some(choice) = choice {
        stop_reason = choice.finish_reason.map(|r| match r.as_str() {
            "stop" => "end_turn".to_string(),
            "tool_calls" => "tool_use".to_string(),
            "length" => "max_tokens".to_string(),
            other => other.to_string(),
        });

        // Add text content if present
        if let Some(text) = choice.message.content {
            if !text.is_empty() {
                content.push(ContentBlock::Text { text });
            }
        }

        // Add tool use blocks
        if let Some(tool_calls) = choice.message.tool_calls {
            for tc in tool_calls {
                let input: serde_json::Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));
                content.push(ContentBlock::ToolUse {
                    id: tc.id,
                    name: tc.function.name,
                    input,
                });
            }
        }
    }

    let usage = oai.usage.map_or(
        AnthropicUsage {
            input_tokens: 0,
            output_tokens: 0,
            speed: None,
        },
        |u| AnthropicUsage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            speed: None,
        },
    );

    MessageResponse {
        id: oai.id,
        type_field: "message".to_string(),
        role: Role::Assistant,
        content,
        model: oai.model,
        stop_reason,
        stop_sequence: None,
        usage,
    }
}

/// Convert an OpenAI streaming chunk to our StreamEvent.
///
/// Returns None for chunks that don't map to a meaningful event.
pub fn from_oai_stream_chunk(chunk: &OaiStreamChunk, block_index: &mut u32) -> Vec<StreamEvent> {
    let mut events = Vec::new();

    for choice in &chunk.choices {
        // Text content delta
        if let Some(ref text) = choice.delta.content {
            if !text.is_empty() {
                events.push(StreamEvent::ContentBlockDelta {
                    index: *block_index,
                    delta: ContentDelta::TextDelta { text: text.clone() },
                });
            }
        }

        // Tool call deltas
        if let Some(ref tool_calls) = choice.delta.tool_calls {
            for tc in tool_calls {
                if let Some(ref func) = tc.function {
                    // New tool call starting
                    if tc.id.is_some() {
                        *block_index += 1;
                        events.push(StreamEvent::ContentBlockStart {
                            index: *block_index,
                            content_block: ContentBlockStart::ToolUse {
                                id: tc.id.clone().unwrap_or_default(),
                                name: func.name.clone().unwrap_or_default(),
                            },
                        });
                    }
                    // Argument delta
                    if let Some(ref args) = func.arguments {
                        if !args.is_empty() {
                            events.push(StreamEvent::ContentBlockDelta {
                                index: *block_index,
                                delta: ContentDelta::InputJsonDelta {
                                    partial_json: args.clone(),
                                },
                            });
                        }
                    }
                }
            }
        }

        // Finish
        if choice.finish_reason.is_some() {
            events.push(StreamEvent::MessageStop);
        }
    }

    events
}

// ---------------------------------------------------------------------------
// Copilot API call execution
// ---------------------------------------------------------------------------

/// Execute a non-streaming chat completion against the active API endpoint.
pub async fn create_message(
    http_client: &HttpClient,
    auth: &CopilotAuth,
    request: &CreateMessageRequest,
) -> ClientResult<MessageResponse> {
    let token = auth.get_token();
    let chat_url = auth.chat_url();
    let mut oai_request = to_oai_request(request);
    oai_request.model = auth.qualify_model(&oai_request.model);

    let mut req_builder = http_client
        .post(chat_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("User-Agent", COPILOT_USER_AGENT);

    // Copilot-specific headers are only meaningful for the Copilot endpoint
    if auth.endpoint() == CopilotEndpoint::Copilot {
        req_builder = req_builder
            .header("Copilot-Integration-Id", "rustyclawd")
            .header("Editor-Version", "RustyClawd/0.1.0");
    }

    let response = req_builder.json(&oai_request).send().await?;

    if !response.status().is_success() {
        return Err(ClientError::from_response(response).await);
    }

    let oai_response: OaiChatResponse = response.json().await?;
    Ok(from_oai_response(oai_response))
}

/// Execute a streaming chat completion against the active API endpoint.
///
/// Returns a stream of our internal StreamEvent types.
pub async fn create_message_stream(
    http_client: &HttpClient,
    auth: &CopilotAuth,
    request: &CreateMessageRequest,
) -> ClientResult<impl futures::Stream<Item = ClientResult<StreamEvent>>> {
    let token = auth.get_token();
    let chat_url = auth.chat_url();
    let mut oai_request = to_oai_request(request);
    oai_request.model = auth.qualify_model(&oai_request.model);
    oai_request.stream = true;

    let mut req_builder = http_client
        .post(chat_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .header("User-Agent", COPILOT_USER_AGENT);

    if auth.endpoint() == CopilotEndpoint::Copilot {
        req_builder = req_builder
            .header("Copilot-Integration-Id", "rustyclawd")
            .header("Editor-Version", "RustyClawd/0.1.0");
    }

    let response = req_builder.json(&oai_request).send().await?;

    if !response.status().is_success() {
        return Err(ClientError::from_response(response).await);
    }

    // Parse SSE stream and translate to our StreamEvent format
    let byte_stream = response.bytes_stream();
    Ok(parse_oai_sse_stream(byte_stream))
}

/// Parse an OpenAI-style SSE byte stream into our StreamEvent types.
fn parse_oai_sse_stream(
    byte_stream: impl futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>>,
) -> impl futures::Stream<Item = ClientResult<StreamEvent>> {
    use futures::StreamExt;
    use std::collections::VecDeque;

    let block_index: u32 = 0;
    let buffer = String::new();
    let pending: VecDeque<StreamEvent> = VecDeque::new();

    futures::stream::unfold(
        (Box::pin(byte_stream), buffer, block_index, pending),
        |(mut stream, mut buf, mut idx, mut pending)| async move {
            loop {
                // Drain any buffered events first
                if let Some(event) = pending.pop_front() {
                    return Some((Ok(event), (stream, buf, idx, pending)));
                }

                // Try to extract a complete SSE event from the buffer
                if let Some(pos) = buf.find("\n\n") {
                    let event_text = buf[..pos].to_string();
                    buf = buf[pos + 2..].to_string();

                    // Parse SSE data lines
                    for line in event_text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                return Some((
                                    Ok(StreamEvent::MessageStop),
                                    (stream, buf, idx, pending),
                                ));
                            }

                            match serde_json::from_str::<OaiStreamChunk>(data) {
                                Ok(chunk) => {
                                    let mut events = from_oai_stream_chunk(&chunk, &mut idx);
                                    if !events.is_empty() {
                                        // Yield first event, queue the rest
                                        let first = events.remove(0);
                                        pending.extend(events);
                                        return Some((Ok(first), (stream, buf, idx, pending)));
                                    }
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        "Failed to parse Copilot SSE chunk: {} (data: {})",
                                        e,
                                        data
                                    );
                                }
                            }
                        }
                    }
                    continue;
                }

                // Need more data
                match stream.next().await {
                    Some(Ok(bytes)) => {
                        buf.push_str(&String::from_utf8_lossy(&bytes));
                    }
                    Some(Err(e)) => {
                        return Some((
                            Err(ClientError::Stream(format!("Copilot stream error: {}", e))),
                            (stream, buf, idx, pending),
                        ));
                    }
                    None => return None,
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::types::{Message, ToolDefinition};

    #[test]
    fn test_to_oai_request_simple() {
        let request = CreateMessageRequest::new("gpt-4o", vec![Message::user("Hello")], 1024)
            .with_system("You are helpful.".to_string());

        let oai = to_oai_request(&request);

        assert_eq!(oai.model, "gpt-4o");
        assert_eq!(oai.messages.len(), 2); // system + user
        assert_eq!(oai.messages[0].role, "system");
        assert_eq!(oai.messages[1].role, "user");
        assert_eq!(oai.max_tokens, Some(1024));
    }

    #[test]
    fn test_to_oai_request_with_tools() {
        let request = CreateMessageRequest::new(
            "gpt-4o",
            vec![Message::user("List files")],
            1024,
        )
        .with_tools(vec![ToolDefinition::new(
            "Bash",
            "Run a bash command",
            serde_json::json!({"type": "object", "properties": {"command": {"type": "string"}}}),
        )]);

        let oai = to_oai_request(&request);

        assert!(oai.tools.is_some());
        let tools = oai.tools.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "Bash");
    }

    #[test]
    fn test_from_oai_response_text() {
        let oai = OaiChatResponse {
            id: "chatcmpl-123".to_string(),
            model: "gpt-4o".to_string(),
            choices: vec![OaiChoice {
                index: 0,
                message: OaiResponseMessage {
                    role: "assistant".to_string(),
                    content: Some("Hello there!".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(OaiUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };

        let msg = from_oai_response(oai);

        assert_eq!(msg.id, "chatcmpl-123");
        assert_eq!(msg.model, "gpt-4o");
        assert_eq!(msg.stop_reason, Some("end_turn".to_string()));
        assert_eq!(msg.content.len(), 1);
        assert!(matches!(&msg.content[0], ContentBlock::Text { text } if text == "Hello there!"));
        assert_eq!(msg.usage.input_tokens, 10);
        assert_eq!(msg.usage.output_tokens, 5);
    }

    #[test]
    fn test_from_oai_response_tool_calls() {
        let oai = OaiChatResponse {
            id: "chatcmpl-456".to_string(),
            model: "gpt-4o".to_string(),
            choices: vec![OaiChoice {
                index: 0,
                message: OaiResponseMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_calls: Some(vec![OaiToolCall {
                        id: "call_abc".to_string(),
                        type_field: "function".to_string(),
                        function: OaiFunction {
                            name: "Bash".to_string(),
                            arguments: r#"{"command":"ls"}"#.to_string(),
                        },
                    }]),
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: None,
        };

        let msg = from_oai_response(oai);

        assert_eq!(msg.stop_reason, Some("tool_use".to_string()));
        assert_eq!(msg.content.len(), 1);
        assert!(matches!(
            &msg.content[0],
            ContentBlock::ToolUse { id, name, input }
            if id == "call_abc" && name == "Bash" && input["command"] == "ls"
        ));
    }

    #[test]
    fn test_copilot_model_display() {
        let model = CopilotModel {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            version: Some("2024-08-06".to_string()),
            tokenizer: None,
            max_input_tokens: Some(128000),
            max_output_tokens: Some(16384),
        };

        let display = format!("{}", model);
        assert!(display.contains("gpt-4o"), "Should show model ID");
        assert!(display.contains("GPT-4o"), "Should show friendly name");
    }

    #[test]
    fn test_to_oai_request_tool_results() {
        // Simulate the tool loop: assistant message with tool_use, then user with tool_result
        let request = CreateMessageRequest::new(
            "gpt-4o",
            vec![
                Message::user("Run ls"),
                Message::with_blocks(
                    Role::Assistant,
                    vec![ContentBlock::ToolUse {
                        id: "call_1".to_string(),
                        name: "Bash".to_string(),
                        input: serde_json::json!({"command": "ls"}),
                    }],
                ),
                Message::with_blocks(
                    Role::User,
                    vec![ContentBlock::ToolResult {
                        tool_use_id: "call_1".to_string(),
                        content: vec![ContentBlock::Text {
                            text: "file1.txt\nfile2.txt".to_string(),
                        }],
                        is_error: None,
                    }],
                ),
            ],
            1024,
        );

        let oai = to_oai_request(&request);

        // Should have: user, assistant (with tool_calls), tool (with result)
        assert_eq!(oai.messages.len(), 3);
        assert_eq!(oai.messages[0].role, "user");
        assert_eq!(oai.messages[1].role, "assistant");
        assert!(oai.messages[1].tool_calls.is_some());
        assert_eq!(oai.messages[2].role, "tool");
        assert_eq!(oai.messages[2].tool_call_id, Some("call_1".to_string()));
    }
}
