//! Hook configuration types, matchers, and serde implementations

use crate::hooks::event::HookEvent;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Hook type - command (bash), prompt (LLM), or http (POST JSON to URL)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookType {
    Command,
    Prompt,
    Http,
}

/// Hook matcher for filtering tools/events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookMatcher {
    /// Match exact string
    Exact(String),
    /// Match regex pattern
    Regex(String),
}

impl Serialize for HookMatcher {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            HookMatcher::Exact(s) | HookMatcher::Regex(s) => s.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for HookMatcher {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        // Determine if it's a regex pattern or exact match
        if s == "*" {
            // "*" is an exact match that matches everything
            Ok(HookMatcher::Exact(s))
        } else if s.contains('|') || s.contains(".*") {
            // Contains regex special characters: pipe (alternation) or .* (wildcard)
            // Note: a bare "." is NOT treated as regex — it's too common in tool
            // names like "file.read" or "mcp__server__tool.action".
            Ok(HookMatcher::Regex(s))
        } else if is_mcp_server_wildcard(&s) {
            // MCP server wildcard pattern: mcp__<server>__*
            Ok(HookMatcher::Regex(s))
        } else {
            // Simple string - exact match
            Ok(HookMatcher::Exact(s))
        }
    }
}

/// Check if a pattern is an MCP server wildcard (mcp__<server>__*)
fn is_mcp_server_wildcard(pattern: &str) -> bool {
    pattern.starts_with("mcp__") && pattern.ends_with("__*") && pattern.matches("__").count() == 2
}

impl HookMatcher {
    /// Check if a tool name matches this matcher
    pub fn matches(&self, tool_name: &str) -> bool {
        match self {
            HookMatcher::Exact(pattern) => {
                // "*" matches everything
                if pattern == "*" {
                    return true;
                }
                tool_name == pattern
            }
            HookMatcher::Regex(pattern) => {
                // Simple regex matching for common patterns
                // Order matters: check specific patterns before generic ones

                // 1. Check exact "mcp__.*" (all MCP tools)
                if pattern == "mcp__.*" {
                    return tool_name.starts_with("mcp__");
                }

                // 2. Check MCP server wildcard pattern: mcp__<server>__*
                if is_mcp_server_wildcard(pattern) {
                    // Extract server name: "mcp__filesystem__*" → "filesystem"
                    let parts: Vec<&str> = pattern.split("__").collect();
                    if parts.len() == 3 {
                        let server_name = parts[1];
                        // Match: "mcp__<server>__<anything>"
                        return tool_name.starts_with(&format!("mcp__{}__", server_name));
                    }
                }

                // 3. Check full MCP pattern: mcp__.*__.*
                if pattern == "mcp__.*__.*" {
                    return tool_name.starts_with("mcp__") && tool_name.matches("__").count() >= 2;
                }

                // 4. Check alternation pattern like "Edit|Write"
                if pattern.contains('|') {
                    return pattern.split('|').any(|p| tool_name.contains(p.trim()));
                }

                // 5. Check generic prefix matching (ends with .*)
                if pattern.ends_with(".*") {
                    let prefix = pattern.trim_end_matches(".*");
                    return tool_name.starts_with(prefix);
                }

                // 6. Default: contains matching
                tool_name.contains(pattern)
            }
        }
    }
}

/// Individual hook configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hook {
    #[serde(rename = "type")]
    pub hook_type: HookType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// URL for HTTP hooks — POST JSON to this URL and receive JSON response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(rename = "timeout", skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
}

impl Hook {
    /// Create a new command hook
    pub fn command(command: String, timeout_ms: Option<u32>) -> Self {
        Self {
            hook_type: HookType::Command,
            command: Some(command),
            prompt: None,
            url: None,
            timeout_ms,
        }
    }

    /// Create a new prompt hook
    pub fn prompt(prompt: Option<String>, timeout_ms: Option<u32>) -> Self {
        Self {
            hook_type: HookType::Prompt,
            command: None,
            prompt,
            url: None,
            timeout_ms,
        }
    }

