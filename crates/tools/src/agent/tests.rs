use super::*;
use crate::agent_memory::MemoryScope;
use crate::agent_registry::global_agent_registry;
use crate::{ExecutionContext, RuntimeAgentInfo, Tool, ToolContext, ToolEvent};
use futures::StreamExt;
use std::collections::HashMap;
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

    let prompt = AgentTool::load_agent_prompt("test_agent", &cwd, &ToolContext::default())
        .await
        .unwrap();

    assert!(prompt.contains("test agent"));
    assert!(prompt.contains("concisely"));
}

#[tokio::test]
async fn test_load_agent_prompt_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let cwd = temp_dir.path().to_path_buf();

    let result = AgentTool::load_agent_prompt("nonexistent", &cwd, &ToolContext::default()).await;
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
        runtime_agents: std::collections::HashMap::new(),
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
        runtime_agents: std::collections::HashMap::new(),
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
    let raw = AgentTool::load_agent_prompt("bg_agent", temp_dir.path(), &ToolContext::default())
        .await
        .unwrap();
    let (fm, prompt) = AgentFrontmatter::parse(&raw);

    assert!(fm.background);
    assert_eq!(fm.memory_scope, Some(MemoryScope::Project));
    assert!(prompt.contains("BG Agent"));
    assert!(!prompt.contains("background: true"));
}

// =============================================================================
// AgentTool::execute coverage (issue #514)
//
// The tests below exercise the core `AgentTool::execute` Tool implementation
// directly. They deterministically cover the network-free code paths:
//   - missing agent definition -> Error event
//   - worktree-isolation failure -> Error event
//   - background mode early-return -> Result event (success path), including the
//     runtime-agent (`--agents`) fallback, model resolution, frontmatter-forced
//     background, and explicit resume IDs
//   - tool metadata / capability flags
//   - AgentParams (argument) deserialization, including malformed input
//
// No assertion here depends on the Claude API / provider boundary: error paths
// return before any HTTP client is built, and background mode hands the network
// call to a detached `tokio::spawn` task that the foreground stream (and thus
// these tests) never awaits, so its outcome cannot influence any assertion. (If
// real credentials happen to be present, that detached task may briefly start an
// outbound request before the test runtime drops and cancels it; this is never
// observed by the assertions.) Tests that depend on background mode being active
// short-circuit when `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` forces foreground
// execution, keeping them deterministic in any environment.
// =============================================================================

/// Build a `ToolContext` rooted at `cwd` with the given runtime agents.
fn execute_ctx(cwd: PathBuf, runtime_agents: HashMap<String, RuntimeAgentInfo>) -> ToolContext {
    ToolContext {
        cwd,
        debug: false,
        metadata: serde_json::Value::Null,
        execution_context: ExecutionContext::default(),
        allowed_tools: vec![],
        disallowed_tools: vec![],
        runtime_agents,
    }
}

/// Build `AgentParams` with sensible defaults for the fields a test does not care about.
fn execute_params(
    subagent_type: &str,
    model: Option<&str>,
    resume: Option<&str>,
    run_in_background: bool,
) -> AgentParams {
    AgentParams {
        description: "Test task".to_string(),
        prompt: "Do the thing".to_string(),
        subagent_type: subagent_type.to_string(),
        model: model.map(str::to_string),
        resume: resume.map(str::to_string),
        run_in_background,
        memory_scope: None,
    }
}

/// Drive `AgentTool::execute` to completion and collect every emitted event.
async fn run_execute(params: AgentParams, ctx: &ToolContext) -> Vec<ToolEvent<AgentOutput>> {
    let tool = AgentTool;
    let stream = tool
        .execute(params, ctx)
        .await
        .expect("execute returns a stream");
    stream.collect().await
}

fn first_error(events: &[ToolEvent<AgentOutput>]) -> Option<&str> {
    events.iter().find_map(|e| match e {
        ToolEvent::Error { message } => Some(message.as_str()),
        _ => None,
    })
}

fn first_result(events: &[ToolEvent<AgentOutput>]) -> Option<&AgentOutput> {
    events.iter().find_map(|e| match e {
        ToolEvent::Result(out) => Some(out),
        _ => None,
    })
}

fn has_progress(events: &[ToolEvent<AgentOutput>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ToolEvent::Progress { .. }))
}

/// Returns true when background mode is force-disabled by the environment.
/// Tests that rely on background mode's deterministic early-return skip in that case.
fn background_disabled() -> bool {
    rustyclawd_core::is_background_tasks_disabled()
}

#[tokio::test]
async fn test_execute_agent_not_found_yields_error() {
    // Table of agent types that exist neither on disk nor as runtime agents.
    let cases = ["nonexistent", "missing-agent", "no_such_agent"];
    for agent_type in cases {
        let temp_dir = TempDir::new().unwrap();
        let ctx = execute_ctx(temp_dir.path().to_path_buf(), HashMap::new());
        let events = run_execute(execute_params(agent_type, None, None, false), &ctx).await;

        // Loading progress is always emitted before resolution is attempted.
        assert!(
            has_progress(&events),
            "case `{agent_type}`: expected a progress event"
        );
        let err = first_error(&events)
            .unwrap_or_else(|| panic!("case `{agent_type}`: expected an error event"));
        assert!(
            err.contains("not found"),
            "case `{agent_type}`: error should mention `not found`, got: {err}"
        );
        // A failed load must not produce a result.
        assert!(
            first_result(&events).is_none(),
            "case `{agent_type}`: must not yield a result on failure"
        );
    }
}

