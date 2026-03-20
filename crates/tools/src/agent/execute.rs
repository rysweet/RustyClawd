//! Agent tool execution - the Tool impl and supporting functions.

use super::frontmatter::{AgentFrontmatter, AgentIsolation};
use super::types::{AgentOutput, AgentParams, TokenUsage};
use crate::agent_memory::{global_agent_memory, MemoryScope};
use crate::agent_registry::global_agent_registry;
use crate::worktree_isolation::{self, WorktreeInfo};
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use std::path::Path;
use std::time::Instant;
use tokio::fs;

/// The Agent tool - enables agent orchestration
pub struct AgentTool;

impl AgentTool {
    /// Load agent system prompt from .claude/agents/{agent_type}.md,
    /// falling back to runtime agents registered via --agents flag.
    pub(crate) async fn load_agent_prompt(
        agent_type: &str,
        cwd: &Path,
        ctx: &ToolContext,
    ) -> Result<String, String> {
        // Try file-based agent first
        let agent_path = cwd
            .join(".claude")
            .join("agents")
            .join(format!("{}.md", agent_type));

        if agent_path.exists() {
            return fs::read_to_string(&agent_path).await.map_err(|e| {
                format!(
                    "Failed to read agent prompt {}: {}",
                    agent_path.display(),
                    e
                )
            });
        }

        // Fall back to runtime agents from --agents flag
        if let Some(runtime_agent) = ctx.runtime_agents.get(agent_type) {
            tracing::info!(
                agent_type = %agent_type,
                "Using runtime agent definition from --agents flag"
            );
            return Ok(runtime_agent.prompt.clone());
        }

        Err(format!(
            "Agent prompt not found: {}. Expected at: {} (also not found in --agents runtime agents)",
            agent_type,
            agent_path.display()
        ))
    }

    /// Convert model name to API model ID
    pub(crate) fn resolve_model_id(model_name: Option<&str>) -> String {
        match model_name {
            Some("haiku") => "claude-haiku-4-5-20251001",
            Some("sonnet") => "claude-sonnet-4-6",
            Some("opus") => "claude-opus-4-6",
            Some(custom) if custom.starts_with("claude-") => custom,
            _ => "claude-sonnet-4-6", // Default to sonnet
        }
        .to_string()
    }

    /// Generate unique agent ID
    pub(crate) fn generate_agent_id(agent_type: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("agent_{}_t{}", agent_type, timestamp)
    }
}

/// Default max tokens for agent API requests.
const DEFAULT_AGENT_MAX_TOKENS: u32 = 4096;

