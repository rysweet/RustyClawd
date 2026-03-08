//! Agent type definitions - parameters, output, and token usage.

use serde::{Deserialize, Serialize};

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
