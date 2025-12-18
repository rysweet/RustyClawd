//! Integration tests for hook lifecycle events (TDD)
//!
//! This test suite verifies that the 5 key hook lifecycle events are properly
//! wired up and function correctly in the CLI:
//! - UserPromptSubmit: Fires when user submits prompts
//! - PreToolUse: Fires before tool execution with permission control
//! - PostToolUse: Fires after tool execution (non-blocking)
//! - Stop: Fires when checking if work is complete
//! - SubagentStop: Fires when agent commands complete
//!
//! These tests follow Test-Driven Development (TDD) principles:
//! - Tests are written BEFORE implementation
//! - Tests should FAIL initially (feature not yet implemented)
//! - Tests define the expected behavior of the feature

#![allow(unused_imports)]
#![allow(dead_code)]

use anyhow::Result;
use rustyclawd::hooks::types::{
    Hook, HookConfig, HookMatcher, HookOutput, HooksConfiguration, PermissionDecision,
    SessionStartMatcher, StopDecision,
};
use rustyclawd::hooks::{
    HookContext, HookEvent, HookExecutor, HookRegistry, HookResult, HooksSystem,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// TEST HELPERS
// ============================================================================

/// Helper to create a temporary test configuration directory
fn setup_test_config_dir() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir)?;
    Ok(temp_dir)
}

/// Helper to create a test hooks configuration file
fn create_test_hooks_config(dir: &TempDir, config: &HooksConfiguration) -> Result<PathBuf> {
    let config_path = dir.path().join(".claude").join("settings.json");
    let config_json = serde_json::to_string_pretty(&json!({
        "hooks": config
    }))?;
    fs::write(&config_path, config_json)?;
    Ok(config_path)
}

/// Helper to create a simple command hook that writes to a file
fn create_tracking_hook(output_file: &str, message: &str) -> Hook {
    Hook::command(format!("echo '{}' > {}", message, output_file), Some(5000))
}

/// Helper to create a hook that returns JSON decision
fn create_decision_hook(decision_json: &str) -> Hook {
    Hook::command(format!("echo '{}'", decision_json), Some(5000))
}

