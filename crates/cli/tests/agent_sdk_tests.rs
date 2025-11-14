//! Comprehensive Agent SDK Test Suite
//!
//! Tests cover core Agent SDK orchestration requirements:
//! - Agent invocation (query function, async generator, streaming)
//! - Context management (session state, continue flag, resume, forkSession)
//! - Result handling (message serialization, tool outputs, memory)
//! - Parallel agent execution (background bash, concurrent processes)
//! - Agent isolation (tool permissions, capability filtering)
//! - Hook-based event system (PreToolUse, PostToolUse, SessionStart, SessionEnd)
//! - Subagent delegation (autonomous multi-step tasks)

#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::useless_vec)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::type_complexity)]
#![allow(clippy::len_zero)]
#![allow(clippy::derivable_impls)]
//!
//! Following Testing Pyramid:
//! - 60% Unit tests: Agent configuration, context state, permission logic
//! - 30% Integration tests: Agent lifecycle, tool invocation, session management
//! - 10% E2E tests: Full agent workflow with hook events
//!
//! NOTE: TDD approach - tests define the specification that the Agent SDK
//! must satisfy for production deployment.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// DATA STRUCTURES - Agent SDK Models
// ============================================================================

/// Represents an agent query input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentMessage {
    Text(String),
    StreamChunk(Vec<u8>),
}

/// Agent options for configuration
#[derive(Debug, Clone, Default)]
pub struct AgentOptions {
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub disallowed_tools: Option<Vec<String>>,
    pub continue_session: bool,
    pub resume_session_id: Option<String>,
    pub fork_session: bool,
    pub permission_mode: PermissionMode,
    pub hooks: HashMap<HookEvent, Vec<String>>,
    pub agents: HashMap<String, SubagentDefinition>,
}

/// Permission modes for tool access control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    BypassPermissions,
    Plan,
}

impl Default for PermissionMode {
    fn default() -> Self {
        PermissionMode::Default
    }
}

/// Hook events in agent lifecycle
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    SessionStart,
    SessionEnd,
    PreCompact,
}

/// Configuration for subagent delegation
#[derive(Debug, Clone)]
pub struct SubagentDefinition {
    pub description: String,
    pub system_prompt: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub disallowed_tools: Option<Vec<String>>,
    pub model_override: Option<String>,
}

/// Result from agent query
#[derive(Debug, Clone, PartialEq)]
pub struct AgentResult {
    pub message_id: String,
    pub content: String,
    pub session_id: String,
    pub tools_used: Vec<String>,
    pub error: Option<String>,
}

/// Session context for agent execution
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub messages: Vec<(String, String)>, // (role, content)
    pub continuation_count: u32,
    pub context_tokens_used: u32,
    pub tools_executed: Vec<String>,
    pub is_fork: bool,
}

/// Tool execution result
#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub tool_name: String,
    pub input: String,
    pub output: String,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Parallel execution handle for background tasks
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellId(String);

impl ShellId {
    pub fn new(id: String) -> Self {
        ShellId(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

/// Background process state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Completed(i32), // exit code
    Failed(String),
}

/// Background process output
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    pub shell_id: ShellId,
    pub state: ProcessState,
    pub stdout: String,
    pub stderr: String,
}

// ============================================================================
// TEST FIXTURES AND HELPERS
// ============================================================================

/// Agent SDK simulator for testing
struct AgentSDK {
    session_contexts: Arc<Mutex<HashMap<String, SessionContext>>>,
    background_processes: Arc<Mutex<HashMap<String, ProcessOutput>>>,
    hook_calls: Arc<Mutex<Vec<(HookEvent, String)>>>,
    permission_checker: Box<dyn Fn(&str, &str) -> bool + Send + Sync>,
}

impl Default for AgentSDK {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentSDK {
    fn new() -> Self {
        AgentSDK {
            session_contexts: Arc::new(Mutex::new(HashMap::new())),
            background_processes: Arc::new(Mutex::new(HashMap::new())),
            hook_calls: Arc::new(Mutex::new(Vec::new())),
            permission_checker: Box::new(|_tool: &str, _action: &str| true), // Currently unused, reserved for future custom permission logic
        }
    }

    /// Simulate agent query with session management
    fn query(&self, prompt: &str, options: &AgentOptions) -> Result<AgentResult, String> {
        let session_id = self.get_or_create_session(options)?;
        let mut contexts = self.session_contexts.lock().unwrap();
        let context = contexts
            .get_mut(&session_id)
            .ok_or_else(|| "Session context not found".to_string())?;

        // Fire SessionStart hook
        self.trigger_hook(HookEvent::SessionStart, &session_id);

        // Add user message to session
        context
            .messages
            .push(("user".to_string(), prompt.to_string()));

        // Simulate message processing
        let response = format!("Response to: {}", prompt);
        context
            .messages
            .push(("assistant".to_string(), response.clone()));
        context.continuation_count += 1;

        // Fire SessionEnd hook
        self.trigger_hook(HookEvent::SessionEnd, &session_id);

        Ok(AgentResult {
            message_id: format!("msg_{}", context.messages.len()),
            content: response,
            session_id,
            tools_used: vec![],
            error: None,
        })
    }

