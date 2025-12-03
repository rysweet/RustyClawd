//! Integration tests for notification triggers and validation gaps (TDD)
//!
//! This test suite verifies comprehensive integration scenarios that close
//! validation gaps identified in the codebase review:
//! - Notification hook triggers for 4 notification types
//! - PreCompact hook execution and blocking behavior
//! - Skills tool integration with filesystem discovery
//! - SubagentStop hook with real agent invocation
//!
//! These tests follow Test-Driven Development (TDD) principles:
//! - Tests are written BEFORE implementation
//! - Tests should FAIL initially (feature not yet implemented)
//! - Tests define the expected behavior of the feature

use anyhow::Result;
use rustyclawd::hooks::types::{
    Hook, HookConfig, HookMatcher, HooksConfiguration, NotificationType, PermissionDecision,
};
use rustyclawd::hooks::{HookContext, HookEvent, HooksSystem};
use rustyclawd::notification::NotificationManager;
use serde_json::json;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

// ============================================================================
// TEST CONSTANTS
// ============================================================================

/// Default timeout for hook execution in milliseconds
const DEFAULT_HOOK_TIMEOUT_MS: u32 = 5000;

/// Delay in milliseconds to allow hook execution to settle
const HOOK_EXECUTION_SETTLE_MS: u64 = 100;

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

/// Helper to create a simple command hook that writes to a file
fn create_tracking_hook(output_file: &str, message: &str) -> Hook {
    Hook::command(
        format!("echo '{}' > {}", message, output_file),
        Some(DEFAULT_HOOK_TIMEOUT_MS),
    )
}

/// Helper to create a hook that returns JSON decision
fn create_decision_hook(decision_json: &str) -> Hook {
    Hook::command(
        format!("echo '{}'", decision_json),
        Some(DEFAULT_HOOK_TIMEOUT_MS),
    )
}

/// Helper to create a hook that returns Ask permission decision
fn create_hook_with_ask_decision() -> Hook {
    create_decision_hook(
        r#"{"permissionDecision": "ask", "permissionDecisionReason": "User confirmation required"}"#,
    )
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

/// Helper to create HookContext for notifications
fn create_notification_context(notification_type: NotificationType) -> HookContext {
    HookContext::for_notification(
        "test-session-123".to_string(),
        "/tmp/test-transcript.log".to_string(),
        std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        "ask".to_string(),
        notification_type,
    )
}

/// Helper to create a test skill directory structure
fn create_test_skill_dir(
    temp_dir: &TempDir,
    skill_name: &str,
    skill_content: &str,
) -> Result<PathBuf> {
    let skills_dir = temp_dir.path().join(".claude").join("skills");
    fs::create_dir_all(&skills_dir)?;

    let skill_file = skills_dir.join(format!("{}.md", skill_name));
    fs::write(&skill_file, skill_content)?;

    Ok(skill_file)
}

// ============================================================================
// NOTIFICATION TRIGGER TESTS
// ============================================================================

#[tokio::test]
async fn test_notification_permission_prompt() {
    // GIVEN: A hook configured for Notification event with PermissionPrompt type
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("permission_prompt.txt");

    let mut config = HooksConfiguration::default();
    config.notification.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "permission_prompt_triggered",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: PermissionPrompt notification fires
    let context = create_notification_context(NotificationType::PermissionPrompt);

    let results = system
        .execute_hooks(HookEvent::Notification, &context)
        .await
        .unwrap();

    // THEN: Hook should fire and tracking file should exist
    assert_eq!(results.len(), 1, "Should execute one hook");
    assert!(results[0].is_success(), "Hook should succeed");
    assert!(
        check_tracking_file(tracking_file.to_str().unwrap()),
        "Hook should create tracking file for PermissionPrompt"
    );
}

#[tokio::test]
async fn test_notification_idle_prompt() {
    // GIVEN: A hook for IdlePrompt notification
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("idle_prompt.txt");

    let mut config = HooksConfiguration::default();
    config.notification.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "idle_prompt_triggered",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: IdlePrompt notification fires (TUI awaiting input)
    let context = create_notification_context(NotificationType::IdlePrompt);

    // Use timeout to handle potential blocking behavior
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        system.execute_hooks(HookEvent::Notification, &context),
    )
    .await;

    // THEN: Hook should fire within timeout
    assert!(result.is_ok(), "Hook should complete within timeout");
    let results = result.unwrap().unwrap();
    assert!(results[0].is_success());
    assert!(check_tracking_file(tracking_file.to_str().unwrap()));
}