/// Helper to check if a tracking file was created
fn check_tracking_file(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

/// Helper to read tracking file content
fn read_tracking_file(path: &str) -> Result<String> {
    Ok(fs::read_to_string(path)?)
}

/// Helper to create a basic HookContext for testing
fn create_test_context(event: HookEvent) -> HookContext {
    HookContext::for_session(
        "test-session-123".to_string(),
        "/tmp/test-transcript.log".to_string(),
        std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        "auto".to_string(),
        event,
    )
}

// ============================================================================
// USER PROMPT SUBMIT TESTS
// ============================================================================

#[tokio::test]
async fn test_user_prompt_submit_hook_fires_on_interactive_input() {
    // GIVEN: A hook configured for UserPromptSubmit event
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("prompt_submitted.txt");

    let mut config = HooksConfiguration::default();
    config.user_prompt_submit.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "prompt_received",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: User submits a prompt in interactive mode
    let context = HookContext::for_user_prompt(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        "Write a hello world program".to_string(),
    );

    let results = system
        .execute_hooks(HookEvent::UserPromptSubmit, &context)
        .await
        .unwrap();

    // THEN: Hook should fire and tracking file should exist
    assert_eq!(results.len(), 1, "Should execute one hook");
    assert!(results[0].is_success(), "Hook should succeed");
    assert!(
        check_tracking_file(tracking_file.to_str().unwrap()),
        "Hook should create tracking file"
    );
}

#[tokio::test]
async fn test_user_prompt_submit_hook_fires_on_cli_prompt() {
    // GIVEN: A hook for UserPromptSubmit
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("cli_prompt.txt");

    let mut config = HooksConfiguration::default();
    config.user_prompt_submit.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "cli_prompt_received",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: Prompt provided via CLI --prompt flag
    let context = HookContext::for_user_prompt(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        "List all files".to_string(),
    );

    let results = system
        .execute_hooks(HookEvent::UserPromptSubmit, &context)
        .await
        .unwrap();

    // THEN: Hook fires for CLI prompts too
    assert!(results[0].is_success());
    assert!(check_tracking_file(tracking_file.to_str().unwrap()));
}

#[tokio::test]
async fn test_user_prompt_submit_receives_correct_context() {
    // GIVEN: A hook that echoes the user prompt
    let temp_dir = setup_test_config_dir().unwrap();
    let output_file = temp_dir.path().join("prompt_context.txt");

    let mut config = HooksConfiguration::default();
    config.user_prompt_submit.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command(
            format!(
                "echo \"$CLAUDE_SESSION_ID\" > {}",
                output_file.to_str().unwrap()
            ),
            Some(5000),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let user_prompt = "Test prompt for context verification";
    let context = HookContext::for_user_prompt(
        "session-abc-123".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        user_prompt.to_string(),
    );

    // WHEN: Hook executes
    let results = system
        .execute_hooks(HookEvent::UserPromptSubmit, &context)
        .await
        .unwrap();

    // THEN: Hook receives correct context via environment variables
    assert!(results[0].is_success());
    let content = read_tracking_file(output_file.to_str().unwrap()).unwrap();
    assert!(
        content.contains("session-abc-123"),
        "Should have session ID in context"
    );
}

#[tokio::test]
async fn test_user_prompt_submit_blocking_behavior() {
    // GIVEN: A hook that returns blocking decision (exit code 2)
    let temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.user_prompt_submit.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command(
            "echo '{\"continue\": false, \"stopReason\": \"Prompt validation failed\"}' && exit 2"
                .to_string(),
            Some(5000),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = HookContext::for_user_prompt(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        "malicious prompt".to_string(),
    );

    // WHEN: Hook returns blocking decision
    let results = system
        .execute_hooks(HookEvent::UserPromptSubmit, &context)
        .await
        .unwrap();

    // THEN: Hook should return exit code 2 (blocking)
    assert_eq!(results.len(), 1);
    assert!(
        results[0].is_blocking(),
        "Hook should block with exit code 2"
    );

    // Parse the output to verify blocking decision
    let output = results[0].parse_output();
    assert!(output.is_some(), "Should have JSON output");
    assert_eq!(
        output.unwrap().continue_execution,
        Some(false),
        "Should signal to stop execution"
    );
}

#[tokio::test]
async fn test_user_prompt_submit_non_blocking_errors() {
    // GIVEN: A hook that fails with non-blocking error (exit code 1)
    let temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.user_prompt_submit.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command("exit 1".to_string(), Some(5000))],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = HookContext::for_user_prompt(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        "any prompt".to_string(),
    );

    // WHEN: Hook fails with non-blocking error
    let results = system
        .execute_hooks(HookEvent::UserPromptSubmit, &context)
        .await
        .unwrap();

    // THEN: Error should be non-blocking (prompt should still process)
    assert_eq!(results.len(), 1);
    assert!(
        results[0].is_non_blocking_error(),
        "Exit code 1 should be non-blocking"
    );

    // In real implementation, prompt processing should continue despite error
    // This test verifies the hook system returns the error correctly
}

// ============================================================================
// PRE TOOL USE TESTS
// ============================================================================

#[tokio::test]
async fn test_pre_tool_use_hook_fires_before_any_tool() {
    // GIVEN: A hook for PreToolUse that logs tool execution
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("tool_execution.txt");

    let mut config = HooksConfiguration::default();
    config.pre_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "tool_about_to_execute",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: Any tool is about to be executed
    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PreToolUse,
        "Write".to_string(),
        None,
    );

    let results = system
        .execute_hooks(HookEvent::PreToolUse, &context)
        .await
        .unwrap();

    // THEN: PreToolUse hook should fire
    assert!(results[0].is_success());
    assert!(check_tracking_file(tracking_file.to_str().unwrap()));
}

#[tokio::test]
async fn test_pre_tool_use_receives_tool_name_and_params() {
    // GIVEN: A hook that captures tool information
    let temp_dir = setup_test_config_dir().unwrap();
    let output_file = temp_dir.path().join("tool_info.txt");

    let mut config = HooksConfiguration::default();
    config.pre_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("Bash".to_string()),
        hooks: vec![Hook::command(
            format!(
                "echo \"$CLAUDE_TOOL_NAME\" > {}",
                output_file.to_str().unwrap()
            ),
            Some(5000),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: Bash tool is about to execute
    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PreToolUse,
        "Bash".to_string(),
    )
    .with_tool_params(json!({
        "command": "ls -la",
        "timeout": 5000
    }));

    let results = system
        .execute_hooks(HookEvent::PreToolUse, &context)
        .await
        .unwrap();

    // THEN: Hook receives tool name and parameters
    assert!(results[0].is_success());
    let content = read_tracking_file(output_file.to_str().unwrap()).unwrap();
    assert!(content.contains("Bash"), "Should capture tool name");
}

#[tokio::test]
async fn test_pre_tool_use_allow_decision() {
    // GIVEN: A hook that returns "allow" permission decision
    let temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.pre_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_decision_hook(r#"{"permissionDecision": "allow"}"#)],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PreToolUse,
        "Write".to_string(),
        None,
    );

    // WHEN: Hook returns allow decision
    let results = system
        .execute_hooks(HookEvent::PreToolUse, &context)
        .await
        .unwrap();

    // THEN: Tool execution should be allowed
    assert!(results[0].is_success());
    let output = results[0].parse_output().unwrap();
    assert_eq!(output.permission_decision, Some(PermissionDecision::Allow));
}

#[tokio::test]
async fn test_pre_tool_use_deny_decision() {
    // GIVEN: A hook that returns "deny" permission decision
    let temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.pre_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("Bash".to_string()),
        hooks: vec![Hook::command(
            r#"echo '{"permissionDecision": "deny", "permissionDecisionReason": "Bash not allowed in this context"}' && exit 2"#.to_string(),
            Some(5000),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PreToolUse,
        "Bash".to_string(),
    );

    // WHEN: Hook returns deny decision
    let results = system
        .execute_hooks(HookEvent::PreToolUse, &context)
        .await
        .unwrap();

    // THEN: Tool execution should be blocked
    assert!(results[0].is_blocking(), "Should block with exit code 2");
    let output = results[0].parse_output().unwrap();
    assert_eq!(output.permission_decision, Some(PermissionDecision::Deny));
    assert!(output.permission_decision_reason.is_some());
}

#[tokio::test]
async fn test_pre_tool_use_ask_decision() {
    // GIVEN: A hook that returns "ask" permission decision
    let temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.pre_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_decision_hook(
            r#"{"permissionDecision": "ask", "permissionDecisionReason": "User confirmation required"}"#,
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PreToolUse,
        "Write".to_string(),
        None,
    );

    // WHEN: Hook returns ask decision
    let results = system
        .execute_hooks(HookEvent::PreToolUse, &context)
        .await
        .unwrap();

    // THEN: Should prompt user (in tests, default to allow)
    assert!(results[0].is_success());
    let output = results[0].parse_output().unwrap();
    assert_eq!(output.permission_decision, Some(PermissionDecision::Ask));
}

#[tokio::test]
async fn test_pre_tool_use_fail_open_on_error() {
    // GIVEN: A hook that fails with an error
    let temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.pre_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command(
            "exit 1".to_string(), // Non-blocking error
            Some(5000),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PreToolUse,
        "Read".to_string(),
    );

    // WHEN: Hook fails
    let results = system
        .execute_hooks(HookEvent::PreToolUse, &context)
        .await
        .unwrap();

    // THEN: Should fail-open (allow tool execution despite error)
    assert!(results[0].is_non_blocking_error());
    // In real implementation, tool should still execute (fail-open security)
}

#[tokio::test]
async fn test_pre_tool_use_multiple_matchers() {
    // GIVEN: Hooks for specific tools
    let temp_dir = setup_test_config_dir().unwrap();
    let write_file = temp_dir.path().join("write_hook.txt");
    let bash_file = temp_dir.path().join("bash_hook.txt");

    let mut config = HooksConfiguration::default();
    config.pre_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("Write".to_string()),
        hooks: vec![create_tracking_hook(
            write_file.to_str().unwrap(),
            "write_hook",
        )],
    });
    config.pre_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("Bash".to_string()),
        hooks: vec![create_tracking_hook(
            bash_file.to_str().unwrap(),
            "bash_hook",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: Write tool executes
    let write_context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PreToolUse,
        "Write".to_string(),
    );

    let write_results = system
        .execute_hooks(HookEvent::PreToolUse, &write_context)
        .await
        .unwrap();

    // THEN: Only Write hook should fire
    assert_eq!(write_results.len(), 1);
    assert!(check_tracking_file(write_file.to_str().unwrap()));
    assert!(!check_tracking_file(bash_file.to_str().unwrap()));
}

// ============================================================================
// POST TOOL USE TESTS
// ============================================================================

#[tokio::test]
async fn test_post_tool_use_fires_after_successful_execution() {
    // GIVEN: A hook for PostToolUse
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("tool_completed.txt");

    let mut config = HooksConfiguration::default();
    config.post_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "tool_execution_complete",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: Tool completes successfully
    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PostToolUse,
        "Read".to_string(),
    )
    .with_tool_result(json!({
        "content": "file content",
        "success": true
    }));

    let results = system
        .execute_hooks(HookEvent::PostToolUse, &context)
        .await
        .unwrap();

    // THEN: PostToolUse hook should fire
    assert!(results[0].is_success());
    assert!(check_tracking_file(tracking_file.to_str().unwrap()));
}

#[tokio::test]
async fn test_post_tool_use_fires_after_failed_execution() {
    // GIVEN: A hook for PostToolUse
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("tool_failed.txt");

    let mut config = HooksConfiguration::default();
    config.post_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "tool_execution_failed",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: Tool execution fails
    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PostToolUse,
        "Bash".to_string(),
    )
    .with_tool_result(json!({
        "error": "Command not found",
        "exit_code": 127
    }));

    let results = system
        .execute_hooks(HookEvent::PostToolUse, &context)
        .await
        .unwrap();

    // THEN: Hook should still fire even on tool failure
    assert!(results[0].is_success());
    assert!(check_tracking_file(tracking_file.to_str().unwrap()));
}

