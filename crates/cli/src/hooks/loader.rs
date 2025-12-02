//! Hook configuration loader from .claude/settings.json or .claude/hooks/config.json

use crate::hooks::types::HooksConfiguration;
use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use tokio::fs;

/// Hook configuration loader
pub struct HookLoader;

impl HookLoader {
    /// Load hooks configuration from a file
    pub async fn load_from_file(path: &str) -> Result<HooksConfiguration> {
        let path = Path::new(path);

        // Check if file exists
        if !path.exists() {
            // Return empty configuration if file doesn't exist
            return Ok(HooksConfiguration::default());
        }

        // Read and parse the configuration file
        let content = fs::read_to_string(path)
            .await
            .context("Failed to read hooks configuration file")?;

        // Try to parse as nested settings.json format first (new format)
        if let Ok(value) = serde_json::from_str::<Value>(&content) {
            if let Some(hooks_value) = value.get("hooks") {
                if let Ok(config) =
                    serde_json::from_value::<HooksConfiguration>(hooks_value.clone())
                {
                    eprintln!(
                        "[hooks] Loaded from nested 'hooks' field: {}",
                        path.display()
                    );
                    return Ok(config);
                }
            }
        }

        // Fallback to direct HooksConfiguration (legacy format)
        if let Ok(config) = serde_json::from_str::<HooksConfiguration>(&content) {
            eprintln!("[hooks] Loaded from legacy format: {}", path.display());
            return Ok(config);
        }

        // If neither format works, return parse error
        Err(anyhow::anyhow!(
            "Failed to parse hooks configuration from {} - not a valid hooks configuration or settings file",
            path.display()
        ))
    }