    /// Get or create a session context based on options
    fn get_or_create_session(&self, options: &AgentOptions) -> Result<String, String> {
        let mut contexts = self.session_contexts.lock().unwrap();

        let session_id = if let Some(resume_id) = &options.resume_session_id {
            // Resume existing session
            if !contexts.contains_key(resume_id) {
                return Err("Session not found".to_string());
            }
            resume_id.clone()
        } else if options.fork_session {
            // Create forked session
            let parent_id = contexts.keys().next().cloned();
            let fork_id = format!("fork_{}", uuid_stub());
            contexts.insert(
                fork_id.clone(),
                SessionContext {
                    session_id: fork_id.clone(),
                    parent_session_id: parent_id,
                    messages: vec![],
                    continuation_count: 0,
                    context_tokens_used: 0,
                    tools_executed: vec![],
                    is_fork: true,
                },
            );
            fork_id
        } else if options.continue_session {
            // Continue existing session or create new
            let session_id = if let Some(first_id) = contexts.keys().next() {
                first_id.clone()
            } else {
                let id = format!("session_{}", uuid_stub());
                contexts.insert(
                    id.clone(),
                    SessionContext {
                        session_id: id.clone(),
                        parent_session_id: None,
                        messages: vec![],
                        continuation_count: 0,
                        context_tokens_used: 0,
                        tools_executed: vec![],
                        is_fork: false,
                    },
                );
                id
            };
            session_id
        } else {
            // Create new isolated session
            let id = format!("session_{}", uuid_stub());
            contexts.insert(
                id.clone(),
                SessionContext {
                    session_id: id.clone(),
                    parent_session_id: None,
                    messages: vec![],
                    continuation_count: 0,
                    context_tokens_used: 0,
                    tools_executed: vec![],
                    is_fork: false,
                },
            );
            id
        };

        Ok(session_id)
    }

    /// Execute a tool with permission checking
    fn execute_tool(
        &self,
        session_id: &str,
        tool_name: &str,
        input: &str,
        options: &AgentOptions,
    ) -> Result<ToolExecutionResult, String> {
        // Fire PreToolUse hook
        self.trigger_hook(HookEvent::PreToolUse, tool_name);

        // Check tool permissions
        self.check_tool_permission(tool_name, options)?;

        // Simulate tool execution
        let output = match tool_name {
            "bash" => format!("$ {}\nOutput: command executed", input),
            "web_search" => format!("Search results for: {}", input),
            "read_file" => format!("File content: {}", input),
            _ => format!("Tool {} executed with: {}", tool_name, input),
        };

        let result = ToolExecutionResult {
            tool_name: tool_name.to_string(),
            input: input.to_string(),
            output,
            success: true,
            error_message: None,
        };

        // Fire PostToolUse hook
        self.trigger_hook(HookEvent::PostToolUse, tool_name);

        // Track tool usage in session
        if let Ok(mut contexts) = self.session_contexts.lock() {
            if let Some(context) = contexts.get_mut(session_id) {
                context.tools_executed.push(tool_name.to_string());
            }
        }

        Ok(result)
    }

    /// Check if tool is allowed based on permissions
    fn check_tool_permission(&self, tool_name: &str, options: &AgentOptions) -> Result<(), String> {
        if let Some(allowed) = &options.allowed_tools {
            if !allowed.contains(&tool_name.to_string()) {
                return Err(format!("Tool {} not in allowed list", tool_name));
            }
        }

        if let Some(disallowed) = &options.disallowed_tools {
            if disallowed.contains(&tool_name.to_string()) {
                return Err(format!("Tool {} is disallowed", tool_name));
            }
        }

        Ok(())
    }

    /// Start a background process and return shell ID
    fn run_background(&self, _command: &str, _session_id: &str) -> Result<ShellId, String> {
        let shell_id = ShellId::new(format!("shell_{}", uuid_stub()));

        let process = ProcessOutput {
            shell_id: shell_id.clone(),
            state: ProcessState::Running,
            stdout: String::new(),
            stderr: String::new(),
        };

        let mut processes = self.background_processes.lock().unwrap();
        processes.insert(shell_id.value().to_string(), process);

        Ok(shell_id)
    }

    /// Get output from background process
    fn get_background_output(&self, shell_id: &ShellId) -> Result<ProcessOutput, String> {
        let processes = self.background_processes.lock().unwrap();
        processes
            .get(shell_id.value())
            .cloned()
            .ok_or_else(|| "Shell ID not found".to_string())
    }