#[tokio::test]
async fn test_post_tool_use_receives_tool_result() {
    // GIVEN: A hook that processes tool results
    let temp_dir = setup_test_config_dir().unwrap();
    let output_file = temp_dir.path().join("tool_result.txt");

    let mut config = HooksConfiguration::default();
    config.post_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command(
            format!(
                "echo \"Tool: $CLAUDE_TOOL_NAME\" > {}",
                output_file.to_str().unwrap()
            ),
            Some(5000),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PostToolUse,
        "Write".to_string(),
    )
    .with_tool_result(json!({
        "file_path": "/tmp/test.txt",
        "bytes_written": 42
    }));

    // WHEN: Hook executes
    let results = system
        .execute_hooks(HookEvent::PostToolUse, &context)
        .await
        .unwrap();

    // THEN: Hook receives tool result in context
    assert!(results[0].is_success());
    let content = read_tracking_file(output_file.to_str().unwrap()).unwrap();
    assert!(content.contains("Write"));
}

#[tokio::test]
async fn test_post_tool_use_is_non_blocking() {
    // GIVEN: A hook that tries to block execution
    let temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.post_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command(
            "exit 2".to_string(), // Try to block
            Some(5000),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PostToolUse,
        "Read".to_string(),
    );

    // WHEN: Hook returns blocking exit code
    let results = system
        .execute_hooks(HookEvent::PostToolUse, &context)
        .await
        .unwrap();

    // THEN: PostToolUse hooks cannot block (tool result already captured)
    // The hook may return exit code 2, but it should NOT prevent tool result from being used
    assert_eq!(results.len(), 1);
    // In real implementation, tool result should still be returned to Claude
}

