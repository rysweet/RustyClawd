//! Tests for tool_use_id correlation in hooks
//!
//! Verifies that tool_use_id is properly threaded through PreToolUse and PostToolUse hooks,
//! allowing hooks to correlate the two events for the same tool invocation.

use crate::hooks::{HookContext, HookEvent};

#[test]
fn test_hook_context_includes_tool_use_id() {
    // Verify tool_use_id is included in hook context
    let context = HookContext::for_tool(
        "session-123".to_string(),
        "/tmp/transcript.json".to_string(),
        "/home/user".to_string(),
        "ask".to_string(),
        HookEvent::PreToolUse,
        "Write".to_string(),
        Some("toolu_abc123".to_string()),
    );

    assert_eq!(context.tool_use_id, Some("toolu_abc123".to_string()));
    assert_eq!(context.tool_name, Some("Write".to_string()));
}

#[test]
fn test_hook_context_without_tool_use_id() {
    // Verify tool_use_id can be None for backward compatibility
    let context = HookContext::for_tool(
        "session-123".to_string(),
        "/tmp/transcript.json".to_string(),
        "/home/user".to_string(),
        "ask".to_string(),
        HookEvent::PreToolUse,
        "Write".to_string(),
        None,
    );

    assert_eq!(context.tool_use_id, None);
    assert_eq!(context.tool_name, Some("Write".to_string()));
}

#[test]
fn test_pre_and_post_tool_use_correlation() {
    // Simulate PreToolUse and PostToolUse events with same tool_use_id
    let tool_use_id = Some("toolu_xyz789".to_string());

    let pre_context = HookContext::for_tool(
        "session-456".to_string(),
        "/tmp/transcript.json".to_string(),
        "/home/user".to_string(),
        "ask".to_string(),
        HookEvent::PreToolUse,
        "Bash".to_string(),
        tool_use_id.clone(),
    );

    let post_context = HookContext::for_tool(
        "session-456".to_string(),
        "/tmp/transcript.json".to_string(),
        "/home/user".to_string(),
        "ask".to_string(),
        HookEvent::PostToolUse,
        "Bash".to_string(),
        tool_use_id.clone(),
    );

    // Verify same tool_use_id in both contexts
    assert_eq!(pre_context.tool_use_id, post_context.tool_use_id);
    assert_eq!(pre_context.tool_use_id, Some("toolu_xyz789".to_string()));

    // Verify events are different
    assert_eq!(pre_context.hook_event_name, "PreToolUse");
    assert_eq!(post_context.hook_event_name, "PostToolUse");
}

#[test]
fn test_tool_use_id_serialization() {
    // Verify tool_use_id serializes/deserializes correctly
    let context = HookContext::for_tool(
        "session-789".to_string(),
        "/tmp/transcript.json".to_string(),
        "/home/user".to_string(),
        "ask".to_string(),
        HookEvent::PreToolUse,
        "Read".to_string(),
        Some("toolu_test123".to_string()),
    );

    // Serialize to JSON
    let json = serde_json::to_string(&context).expect("Failed to serialize");

    // Verify tool_use_id is in JSON
    assert!(json.contains("tool_use_id"));
    assert!(json.contains("toolu_test123"));

    // Deserialize back
    let deserialized: HookContext = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.tool_use_id, Some("toolu_test123".to_string()));
    assert_eq!(deserialized.tool_name, Some("Read".to_string()));
}

#[test]
fn test_tool_use_id_not_in_json_when_none() {
    // Verify tool_use_id is omitted from JSON when None (backward compatibility)
    let context = HookContext::for_tool(
        "session-999".to_string(),
        "/tmp/transcript.json".to_string(),
        "/home/user".to_string(),
        "ask".to_string(),
        HookEvent::PostToolUse,
        "Edit".to_string(),
        None,
    );

    // Serialize to JSON
    let json = serde_json::to_string(&context).expect("Failed to serialize");

    // Verify tool_use_id is NOT in JSON when None
    assert!(!json.contains("tool_use_id"));

    // Deserialize back
    let deserialized: HookContext = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.tool_use_id, None);
}

#[test]
fn test_multiple_tool_invocations_different_ids() {
    // Verify different tool invocations get different tool_use_ids
    let tool_use_id_1 = Some("toolu_call_001".to_string());
    let tool_use_id_2 = Some("toolu_call_002".to_string());

    let context_1 = HookContext::for_tool(
        "session-100".to_string(),
        "/tmp/transcript.json".to_string(),
        "/home/user".to_string(),
        "ask".to_string(),
        HookEvent::PreToolUse,
        "Write".to_string(),
        tool_use_id_1.clone(),
    );

    let context_2 = HookContext::for_tool(
        "session-100".to_string(),
        "/tmp/transcript.json".to_string(),
        "/home/user".to_string(),
        "ask".to_string(),
        HookEvent::PreToolUse,
        "Write".to_string(),
        tool_use_id_2.clone(),
    );

    // Verify they have different tool_use_ids
    assert_ne!(context_1.tool_use_id, context_2.tool_use_id);
    assert_eq!(context_1.tool_use_id, Some("toolu_call_001".to_string()));
    assert_eq!(context_2.tool_use_id, Some("toolu_call_002".to_string()));
}