    /// Create a new HTTP hook that POSTs JSON to a URL
    pub fn http(url: String, timeout_ms: Option<u32>) -> Self {
        Self {
            hook_type: HookType::Http,
            command: None,
            prompt: None,
            url: Some(url),
            timeout_ms,
        }
    }

    /// Get the default timeout (60 seconds)
    pub fn default_timeout() -> u32 {
        60000
    }

    /// Get the effective timeout for this hook
    pub fn effective_timeout(&self) -> u32 {
        self.timeout_ms.unwrap_or_else(Self::default_timeout)
    }
}

/// Hook configuration for a specific event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub matcher: HookMatcher,
    pub hooks: Vec<Hook>,
}

/// Complete hooks configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksConfiguration {
    #[serde(rename = "SessionStart", default)]
    pub session_start: Vec<HookConfig>,
    #[serde(rename = "SessionEnd", default)]
    pub session_end: Vec<HookConfig>,
    #[serde(rename = "PreToolUse", default)]
    pub pre_tool_use: Vec<HookConfig>,
    #[serde(rename = "PostToolUse", default)]
    pub post_tool_use: Vec<HookConfig>,
    #[serde(rename = "UserPromptSubmit", default)]
    pub user_prompt_submit: Vec<HookConfig>,
    #[serde(rename = "Stop", default)]
    pub stop: Vec<HookConfig>,
    #[serde(rename = "SubagentStop", default)]
    pub subagent_stop: Vec<HookConfig>,
    #[serde(rename = "Notification", default)]
    pub notification: Vec<HookConfig>,
    #[serde(rename = "PreCompact", default)]
    pub pre_compact: Vec<HookConfig>,
    /// PermissionRequest hooks fire when a tool requires permission and user would be prompted.
    /// Hook can return allow/deny/ask to automatically handle permission decisions.
    #[serde(rename = "PermissionRequest", default)]
    pub permission_request: Vec<HookConfig>,
    /// TeammateIdle hooks fire when an agent becomes idle and available for new tasks.
    /// Enables multi-agent coordination by notifying when agents finish their work.
    #[serde(rename = "TeammateIdle", default)]
    pub teammate_idle: Vec<HookConfig>,
    /// TaskCompleted hooks fire when an agent completes its assigned task.
    /// Allows teams to coordinate task hand-off and track progress.
    #[serde(rename = "TaskCompleted", default)]
    pub task_completed: Vec<HookConfig>,
    /// WorktreeCreate hooks fire when a git worktree is created for agent isolation (v2.1.50).
    #[serde(rename = "WorktreeCreate", default)]
    pub worktree_create: Vec<HookConfig>,
    /// WorktreeRemove hooks fire when a git worktree is removed after agent completes (v2.1.50).
    #[serde(rename = "WorktreeRemove", default)]
    pub worktree_remove: Vec<HookConfig>,
    /// ConfigChange hooks fire when a configuration file changes on disk (v2.1.49).
    #[serde(rename = "ConfigChange", default)]
    pub config_change: Vec<HookConfig>,
    /// InstructionsLoaded hooks fire when CLAUDE.md or rules files are loaded (v2.1.69).
    #[serde(rename = "InstructionsLoaded", default)]
    pub instructions_loaded: Vec<HookConfig>,
    /// Setup hooks fire during initial setup/first run (v2.1.10).
    #[serde(rename = "Setup", default)]
    pub setup: Vec<HookConfig>,
}