#[tokio::test]
async fn test_post_tool_use_failures_logged_not_blocking() {
    // GIVEN: A hook that fails
    let temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.post_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command("exit 1".to_string(), Some(5000))],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PostToolUse,
        "Bash".to_string(),
    );

    // WHEN: Hook fails
    let results = system
        .execute_hooks(HookEvent::PostToolUse, &context)
        .await
        .unwrap();

    // THEN: Failure should be logged but not affect tool result
    assert!(results[0].is_non_blocking_error());
    // In real implementation, error should be logged but tool result still returned
}

// ============================================================================
// STOP TESTS
// ============================================================================

#[tokio::test]
async fn test_stop_hook_fires_on_exit_command() {
    // GIVEN: A hook for Stop event
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("exit_check.txt");

    let mut config = HooksConfiguration::default();
    config.stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "checking_if_complete",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: User issues /exit or /quit command
    let context = create_test_context(HookEvent::Stop);

    let results = system
        .execute_hooks(HookEvent::Stop, &context)
        .await
        .unwrap();

    // THEN: Stop hook should fire
    assert!(results[0].is_success());
    assert!(check_tracking_file(tracking_file.to_str().unwrap()));
}

#[tokio::test]
async fn test_stop_hook_fires_before_session_end() {
    // GIVEN: A Stop hook
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("session_ending.txt");

    let mut config = HooksConfiguration::default();
    config.stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "session_end_check",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: Session is ending in non-interactive mode
    let context = create_test_context(HookEvent::Stop);

    let results = system
        .execute_hooks(HookEvent::Stop, &context)
        .await
        .unwrap();

    // THEN: Stop hook fires before session ends
    assert!(results[0].is_success());
    assert!(check_tracking_file(tracking_file.to_str().unwrap()));
}