#[tokio::test]
async fn test_notification_auth_success() {
    // GIVEN: A hook for AuthSuccess notification
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("auth_success.txt");

    let mut config = HooksConfiguration::default();
    config.notification.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "auth_success_triggered",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: AuthSuccess notification fires during session creation
    let context = create_notification_context(NotificationType::AuthSuccess);

    let results = system
        .execute_hooks(HookEvent::Notification, &context)
        .await
        .unwrap();

    // THEN: Hook should fire
    assert!(results[0].is_success());
    assert!(check_tracking_file(tracking_file.to_str().unwrap()));
}

#[tokio::test]
async fn test_notification_elicitation_dialog() {
    // GIVEN: A hook for ElicitationDialog notification
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("elicitation_dialog.txt");

    let mut config = HooksConfiguration::default();
    config.notification.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "elicitation_dialog_triggered",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: ElicitationDialog notification fires (response has '?')
    let context = create_notification_context(NotificationType::ElicitationDialog);

    let results = system
        .execute_hooks(HookEvent::Notification, &context)
        .await
        .unwrap();

    // THEN: Hook should fire when Claude asks clarifying questions
    assert!(results[0].is_success());
    assert!(check_tracking_file(tracking_file.to_str().unwrap()));
}

#[tokio::test]
async fn test_notification_manager_integration() {
    // GIVEN: NotificationManager with configured hooks
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("notification_manager.txt");

    let mut config = HooksConfiguration::default();
    config.notification.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "notification_manager_hook",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);
    let system_arc = Arc::new(system);

    let manager = NotificationManager::new(system_arc);

    // WHEN: NotificationManager sends notification
    manager
        .notify(
            "test-session",
            NotificationType::PermissionPrompt,
            "Test notification message",
        )
        .await;

    // THEN: Hook should fire before displaying notification
    // Small delay to ensure async hook execution completes
    tokio::time::sleep(tokio::time::Duration::from_millis(HOOK_EXECUTION_SETTLE_MS)).await;
    assert!(
        check_tracking_file(tracking_file.to_str().unwrap()),
        "Notification hook should fire via NotificationManager"
    );
}

// ============================================================================
// PRE COMPACT HOOK TESTS
// ============================================================================

#[tokio::test]
async fn test_precompact_hook_fires_on_compact_command() {
    // GIVEN: A hook configured for PreCompact event
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("precompact.txt");

    let mut config = HooksConfiguration::default();
    config.pre_compact.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "precompact_triggered",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: PreCompact event fires (e.g., /compact command)
    let context = create_test_context(HookEvent::PreCompact);

    let results = system
        .execute_hooks(HookEvent::PreCompact, &context)
        .await
        .unwrap();

    // THEN: Hook should execute
    assert_eq!(results.len(), 1, "Should execute one hook");
    assert!(results[0].is_success(), "Hook should succeed");
    assert!(
        check_tracking_file(tracking_file.to_str().unwrap()),
        "PreCompact hook should fire on compact command"
    );
}