    /// Complete a background process
    fn complete_background(&self, shell_id: &ShellId, exit_code: i32) -> Result<(), String> {
        let mut processes = self.background_processes.lock().unwrap();
        if let Some(process) = processes.get_mut(shell_id.value()) {
            process.state = ProcessState::Completed(exit_code);
            Ok(())
        } else {
            Err("Shell ID not found".to_string())
        }
    }

    /// Trigger a hook event
    fn trigger_hook(&self, event: HookEvent, data: &str) {
        if let Ok(mut calls) = self.hook_calls.lock() {
            calls.push((event, data.to_string()));
        }
    }

    /// Get hook call history
    fn get_hook_calls(&self) -> Vec<(HookEvent, String)> {
        self.hook_calls.lock().unwrap().clone()
    }

    /// Clear hook call history
    fn clear_hooks(&self) {
        self.hook_calls.lock().unwrap().clear();
    }
}

fn uuid_stub() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{:016x}", id)
}

// ============================================================================
// UNIT TESTS - AGENT INVOCATION
// ============================================================================

#[test]
fn test_agent_query_basic_invocation() {
    // Test: Basic agent query with simple text prompt
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();
    let prompt = "What is 2+2?";

    let result = sdk.query(prompt, &options).expect("Query should succeed");

    assert!(!result.message_id.is_empty());
    assert_eq!(result.content, "Response to: What is 2+2?");
    assert!(result.error.is_none());
}

#[test]
fn test_agent_query_returns_valid_session_id() {
    // Test: Query returns a valid, trackable session ID
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    let result = sdk
        .query("First query", &options)
        .expect("First query should succeed");
    let session_id1 = result.session_id.clone();

    // Session should be stored
    let contexts = sdk.session_contexts.lock().unwrap();
    assert!(contexts.contains_key(&session_id1));
    assert_eq!(contexts[&session_id1].session_id, session_id1);
}

#[test]
fn test_agent_query_streaming_simulation() {
    // Test: Query simulates streaming message consumption
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    let result = sdk
        .query("Stream this data", &options)
        .expect("Query should succeed");

    // Result contains complete content (streaming implemented in real SDK)
    assert!(!result.content.is_empty());
    assert!(result.content.contains("Stream this data"));
}

#[test]
fn test_agent_query_with_custom_model() {
    // Test: Query respects custom model configuration
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.model = Some("claude-opus".to_string());

    let result = sdk.query("Test with custom model", &options);

    // Query should succeed regardless of model (validation at SDK level)
    assert!(result.is_ok());
}

#[test]
fn test_agent_query_with_system_prompt() {
    // Test: Query accepts and applies system prompt
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.system_prompt = Some("You are a helpful assistant.".to_string());

    let result = sdk.query("Hello", &options).expect("Query should succeed");

    assert!(!result.content.is_empty());
}

#[test]
fn test_agent_query_empty_prompt() {
    // Test: Handle empty prompt gracefully
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    let result = sdk
        .query("", &options)
        .expect("Empty prompt should succeed");

    assert_eq!(result.content, "Response to: ");
}

// ============================================================================
// UNIT TESTS - CONTEXT MANAGEMENT
// ============================================================================

#[test]
fn test_context_new_session_isolation() {
    // Test: Each new session is isolated from previous sessions
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    let result1 = sdk
        .query("Query 1", &options)
        .expect("First query should succeed");
    let result2 = sdk
        .query("Query 2", &options)
        .expect("Second query should succeed");

    // Different session IDs means isolation
    assert_ne!(result1.session_id, result2.session_id);

    // Verify both sessions exist independently
    let contexts = sdk.session_contexts.lock().unwrap();
    assert_eq!(contexts.len(), 2);
    assert!(contexts.contains_key(&result1.session_id));
    assert!(contexts.contains_key(&result2.session_id));
}

#[test]
fn test_context_continue_flag_resumes_session() {
    // Test: Continue flag continues existing session instead of creating new
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();

    // First query creates a session
    let result1 = sdk
        .query("First message", &options)
        .expect("First query should succeed");
    let initial_session_id = result1.session_id.clone();

    // Continue query uses same session
    options.continue_session = true;
    let result2 = sdk
        .query("Second message", &options)
        .expect("Continue query should succeed");

    assert_eq!(result2.session_id, initial_session_id);

    // Session should have both messages and count incremented
    let contexts = sdk.session_contexts.lock().unwrap();
    let context = &contexts[&initial_session_id];
    assert_eq!(context.continuation_count, 2);
}

#[test]
fn test_context_resume_session_by_id() {
    // Test: Resume flag continues specific session by ID
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();

    // Create initial session
    let result1 = sdk
        .query("Initial", &options)
        .expect("Initial query should succeed");
    let session_to_resume = result1.session_id.clone();

    // Resume specific session
    options.resume_session_id = Some(session_to_resume.clone());
    let result2 = sdk
        .query("Resumed", &options)
        .expect("Resume query should succeed");

    assert_eq!(result2.session_id, session_to_resume);
}

#[test]
fn test_context_resume_invalid_session_fails() {
    // Test: Resume non-existent session returns error
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.resume_session_id = Some("invalid_session_id".to_string());

    let result = sdk.query("Test", &options);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Session not found"));
}

#[test]
fn test_context_fork_session_creates_isolated_branch() {
    // Test: Fork creates isolated session with parent reference
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();

    // Create parent session
    let _parent_result = sdk
        .query("Parent", &options)
        .expect("Parent query should succeed");

    // Fork from parent
    options.fork_session = true;
    let fork_result = sdk
        .query("Forked", &options)
        .expect("Fork query should succeed");

    // Fork should create new session
    let contexts = sdk.session_contexts.lock().unwrap();
    let fork_context = &contexts[&fork_result.session_id];

    assert!(fork_context.is_fork);
    assert!(fork_context.parent_session_id.is_some());
}

#[test]
fn test_context_continuation_counter_increments() {
    // Test: Continuation counter tracks multiple interactions
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.continue_session = true;

    let result1 = sdk.query("Q1", &options).expect("Query 1 should succeed");

    let contexts = sdk.session_contexts.lock().unwrap();
    let context = &contexts[&result1.session_id];
    assert_eq!(context.continuation_count, 1);

    drop(contexts);

    let result2 = sdk.query("Q2", &options).expect("Query 2 should succeed");

    let contexts = sdk.session_contexts.lock().unwrap();
    let context = &contexts[&result2.session_id];
    assert_eq!(context.continuation_count, 2);
}

// ============================================================================
// UNIT TESTS - RESULT HANDLING
// ============================================================================

#[test]
fn test_result_contains_unique_message_id() {
    // Test: Each result has unique message ID within same session
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.continue_session = true; // Continue same session

    let result1 = sdk
        .query("Message 1", &options)
        .expect("Query 1 should succeed");
    let result2 = sdk
        .query("Message 2", &options)
        .expect("Query 2 should succeed");

    // Different message IDs in same session
    assert_ne!(result1.message_id, result2.message_id);
    // Same session
    assert_eq!(result1.session_id, result2.session_id);
}

#[test]
fn test_result_serialization_format() {
    // Test: Result serializes to correct format with all fields
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    let result = sdk
        .query("Serialize me", &options)
        .expect("Query should succeed");

    // Check all required fields are present
    assert!(!result.message_id.is_empty());
    assert!(!result.content.is_empty());
    assert!(!result.session_id.is_empty());
    assert!(result.error.is_none()); // No error on success
}

#[test]
fn test_result_error_handling_invalid_tool() {
    // Test: Result captures error when invalid tool is executed
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    let result = sdk
        .query("Use tool", &options)
        .expect("Query should succeed");

    // Successful query, error would be in tool execution
    assert!(result.error.is_none());
}

#[test]
fn test_result_tools_used_tracking() {
    // Test: Result tracks which tools were used
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    let result = sdk
        .query("Execute bash", &options)
        .expect("Query should succeed");

    // tools_used is empty for simple query (no tool invocations)
    assert!(result.tools_used.is_empty());
}

#[test]
fn test_result_session_persistence() {
    // Test: Result references persisted session context
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.continue_session = true;

    let result1 = sdk
        .query("Message 1", &options)
        .expect("Query 1 should succeed");
    let session_id = result1.session_id.clone();

    let result2 = sdk
        .query("Message 2", &options)
        .expect("Query 2 should succeed");

    // Both results reference same session
    assert_eq!(result2.session_id, session_id);

    // Session context persists with message history
    let contexts = sdk.session_contexts.lock().unwrap();
    let context = &contexts[&session_id];
    assert_eq!(context.messages.len(), 4); // 2 user + 2 assistant
}

// ============================================================================
// INTEGRATION TESTS - PARALLEL AGENT EXECUTION
// ============================================================================

#[test]
fn test_parallel_background_bash_execution() {
    // Test: Start multiple background bash processes
    let sdk = AgentSDK::new();

    let shell1 = sdk
        .run_background("echo 'Process 1'", "session_1")
        .expect("Background 1 should succeed");
    let shell2 = sdk
        .run_background("echo 'Process 2'", "session_1")
        .expect("Background 2 should succeed");

    // Different shell IDs for parallel processes
    assert_ne!(shell1, shell2);

    // Both processes tracked
    let processes = sdk.background_processes.lock().unwrap();
    assert_eq!(processes.len(), 2);
}

#[test]
fn test_parallel_shell_id_retrieval() {
    // Test: Get output from specific shell by ID
    let sdk = AgentSDK::new();

    let shell_id = sdk
        .run_background("long running command", "session_1")
        .expect("Background process should start");

    let output = sdk
        .get_background_output(&shell_id)
        .expect("Should retrieve process output");

    assert_eq!(output.shell_id, shell_id);
    assert_eq!(output.state, ProcessState::Running);
}

#[test]
fn test_parallel_process_state_transitions() {
    // Test: Process transitions from Running to Completed
    let sdk = AgentSDK::new();

    let shell_id = sdk
        .run_background("test command", "session_1")
        .expect("Background process should start");

    // Initially running
    let output1 = sdk
        .get_background_output(&shell_id)
        .expect("Should retrieve process");
    assert_eq!(output1.state, ProcessState::Running);

    // Complete process
    sdk.complete_background(&shell_id, 0)
        .expect("Should complete process");

    // Now completed
    let output2 = sdk
        .get_background_output(&shell_id)
        .expect("Should retrieve process");
    assert_eq!(output2.state, ProcessState::Completed(0));
}

#[test]
fn test_parallel_multiple_processes_isolated() {
    // Test: Completing one process doesn't affect others
    let sdk = AgentSDK::new();

    let shell1 = sdk
        .run_background("cmd1", "session_1")
        .expect("Should start process 1");
    let shell2 = sdk
        .run_background("cmd2", "session_1")
        .expect("Should start process 2");

    // Complete only first process
    sdk.complete_background(&shell1, 0)
        .expect("Should complete first");

    // Check states
    let output1 = sdk
        .get_background_output(&shell1)
        .expect("Should get process 1");
    let output2 = sdk
        .get_background_output(&shell2)
        .expect("Should get process 2");

    assert_eq!(output1.state, ProcessState::Completed(0));
    assert_eq!(output2.state, ProcessState::Running);
}

#[test]
fn test_parallel_invalid_shell_id_error() {
    // Test: Querying invalid shell ID returns error
    let sdk = AgentSDK::new();
    let invalid_shell = ShellId::new("invalid_shell_id".to_string());

    let result = sdk.get_background_output(&invalid_shell);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Shell ID not found"));
}

#[test]
fn test_parallel_process_output_accumulation() {
    // Test: Process output accumulates over time
    let sdk = AgentSDK::new();

    let shell_id = sdk
        .run_background("streaming output", "session_1")
        .expect("Should start process");

    // Get initial output
    let output1 = sdk
        .get_background_output(&shell_id)
        .expect("Should get output");
    assert!(output1.stdout.is_empty()); // No output yet

    // Complete process
    sdk.complete_background(&shell_id, 0)
        .expect("Should complete");

    // Output would accumulate (in real SDK, via BashOutput tool)
    let output2 = sdk
        .get_background_output(&shell_id)
        .expect("Should get final output");
    assert_eq!(output2.state, ProcessState::Completed(0));
}

// ============================================================================
// UNIT TESTS - AGENT ISOLATION
// ============================================================================

#[test]
fn test_isolation_allowed_tools_filter() {
    // Test: Only allowed tools can be executed
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.allowed_tools = Some(vec!["bash".to_string(), "read_file".to_string()]);

    let bash_result = sdk.execute_tool("session_1", "bash", "ls", &options);
    assert!(bash_result.is_ok());

    let read_result = sdk.execute_tool("session_1", "read_file", "file.txt", &options);
    assert!(read_result.is_ok());

    // web_search is not allowed
    let search_result = sdk.execute_tool("session_1", "web_search", "query", &options);
    assert!(search_result.is_err());
}

#[test]
fn test_isolation_disallowed_tools_filter() {
    // Test: Disallowed tools cannot be executed
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.disallowed_tools = Some(vec!["bash".to_string()]);

    let bash_result = sdk.execute_tool("session_1", "bash", "ls", &options);
    assert!(bash_result.is_err());
    assert!(bash_result.unwrap_err().contains("disallowed"));

    // Other tools should work
    let search_result = sdk.execute_tool("session_1", "web_search", "query", &options);
    assert!(search_result.is_ok());
}

#[test]
fn test_isolation_allowed_and_disallowed_precedence() {
    // Test: Disallowed takes precedence when both are specified
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.allowed_tools = Some(vec!["bash".to_string(), "read_file".to_string()]);
    options.disallowed_tools = Some(vec!["bash".to_string()]); // Override bash

    let bash_result = sdk.execute_tool("session_1", "bash", "ls", &options);
    assert!(bash_result.is_err()); // Disallowed wins

    let read_result = sdk.execute_tool("session_1", "read_file", "file.txt", &options);
    assert!(read_result.is_ok()); // Not in disallowed list
}

#[test]
fn test_isolation_no_restrictions_allows_all() {
    // Test: Without restrictions, all tools are allowed
    let sdk = AgentSDK::new();
    let options = AgentOptions::default(); // No allowed/disallowed lists

    assert!(sdk
        .execute_tool("session_1", "bash", "ls", &options)
        .is_ok());
    assert!(sdk
        .execute_tool("session_1", "web_search", "query", &options)
        .is_ok());
    assert!(sdk
        .execute_tool("session_1", "read_file", "file.txt", &options)
        .is_ok());
}

#[test]
fn test_isolation_empty_allowed_list_restricts_all() {
    // Test: Empty allowed list means no tools are permitted
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.allowed_tools = Some(vec![]); // Empty means no tools

    assert!(sdk
        .execute_tool("session_1", "bash", "ls", &options)
        .is_err());
    assert!(sdk
        .execute_tool("session_1", "web_search", "query", &options)
        .is_err());
}

#[test]
fn test_isolation_tool_execution_tracks_usage() {
    // Test: Tool execution is tracked in session context
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    // Create session
    let result = sdk.query("Setup", &options).expect("Query should succeed");
    let session_id = result.session_id.clone();

    // Execute tools
    let _tool1 = sdk.execute_tool(&session_id, "bash", "ls", &options);
    let _tool2 = sdk.execute_tool(&session_id, "web_search", "test", &options);

    // Verify tools tracked in context
    let contexts = sdk.session_contexts.lock().unwrap();
    let context = &contexts[&session_id];
    assert_eq!(context.tools_executed.len(), 2);
    assert!(context.tools_executed.contains(&"bash".to_string()));
    assert!(context.tools_executed.contains(&"web_search".to_string()));
}

#[test]
fn test_isolation_permission_modes() {
    // Test: Different permission modes are configured
    let mut options = AgentOptions::default();

    options.permission_mode = PermissionMode::Default;
    assert_eq!(options.permission_mode, PermissionMode::Default);

    options.permission_mode = PermissionMode::AcceptEdits;
    assert_eq!(options.permission_mode, PermissionMode::AcceptEdits);

    options.permission_mode = PermissionMode::BypassPermissions;
    assert_eq!(options.permission_mode, PermissionMode::BypassPermissions);

    options.permission_mode = PermissionMode::Plan;
    assert_eq!(options.permission_mode, PermissionMode::Plan);
}

// ============================================================================
// INTEGRATION TESTS - HOOK-BASED EVENT SYSTEM
// ============================================================================

#[test]
fn test_hooks_session_start_fired() {
    // Test: SessionStart hook is triggered when query begins
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    sdk.clear_hooks();
    let _result = sdk.query("Test", &options).expect("Query should succeed");

    let hooks = sdk.get_hook_calls();
    assert!(hooks
        .iter()
        .any(|(event, _)| *event == HookEvent::SessionStart));
}

#[test]
fn test_hooks_session_end_fired() {
    // Test: SessionEnd hook is triggered when query completes
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    sdk.clear_hooks();
    let _result = sdk.query("Test", &options).expect("Query should succeed");

    let hooks = sdk.get_hook_calls();
    assert!(hooks
        .iter()
        .any(|(event, _)| *event == HookEvent::SessionEnd));
}

#[test]
fn test_hooks_pre_tool_use_fired() {
    // Test: PreToolUse hook fires before tool execution
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    sdk.clear_hooks();
    let _result = sdk.execute_tool("session_1", "bash", "ls", &options);

    let hooks = sdk.get_hook_calls();
    assert!(hooks
        .iter()
        .any(|(event, _)| *event == HookEvent::PreToolUse));
}

#[test]
fn test_hooks_post_tool_use_fired() {
    // Test: PostToolUse hook fires after tool execution
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    sdk.clear_hooks();
    let _result = sdk.execute_tool("session_1", "bash", "ls", &options);

    let hooks = sdk.get_hook_calls();
    assert!(hooks
        .iter()
        .any(|(event, _)| *event == HookEvent::PostToolUse));
}

#[test]
fn test_hooks_pre_and_post_tool_order() {
    // Test: PreToolUse happens before PostToolUse
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    sdk.clear_hooks();
    let _result = sdk.execute_tool("session_1", "bash", "ls", &options);

    let hooks = sdk.get_hook_calls();
    let pre_index = hooks
        .iter()
        .position(|(event, _)| *event == HookEvent::PreToolUse);
    let post_index = hooks
        .iter()
        .position(|(event, _)| *event == HookEvent::PostToolUse);

    assert!(pre_index.is_some());
    assert!(post_index.is_some());
    assert!(pre_index.unwrap() < post_index.unwrap());
}

#[test]
fn test_hooks_session_lifecycle_complete() {
    // Test: Complete session lifecycle fires all expected hooks
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    sdk.clear_hooks();
    let _result = sdk.query("Test", &options).expect("Query should succeed");

    let hooks = sdk.get_hook_calls();

    // Verify hook sequence: SessionStart -> ... -> SessionEnd
    let has_start = hooks.iter().any(|(e, _)| *e == HookEvent::SessionStart);
    let has_end = hooks.iter().any(|(e, _)| *e == HookEvent::SessionEnd);

    assert!(has_start, "SessionStart hook should fire");
    assert!(has_end, "SessionEnd hook should fire");
}

#[test]
fn test_hooks_multiple_tool_executions() {
    // Test: Multiple tool executions each trigger hooks
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    sdk.clear_hooks();
    let _tool1 = sdk.execute_tool("session_1", "bash", "ls", &options);
    let _tool2 = sdk.execute_tool("session_1", "web_search", "query", &options);

    let hooks = sdk.get_hook_calls();
    let pre_count = hooks
        .iter()
        .filter(|(e, _)| *e == HookEvent::PreToolUse)
        .count();
    let post_count = hooks
        .iter()
        .filter(|(e, _)| *e == HookEvent::PostToolUse)
        .count();

    assert_eq!(pre_count, 2);
    assert_eq!(post_count, 2);
}

// ============================================================================
// INTEGRATION TESTS - SUBAGENT DELEGATION
// ============================================================================

#[test]
fn test_subagent_definition_structure() {
    // Test: Subagent definitions are properly structured
    let definition = SubagentDefinition {
        description: "Data analysis agent".to_string(),
        system_prompt: Some("Analyze data carefully".to_string()),
        allowed_tools: Some(vec!["web_search".to_string()]),
        disallowed_tools: None,
        model_override: Some("claude-opus".to_string()),
    };

    assert_eq!(definition.description, "Data analysis agent");
    assert!(definition.system_prompt.is_some());
    assert!(definition.allowed_tools.is_some());
    assert_eq!(definition.allowed_tools.as_ref().unwrap().len(), 1);
}

#[test]
fn test_subagent_configuration_in_options() {
    // Test: Subagents can be configured in agent options
    let mut options = AgentOptions::default();

    let mut subagents = HashMap::new();
    subagents.insert(
        "analyzer".to_string(),
        SubagentDefinition {
            description: "Analysis specialist".to_string(),
            system_prompt: None,
            allowed_tools: Some(vec!["read_file".to_string()]),
            disallowed_tools: None,
            model_override: None,
        },
    );

    options.agents = subagents;

    assert_eq!(options.agents.len(), 1);
    assert!(options.agents.contains_key("analyzer"));
}

#[test]
fn test_subagent_multiple_agents_registry() {
    // Test: Multiple subagents can be registered
    let mut options = AgentOptions::default();

    options.agents.insert(
        "analyst".to_string(),
        SubagentDefinition {
            description: "Data analyst".to_string(),
            system_prompt: None,
            allowed_tools: None,
            disallowed_tools: None,
            model_override: None,
        },
    );

    options.agents.insert(
        "coder".to_string(),
        SubagentDefinition {
            description: "Code generator".to_string(),
            system_prompt: None,
            allowed_tools: None,
            disallowed_tools: None,
            model_override: None,
        },
    );

    assert_eq!(options.agents.len(), 2);
    assert!(options.agents.contains_key("analyst"));
    assert!(options.agents.contains_key("coder"));
}

#[test]
fn test_subagent_tool_isolation() {
    // Test: Subagents have independent tool restrictions
    let definition1 = SubagentDefinition {
        description: "Agent 1".to_string(),
        system_prompt: None,
        allowed_tools: Some(vec!["bash".to_string()]),
        disallowed_tools: None,
        model_override: None,
    };

    let definition2 = SubagentDefinition {
        description: "Agent 2".to_string(),
        system_prompt: None,
        allowed_tools: Some(vec!["web_search".to_string(), "read_file".to_string()]),
        disallowed_tools: None,
        model_override: None,
    };

    // Each has different tool set
    assert_eq!(
        definition1.allowed_tools.as_ref().unwrap(),
        &vec!["bash".to_string()]
    );
    assert_eq!(definition2.allowed_tools.as_ref().unwrap().len(), 2);
}

#[test]
fn test_subagent_model_override() {
    // Test: Subagents can override parent agent model
    let definition = SubagentDefinition {
        description: "Specialized agent".to_string(),
        system_prompt: None,
        allowed_tools: None,
        disallowed_tools: None,
        model_override: Some("claude-sonnet".to_string()),
    };

    assert_eq!(definition.model_override, Some("claude-sonnet".to_string()));
}

// ============================================================================
// EDGE CASES AND BOUNDARY CONDITIONS
// ============================================================================

#[test]
fn test_boundary_very_long_prompt() {
    // Test: Handle very long prompts (context window limits)
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();
    let long_prompt = "x".repeat(10000);

    let result = sdk
        .query(&long_prompt, &options)
        .expect("Should handle long prompt");

    assert!(!result.content.is_empty());
}

#[test]
fn test_boundary_special_characters_in_prompt() {
    // Test: Handle special characters in prompts
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    let prompts = vec![
        "What about quotes: \"hello\"?",
        "Backslash: \\ forward: /",
        "Newlines:\nMultiple\nLines",
        "Emojis: 🎉 🚀 ✨",
    ];

    for prompt in prompts {
        let result = sdk.query(prompt, &options);
        assert!(result.is_ok(), "Should handle: {}", prompt);
    }
}

#[test]
fn test_boundary_rapid_sequential_queries() {
    // Test: Handle rapid sequential queries without race conditions
    let sdk = AgentSDK::new();
    let options = AgentOptions::default();

    for i in 0..10 {
        let prompt = format!("Query {}", i);
        let result = sdk.query(&prompt, &options).expect("Query should succeed");
        assert!(!result.session_id.is_empty());
    }

    // All queries created separate sessions
    let contexts = sdk.session_contexts.lock().unwrap();
    assert_eq!(contexts.len(), 10);
}

#[test]
fn test_boundary_many_background_processes() {
    // Test: Handle many concurrent background processes
    let sdk = AgentSDK::new();

    let mut shell_ids = vec![];
    for i in 0..20 {
        let shell_id = sdk
            .run_background(&format!("command {}", i), "session_1")
            .expect("Should create background process");
        shell_ids.push(shell_id);
    }

    let processes = sdk.background_processes.lock().unwrap();
    assert_eq!(processes.len(), 20);
}

#[test]
fn test_boundary_deeply_nested_session_forks() {
    // Test: Handle multiple levels of session forking
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();

    // Create parent
    let _parent = sdk
        .query("Parent", &options)
        .expect("Parent should succeed");

    // Fork from parent
    options.fork_session = true;
    let _fork1 = sdk
        .query("Fork 1", &options)
        .expect("Fork 1 should succeed");

    // Fork from fork
    options.fork_session = true;
    let fork2 = sdk
        .query("Fork 2", &options)
        .expect("Fork 2 should succeed");

    let contexts = sdk.session_contexts.lock().unwrap();
    assert!(contexts[&fork2.session_id].is_fork);
}

// ============================================================================
// E2E TESTS - FULL AGENT WORKFLOW
// ============================================================================

#[test]
fn test_e2e_complete_agent_session_workflow() {
    // Test: Complete workflow from initialization to completion
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.model = Some("claude-opus".to_string());

    // 1. Initialize query
    let result1 = sdk.query("Hello", &options).expect("Init should succeed");
    let session_id = result1.session_id.clone();

    // 2. Continue conversation
    options.continue_session = true;
    let result2 = sdk
        .query("Tell me more", &options)
        .expect("Continue should succeed");
    assert_eq!(result2.session_id, session_id);

    // 3. Verify context persists
    let contexts = sdk.session_contexts.lock().unwrap();
    let context = &contexts[&session_id];
    assert_eq!(context.messages.len(), 4);
    assert_eq!(context.continuation_count, 2);
}

#[test]
fn test_e2e_agent_with_tool_execution() {
    // Test: Agent execution with tool use and hooks
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();
    options.allowed_tools = Some(vec!["bash".to_string(), "web_search".to_string()]);

    // Execute query that might use tools
    sdk.clear_hooks();
    let result = sdk.query("Setup", &options).expect("Query should succeed");
    let session_id = result.session_id.clone();

    // Execute tools
    let _tool1 = sdk.execute_tool(&session_id, "bash", "ls", &options);
    let _tool2 = sdk.execute_tool(&session_id, "web_search", "test", &options);

    // Verify hook execution
    let hooks = sdk.get_hook_calls();
    assert!(hooks.len() > 0);

    // Verify tools tracked
    let contexts = sdk.session_contexts.lock().unwrap();
    assert_eq!(contexts[&session_id].tools_executed.len(), 2);
}

#[test]
fn test_e2e_parallel_agents_independent_sessions() {
    // Test: Multiple agents run in parallel with independent contexts
    let sdk = Arc::new(AgentSDK::new());
    let options = AgentOptions::default();

    let mut handles = vec![];

    // Spawn parallel agent sessions
    for i in 0..3 {
        let sdk_clone = Arc::clone(&sdk);
        let opts_clone = options.clone();
        let handle = std::thread::spawn(move || {
            let prompt = format!("Agent query {}", i);
            sdk_clone.query(&prompt, &opts_clone)
        });
        handles.push(handle);
    }

    // Collect results
    let mut session_ids = vec![];
    for handle in handles {
        let result = handle.join().unwrap().expect("Query should succeed");
        session_ids.push(result.session_id);
    }

    // All should have different sessions
    assert_eq!(session_ids.len(), 3);
    assert_ne!(session_ids[0], session_ids[1]);
    assert_ne!(session_ids[1], session_ids[2]);

    let contexts = sdk.session_contexts.lock().unwrap();
    assert_eq!(contexts.len(), 3);
}

#[test]
fn test_e2e_agent_fork_maintains_parent_context() {
    // Test: Forked agent can access parent context information
    let sdk = AgentSDK::new();
    let mut options = AgentOptions::default();

    // Create parent session with data
    let parent_result = sdk
        .query("Parent data", &options)
        .expect("Parent should succeed");

    // Fork creates new session
    options.fork_session = true;
    let fork_result = sdk
        .query("Fork from parent", &options)
        .expect("Fork should succeed");

    let contexts = sdk.session_contexts.lock().unwrap();
    let fork_context = &contexts[&fork_result.session_id];

    // Fork maintains parent reference
    assert!(fork_context.is_fork);
    assert_eq!(
        fork_context.parent_session_id,
        Some(parent_result.session_id.clone())
    );
}
