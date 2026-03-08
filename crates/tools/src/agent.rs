//! Agent/Task tool - Enables agent orchestration through Claude API
//!
//! This is the CRITICAL tool that enables multi-agent workflows. It:
//! - Invokes sub-agents with specialized prompts
//! - Forks context for agent isolation
//! - Streams agent responses in real-time
//! - Supports model selection (haiku/sonnet/opus)
//! - Allows resuming previous agent executions
//! - Supports background execution (run_in_background)

use crate::agent_memory::{global_agent_memory, MemoryScope};
use crate::agent_registry::global_agent_registry;
use crate::worktree_isolation::{self, WorktreeInfo};
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;
use tokio::fs;

/// Agent isolation mode parsed from frontmatter.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentIsolation {
    /// Run the agent in an isolated git worktree
    Worktree,
}

/// Metadata extracted from agent definition frontmatter (YAML between `---` delimiters).
///
/// Example agent.md:
/// ```markdown
/// ---
/// background: true
/// memory: project
/// isolation: worktree
/// ---
/// # Agent Name
/// You are a specialized agent...
/// ```
#[derive(Debug, Clone, Default)]
pub struct AgentFrontmatter {
    /// If true, this agent should always run in the background
    pub background: bool,
    /// Memory scope for this agent's memory operations (user, project, local)
    pub memory_scope: Option<MemoryScope>,
    /// Isolation mode for agent execution
    pub isolation: Option<AgentIsolation>,
}

impl AgentFrontmatter {
    /// Parse frontmatter from agent markdown content.
    ///
    /// Looks for YAML frontmatter between `---` delimiters at the start of the file.
    /// Returns the parsed frontmatter and the remaining content (the system prompt).
    pub fn parse(content: &str) -> (Self, String) {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return (Self::default(), content.to_string());
        }

        // Find the closing `---`
        let after_first = &trimmed[3..];
        let closing = after_first.find("---");
        match closing {
            Some(end_pos) => {
                let yaml_block = &after_first[..end_pos];
                let rest = &after_first[end_pos + 3..];

                let mut frontmatter = Self::default();

                // Simple line-by-line key: value parsing (avoids YAML dependency)
                for line in yaml_block.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once(':') {
                        let key = key.trim();
                        let value = value.trim();
                        match key {
                            "background" => {
                                frontmatter.background = value == "true";
                            }
                            "memory" | "memory_scope" => {
                                frontmatter.memory_scope = match value {
                                    "user" => Some(MemoryScope::User),
                                    "project" => Some(MemoryScope::Project),
                                    "local" => Some(MemoryScope::Local),
                                    _ => None,
                                };
                            }
                            "isolation" => {
                                frontmatter.isolation = match value {
                                    "worktree" => Some(AgentIsolation::Worktree),
                                    _ => None,
                                };
                            }
                            _ => {
                                // Ignore unknown frontmatter keys
                            }
                        }
                    }
                }

                (frontmatter, rest.to_string())
            }
            None => {
                // No closing `---`, treat entire content as prompt
                (Self::default(), content.to_string())
            }
        }
    }
}

/// Parameters for the Agent tool
#[derive(Debug, Deserialize)]
pub struct AgentParams {
    /// Brief 3-5 word description of the task
    pub description: String,

    /// Full prompt/task for the agent to execute
    pub prompt: String,

    /// Name of the agent (loads from .claude/agents/{subagent_type}.md)
    pub subagent_type: String,

    /// Optional model override (haiku, sonnet, opus)
    #[serde(default)]
    pub model: Option<String>,

    /// Optional agent ID to resume a previous execution
    #[serde(default)]
    pub resume: Option<String>,

    /// Run the agent in the background (returns immediately with agent_id)
    #[serde(default)]
    pub run_in_background: bool,

    /// Memory scope override for agent memory operations.
    /// If not set, falls back to agent definition frontmatter, then defaults to Local.
    #[serde(default)]
    pub memory_scope: Option<String>,
}

