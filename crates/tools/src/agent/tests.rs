use super::*;
use crate::agent_memory::MemoryScope;
use crate::{ExecutionContext, Tool, ToolContext, ToolEvent};
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