    /// Load hooks configuration from the default location
    /// Priority: 1) .claude/settings.json, 2) .claude/hooks/config.json
    pub async fn load_default() -> Result<HooksConfiguration> {
        let mut current_dir = std::env::current_dir()?;

        loop {
            // Try .claude/settings.json FIRST (amplihack format)
            let settings_path = current_dir.join(".claude/settings.json");
            if settings_path.exists() {
                if let Ok(config) = Self::load_from_file(settings_path.to_str().unwrap()).await {
                    // Only return if we successfully loaded hooks from settings.json
                    if !config.is_empty() {
                        return Ok(config);
                    }
                }
            }

            // Fallback to .claude/hooks/config.json (legacy format)
            let config_path = current_dir.join(".claude/hooks/config.json");
            if config_path.exists() {
                return Self::load_from_file(config_path.to_str().unwrap()).await;
            }

            // Move to parent directory
            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                // Reached root, return empty configuration
                return Ok(HooksConfiguration::default());
            }
        }
    }

    /// Load hooks from a JSON string
    pub fn load_from_string(json: &str) -> Result<HooksConfiguration> {
        // Try to parse as nested settings.json format first (new format)
        if let Ok(value) = serde_json::from_str::<Value>(json) {
            if let Some(hooks_value) = value.get("hooks") {
                if let Ok(config) =
                    serde_json::from_value::<HooksConfiguration>(hooks_value.clone())
                {
                    return Ok(config);
                }
            }
        }

        // Fallback to direct HooksConfiguration (legacy format)
        if let Ok(config) = serde_json::from_str::<HooksConfiguration>(json) {
            return Ok(config);
        }

        // If neither format works, return parse error
        Err(anyhow::anyhow!(
            "Failed to parse hooks configuration - not a valid hooks configuration or settings file"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::types::{HookMatcher, HookType};

    #[tokio::test]
    async fn test_load_empty_config() {
        let config = HookLoader::load_from_string("{}").unwrap();
        assert_eq!(config.session_start.len(), 0);
        assert_eq!(config.session_end.len(), 0);
    }

    #[tokio::test]
    async fn test_load_session_start_config() {
        let json = r#"{
            "SessionStart": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "echo 'session started'",
                            "timeout": 60000
                        }
                    ]
                }
            ]
        }"#;

        let config = HookLoader::load_from_string(json).unwrap();
        assert_eq!(config.session_start.len(), 1);
        assert_eq!(config.session_start[0].hooks.len(), 1);
        assert_eq!(
            config.session_start[0].hooks[0].hook_type,
            HookType::Command
        );
    }

    #[tokio::test]
    async fn test_load_pre_tool_use_config() {
        let json = r#"{
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        {
                            "type": "prompt",
                            "timeout": 60000
                        }
                    ]
                }
            ]
        }"#;

        let config = HookLoader::load_from_string(json).unwrap();
        assert_eq!(config.pre_tool_use.len(), 1);
        assert_eq!(config.pre_tool_use[0].hooks[0].hook_type, HookType::Prompt);
    }

    #[tokio::test]
    async fn test_load_all_events_config() {
        let json = r#"{
            "SessionStart": [{"matcher": "*", "hooks": [{"type": "command", "command": "echo start"}]}],
            "SessionEnd": [{"matcher": "*", "hooks": [{"type": "command", "command": "echo end"}]}],
            "PreToolUse": [{"matcher": "*", "hooks": [{"type": "prompt"}]}],
            "PostToolUse": [{"matcher": "*", "hooks": [{"type": "command", "command": "echo post"}]}],
            "UserPromptSubmit": [{"matcher": "*", "hooks": [{"type": "command", "command": "echo prompt"}]}],
            "Stop": [{"matcher": "*", "hooks": [{"type": "prompt"}]}],
            "SubagentStop": [{"matcher": "*", "hooks": [{"type": "prompt"}]}],
            "Notification": [{"matcher": "*", "hooks": [{"type": "command", "command": "echo notify"}]}],
            "PreCompact": [{"matcher": "*", "hooks": [{"type": "command", "command": "echo compact"}]}]
        }"#;

        let config = HookLoader::load_from_string(json).unwrap();
        assert_eq!(config.session_start.len(), 1);
        assert_eq!(config.session_end.len(), 1);
        assert_eq!(config.pre_tool_use.len(), 1);
        assert_eq!(config.post_tool_use.len(), 1);
        assert_eq!(config.user_prompt_submit.len(), 1);
        assert_eq!(config.stop.len(), 1);
        assert_eq!(config.subagent_stop.len(), 1);
        assert_eq!(config.notification.len(), 1);
        assert_eq!(config.pre_compact.len(), 1);
    }

    #[tokio::test]
    async fn test_load_nonexistent_file() {
        let config = HookLoader::load_from_file("/nonexistent/path/config.json")
            .await
            .unwrap();
        assert_eq!(config.session_start.len(), 0);
    }

    #[tokio::test]
    async fn test_load_regex_matcher() {
        let json = r#"{
            "PreToolUse": [
                {
                    "matcher": "Edit|Write",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "validate.sh"
                        }
                    ]
                }
            ]
        }"#;

        let config = HookLoader::load_from_string(json).unwrap();
        assert_eq!(config.pre_tool_use.len(), 1);
        if let HookMatcher::Regex(pattern) = &config.pre_tool_use[0].matcher {
            assert_eq!(pattern, "Edit|Write");
        } else {
            panic!("Expected Regex matcher");
        }
    }

    #[tokio::test]
    async fn test_load_mcp_pattern() {
        let json = r#"{
            "PreToolUse": [
                {
                    "matcher": "mcp__.*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "validate_mcp.sh"
                        }
                    ]
                }
            ]
        }"#;

        let config = HookLoader::load_from_string(json).unwrap();
        assert_eq!(config.pre_tool_use.len(), 1);
    }

    #[tokio::test]
    async fn test_load_multiple_hooks_same_event() {
        let json = r#"{
            "SessionStart": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "hook1.sh",
                            "timeout": 30000
                        },
                        {
                            "type": "command",
                            "command": "hook2.sh",
                            "timeout": 30000
                        }
                    ]
                }
            ]
        }"#;

        let config = HookLoader::load_from_string(json).unwrap();
        assert_eq!(config.session_start.len(), 1);
        assert_eq!(config.session_start[0].hooks.len(), 2);
    }

    #[tokio::test]
    async fn test_load_nested_settings_format() {
        // Test the new .claude/settings.json format with nested "hooks" field
        let json = r#"{
            "hooks": {
                "SessionStart": [
                    {
                        "hooks": [
                            {
                                "type": "command",
                                "command": "/path/to/session_start.py",
                                "timeout": 10000
                            }
                        ]
                    }
                ],
                "Stop": [
                    {
                        "hooks": [
                            {
                                "type": "command",
                                "command": "/path/to/stop.py",
                                "timeout": 30000
                            }
                        ]
                    }
                ]
            }
        }"#;

        let config = HookLoader::load_from_string(json).unwrap();
        assert_eq!(config.session_start.len(), 1);
        assert_eq!(config.session_start[0].hooks.len(), 1);
        assert_eq!(config.stop.len(), 1);
        assert_eq!(config.stop[0].hooks.len(), 1);
    }

    #[tokio::test]
    async fn test_load_nested_settings_with_matcher() {
        // Test nested format with explicit matcher
        let json = r#"{
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "*",
                        "hooks": [
                            {
                                "type": "command",
                                "command": "/path/to/pre_tool_use.py"
                            }
                        ]
                    }
                ]
            }
        }"#;

        let config = HookLoader::load_from_string(json).unwrap();
        assert_eq!(config.pre_tool_use.len(), 1);
        assert_eq!(config.pre_tool_use[0].hooks.len(), 1);
    }

    #[tokio::test]
    async fn test_is_empty() {
        let empty_config = HooksConfiguration::default();
        assert!(empty_config.is_empty());

        let json = r#"{
            "SessionStart": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "echo 'test'"
                        }
                    ]
                }
            ]
        }"#;

        let non_empty_config = HookLoader::load_from_string(json).unwrap();
        assert!(!non_empty_config.is_empty());
    }
}
