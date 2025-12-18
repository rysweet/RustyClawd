//! Hook configuration loader
//!
//! Loads hooks configuration from:
//! - .claude/settings.json (priority 1, amplihack standard)
//! - .claude/hooks/config.json (priority 2, legacy location)

use crate::hooks::types::HooksConfiguration;
use anyhow::{Context, Result};
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

        let config: HooksConfiguration =
            serde_json::from_str(&content).context("Failed to parse hooks configuration JSON")?;

        Ok(config)
    }

    /// Load hooks configuration from the default location
    ///
    /// Checks for configuration files in the following priority order:
    /// 1. .claude/settings.json (amplihack standard location)
    /// 2. .claude/hooks/config.json (legacy location for backward compatibility)
    ///
    /// Searches in current directory and walks up parent directories.
    /// Returns empty configuration if no config file is found.
    pub async fn load_default() -> Result<HooksConfiguration> {
        // Try to find config in current directory or parent directories
        let mut current_dir = std::env::current_dir()?;

        loop {
            // Priority 1: Check .claude/settings.json first (amplihack standard)
            let settings_path = current_dir.join(".claude/settings.json");
            if settings_path.exists() {
                if let Some(path_str) = settings_path.to_str() {
                    return Self::load_from_file(path_str).await;
                }
            }

            // Priority 2: Fallback to .claude/hooks/config.json (legacy location)
            let legacy_path = current_dir.join(".claude/hooks/config.json");
            if legacy_path.exists() {
                if let Some(path_str) = legacy_path.to_str() {
                    return Self::load_from_file(path_str).await;
                }
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
        let config: HooksConfiguration =
            serde_json::from_str(json).context("Failed to parse hooks configuration JSON")?;
        Ok(config)
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
    #[serial_test::serial]
    async fn test_load_default_with_settings_json() {
        // Test that settings.json is preferred over hooks/config.json
        use tempfile::TempDir;
        use tokio::fs;

        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().to_path_buf();

        // Create .claude directory
        let claude_dir = test_dir.join(".claude");
        fs::create_dir(&claude_dir).await.unwrap();

        // Create both config files
        let settings_json = r#"{
            "SessionStart": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "echo settings.json"
                        }
                    ]
                }
            ]
        }"#;

        let hooks_config_json = r#"{
            "SessionStart": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "echo hooks/config.json"
                        }
                    ]
                }
            ]
        }"#;

        // Write settings.json
        fs::write(claude_dir.join("settings.json"), settings_json)
            .await
            .unwrap();

        // Write hooks/config.json
        let hooks_dir = claude_dir.join("hooks");
        fs::create_dir(&hooks_dir).await.unwrap();
        fs::write(hooks_dir.join("config.json"), hooks_config_json)
            .await
            .unwrap();

        // Change to test directory with proper error handling
        let original_dir = std::env::current_dir().unwrap().canonicalize().unwrap();

        // Ensure cleanup happens even if test panics
        struct DirGuard(std::path::PathBuf);
        impl Drop for DirGuard {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }
        let _guard = DirGuard(original_dir);

        std::env::set_current_dir(&test_dir).unwrap();

        // Load config - should prefer settings.json
        let config = HookLoader::load_default().await.unwrap();

        // Verify it loaded from settings.json (has the "echo settings.json" command)
        assert_eq!(config.session_start.len(), 1);
        assert_eq!(config.session_start[0].hooks.len(), 1);
        if let crate::hooks::types::HookType::Command = config.session_start[0].hooks[0].hook_type {
            assert_eq!(
                config.session_start[0].hooks[0].command.as_ref().unwrap(),
                "echo settings.json"
            );
        } else {
            panic!("Expected Command hook type");
        }

        // DirGuard will restore directory automatically
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_load_default_fallback_to_hooks_config() {
        // Test that hooks/config.json is used when settings.json doesn't exist
        use tempfile::TempDir;
        use tokio::fs;

        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().to_path_buf();

        // Create .claude/hooks directory
        let claude_dir = test_dir.join(".claude");
        let hooks_dir = claude_dir.join("hooks");
        fs::create_dir_all(&hooks_dir).await.unwrap();

        // Only create hooks/config.json (no settings.json)
        let hooks_config_json = r#"{
            "SessionStart": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "echo legacy"
                        }
                    ]
                }
            ]
        }"#;

        fs::write(hooks_dir.join("config.json"), hooks_config_json)
            .await
            .unwrap();

        // Change to test directory with guard
        let original_dir = std::env::current_dir().unwrap().canonicalize().unwrap();
        struct DirGuard(std::path::PathBuf);
        impl Drop for DirGuard {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }
        let _guard = DirGuard(original_dir);

        std::env::set_current_dir(&test_dir).unwrap();

        // Load config - should fallback to hooks/config.json
        let config = HookLoader::load_default().await.unwrap();

        // Verify it loaded from hooks/config.json
        assert_eq!(config.session_start.len(), 1);
        assert_eq!(config.session_start[0].hooks.len(), 1);
        if let crate::hooks::types::HookType::Command = config.session_start[0].hooks[0].hook_type {
            assert_eq!(
                config.session_start[0].hooks[0].command.as_ref().unwrap(),
                "echo legacy"
            );
        } else {
            panic!("Expected Command hook type");
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_load_default_no_config_files() {
        // Test that empty config is returned when no files exist
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().to_path_buf();

        // Change to test directory (no .claude directory at all)
        let original_dir = std::env::current_dir().unwrap().canonicalize().unwrap();
        struct DirGuard(std::path::PathBuf);
        impl Drop for DirGuard {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }
        let _guard = DirGuard(original_dir);

        std::env::set_current_dir(&test_dir).unwrap();

        // Load config - should return empty default
        let config = HookLoader::load_default().await.unwrap();

        // Verify empty config
        assert_eq!(config.session_start.len(), 0);
        assert_eq!(config.session_end.len(), 0);
        assert_eq!(config.pre_tool_use.len(), 0);
        assert_eq!(config.post_tool_use.len(), 0);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_load_default_walks_parent_directories() {
        // Test that config search walks up parent directories
        use tempfile::TempDir;
        use tokio::fs;

        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().to_path_buf();

        // Create .claude/settings.json in parent directory
        let claude_dir = test_dir.join(".claude");
        fs::create_dir(&claude_dir).await.unwrap();

        let settings_json = r#"{
            "SessionStart": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "echo parent"
                        }
                    ]
                }
            ]
        }"#;

        fs::write(claude_dir.join("settings.json"), settings_json)
            .await
            .unwrap();

        // Create subdirectory and change to it
        let sub_dir = test_dir.join("subdir");
        fs::create_dir(&sub_dir).await.unwrap();

        let original_dir = std::env::current_dir().unwrap().canonicalize().unwrap();
        struct DirGuard(std::path::PathBuf);
        impl Drop for DirGuard {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }
        let _guard = DirGuard(original_dir);

        std::env::set_current_dir(&sub_dir).unwrap();

        // Load config - should find parent's settings.json
        let config = HookLoader::load_default().await.unwrap();

        // Verify it found the parent config
        assert_eq!(config.session_start.len(), 1);
        assert_eq!(config.session_start[0].hooks.len(), 1);
        if let crate::hooks::types::HookType::Command = config.session_start[0].hooks[0].hook_type {
            assert_eq!(
                config.session_start[0].hooks[0].command.as_ref().unwrap(),
                "echo parent"
            );
        } else {
            panic!("Expected Command hook type");
        }
    }
}