impl HooksConfiguration {
    /// Get hook configs for a specific event
    pub fn get_hooks_for_event(&self, event: &HookEvent) -> &[HookConfig] {
        match event {
            HookEvent::SessionStart => &self.session_start,
            HookEvent::SessionEnd => &self.session_end,
            HookEvent::PreToolUse => &self.pre_tool_use,
            HookEvent::PostToolUse => &self.post_tool_use,
            HookEvent::UserPromptSubmit => &self.user_prompt_submit,
            HookEvent::Stop => &self.stop,
            HookEvent::SubagentStop => &self.subagent_stop,
            HookEvent::Notification => &self.notification,
            HookEvent::PreCompact => &self.pre_compact,
            HookEvent::PermissionRequest => &self.permission_request,
            HookEvent::TeammateIdle => &self.teammate_idle,
            HookEvent::TaskCompleted => &self.task_completed,
            HookEvent::WorktreeCreate => &self.worktree_create,
            HookEvent::WorktreeRemove => &self.worktree_remove,
            HookEvent::ConfigChange => &self.config_change,
            HookEvent::InstructionsLoaded => &self.instructions_loaded,
            HookEvent::Setup => &self.setup,
        }
    }
}

/// Hook execution result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl HookResult {
    /// Check if hook succeeded (exit code 0)
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Check if hook is blocking error (exit code 2)
    pub fn is_blocking(&self) -> bool {
        self.exit_code == 2
    }

    /// Check if hook is non-blocking error (exit code 1)
    pub fn is_non_blocking_error(&self) -> bool {
        self.exit_code == 1
    }

    /// Parse hook output as JSON
    pub fn parse_output(&self) -> Option<HookOutput> {
        if self.stdout.is_empty() {
            return None;
        }
        serde_json::from_str(&self.stdout).ok()
    }
}

/// Permission decision for PreToolUse hooks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionDecision {
    Allow,
    Deny,
    Ask,
}

/// Stop decision for Stop/SubagentStop hooks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StopDecision {
    Approve,
    Block,
}

/// Hook output structure (JSON response)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookOutput {
    /// Whether to continue execution (default: true)
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_execution: Option<bool>,
    /// Message to show when continue=false
    #[serde(rename = "stopReason", skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Hide output from transcript (default: false)
    #[serde(rename = "suppressOutput", skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
    /// Warning message to show to user
    #[serde(rename = "systemMessage", skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
    /// Permission decision for PreToolUse hooks
    #[serde(rename = "permissionDecision", skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<PermissionDecision>,
    /// Reason for permission decision
    #[serde(
        rename = "permissionDecisionReason",
        skip_serializing_if = "Option::is_none"
    )]
    pub permission_decision_reason: Option<String>,
    /// Stop decision for Stop/SubagentStop hooks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<StopDecision>,
    /// Reason for decision (required when blocking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Additional context to inject
    #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
    /// Hook-specific output nested structure
    #[serde(rename = "hookSpecificOutput", skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