#[tokio::test]
async fn test_execute_first_event_is_progress_then_error() {
    // The streaming contract: progress is yielded before the failure is known.
    let temp_dir = TempDir::new().unwrap();
    let ctx = execute_ctx(temp_dir.path().to_path_buf(), HashMap::new());
    let events = run_execute(execute_params("nonexistent", None, None, false), &ctx).await;

    assert!(
        matches!(events.first(), Some(ToolEvent::Progress { .. })),
        "first event should be a progress update, got: {:?}",
        events.first()
    );
    assert!(
        matches!(events.last(), Some(ToolEvent::Error { .. })),
        "last event should be the error, got: {:?}",
        events.last()
    );
}

#[tokio::test]
async fn test_execute_worktree_isolation_failure_yields_error() {
    let temp_dir = TempDir::new().unwrap();
    // This path only fails (deterministically) when cwd is NOT inside a git repo.
    // Skip if the temp dir is unexpectedly within one (keeps the test non-flaky).
    if git2::Repository::discover(temp_dir.path()).is_ok() {
        return;
    }

    let agents_dir = temp_dir.path().join(".claude").join("agents");
    fs::create_dir_all(&agents_dir).await.unwrap();
    fs::write(
        agents_dir.join("wt_agent.md"),
        "---\nisolation: worktree\n---\n# WT Agent\nYou run isolated.",
    )
    .await
    .unwrap();

    let ctx = execute_ctx(temp_dir.path().to_path_buf(), HashMap::new());
    let events = run_execute(execute_params("wt_agent", None, None, false), &ctx).await;

    let err = first_error(&events).expect("expected a worktree creation error");
    assert!(
        err.contains("worktree"),
        "error should mention worktree isolation, got: {err}"
    );
    assert!(
        first_result(&events).is_none(),
        "worktree failure must not yield a result"
    );
}

#[tokio::test]
async fn test_execute_background_mode_returns_result_for_file_agent() {
    if background_disabled() {
        return; // environment forces foreground (network) execution; skip.
    }
    let temp_dir = TempDir::new().unwrap();
    let cwd = setup_test_agent(&temp_dir).await; // writes .claude/agents/test_agent.md

    let ctx = execute_ctx(cwd, HashMap::new());
    let events = run_execute(execute_params("test_agent", None, None, true), &ctx).await;

    assert!(
        first_error(&events).is_none(),
        "background mode should not error, got: {:?}",
        first_error(&events)
    );
    let out = first_result(&events).expect("background mode yields a result immediately");
    assert_eq!(out.agent_name, "test_agent");
    // Background returns immediately; the response is fetched later via AgentOutput tool.
    assert!(
        out.response.is_empty(),
        "background result response should be empty"
    );
    assert_eq!(
        out.model, "claude-sonnet-4-6",
        "default model should be sonnet"
    );
    assert!(
        out.agent_id.starts_with("agent_test_agent_t"),
        "agent_id should be generated for this agent type, got: {}",
        out.agent_id
    );
    assert_eq!(out.tokens_used.total_tokens, 0);

    // The agent should have been registered in the global registry.
    let status = global_agent_registry().get_status(&out.agent_id).await;
    assert!(
        status.is_ok(),
        "background agent should be registered, got: {:?}",
        status
    );
}

#[tokio::test]
async fn test_execute_background_mode_runtime_agent_fallback() {
    if background_disabled() {
        return;
    }
    // No .claude/agents file exists; the agent is supplied via --agents runtime map.
    let temp_dir = TempDir::new().unwrap();
    let mut runtime_agents = HashMap::new();
    runtime_agents.insert(
        "runtime_agent".to_string(),
        RuntimeAgentInfo {
            prompt: "You are a runtime-defined agent.".to_string(),
            model: None,
            allowed_tools: vec![],
            disallowed_tools: vec![],
        },
    );
    let ctx = execute_ctx(temp_dir.path().to_path_buf(), runtime_agents);

    let events = run_execute(execute_params("runtime_agent", None, None, true), &ctx).await;

    assert!(
        first_error(&events).is_none(),
        "runtime fallback should not error"
    );
    let out = first_result(&events).expect("runtime agent should resolve and yield a result");
    assert_eq!(out.agent_name, "runtime_agent");
    assert!(out.agent_id.starts_with("agent_runtime_agent_t"));
}