/// Output from the Agent tool
#[derive(Debug, Serialize)]
pub struct AgentOutput {
    /// Agent ID for this execution (for resuming)
    pub agent_id: String,

    /// Name of the agent that was invoked
    pub agent_name: String,

    /// Complete response from the agent
    pub response: String,

    /// Model used for execution
    pub model: String,

    /// Tokens used (input + output)
    pub tokens_used: TokenUsage,
}

/// Token usage statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

/// The Agent tool - enables agent orchestration
pub struct AgentTool;

impl AgentTool {
    /// Load agent system prompt from .claude/agents/{agent_type}.md
    async fn load_agent_prompt(agent_type: &str, cwd: &Path) -> Result<String, String> {
        let agent_path = cwd
            .join(".claude")
            .join("agents")
            .join(format!("{}.md", agent_type));

        if !agent_path.exists() {
            return Err(format!(
                "Agent prompt not found: {}. Expected at: {}",
                agent_type,
                agent_path.display()
            ));
        }

        fs::read_to_string(&agent_path).await.map_err(|e| {
            format!(
                "Failed to read agent prompt {}: {}",
                agent_path.display(),
                e
            )
        })
    }

    /// Convert model name to API model ID
    fn resolve_model_id(model_name: Option<&str>) -> String {
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
    fn generate_agent_id(agent_type: &str) -> String {
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
        let mut run_in_background = params.run_in_background;
        let param_memory_scope = params.memory_scope.clone();

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
            let raw_content = match Self::load_agent_prompt(&agent_type, &cwd).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionContext, Tool};
    use futures::StreamExt;
    use std::env;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::fs;

    async fn setup_test_agent(temp_dir: &TempDir) -> PathBuf {
        let claude_dir = temp_dir.path().join(".claude");
        let agents_dir = claude_dir.join("agents");
        fs::create_dir_all(&agents_dir).await.unwrap();

        let agent_file = agents_dir.join("test_agent.md");
        fs::write(
            &agent_file,
            "You are a test agent. Respond concisely to user requests.",
        )
        .await
        .unwrap();

        temp_dir.path().to_path_buf()
    }

    #[test]
    fn test_model_resolution() {
        assert_eq!(
            AgentTool::resolve_model_id(Some("haiku")),
            "claude-haiku-4-5-20251001"
        );
        assert_eq!(
            AgentTool::resolve_model_id(Some("sonnet")),
            "claude-sonnet-4-6"
        );
        assert_eq!(AgentTool::resolve_model_id(Some("opus")), "claude-opus-4-6");
        assert_eq!(AgentTool::resolve_model_id(None), "claude-sonnet-4-6");
        assert_eq!(
            AgentTool::resolve_model_id(Some("claude-custom-model")),
            "claude-custom-model"
        );
    }

    #[test]
    fn test_agent_id_generation() {
        let id1 = AgentTool::generate_agent_id("test");

        // IDs should have the correct format
        assert!(id1.starts_with("agent_test_t"));

        // Sleep briefly to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(2));

        let id2 = AgentTool::generate_agent_id("test");
        assert!(id2.starts_with("agent_test_t"));

        // IDs should be unique (different timestamps)
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_load_agent_prompt_success() {
        let temp_dir = TempDir::new().unwrap();
        let cwd = setup_test_agent(&temp_dir).await;

        let prompt = AgentTool::load_agent_prompt("test_agent", &cwd)
            .await
            .unwrap();

        assert!(prompt.contains("test agent"));
        assert!(prompt.contains("concisely"));
    }

    #[tokio::test]
    async fn test_load_agent_prompt_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let cwd = temp_dir.path().to_path_buf();

        let result = AgentTool::load_agent_prompt("nonexistent", &cwd).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_agent_tool_missing_prompt() {
        let temp_dir = TempDir::new().unwrap();

        let tool = AgentTool;
        let params = AgentParams {
            description: "Test task".to_string(),
            prompt: "Say hello".to_string(),
            subagent_type: "nonexistent".to_string(),
            model: None,
            resume: None,
            run_in_background: false,
            memory_scope: None,
        };
        let ctx = ToolContext {
            cwd: temp_dir.path().to_path_buf(),
            debug: false,
            metadata: serde_json::Value::Null,
            execution_context: ExecutionContext::default(),
            allowed_tools: vec![],
            disallowed_tools: vec![],
        };

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should have an error about missing agent prompt
        let has_error = events
            .iter()
            .any(|e| matches!(e, ToolEvent::Error { message } if message.contains("not found")));
        assert!(has_error);
    }

    // Integration test (requires API key)
    #[tokio::test]
    #[ignore] // Only run with --ignored when testing with real API
    async fn test_agent_tool_real_execution() {
        // Check if API key is available
        if env::var("ANTHROPIC_API_KEY").is_err() {
            println!("Skipping: ANTHROPIC_API_KEY not set");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let cwd = setup_test_agent(&temp_dir).await;

        let tool = AgentTool;
        let params = AgentParams {
            description: "Simple test".to_string(),
            prompt: "Say 'Hello from agent test!' and nothing else.".to_string(),
            subagent_type: "test_agent".to_string(),
            model: Some("haiku".to_string()),
            resume: None,
            run_in_background: false,
            memory_scope: None,
        };
        let ctx = ToolContext {
            cwd,
            debug: true,
            metadata: serde_json::Value::Null,
            execution_context: ExecutionContext::default(),
            allowed_tools: vec![],
            disallowed_tools: vec![],
        };

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let mut result: Option<AgentOutput> = None;

        while let Some(event) = stream.next().await {
            match event {
                ToolEvent::Result(output) => {
                    result = Some(output);
                }
                ToolEvent::Error { message } => {
                    panic!("Agent execution failed: {}", message);
                }
                ToolEvent::Progress { step, .. } => {
                    println!("Progress: {}", step);
                }
            }
        }

        let output = result.expect("Should have result");
        assert_eq!(output.agent_name, "test_agent");
        assert!(output.response.contains("Hello"));
        assert!(output.tokens_used.total_tokens > 0);
        assert!(output.agent_id.starts_with("agent_test_agent_t"));
    }

    #[test]
    fn test_token_usage_has_duration_ms() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            duration_ms: 1234,
        };
        assert_eq!(usage.duration_ms, 1234);
    }

    #[test]
    fn test_token_usage_serializes_duration_ms() {
        let usage = TokenUsage {
            input_tokens: 10,
            output_tokens: 20,
            total_tokens: 30,
            duration_ms: 500,
        };
        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("\"duration_ms\":500"));
    }