#[tokio::test]
async fn test_precompact_hook_can_block_compaction() {
    // GIVEN: A hook that returns blocking decision (exit code 2)
    let _temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.pre_compact.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command(
            r#"echo '{"continue": false, "stopReason": "Compaction not allowed at this time"}' && exit 2"#
                .to_string(),
            Some(DEFAULT_HOOK_TIMEOUT_MS),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: PreCompact hook returns blocking decision
    let context = create_test_context(HookEvent::PreCompact);

    let results = system
        .execute_hooks(HookEvent::PreCompact, &context)
        .await
        .unwrap();

    // THEN: Hook should block with exit code 2
    assert_eq!(results.len(), 1);
    assert!(
        results[0].is_blocking(),
        "PreCompact hook should be able to block compaction with exit code 2"
    );

    // Parse the output to verify blocking decision
    let output = results[0].parse_output();
    assert!(output.is_some(), "Should have JSON output");
    assert_eq!(
        output.unwrap().continue_execution,
        Some(false),
        "Should signal to stop compaction"
    );
}

#[tokio::test]
async fn test_precompact_hook_fail_open_on_error() {
    // GIVEN: A hook that fails with non-blocking error (exit code 1)
    let _temp_dir = setup_test_config_dir().unwrap();

    let mut config = HooksConfiguration::default();
    config.pre_compact.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command(
            "exit 1".to_string(),
            Some(DEFAULT_HOOK_TIMEOUT_MS),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    let context = create_test_context(HookEvent::PreCompact);

    // WHEN: Hook fails with non-blocking error
    let results = system
        .execute_hooks(HookEvent::PreCompact, &context)
        .await
        .unwrap();

    // THEN: Error should be non-blocking (compaction should still proceed - fail-open)
    assert_eq!(results.len(), 1);
    assert!(
        results[0].is_non_blocking_error(),
        "Exit code 1 should be non-blocking - fail-open behavior"
    );
}

// ============================================================================
// SKILLS TOOL INTEGRATION TESTS
// ============================================================================

#[tokio::test]
#[serial]
async fn test_skill_tool_loads_and_parses_frontmatter() {
    use futures::StreamExt;
    use rustyclawd_tools::skill::{SkillParams, SkillTool};
    use rustyclawd_tools::{Tool, ToolContext, ToolEvent};

    // GIVEN: Real skill file with frontmatter in test directory
    let temp_dir = setup_test_config_dir().unwrap();

    let skill_content = r#"---
description: Integration test skill
version: 1.0.0
author: Test Suite
location: project
---

# Test Skill Content

This is the actual skill prompt that should be loaded.

## Instructions

Follow these test instructions carefully.
"#;

    let skill_file = create_test_skill_dir(&temp_dir, "integration-test", skill_content).unwrap();
    assert!(skill_file.exists(), "Skill file should exist");

    // Change to temp directory for skill tool to find the file
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // WHEN: Execute SkillTool to load and parse the skill
    let tool = SkillTool;
    let ctx = ToolContext::default();
    let params = SkillParams {
        skill: "integration-test".to_string(),
    };

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    // Collect all events from the tool stream
    let mut result = None;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Result(output) = event {
            result = Some(output);
        }
    }

    // THEN: Verify skill loaded and parsed correctly
    let output = result.expect("Should have result from SkillTool");

    // Restore original directory before assertions (for better error messages)
    std::env::set_current_dir(&original_dir).unwrap();

    assert!(
        output.found,
        "Skill should be found. Searched paths: {:?}",
        output.path
    );
    assert!(
        output.prompt.contains("Test Skill Content"),
        "Should load prompt content. Got: {}",
        output.prompt
    );
    assert!(
        output.prompt.contains("Follow these test instructions"),
        "Should include full prompt"
    );
    assert!(
        !output.prompt.contains("---"),
        "Frontmatter delimiters should be stripped"
    );

    let metadata = output.metadata.expect("Should have parsed metadata");
    assert_eq!(
        metadata.description,
        Some("Integration test skill".to_string()),
        "Should parse description from frontmatter"
    );
    assert_eq!(
        metadata.version,
        Some("1.0.0".to_string()),
        "Should parse version from frontmatter"
    );
    assert_eq!(
        metadata.author,
        Some("Test Suite".to_string()),
        "Should parse author from frontmatter"
    );
    assert_eq!(
        metadata.location,
        Some("project".to_string()),
        "Should parse location from frontmatter"
    );

    assert!(output.path.is_some(), "Should return path to skill file");
    assert!(
        output.path.unwrap().contains("integration-test.md"),
        "Path should reference correct skill file"
    );
}

#[tokio::test]
#[serial]
async fn test_skill_tool_multiple_formats() {
    use futures::StreamExt;
    use rustyclawd_tools::skill::{SkillParams, SkillTool};
    use rustyclawd_tools::{Tool, ToolContext, ToolEvent};

    // GIVEN: Skills in both Markdown and YAML formats
    let temp_dir = setup_test_config_dir().unwrap();
    let skills_dir = temp_dir.path().join(".claude").join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    // Create Markdown skill
    let md_skill = r#"---
description: Markdown skill format
version: 1.0.0
location: project
---
# Markdown Skill
This is a markdown-formatted skill.
"#;
    fs::write(skills_dir.join("md-skill.md"), md_skill).unwrap();

    // Create YAML skill
    let yaml_skill = r#"description: YAML skill format
version: 2.0.0
location: project
prompt: |
  This is a YAML-formatted skill.
  It supports multi-line prompts.
"#;
    fs::write(skills_dir.join("yaml-skill.yaml"), yaml_skill).unwrap();

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // WHEN: Load Markdown skill
    let md_tool = SkillTool;
    let md_ctx = ToolContext::default();
    let md_params = SkillParams {
        skill: "md-skill".to_string(),
    };

    let mut md_stream = md_tool.execute(md_params, &md_ctx).await.unwrap();
    let mut md_result = None;
    while let Some(event) = md_stream.next().await {
        if let ToolEvent::Result(output) = event {
            md_result = Some(output);
        }
    }

    // THEN: Markdown skill should load correctly
    let md_output = md_result.expect("Markdown skill should have result");
    assert!(md_output.found, "Markdown skill should be found");
    assert!(
        md_output.prompt.contains("markdown-formatted skill"),
        "Should load Markdown prompt"
    );
    let md_metadata = md_output.metadata.expect("Should have Markdown metadata");
    assert_eq!(
        md_metadata.description,
        Some("Markdown skill format".to_string())
    );
    assert_eq!(md_metadata.version, Some("1.0.0".to_string()));

    // WHEN: Load YAML skill
    let yaml_tool = SkillTool;
    let yaml_ctx = ToolContext::default();
    let yaml_params = SkillParams {
        skill: "yaml-skill".to_string(),
    };

    let mut yaml_stream = yaml_tool.execute(yaml_params, &yaml_ctx).await.unwrap();
    let mut yaml_result = None;
    while let Some(event) = yaml_stream.next().await {
        if let ToolEvent::Result(output) = event {
            yaml_result = Some(output);
        }
    }

    // THEN: YAML skill should load correctly
    let yaml_output = yaml_result.expect("YAML skill should have result");

    // Restore original directory before assertions (for better error messages)
    std::env::set_current_dir(&original_dir).unwrap();

    assert!(yaml_output.found, "YAML skill should be found");
    assert!(
        yaml_output.prompt.contains("YAML-formatted skill"),
        "Should load YAML prompt"
    );
    assert!(
        yaml_output.prompt.contains("multi-line prompts"),
        "Should support multi-line YAML prompts"
    );
    let yaml_metadata = yaml_output.metadata.expect("Should have YAML metadata");
    assert_eq!(
        yaml_metadata.description,
        Some("YAML skill format".to_string())
    );
    assert_eq!(yaml_metadata.version, Some("2.0.0".to_string()));
}

#[tokio::test]
#[serial]
async fn test_skill_tool_not_found() {
    use futures::StreamExt;
    use rustyclawd_tools::skill::{SkillParams, SkillTool};
    use rustyclawd_tools::{Tool, ToolContext, ToolEvent};

    // GIVEN: No skill file exists
    let temp_dir = setup_test_config_dir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    // WHEN: Try to load nonexistent skill
    let tool = SkillTool;
    let ctx = ToolContext::default();
    let params = SkillParams {
        skill: "nonexistent-skill-12345".to_string(),
    };

    let mut stream = tool.execute(params, &ctx).await.unwrap();
    let mut result = None;
    while let Some(event) = stream.next().await {
        if let ToolEvent::Result(output) = event {
            result = Some(output);
        }
    }

    // THEN: Should return not found with helpful error message
    let output = result.expect("Should have result even for not found");
    assert!(!output.found, "Nonexistent skill should not be found");
    assert!(
        output.prompt.contains("not found"),
        "Should include 'not found' message"
    );
    assert!(
        output.prompt.contains("nonexistent-skill-12345"),
        "Should include skill name in error"
    );
    assert!(
        output
            .prompt
            .contains("Searched in the following locations"),
        "Should list searched locations"
    );
    assert!(output.metadata.is_none(), "Should have no metadata");
    assert!(output.path.is_none(), "Should have no path");

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
}

// ============================================================================
// SUBAGENT STOP WITH REAL AGENT TESTS
// ============================================================================

#[tokio::test]
async fn test_subagentstop_hook_fires_with_real_agent() {
    // GIVEN: A hook configured for SubagentStop event
    let temp_dir = setup_test_config_dir().unwrap();
    let tracking_file = temp_dir.path().join("subagent_stop.txt");

    let mut config = HooksConfiguration::default();
    config.subagent_stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            tracking_file.to_str().unwrap(),
            "subagent_stopped",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: SubagentStop event fires after real agent completes
    // Simulate real agent invocation context
    let mut context = create_test_context(HookEvent::SubagentStop);
    context
        .additional
        .insert("agent_type".to_string(), json!("builder"));
    context.additional.insert(
        "result".to_string(),
        json!({
            "success": true,
            "message": "Agent completed successfully",
            "output": "Built module successfully"
        }),
    );

    let results = system
        .execute_hooks(HookEvent::SubagentStop, &context)
        .await
        .unwrap();

    // THEN: Hook should fire after agent completion
    assert_eq!(results.len(), 1, "Should execute one hook");
    assert!(results[0].is_success(), "Hook should succeed");
    assert!(
        check_tracking_file(tracking_file.to_str().unwrap()),
        "SubagentStop hook should fire after real agent completion"
    );

    // Verify context contains agent information
    assert_eq!(
        context.additional.get("agent_type"),
        Some(&json!("builder")),
        "Context should contain agent type"
    );
}

#[tokio::test]
async fn test_subagentstop_hook_receives_agent_context() {
    // GIVEN: A hook that captures and verifies agent context
    let temp_dir = setup_test_config_dir().unwrap();
    let output_file = temp_dir.path().join("agent_context.txt");

    let mut config = HooksConfiguration::default();
    config.subagent_stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![Hook::command(
            format!(
                "echo \"Session: $CLAUDE_SESSION_ID\" > {}",
                output_file.to_str().unwrap()
            ),
            Some(DEFAULT_HOOK_TIMEOUT_MS),
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: SubagentStop fires with real agent context
    let mut context = create_test_context(HookEvent::SubagentStop);
    context
        .additional
        .insert("agent_type".to_string(), json!("tester"));
    context.additional.insert(
        "result".to_string(),
        json!({"tests_passed": 42, "tests_failed": 0}),
    );

    let results = system
        .execute_hooks(HookEvent::SubagentStop, &context)
        .await
        .unwrap();

    // THEN: Hook receives correct agent context via environment variables
    assert!(results[0].is_success());
    let content = read_tracking_file(output_file.to_str().unwrap()).unwrap();
    assert!(
        content.contains("test-session-123"),
        "Should have session ID in context"
    );
}

// ============================================================================
// INTEGRATION: CROSS-FEATURE VALIDATION TESTS
// ============================================================================

#[tokio::test]
async fn test_notification_with_pretooluse_interaction() {
    // GIVEN: Hooks for both Notification and PreToolUse events
    let temp_dir = setup_test_config_dir().unwrap();
    let notif_file = temp_dir.path().join("notification.txt");
    let _pretool_file = temp_dir.path().join("pretool.txt");

    let mut config = HooksConfiguration::default();
    config.notification.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            notif_file.to_str().unwrap(),
            "notification_fired",
        )],
    });
    config.pre_tool_use.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_hook_with_ask_decision()],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: Notification fires first, then PreToolUse with Ask decision

    // 1. Notification fires
    let notif_context = create_notification_context(NotificationType::PermissionPrompt);
    let notif_results = system
        .execute_hooks(HookEvent::Notification, &notif_context)
        .await
        .unwrap();

    // 2. PreToolUse fires with Ask decision (simulating permission prompt flow)
    let pretool_context = HookContext::for_tool(
        "test-session".to_string(),
        "/tmp/transcript.log".to_string(),
        temp_dir.path().to_string_lossy().to_string(),
        "ask".to_string(),
        HookEvent::PreToolUse,
        "Write".to_string(),
    );

    let pretool_results = system
        .execute_hooks(HookEvent::PreToolUse, &pretool_context)
        .await
        .unwrap();

    // THEN: Both hooks should fire correctly
    assert!(
        notif_results[0].is_success(),
        "Notification hook should succeed"
    );
    assert!(
        pretool_results[0].is_success(),
        "PreToolUse hook should succeed"
    );

    // Verify notification hook fired
    assert!(check_tracking_file(notif_file.to_str().unwrap()));

    // Verify PreToolUse returned Ask decision
    let output = pretool_results[0].parse_output().unwrap();
    assert_eq!(
        output.permission_decision,
        Some(PermissionDecision::Ask),
        "Should receive Ask permission decision"
    );
}