#[tokio::test]
async fn test_stop_hook_approve_decision() {
    // GIVEN: A hook that approves stopping
    let _temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_decision_hook(r#"{"decision": "approve"}"#)],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = create_test_context(HookEvent::Stop);

    // WHEN: Hook returns approve decision
    let results = system
        .execute_hooks(HookEvent::Stop, &context)
        .await
        .unwrap();

    // THEN: Exit should be allowed
    assert!(results[0].is_success());
    let output = results[0].parse_output().unwrap();
    assert_eq!(output.decision, Some(StopDecision::Approve));
}

#[tokio::test]
async fn test_stop_hook_block_decision_advisory() {
    // GIVEN: A hook that tries to block stopping
    let _temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command(
            r#"echo '{"decision": "block", "reason": "Tests are still running"}' && exit 2"#
                .to_string(),
            Some(5000),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = create_test_context(HookEvent::Stop);

    // WHEN: Hook returns block decision
    let results = system
        .execute_hooks(HookEvent::Stop, &context)
        .await
        .unwrap();

    // THEN: Block decision is advisory (can show warning but allow exit)
    assert!(results[0].is_blocking());
    let output = results[0].parse_output().unwrap();
    assert_eq!(output.decision, Some(StopDecision::Block));
    assert!(output.reason.is_some());
}

#[tokio::test]
async fn test_stop_hook_fail_open() {
    // GIVEN: A hook that fails
    let _temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command("exit 1".to_string(), Some(5000))],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = create_test_context(HookEvent::Stop);

    // WHEN: Hook fails
    let results = system
        .execute_hooks(HookEvent::Stop, &context)
        .await
        .unwrap();

    // THEN: Failure should not prevent exit (fail-open)
    assert!(results[0].is_non_blocking_error());
    // In real implementation, user should still be able to exit
}

// ============================================================================
// SUBAGENT STOP TESTS
// ============================================================================

#[tokio::test]
async fn test_subagent_stop_fires_on_successful_completion() {
    // GIVEN: A hook for SubagentStop event
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("agent_complete.txt");

    let mut config = HooksConfiguration::default();
    config.subagent_stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "agent_completed_successfully",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: Agent command completes successfully
    let mut context = create_test_context(HookEvent::SubagentStop);
    context
        .additional
        .insert("agent_type".to_string(), json!("tester"));
    context.additional.insert(
        "result".to_string(),
        json!({"success": true, "message": "All tests passed"}),
    );

    let results = system
        .execute_hooks(HookEvent::SubagentStop, &context)
        .await
        .unwrap();

    // THEN: SubagentStop hook should fire
    assert!(results[0].is_success());
    assert!(check_tracking_file(tracking_file.to_str().unwrap()));
}

#[tokio::test]
async fn test_subagent_stop_fires_on_agent_failure() {
    // GIVEN: A hook for SubagentStop
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("agent_failed.txt");

    let mut config = HooksConfiguration::default();
    config.subagent_stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "agent_failed",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: Agent command fails
    let mut context = create_test_context(HookEvent::SubagentStop);
    context
        .additional
        .insert("agent_type".to_string(), json!("builder"));
    context.additional.insert(
        "result".to_string(),
        json!({"success": false, "error": "Build failed"}),
    );

    let results = system
        .execute_hooks(HookEvent::SubagentStop, &context)
        .await
        .unwrap();

    // THEN: Hook fires even when agent fails
    assert!(results[0].is_success());
    assert!(check_tracking_file(tracking_file.to_str().unwrap()));
}

#[tokio::test]
async fn test_subagent_stop_receives_agent_context() {
    // GIVEN: A hook that captures agent information
    let temp_dir = setup_test_config_dir().unwrap();
    let output_file = temp_dir.path().join("agent_info.txt");

    let mut config = HooksConfiguration::default();
    config.subagent_stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command(
            format!(
                "echo \"Session: $CLAUDE_SESSION_ID\" > {}",
                output_file.to_str().unwrap()
            ),
            Some(5000),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let mut context = create_test_context(HookEvent::SubagentStop);
    context
        .additional
        .insert("agent_type".to_string(), json!("reviewer"));

    // WHEN: Hook executes
    let results = system
        .execute_hooks(HookEvent::SubagentStop, &context)
        .await
        .unwrap();

    // THEN: Hook receives agent type and result in context
    assert!(results[0].is_success());
    let content = read_tracking_file(output_file.to_str().unwrap()).unwrap();
    assert!(content.contains("test-session-123"));
}

#[tokio::test]
async fn test_subagent_stop_is_non_blocking() {
    // GIVEN: A hook that tries to block
    let _temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.subagent_stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command("exit 2".to_string(), Some(5000))],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = create_test_context(HookEvent::SubagentStop);

    // WHEN: Hook returns blocking exit code
    let results = system
        .execute_hooks(HookEvent::SubagentStop, &context)
        .await
        .unwrap();

    // THEN: SubagentStop is advisory/notification only (cannot block)
    assert_eq!(results.len(), 1);
    // Exit code may be 2, but should not affect agent completion handling
}

// ============================================================================
// INTEGRATION TESTS: MULTIPLE HOOKS
// ============================================================================

#[tokio::test]
async fn test_multiple_hooks_execute_in_parallel() {
    // GIVEN: Multiple hooks for the same event
    let temp_dir = setup_test_config_dir().unwrap();
    let file1 = temp_dir.path().join("hook1.txt");
    let file2 = temp_dir.path().join("hook2.txt");
    let file3 = temp_dir.path().join("hook3.txt");

    let mut config = HooksConfiguration::default();
    config.user_prompt_submit.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![
            create_tracking_hook(file1.to_str().unwrap(), "hook1"),
            create_tracking_hook(file2.to_str().unwrap(), "hook2"),
            create_tracking_hook(file3.to_str().unwrap(), "hook3"),
        ],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = HookContext::for_user_prompt(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        "test prompt".to_string(),
    );

    // WHEN: Multiple hooks execute
    let results = system
        .execute_hooks(HookEvent::UserPromptSubmit, &context)
        .await
        .unwrap();

    // THEN: All hooks should execute (in parallel)
    assert_eq!(results.len(), 3, "Should execute all 3 hooks");
    assert!(check_tracking_file(file1.to_str().unwrap()));
    assert!(check_tracking_file(file2.to_str().unwrap()));
    assert!(check_tracking_file(file3.to_str().unwrap()));
}

#[tokio::test]
async fn test_hook_deduplication() {
    // GIVEN: Duplicate hooks (same command)
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("dedupe.txt");

    let duplicate_hook = create_tracking_hook(tracking_file.to_str().unwrap(), "test");

    let mut config = HooksConfiguration::default();
    config.stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![
            duplicate_hook.clone(),
            duplicate_hook.clone(),
            duplicate_hook.clone(),
        ],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = create_test_context(HookEvent::Stop);

    // WHEN: Hooks execute
    let results = system
        .execute_hooks(HookEvent::Stop, &context)
        .await
        .unwrap();

    // THEN: Duplicate hooks should be deduplicated
    // System should execute the hook only once
    assert_eq!(results.len(), 1, "Should deduplicate identical hooks");
}

#[tokio::test]
async fn test_hook_timeout_handling() {
    // GIVEN: A hook with very short timeout
    let temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.pre_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command(
            "sleep 10".to_string(), // Sleep for 10 seconds
            Some(100),              // But timeout after 100ms
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PreToolUse,
        "Write".to_string(),
        None,
    );

    // WHEN: Hook times out
    let results = system
        .execute_hooks(HookEvent::PreToolUse, &context)
        .await
        .unwrap();

    // THEN: Timeout should be handled gracefully
    assert_eq!(results.len(), 1);
    assert!(results[0].is_non_blocking_error());
    assert!(results[0].stderr.contains("timed out"));
}

// ============================================================================
// END-TO-END WORKFLOW TESTS
// ============================================================================

#[tokio::test]
async fn test_full_lifecycle_with_hooks() {
    // GIVEN: Hooks configured for UserPromptSubmit, PreToolUse, PostToolUse
    let temp_dir = setup_test_config_dir().unwrap();
    let prompt_file = temp_dir.path().join("prompt.txt");
    let pre_file = temp_dir.path().join("pre.txt");
    let post_file = temp_dir.path().join("post.txt");

    let mut config = HooksConfiguration::default();
    config.user_prompt_submit.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            prompt_file.to_str().unwrap(),
            "prompt",
        )],
    });
    config.pre_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(pre_file.to_str().unwrap(), "pre")],
    });
    config.post_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(post_file.to_str().unwrap(), "post")],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: Complete workflow executes
    // 1. User submits prompt
    let prompt_context = HookContext::for_user_prompt(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        "Write hello world".to_string(),
    );

    let prompt_results = system
        .execute_hooks(HookEvent::UserPromptSubmit, &prompt_context)
        .await
        .unwrap();

    // 2. Tool is about to execute
    let pre_context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PreToolUse,
        "Write".to_string(),
    );

    let pre_results = system
        .execute_hooks(HookEvent::PreToolUse, &pre_context)
        .await
        .unwrap();

    // 3. Tool executes and completes
    let post_context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "auto".to_string(),
        HookEvent::PostToolUse,
        "Write".to_string(),
    );

    let post_results = system
        .execute_hooks(HookEvent::PostToolUse, &post_context)
        .await
        .unwrap();

    // THEN: All hooks should have fired in correct order
    assert!(prompt_results[0].is_success(), "Prompt hook should succeed");
    assert!(pre_results[0].is_success(), "Pre hook should succeed");
    assert!(post_results[0].is_success(), "Post hook should succeed");

    assert!(check_tracking_file(prompt_file.to_str().unwrap()));
    assert!(check_tracking_file(pre_file.to_str().unwrap()));
    assert!(check_tracking_file(post_file.to_str().unwrap()));
}