/// Load API config and build an HTTP client for the Claude API.
async fn build_api_client() -> Result<rustyclawd_core::client::Client, String> {
    let config = rustyclawd_core::client::Config::from_default_location()
        .await
        .map_err(|e| format!("Failed to load API config: {}", e))?;
    rustyclawd_core::client::Client::new(config)
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

/// Construct a [`CreateMessageRequest`] with standard agent defaults.
fn build_agent_request(
    model_id: String,
    prompt: String,
    system_prompt: String,
) -> rustyclawd_core::client::types::CreateMessageRequest {
    let messages = vec![rustyclawd_core::client::types::Message::user(prompt)];
    rustyclawd_core::client::types::CreateMessageRequest::new(
        model_id,
        messages,
        DEFAULT_AGENT_MAX_TOKENS,
    )
    .with_system(system_prompt)
    .with_temperature(0.7)
}

#[async_trait]
impl crate::Tool for AgentTool {
    type Params = AgentParams;
    type Output = AgentOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "Agent",
            description: "Invoke specialized sub-agents for complex tasks with context isolation",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let cwd = ctx.cwd.clone();
        let debug = ctx.debug;
        let agent_type = params.subagent_type.clone();
        let _description = params.description.clone(); // Used for display in logs
        let prompt = params.prompt.clone();
        let model_name = params.model.clone();
        let resume_id = params.resume.clone();
        // Feature: CLAUDE_CODE_DISABLE_BACKGROUND_TASKS overrides run_in_background
        let mut run_in_background =
            params.run_in_background && !rustyclawd_core::is_background_tasks_disabled();
        let param_memory_scope = params.memory_scope.clone();
        let runtime_agents = ctx.runtime_agents.clone();

        Ok(Box::pin(stream! {
            let start_time = Instant::now();

            yield ToolEvent::Progress {
                step: format!("Loading {} agent prompt", agent_type),
                percentage: Some(10.0),
            };

            if debug {
                tracing::debug!(
                    agent_type = %agent_type,
                    model = ?model_name,
                    resume = ?resume_id,
                    run_in_background = run_in_background,
                    "Starting agent execution"
                );
            }

            // Load agent definition (may contain frontmatter + system prompt)
            // Build a minimal context to pass runtime agents to the loader
            let load_ctx = ToolContext {
                cwd: cwd.clone(),
                runtime_agents: runtime_agents.clone(),
                ..ToolContext::default()
            };
            let raw_content = match Self::load_agent_prompt(&agent_type, &cwd, &load_ctx).await {
                Ok(content) => content,
                Err(err) => {
                    yield ToolEvent::Error {
                        message: err,
                    };
                    return;
                }
            };

            // Parse frontmatter from agent definition
            let (frontmatter, agent_system_prompt) = AgentFrontmatter::parse(&raw_content);

            // Feature 6: If agent definition has background: true, force background mode
            if frontmatter.background {
                run_in_background = true;
                if debug {
                    tracing::debug!(
                        agent_type = %agent_type,
                        "Agent definition has background: true, forcing background mode"
                    );
                }
            }

            // Feature 7: Resolve memory scope from params > frontmatter > default (Local)
            let resolved_memory_scope = match param_memory_scope.as_deref() {
                Some("user") => MemoryScope::User,
                Some("project") => MemoryScope::Project,
                Some("local") => MemoryScope::Local,
                _ => frontmatter.memory_scope.unwrap_or(MemoryScope::Local),
            };

            if debug {
                tracing::debug!(
                    prompt_length = agent_system_prompt.len(),
                    background = frontmatter.background,
                    memory_scope = ?resolved_memory_scope,
                    isolation = ?frontmatter.isolation,
                    "Loaded agent system prompt with frontmatter"
                );
            }

            // Worktree isolation: create isolated worktree if agent requests it
            let worktree_info: Option<WorktreeInfo> = if frontmatter.isolation == Some(AgentIsolation::Worktree) {
                yield ToolEvent::Progress {
                    step: format!("Creating isolated worktree for {}", agent_type),
                    percentage: Some(20.0),
                };

                match worktree_isolation::create_worktree(&cwd, &AgentTool::generate_agent_id(&agent_type)) {
                    Ok(info) => {
                        if debug {
                            tracing::debug!(
                                worktree_path = %info.worktree_path.display(),
                                branch = %info.branch_name,
                                "Worktree isolation active"
                            );
                        }
                        Some(info)
                    }
                    Err(err) => {
                        yield ToolEvent::Error {
                            message: format!("Failed to create worktree for agent isolation: {}", err),
                        };
                        return;
                    }
                }
            } else {
                None
            };

            yield ToolEvent::Progress {
                step: format!("Preparing context for {}", agent_type),
                percentage: Some(30.0),
            };

            // Resolve model
            let model_id = Self::resolve_model_id(model_name.as_deref());

            if debug {
                let effective_cwd = worktree_info.as_ref()
                    .map(|info| info.worktree_path.clone())
                    .unwrap_or_else(|| cwd.clone());
                tracing::debug!(
                    model_id = %model_id,
                    effective_cwd = %effective_cwd.display(),
                    "Resolved model ID and working directory"
                );
            }

            // Generate agent ID early (needed for background mode)
            let agent_id = resume_id.clone().unwrap_or_else(|| Self::generate_agent_id(&agent_type));

            // Handle background mode
            if run_in_background {
                // Register the agent in the global registry
                let registry = global_agent_registry();
                if let Err(e) = registry.register(
                    agent_id.clone(),
                    agent_type.clone(),
                    model_id.clone(),
                ).await {
                    yield ToolEvent::Error {
                        message: format!("Failed to register background agent: {}", e),
                    };
                    return;
                }

                if debug {
                    tracing::debug!(
                        agent_id = %agent_id,
                        "Background agent registered, spawning execution task"
                    );
                }

                // Spawn the agent execution in the background
                let bg_agent_id = agent_id.clone();
                let bg_agent_type = agent_type.clone();
                let bg_model_id = model_id.clone();
                let bg_prompt = prompt.clone();
                let bg_system_prompt = agent_system_prompt.clone();
                let bg_debug = debug;

                tokio::spawn(async move {
                    let registry = global_agent_registry();

                    let client = match build_api_client().await {
                        Ok(c) => c,
                        Err(err) => {
                            registry.mark_failed(&bg_agent_id, err).await.ok();
                            return;
                        }
                    };

                    let request = build_agent_request(bg_model_id, bg_prompt, bg_system_prompt);

                    // Stream the response
                    let stream_result = client.create_message_stream(request).await;
                    let mut event_stream = match stream_result {
                        Ok(s) => s,
                        Err(err) => {
                            registry.mark_failed(&bg_agent_id, format!("Failed to create stream: {}", err)).await.ok();
                            return;
                        }
                    };

                    let mut input_tokens = 0u32;
                    let mut output_tokens = 0u32;

                    // Process stream events
                    while let Some(event_result) = event_stream.next().await {
                        match event_result {
                            Ok(event) => {
                                match event {
                                    rustyclawd_core::client::types::StreamEvent::MessageStart { message } => {
                                        input_tokens = message.usage.input_tokens;
                                        registry.update_token_usage(&bg_agent_id, input_tokens, output_tokens).await.ok();
                                    }
                                    rustyclawd_core::client::types::StreamEvent::ContentBlockDelta {
                                        delta: rustyclawd_core::client::types::ContentDelta::TextDelta { text },
                                        ..
                                    } => {
                                        registry.append_response(&bg_agent_id, text).await.ok();
                                    }
                                    rustyclawd_core::client::types::StreamEvent::MessageDelta { usage, .. } => {
                                        output_tokens = usage.output_tokens;
                                        registry.update_token_usage(&bg_agent_id, input_tokens, output_tokens).await.ok();
                                    }
                                    rustyclawd_core::client::types::StreamEvent::MessageStop => {
                                        if bg_debug {
                                            tracing::debug!(
                                                agent_id = %bg_agent_id,
                                                "Background agent stream completed"
                                            );
                                        }
                                        break;
                                    }
                                    rustyclawd_core::client::types::StreamEvent::Error { error } => {
                                        registry.mark_failed(&bg_agent_id, format!("Agent error: {}", error.message)).await.ok();
                                        return;
                                    }
                                    _ => {
                                        // Ignore other event types
                                    }
                                }
                            }
                            Err(err) => {
                                registry.mark_failed(&bg_agent_id, format!("Stream error: {}", err)).await.ok();
                                return;
                            }
                        }
                    }

                    // Mark as completed
                    registry.mark_completed(&bg_agent_id).await.ok();

                    if bg_debug {
                        tracing::debug!(
                            agent_id = %bg_agent_id,
                            agent_type = %bg_agent_type,
                            "Background agent execution complete"
                        );
                    }
                });

                // Return immediately with agent_id
                let duration_ms = start_time.elapsed().as_millis() as u64;
                yield ToolEvent::Result(AgentOutput {
                    agent_id,
                    agent_name: agent_type.clone(),
                    response: String::new(), // Empty - use AgentOutput tool to get response
                    model: model_id,
                    tokens_used: TokenUsage {
                        input_tokens: 0,
                        output_tokens: 0,
                        total_tokens: 0,
                        duration_ms,
                    },
                });
                return;
            }

            // Foreground execution (existing logic)
            yield ToolEvent::Progress {
                step: format!("Invoking {} agent", agent_type),
                percentage: Some(50.0),
            };

            let client = match build_api_client().await {
                Ok(c) => c,
                Err(err) => {
                    yield ToolEvent::Error { message: err };
                    return;
                }
            };

            // Augment system prompt with worktree context if isolation is active
            let final_system_prompt = if let Some(ref wt_info) = worktree_info {
                format!(
                    "{}\n\n[WORKTREE ISOLATION] You are running in an isolated git worktree.\n\
                     Working directory: {}\n\
                     Branch: {}\n\
                     Changes you make here will not affect the main working tree.",
                    agent_system_prompt,
                    wt_info.worktree_path.display(),
                    wt_info.branch_name,
                )
            } else {
                agent_system_prompt
            };

            let request = build_agent_request(model_id.clone(), prompt.clone(), final_system_prompt);

            if debug {
                tracing::debug!("Sending request to Claude API");
            }

            // Stream the response
            let stream_result = client.create_message_stream(request).await;
            let mut event_stream = match stream_result {
                Ok(s) => s,
                Err(err) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to create stream: {}", err),
                    };
                    return;
                }
            };

            let mut response_text = String::new();
            let mut input_tokens = 0u32;
            let mut output_tokens = 0u32;
            let mut received_first_content = false;

            // Process stream events
            while let Some(event_result) = event_stream.next().await {
                match event_result {
                    Ok(event) => {
                        match event {
                            rustyclawd_core::client::types::StreamEvent::MessageStart { message } => {
                                input_tokens = message.usage.input_tokens;
                                if debug {
                                    tracing::debug!(
                                        input_tokens = input_tokens,
                                        "Received message start"
                                    );
                                }
                            }
                            rustyclawd_core::client::types::StreamEvent::ContentBlockDelta { delta, .. } => {
                                if !received_first_content {
                                    received_first_content = true;
                                    yield ToolEvent::Progress {
                                        step: format!("{} agent responding...", agent_type),
                                        percentage: Some(70.0),
                                    };
                                }

                                if let rustyclawd_core::client::types::ContentDelta::TextDelta { text } = delta {
                                    response_text.push_str(&text);

                                    // Stream progress updates periodically
                                    if response_text.len() % 500 < text.len() {
                                        yield ToolEvent::Progress {
                                            step: format!("Receiving response ({} chars)...", response_text.len()),
                                            percentage: Some(80.0),
                                        };
                                    }
                                }
                            }
                            rustyclawd_core::client::types::StreamEvent::MessageDelta { usage, .. } => {
                                output_tokens = usage.output_tokens;
                            }
                            rustyclawd_core::client::types::StreamEvent::MessageStop => {
                                if debug {
                                    tracing::debug!(
                                        output_tokens = output_tokens,
                                        response_length = response_text.len(),
                                        "Stream completed"
                                    );
                                }
                                break;
                            }
                            rustyclawd_core::client::types::StreamEvent::Error { error } => {
                                yield ToolEvent::Error {
                                    message: format!("Agent error: {}", error.message),
                                };
                                return;
                            }
                            _ => {
                                // Ignore other event types (ContentBlockStart, ContentBlockStop, Ping)
                            }
                        }
                    }
                    Err(err) => {
                        yield ToolEvent::Error {
                            message: format!("Stream error: {}", err),
                        };
                        return;
                    }
                }
            }

            yield ToolEvent::Progress {
                step: "Finalizing agent response".to_string(),
                percentage: Some(95.0),
            };

            let total_tokens = input_tokens + output_tokens;

            if debug {
                tracing::debug!(
                    agent_id = %agent_id,
                    total_tokens = total_tokens,
                    response_length = response_text.len(),
                    "Agent execution complete"
                );
            }

            // Feature 7: Store agent response in memory system with the resolved scope
            {
                let memory = global_agent_memory();
                let memory_value = serde_json::json!({
                    "response_length": response_text.len(),
                    "tokens": total_tokens,
                    "model": &model_id,
                    "timestamp": start_time.elapsed().as_millis(),
                });
                // Fire-and-forget: memory storage failures should not block agent output
                if let Err(e) = memory.set(
                    resolved_memory_scope,
                    format!("last_response:{}", agent_type),
                    memory_value,
                    agent_id.clone(),
                    None,
                ).await {
                    if debug {
                        tracing::warn!(
                            agent_id = %agent_id,
                            error = %e,
                            "Failed to store agent response in memory"
                        );
                    }
                }
            }

            // Worktree isolation cleanup
            let mut worktree_note = String::new();
            if let Some(ref wt_info) = worktree_info {
                yield ToolEvent::Progress {
                    step: "Cleaning up isolated worktree".to_string(),
                    percentage: Some(97.0),
                };

                match worktree_isolation::cleanup_worktree(wt_info) {
                    Ok(true) => {
                        worktree_note = format!(
                            "\n\n[Worktree isolation] Agent made changes on branch '{}'. \
                             Review with: git log {}..{}",
                            wt_info.branch_name,
                            "HEAD",
                            wt_info.branch_name,
                        );
                    }
                    Ok(false) => {
                        // No changes, branch was cleaned up
                    }
                    Err(e) => {
                        if debug {
                            tracing::warn!(
                                error = %e,
                                "Failed to clean up worktree (non-fatal)"
                            );
                        }
                        worktree_note = format!(
                            "\n\n[Worktree isolation] Warning: cleanup failed: {}. \
                             Worktree may still exist at {}",
                            e,
                            wt_info.worktree_path.display(),
                        );
                    }
                }
            }

            let final_response = if worktree_note.is_empty() {
                response_text
            } else {
                format!("{}{}", response_text, worktree_note)
            };

            let duration_ms = start_time.elapsed().as_millis() as u64;
            yield ToolEvent::Result(AgentOutput {
                agent_id,
                agent_name: agent_type.clone(),
                response: final_response,
                model: model_id,
                tokens_used: TokenUsage {
                    input_tokens,
                    output_tokens,
                    total_tokens,
                    duration_ms,
                },
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        false // Agent execution may modify state via its own tool usage
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Multiple agents can run concurrently
    }
}