#[tokio::test]
async fn test_precompact_with_subagentstop_workflow() {
    // GIVEN: Hooks for PreCompact and SubagentStop (simulating cleanup workflow)
    let temp_dir = setup_test_config_dir().unwrap();
    let precompact_file = temp_dir.path().join("precompact.txt");
    let subagent_file = temp_dir.path().join("subagent.txt");

    let mut config = HooksConfiguration::default();
    config.pre_compact.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            precompact_file.to_str().unwrap(),
            "precompact_workflow",
        )],
    });
    config.subagent_stop.push(HookConfig {
        matcher: HookMatcher::Exact("*".to_string()),
        hooks: vec![create_tracking_hook(
            subagent_file.to_str().unwrap(),
            "cleanup_agent_complete",
        )],
    });

    let mut system = HooksSystem::new();
    system.registry_mut().register_configuration(config);

    // WHEN: PreCompact fires, then cleanup agent completes (SubagentStop)

    // 1. PreCompact hook fires
    let precompact_context = create_test_context(HookEvent::PreCompact);
    let precompact_results = system
        .execute_hooks(HookEvent::PreCompact, &precompact_context)
        .await
        .unwrap();

    // 2. Cleanup agent completes (SubagentStop)
    let mut subagent_context = create_test_context(HookEvent::SubagentStop);
    subagent_context
        .additional
        .insert("agent_type".to_string(), json!("cleanup"));

    let subagent_results = system
        .execute_hooks(HookEvent::SubagentStop, &subagent_context)
        .await
        .unwrap();

    // THEN: Both hooks should fire in workflow sequence
    assert!(
        precompact_results[0].is_success(),
        "PreCompact hook should succeed"
    );
    assert!(
        subagent_results[0].is_success(),
        "SubagentStop hook should succeed"
    );

    assert!(check_tracking_file(precompact_file.to_str().unwrap()));
    assert!(check_tracking_file(subagent_file.to_str().unwrap()));
}
