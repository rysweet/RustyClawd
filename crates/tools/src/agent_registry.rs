//! Agent registry for tracking background agent executions
//!
//! Provides shared state for:
//! - Agent tool (when run_in_background=true)
//! - AgentOutput tool (retrieves agent status/output)
//!
//! The registry maintains a map of running agents, each with:
//! - Execution handle
//! - Output buffers for response streaming
//! - Current execution status
//! - Token usage tracking

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

/// Callback info passed when an agent completes a task
#[derive(Debug, Clone)]
pub struct AgentCompletionInfo {
    pub agent_id: String,
    pub agent_type: String,
}

/// Type alias for completion callback.
/// Called when an agent transitions to Completed status.
pub type CompletionCallback = Arc<dyn Fn(AgentCompletionInfo) + Send + Sync>;

/// Registry for tracking background agent executions
///
/// Uses Arc<Mutex<>> for thread-safe, concurrent access from multiple tools.
/// Each agent maintains output buffers that are read by AgentOutput tool.
pub struct AgentRegistry {
    agents: Arc<Mutex<HashMap<String, AgentHandle>>>,
    /// Optional callback fired when an agent completes its task.
    /// Set by the cli layer to fire TaskCompleted hook events.
    on_task_completed: Arc<Mutex<Option<CompletionCallback>>>,
}

/// Status of a background agent
#[derive(Debug, Clone)]
pub enum AgentStatus {
    /// Agent is currently running
    Running,
    /// Agent completed successfully
    Completed,
    /// Agent failed with error message
    Failed(String),
}

/// Token usage for agent execution
#[derive(Debug, Clone, Default)]
pub struct AgentTokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Handle for a background agent execution
#[derive(Debug)]
pub struct AgentHandle {
    pub id: String,
    pub agent_type: String,
    pub model: String,
    pub response_buffer: String,
    pub status: AgentStatus,
    pub token_usage: AgentTokenUsage,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            on_task_completed: Arc::new(Mutex::new(None)),
        }
    }

    /// Set a callback that fires when an agent completes its task.
    /// This is used by the cli layer to fire TaskCompleted hook events.
    pub async fn set_on_task_completed(&self, callback: CompletionCallback) {
        let mut cb = self.on_task_completed.lock().await;
        *cb = Some(callback);
    }

    /// Generate a unique agent ID
    pub fn generate_id(agent_type: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("agent_{}_t{}", agent_type, timestamp)
    }

    /// Register a new background agent
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the agent
    /// * `agent_type` - Type of agent being executed
    /// * `model` - Model being used
    ///
    /// # Returns
    ///
    /// The agent ID for later retrieval
    pub async fn register(
        &self,
        id: String,
        agent_type: String,
        model: String,
    ) -> Result<String, String> {
        let handle = AgentHandle {
            id: id.clone(),
            agent_type,
            model,
            response_buffer: String::new(),
            status: AgentStatus::Running,
            token_usage: AgentTokenUsage::default(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            completed_at: None,
        };

        let mut agents = self.agents.lock().await;
        agents.insert(id.clone(), handle);
        Ok(id)
    }

    /// Append response text to an agent's buffer
    pub async fn append_response(&self, id: &str, text: String) -> Result<(), String> {
        let mut agents = self.agents.lock().await;
        if let Some(handle) = agents.get_mut(id) {
            handle.response_buffer.push_str(&text);
            Ok(())
        } else {
            Err(format!("Agent not found: {}", id))
        }
    }

    /// Update token usage for an agent
    pub async fn update_token_usage(
        &self,
        id: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Result<(), String> {
        let mut agents = self.agents.lock().await;
        if let Some(handle) = agents.get_mut(id) {
            handle.token_usage.input_tokens = input_tokens;
            handle.token_usage.output_tokens = output_tokens;
            Ok(())
        } else {
            Err(format!("Agent not found: {}", id))
        }
    }

    /// Get output from an agent
    ///
    /// Returns the response, status, and token usage.
    /// Does NOT clear the buffer (agents are one-shot, unlike shells).
    pub async fn get_output(&self, id: &str) -> Result<(String, String, AgentTokenUsage), String> {
        let agents = self.agents.lock().await;
        if let Some(handle) = agents.get(id) {
            let status_str = match &handle.status {
                AgentStatus::Running => "running".to_string(),
                AgentStatus::Completed => "completed".to_string(),
                AgentStatus::Failed(msg) => format!("failed:{}", msg),
            };

            Ok((
                handle.response_buffer.clone(),
                status_str,
                handle.token_usage.clone(),
            ))
        } else {
            Err(format!("Agent not found: {}", id))
        }
    }

    /// Mark agent as completed.
    /// Fires the on_task_completed callback if one is registered.
    pub async fn mark_completed(&self, id: &str) -> Result<(), String> {
        let completion_info = {
            let mut agents = self.agents.lock().await;
            if let Some(handle) = agents.get_mut(id) {
                handle.status = AgentStatus::Completed;
                handle.completed_at = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );
                Some(AgentCompletionInfo {
                    agent_id: handle.id.clone(),
                    agent_type: handle.agent_type.clone(),
                })
            } else {
                return Err(format!("Agent not found: {}", id));
            }
        };

        // Fire callback outside the agents lock to avoid deadlocks
        if let Some(info) = completion_info {
            let cb = self.on_task_completed.lock().await;
            if let Some(callback) = cb.as_ref() {
                callback(info);
            }
        }

        Ok(())
    }

    /// Mark agent as failed
    pub async fn mark_failed(&self, id: &str, error: String) -> Result<(), String> {
        let mut agents = self.agents.lock().await;
        if let Some(handle) = agents.get_mut(id) {
            handle.status = AgentStatus::Failed(error);
            handle.completed_at = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            Ok(())
        } else {
            Err(format!("Agent not found: {}", id))
        }
    }

    /// Check if agent exists
    pub async fn exists(&self, id: &str) -> bool {
        let agents = self.agents.lock().await;
        agents.contains_key(id)
    }

    /// Get agent status
    pub async fn get_status(&self, id: &str) -> Result<AgentStatus, String> {
        let agents = self.agents.lock().await;
        agents
            .get(id)
            .map(|h| h.status.clone())
            .ok_or_else(|| format!("Agent not found: {}", id))
    }

    /// List all active agent IDs
    pub async fn list_ids(&self) -> Vec<String> {
        let agents = self.agents.lock().await;
        agents.keys().cloned().collect()
    }

    /// Remove a completed agent from the registry
    pub async fn remove(&self, id: &str) -> Result<(), String> {
        let mut agents = self.agents.lock().await;
        agents
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| format!("Agent not found: {}", id))
    }
}

/// Global agent registry instance (singleton pattern)
static GLOBAL_AGENT_REGISTRY: OnceLock<Arc<AgentRegistry>> = OnceLock::new();

/// Get or create the global agent registry instance
pub fn global_agent_registry() -> Arc<AgentRegistry> {
    Arc::clone(GLOBAL_AGENT_REGISTRY.get_or_init(|| Arc::new(AgentRegistry::new())))
}

impl Clone for AgentRegistry {
    fn clone(&self) -> Self {
        Self {
            agents: Arc::clone(&self.agents),
            on_task_completed: Arc::clone(&self.on_task_completed),
        }
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "agent_registry_tests.rs"]
mod tests;