    #[test]
    fn test_token_usage_deserializes_duration_ms() {
        let json = r#"{"input_tokens":10,"output_tokens":20,"total_tokens":30,"duration_ms":750}"#;
        let usage: TokenUsage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.duration_ms, 750);
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
    }

    #[test]
    fn test_token_usage_zero_duration() {
        let usage = TokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            duration_ms: 0,
        };
        assert_eq!(usage.duration_ms, 0);
    }

    #[test]
    fn test_agent_output_includes_duration() {
        let output = AgentOutput {
            agent_id: "agent_test_t123".to_string(),
            agent_name: "test".to_string(),
            response: "hello".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            tokens_used: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 150,
                duration_ms: 2000,
            },
        };
        assert_eq!(output.tokens_used.duration_ms, 2000);
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"duration_ms\":2000"));
    }

    // --- Frontmatter parsing tests ---

    #[test]
    fn test_frontmatter_parse_no_frontmatter() {
        let content = "# Agent\nYou are a helpful agent.";
        let (fm, prompt) = AgentFrontmatter::parse(content);
        assert!(!fm.background);
        assert!(fm.memory_scope.is_none());
        assert_eq!(prompt, content);
    }

    #[test]
    fn test_frontmatter_parse_background_true() {
        let content = "---\nbackground: true\n---\n# Agent\nYou are a helpful agent.";
        let (fm, prompt) = AgentFrontmatter::parse(content);
        assert!(fm.background);
        assert!(prompt.contains("# Agent"));
        assert!(!prompt.contains("---"));
    }

    #[test]
    fn test_frontmatter_parse_background_false() {
        let content = "---\nbackground: false\n---\n# Agent";
        let (fm, _prompt) = AgentFrontmatter::parse(content);
        assert!(!fm.background);
    }

    #[test]
    fn test_frontmatter_parse_memory_scopes() {
        let content = "---\nmemory: user\n---\nPrompt";
        let (fm, _) = AgentFrontmatter::parse(content);
        assert_eq!(fm.memory_scope, Some(MemoryScope::User));

        let content = "---\nmemory: project\n---\nPrompt";
        let (fm, _) = AgentFrontmatter::parse(content);
        assert_eq!(fm.memory_scope, Some(MemoryScope::Project));

        let content = "---\nmemory: local\n---\nPrompt";
        let (fm, _) = AgentFrontmatter::parse(content);
        assert_eq!(fm.memory_scope, Some(MemoryScope::Local));

        // memory_scope key also works
        let content = "---\nmemory_scope: user\n---\nPrompt";
        let (fm, _) = AgentFrontmatter::parse(content);
        assert_eq!(fm.memory_scope, Some(MemoryScope::User));
    }

    #[test]
    fn test_frontmatter_parse_invalid_memory_scope() {
        let content = "---\nmemory: unknown_scope\n---\nPrompt";
        let (fm, _) = AgentFrontmatter::parse(content);
        assert!(fm.memory_scope.is_none());
    }

    #[test]
    fn test_frontmatter_parse_combined() {
        let content = "---\nbackground: true\nmemory: project\n---\n# Code Reviewer\nReview code.";
        let (fm, prompt) = AgentFrontmatter::parse(content);
        assert!(fm.background);
        assert_eq!(fm.memory_scope, Some(MemoryScope::Project));
        assert!(prompt.contains("Code Reviewer"));
    }

    #[test]
    fn test_frontmatter_parse_unknown_keys_ignored() {
        let content = "---\nbackground: true\nunknown_key: value\nauthor: test\n---\nPrompt";
        let (fm, prompt) = AgentFrontmatter::parse(content);
        assert!(fm.background);
        assert!(fm.memory_scope.is_none());
        assert_eq!(prompt.trim(), "Prompt");
    }

    #[test]
    fn test_frontmatter_parse_unclosed() {
        // If there's no closing ---, treat entire content as prompt
        let content = "---\nbackground: true\nNo closing delimiter";
        let (fm, prompt) = AgentFrontmatter::parse(content);
        assert!(!fm.background);
        assert_eq!(prompt, content);
    }

    #[test]
    fn test_frontmatter_parse_comments_in_frontmatter() {
        let content = "---\n# This is a comment\nbackground: true\n---\nPrompt";
        let (fm, _) = AgentFrontmatter::parse(content);
        assert!(fm.background);
    }

    #[tokio::test]
    async fn test_agent_with_frontmatter_background() {
        let temp_dir = TempDir::new().unwrap();
        let agents_dir = temp_dir.path().join(".claude").join("agents");
        fs::create_dir_all(&agents_dir).await.unwrap();

        // Write agent file with background: true frontmatter
        let agent_content =
            "---\nbackground: true\nmemory: project\n---\n# BG Agent\nYou run in background.";
        fs::write(agents_dir.join("bg_agent.md"), agent_content)
            .await
            .unwrap();

        // Load and parse
        let raw = AgentTool::load_agent_prompt("bg_agent", temp_dir.path())
            .await
            .unwrap();
        let (fm, prompt) = AgentFrontmatter::parse(&raw);

        assert!(fm.background);
        assert_eq!(fm.memory_scope, Some(MemoryScope::Project));
        assert!(prompt.contains("BG Agent"));
        assert!(!prompt.contains("background: true"));
    }
}
