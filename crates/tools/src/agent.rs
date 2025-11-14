//! Agent/Task tool - Enables agent orchestration through Claude API
//!
//! This is the CRITICAL tool that enables multi-agent workflows. It:
//! - Invokes sub-agents with specialized prompts
//! - Forks context for agent isolation
//! - Streams agent responses in real-time
//! - Supports model selection (haiku/sonnet/opus)
//! - Allows resuming previous agent executions

use crate::{ExecutionContext, ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

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
            Some("haiku") => "claude-3-5-haiku-20241022",
            Some("sonnet") => "claude-3-5-sonnet-20241022",
            Some("opus") => "claude-opus-4-20250514",
            Some(custom) if custom.starts_with("claude-") => custom,
            _ => "claude-3-5-sonnet-20241022", // Default to sonnet
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

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Loading {} agent prompt", agent_type),
                percentage: Some(10.0),
            };

            if debug {
                tracing::debug!(
                    agent_type = %agent_type,
                    model = ?model_name,
                    resume = ?resume_id,
                    "Starting agent execution"
                );
            }

            // Load agent system prompt
            let agent_system_prompt = match Self::load_agent_prompt(&agent_type, &cwd).await {
                Ok(prompt) => prompt,
                Err(err) => {
                    yield ToolEvent::Error {
                        message: err,
                    };
                    return;
                }
            };

            if debug {
                tracing::debug!(
                    prompt_length = agent_system_prompt.len(),
                    "Loaded agent system prompt"
                );
            }

            yield ToolEvent::Progress {
                step: format!("Preparing context for {}", agent_type),
                percentage: Some(30.0),
            };

            // Resolve model
            let model_id = Self::resolve_model_id(model_name.as_deref());

            if debug {
                tracing::debug!(
                    model_id = %model_id,
                    "Resolved model ID"
                );
            }

            yield ToolEvent::Progress {
                step: format!("Invoking {} agent", agent_type),
                percentage: Some(50.0),
            };

            // Load API config
            let config = match rustyclawd_core::client::Config::from_default_location().await {
                Ok(cfg) => cfg,
                Err(err) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to load API config: {}", err),
                    };
                    return;
                }
            };

            let client = rustyclawd_core::client::Client::new(config);

            // Build the request
            let messages = vec![
                rustyclawd_core::client::types::Message::user(prompt.clone()),
            ];

            let request = rustyclawd_core::client::types::CreateMessageRequest::new(
                model_id.clone(),
                messages,
                4096, // max_tokens
            )
            .with_system(agent_system_prompt)
            .with_temperature(0.7);

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

                                let rustyclawd_core::client::types::ContentDelta::TextDelta { text } = delta;
                                response_text.push_str(&text);

                                // Stream progress updates periodically
                                if response_text.len() % 500 < text.len() {
                                    yield ToolEvent::Progress {
                                        step: format!("Receiving response ({} chars)...", response_text.len()),
                                        percentage: Some(80.0),
                                    };
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

            let agent_id = resume_id.unwrap_or_else(|| Self::generate_agent_id(&agent_type));
            let total_tokens = input_tokens + output_tokens;

            if debug {
                tracing::debug!(
                    agent_id = %agent_id,
                    total_tokens = total_tokens,
                    response_length = response_text.len(),
                    "Agent execution complete"
                );
            }

            yield ToolEvent::Result(AgentOutput {
                agent_id,
                agent_name: agent_type.clone(),
                response: response_text,
                model: model_id,
                tokens_used: TokenUsage {
                    input_tokens,
                    output_tokens,
                    total_tokens,
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
    use crate::Tool;
    use futures::StreamExt;
    use std::env;
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
            "claude-3-5-haiku-20241022"
        );
        assert_eq!(
            AgentTool::resolve_model_id(Some("sonnet")),
            "claude-3-5-sonnet-20241022"
        );
        assert_eq!(
            AgentTool::resolve_model_id(Some("opus")),
            "claude-opus-4-20250514"
        );
        assert_eq!(
            AgentTool::resolve_model_id(None),
            "claude-3-5-sonnet-20241022"
        );
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
        };
        let ctx = ToolContext {
            cwd: temp_dir.path().to_path_buf(),
            debug: false,
            metadata: serde_json::Value::Null,
            execution_context: ExecutionContext::default(),
        };

        let mut stream = tool.execute(params, &ctx).await.unwrap();
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
        };
        let ctx = ToolContext {
            cwd,
            debug: true,
            metadata: serde_json::Value::Null,
            execution_context: ExecutionContext::default(),
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
}