/// Hook-specific output for PreToolUse hooks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookSpecificOutput {
    /// Permission decision
    #[serde(rename = "permissionDecision", skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<PermissionDecision>,
    /// Reason for permission decision
    #[serde(
        rename = "permissionDecisionReason",
        skip_serializing_if = "Option::is_none"
    )]
    pub permission_decision_reason: Option<String>,
    /// Updated tool parameters
    #[serde(rename = "updatedInput", skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_matcher_exact() {
        let matcher = HookMatcher::Exact("Write".to_string());
        assert!(matcher.matches("Write"));
        assert!(!matcher.matches("Read"));
    }

    #[test]
    fn test_hook_matcher_wildcard() {
        let matcher = HookMatcher::Exact("*".to_string());
        assert!(matcher.matches("Write"));
        assert!(matcher.matches("Read"));
        assert!(matcher.matches("anything"));
    }

    #[test]
    fn test_hook_matcher_regex_alternation() {
        let matcher = HookMatcher::Regex("Edit|Write".to_string());
        assert!(matcher.matches("Edit"));
        assert!(matcher.matches("Write"));
        assert!(!matcher.matches("Read"));
    }

    #[test]
    fn test_hook_matcher_mcp_prefix() {
        let matcher = HookMatcher::Regex("mcp__.*".to_string());
        assert!(matcher.matches("mcp__server__tool"));
        assert!(!matcher.matches("Bash"));
    }

    #[test]
    fn test_hook_result_success() {
        let result = HookResult {
            exit_code: 0,
            stdout: "success".to_string(),
            stderr: String::new(),
        };
        assert!(result.is_success());
        assert!(!result.is_blocking());
        assert!(!result.is_non_blocking_error());
    }

    #[test]
    fn test_hook_result_blocking() {
        let result = HookResult {
            exit_code: 2,
            stdout: String::new(),
            stderr: "blocked".to_string(),
        };
        assert!(!result.is_success());
        assert!(result.is_blocking());
        assert!(!result.is_non_blocking_error());
    }

    #[test]
    fn test_hook_output_parse() {
        let result = HookResult {
            exit_code: 0,
            stdout: r#"{"continue": true, "permissionDecision": "allow"}"#.to_string(),
            stderr: String::new(),
        };
        let output = result.parse_output().unwrap();
        assert_eq!(output.continue_execution, Some(true));
        assert_eq!(output.permission_decision, Some(PermissionDecision::Allow));
    }

    #[test]
    fn test_hooks_configuration_permission_request() {
        let json = r#"{
            "PermissionRequest": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "scripts/auto-approve.sh",
                            "timeout": 5000
                        }
                    ]
                }
            ]
        }"#;

        let config: HooksConfiguration = serde_json::from_str(json).unwrap();
        assert_eq!(config.permission_request.len(), 1);
        assert_eq!(
            config
                .get_hooks_for_event(&HookEvent::PermissionRequest)
                .len(),
            1
        );
    }

    #[test]
    fn test_permission_request_output_allow() {
        let json = r#"{"permissionDecision": "allow", "permissionDecisionReason": "Safe command"}"#;
        let output: HookOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.permission_decision, Some(PermissionDecision::Allow));
        assert_eq!(
            output.permission_decision_reason,
            Some("Safe command".to_string())
        );
    }

    #[test]
    fn test_permission_request_output_deny() {
        let json = r#"{"permissionDecision": "deny", "permissionDecisionReason": "Dangerous command detected"}"#;
        let output: HookOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.permission_decision, Some(PermissionDecision::Deny));
        assert_eq!(
            output.permission_decision_reason,
            Some("Dangerous command detected".to_string())
        );
    }

    #[test]
    fn test_permission_request_output_ask() {
        let json =
            r#"{"permissionDecision": "ask", "permissionDecisionReason": "Needs user review"}"#;
        let output: HookOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.permission_decision, Some(PermissionDecision::Ask));
        assert_eq!(
            output.permission_decision_reason,
            Some("Needs user review".to_string())
        );
    }

    #[test]
    fn test_http_hook_deserialization() {
        let json = r#"{
            "PreToolUse": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "http",
                            "url": "http://localhost:8080/hooks/pre-tool",
                            "timeout": 10000
                        }
                    ]
                }
            ]
        }"#;

        let config: HooksConfiguration = serde_json::from_str(json).unwrap();
        assert_eq!(config.pre_tool_use.len(), 1);
        let hook = &config.pre_tool_use[0].hooks[0];
        assert_eq!(hook.hook_type, HookType::Http);
        assert_eq!(
            hook.url,
            Some("http://localhost:8080/hooks/pre-tool".to_string())
        );
        assert_eq!(hook.timeout_ms, Some(10000));
        assert!(hook.command.is_none());
    }

    #[test]
    fn test_http_hook_constructor() {
        let hook = Hook::http("http://example.com/hook".to_string(), Some(5000));
        assert_eq!(hook.hook_type, HookType::Http);
        assert_eq!(hook.url, Some("http://example.com/hook".to_string()));
        assert!(hook.command.is_none());
        assert!(hook.prompt.is_none());
        assert_eq!(hook.timeout_ms, Some(5000));
    }

    #[test]
    fn test_http_hook_serialization_roundtrip() {
        let hook = Hook::http("http://example.com/hook".to_string(), Some(5000));
        let json = serde_json::to_string(&hook).unwrap();
        let deserialized: Hook = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.hook_type, HookType::Http);
        assert_eq!(
            deserialized.url,
            Some("http://example.com/hook".to_string())
        );
    }
}