#[tokio::test]
async fn test_execute_background_mode_resolves_model() {
    if background_disabled() {
        return;
    }
    // (model input, expected resolved model id) end-to-end through execute.
    let cases = [
        (Some("haiku"), "claude-haiku-4-5-20251001"),
        (Some("sonnet"), "claude-sonnet-4-6"),
        (Some("opus"), "claude-opus-4-6"),
        (Some("claude-custom-x"), "claude-custom-x"),
        (None, "claude-sonnet-4-6"),
    ];
    for (input, expected) in cases {
        let temp_dir = TempDir::new().unwrap();
        let cwd = setup_test_agent(&temp_dir).await;
        let ctx = execute_ctx(cwd, HashMap::new());

        let events = run_execute(execute_params("test_agent", input, None, true), &ctx).await;
        let out =
            first_result(&events).unwrap_or_else(|| panic!("model `{input:?}`: expected a result"));
        assert_eq!(
            out.model, expected,
            "model `{input:?}` should resolve to `{expected}`"
        );
    }
}

#[tokio::test]
async fn test_execute_background_mode_honors_resume_id() {
    if background_disabled() {
        return;
    }
    let temp_dir = TempDir::new().unwrap();
    let cwd = setup_test_agent(&temp_dir).await;
    let ctx = execute_ctx(cwd, HashMap::new());

    let events = run_execute(
        execute_params("test_agent", None, Some("resume-abc-123"), true),
        &ctx,
    )
    .await;

    let out = first_result(&events).expect("expected a result");
    // A supplied resume id is used verbatim instead of generating a new agent id.
    assert_eq!(out.agent_id, "resume-abc-123");
}

#[tokio::test]
async fn test_execute_frontmatter_forces_background() {
    if background_disabled() {
        return;
    }
    // Agent definition opts into background via frontmatter even though the
    // caller passed run_in_background: false. execute must honor the frontmatter
    // and return immediately with an empty-response result.
    let temp_dir = TempDir::new().unwrap();
    let agents_dir = temp_dir.path().join(".claude").join("agents");
    fs::create_dir_all(&agents_dir).await.unwrap();
    fs::write(
        agents_dir.join("forced_bg.md"),
        "---\nbackground: true\n---\n# Forced BG\nYou always run in background.",
    )
    .await
    .unwrap();

    let ctx = execute_ctx(temp_dir.path().to_path_buf(), HashMap::new());
    let events = run_execute(execute_params("forced_bg", None, None, false), &ctx).await;

    assert!(
        first_error(&events).is_none(),
        "forced background should not error"
    );
    let out = first_result(&events).expect("frontmatter background should yield a result");
    assert_eq!(out.agent_name, "forced_bg");
    assert!(
        out.response.is_empty(),
        "frontmatter-forced background result should have an empty response"
    );
}

#[test]
fn test_agent_tool_metadata_and_capabilities() {
    let tool = AgentTool;
    let meta = tool.metadata();
    assert_eq!(meta.name, "Agent");
    assert!(!meta.description.is_empty());
    // Agent execution can mutate state via its own tool usage and is concurrency-safe.
    assert!(!tool.is_read_only());
    assert!(tool.is_concurrency_safe());
}

#[test]
fn test_agent_params_deserialization() {
    // Minimal valid payload: optional fields default.
    let minimal = serde_json::json!({
        "description": "d",
        "prompt": "p",
        "subagent_type": "t",
    });
    let params: AgentParams = serde_json::from_value(minimal).expect("minimal params deserialize");
    assert_eq!(params.subagent_type, "t");
    assert!(params.model.is_none());
    assert!(params.resume.is_none());
    assert!(!params.run_in_background);
    assert!(params.memory_scope.is_none());

    // Fully specified payload round-trips.
    let full = serde_json::json!({
        "description": "d",
        "prompt": "p",
        "subagent_type": "reviewer",
        "model": "opus",
        "resume": "agent_x",
        "run_in_background": true,
        "memory_scope": "project",
    });
    let params: AgentParams = serde_json::from_value(full).expect("full params deserialize");
    assert_eq!(params.model.as_deref(), Some("opus"));
    assert_eq!(params.resume.as_deref(), Some("agent_x"));
    assert!(params.run_in_background);
    assert_eq!(params.memory_scope.as_deref(), Some("project"));
}

#[test]
fn test_agent_params_malformed_rejected() {
    // Each payload is missing a required field or has a wrong type, and must fail.
    let bad_payloads = [
        // missing prompt
        serde_json::json!({"description": "d", "subagent_type": "t"}),
        // missing subagent_type
        serde_json::json!({"description": "d", "prompt": "p"}),
        // missing description
        serde_json::json!({"prompt": "p", "subagent_type": "t"}),
        // wrong type for run_in_background
        serde_json::json!({
            "description": "d", "prompt": "p", "subagent_type": "t",
            "run_in_background": "yes"
        }),
        // not an object at all
        serde_json::json!("just a string"),
    ];
    for (i, payload) in bad_payloads.iter().enumerate() {
        let result: Result<AgentParams, _> = serde_json::from_value(payload.clone());
        assert!(
            result.is_err(),
            "malformed payload #{i} should fail to deserialize: {payload}"
        );
    }
}
