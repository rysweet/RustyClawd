/// Format parsing for configuration files.
///
/// Supports JSON, TOML, and YAML formats. When a file has no recognized
/// extension, formats are tried in priority order: TOML > YAML > JSON.
use std::path::Path;

use crate::plugins::tool_search_config::ToolSearchConfig;
use crate::settings::types::{PermissionMode, Settings, ToolPermission};

/// Load settings from a file, detecting format by extension or trying all formats.
pub fn parse_settings_from_file(path: &Path) -> Result<Settings, String> {
    use std::fs;

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file {:?}: {}", path, e))?;

    let extension = path.extension().and_then(|s| s.to_str());

    match extension {
        Some("toml") => parse_toml_config(&content),
        Some("yaml") | Some("yml") => parse_yaml_config(&content),
        Some("json") => parse_json_config(&content),
        _ => {
            // No extension or unknown extension
            // Try formats in priority order: TOML > YAML > JSON
            if let Ok(settings) = parse_toml_config(&content) {
                Ok(settings)
            } else if let Ok(settings) = parse_yaml_config(&content) {
                Ok(settings)
            } else if let Ok(settings) = parse_json_config(&content) {
                Ok(settings)
            } else {
                Err(format!(
                    "Unable to parse config file {:?}. \
                     Supported formats: TOML (.toml), YAML (.yaml/.yml), JSON (.json).",
                    path
                ))
            }
        }
    }
}

/// Parse JSON configuration into Settings
pub fn parse_json_config(content: &str) -> Result<Settings, String> {
    let json_value: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {}", e))?;

    let mut settings = Settings::new();

    if let Some(obj) = json_value.as_object() {
        if let Some(model) = obj.get("model").and_then(|v| v.as_str()) {
            settings = settings.with_model(model.to_string());
        }

        if let Some(api_url) = obj.get("api_url").and_then(|v| v.as_str()) {
            settings = settings.with_api_url(api_url.to_string());
        }

        if let Some(timeout) = obj.get("timeout_secs").and_then(|v| v.as_u64()) {
            settings = settings.with_timeout(timeout);
        }

        if let Some(cleanup) = obj.get("cleanup_period_days").and_then(|v| v.as_u64()) {
            settings = settings.with_cleanup_period(cleanup as u32);
        }

        if let Some(disable_bypass) = obj
            .get("disable_bypass_permissions")
            .and_then(|v| v.as_bool())
        {
            if disable_bypass {
                settings = settings.disable_bypass();
            }
        }

        // Parse permissions object
        if let Some(permissions) = obj.get("permissions").and_then(|v| v.as_object()) {
            for (tool_name, tool_config) in permissions {
                if let Some(tool_obj) = tool_config.as_object() {
                    let mode_str = tool_obj
                        .get("mode")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ask");

                    if let Some(mode) = PermissionMode::parse(mode_str) {
                        let patterns = tool_obj
                            .get("patterns")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();

                        let permission = ToolPermission::new(mode, patterns);
                        settings = settings.with_permission(tool_name.clone(), permission);
                    }
                }
            }
        }

        // Parse env_vars object
        if let Some(env_vars) = obj.get("env_vars").and_then(|v| v.as_object()) {
            for (key, value) in env_vars {
                if let Some(value_str) = value.as_str() {
                    settings = settings.with_env_var(key.clone(), value_str.to_string());
                }
            }
        }

        // Parse enabled_plugins object
        if let Some(plugins) = obj.get("enabled_plugins").and_then(|v| v.as_object()) {
            for (plugin_id, enabled) in plugins {
                if let Some(enabled_bool) = enabled.as_bool() {
                    settings = settings.set_plugin(plugin_id.clone(), enabled_bool);
                }
            }
        }

        // Parse tool_search (auto:N syntax)
        if let Some(tool_search) = obj.get("tool_search").and_then(|v| v.as_str()) {
            if let Ok(config) = ToolSearchConfig::parse(tool_search) {
                settings = settings.with_tool_search(config);
            }
        }

        // Parse spinner_tips_override array
        if let Some(tips) = obj.get("spinner_tips_override").and_then(|v| v.as_array()) {
            let tips_vec: Vec<String> = tips
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            settings = settings.with_spinner_tips_override(tips_vec);
        }

        // Parse reduced_motion boolean
        if let Some(reduced) = obj.get("reduced_motion").and_then(|v| v.as_bool()) {
            settings = settings.with_reduced_motion(reduced);
        }

        // Parse includeGitInstructions boolean (default true)
        if let Some(include_git) = obj
            .get("includeGitInstructions")
            .and_then(|v| v.as_bool())
        {
            settings = settings.with_include_git_instructions(include_git);
        }
    }

    Ok(settings)
}

/// Parse YAML configuration into Settings
pub fn parse_yaml_config(content: &str) -> Result<Settings, String> {
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;

    let json_str = serde_json::to_string(&yaml_value)
        .map_err(|e| format!("Failed to convert YAML to JSON: {}", e))?;

    parse_json_config(&json_str)
}

/// Parse TOML configuration into Settings
pub fn parse_toml_config(content: &str) -> Result<Settings, String> {
    let toml_value: toml::Value = toml::from_str(content).map_err(|e| {
        format!(
            "Invalid TOML at {}: {}",
            e.span()
                .map(|s| format!("line {}", s.start))
                .unwrap_or_else(|| "unknown location".to_string()),
            e.message()
        )
    })?;

    let mut settings = Settings::new();

    if let Some(table) = toml_value.as_table() {
        if let Some(model) = table.get("model").and_then(|v| v.as_str()) {
            settings = settings.with_model(model.to_string());
        }

        if let Some(api_url) = table.get("api_url").and_then(|v| v.as_str()) {
            settings = settings.with_api_url(api_url.to_string());
        }

        if let Some(timeout) = table.get("timeout_secs").and_then(|v| v.as_integer()) {
            settings = settings.with_timeout(timeout as u64);
        }

        if let Some(cleanup) = table
            .get("cleanup_period_days")
            .and_then(|v| v.as_integer())
        {
            settings = settings.with_cleanup_period(cleanup as u32);
        }

        if let Some(disable_bypass) = table
            .get("disable_bypass_permissions")
            .and_then(|v| v.as_bool())
        {
            if disable_bypass {
                settings = settings.disable_bypass();
            }
        }

        // Parse permissions table
        if let Some(permissions) = table.get("permissions").and_then(|v| v.as_table()) {
            for (tool_name, tool_config) in permissions {
                if let Some(tool_table) = tool_config.as_table() {
                    let mode_str = tool_table
                        .get("mode")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ask");

                    if let Some(mode) = PermissionMode::parse(mode_str) {
                        let patterns = tool_table
                            .get("patterns")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();

                        let permission = ToolPermission::new(mode, patterns);
                        settings = settings.with_permission(tool_name.clone(), permission);
                    }
                }
            }
        }

        // Parse env_vars table
        if let Some(env_vars) = table.get("env_vars").and_then(|v| v.as_table()) {
            for (key, value) in env_vars {
                if let Some(value_str) = value.as_str() {
                    settings = settings.with_env_var(key.clone(), value_str.to_string());
                }
            }
        }

        // Parse enabled_plugins table
        if let Some(plugins) = table.get("enabled_plugins").and_then(|v| v.as_table()) {
            for (plugin_id, enabled) in plugins {
                if let Some(enabled_bool) = enabled.as_bool() {
                    settings = settings.set_plugin(plugin_id.clone(), enabled_bool);
                }
            }
        }

        // Parse tool_search (auto:N syntax)
        if let Some(tool_search) = table.get("tool_search").and_then(|v| v.as_str()) {
            if let Ok(config) = ToolSearchConfig::parse(tool_search) {
                settings = settings.with_tool_search(config);
            }
        }

        // Parse spinner_tips_override array
        if let Some(tips) = table
            .get("spinner_tips_override")
            .and_then(|v| v.as_array())
        {
            let tips_vec: Vec<String> = tips
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            settings = settings.with_spinner_tips_override(tips_vec);
        }

        // Parse reduced_motion boolean
        if let Some(reduced) = table.get("reduced_motion").and_then(|v| v.as_bool()) {
            settings = settings.with_reduced_motion(reduced);
        }
    }

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toml_config_basic() {
        let toml_content = r#"
model = "claude-3-opus"
api_url = "https://api.example.com"
timeout_secs = 90
cleanup_period_days = 45
"#;

        let settings = parse_toml_config(toml_content);
        assert!(settings.is_ok());

        let settings = settings.unwrap();
        assert_eq!(settings.model, Some("claude-3-opus".to_string()));
        assert_eq!(
            settings.api_url,
            Some("https://api.example.com".to_string())
        );
        assert_eq!(settings.timeout_secs, Some(90));
        assert_eq!(settings.cleanup_period_days, Some(45));
    }

    #[test]
    fn test_parse_toml_config_with_permissions() {
        let toml_content = r#"
model = "claude-3"

[permissions.bash]
mode = "allow"
patterns = ["ls", "cat"]

[permissions.edit]
mode = "deny"
patterns = []
"#;

        let settings = parse_toml_config(toml_content);
        assert!(settings.is_ok());

        let settings = settings.unwrap();
        assert_eq!(settings.model, Some("claude-3".to_string()));
        assert_eq!(settings.permissions.len(), 2);

        let bash_perm = settings.permissions.get("bash").unwrap();
        assert_eq!(
            bash_perm.mode,
            crate::settings::types::PermissionMode::Allow
        );
        assert_eq!(bash_perm.patterns, vec!["ls", "cat"]);

        let edit_perm = settings.permissions.get("edit").unwrap();
        assert_eq!(edit_perm.mode, crate::settings::types::PermissionMode::Deny);
    }

    #[test]
    fn test_parse_toml_config_with_env_vars() {
        let toml_content = r#"
model = "claude-3"

[env_vars]
PROJECT_ID = "proj-123"
RUST_LOG = "debug"
"#;

        let settings = parse_toml_config(toml_content);
        assert!(settings.is_ok());

        let settings = settings.unwrap();
        assert_eq!(
            settings.env_vars.get("PROJECT_ID"),
            Some(&"proj-123".to_string())
        );
        assert_eq!(
            settings.env_vars.get("RUST_LOG"),
            Some(&"debug".to_string())
        );
    }

    #[test]
    fn test_parse_toml_config_with_plugins() {
        let toml_content = r#"
model = "claude-3"

[enabled_plugins]
github = true
gitlab = false
custom_tool = true
"#;

        let settings = parse_toml_config(toml_content);
        assert!(settings.is_ok());

        let settings = settings.unwrap();
        assert_eq!(settings.enabled_plugins.get("github"), Some(&true));
        assert_eq!(settings.enabled_plugins.get("gitlab"), Some(&false));
        assert_eq!(settings.enabled_plugins.get("custom_tool"), Some(&true));
    }

    #[test]
    fn test_parse_toml_config_with_disable_bypass() {
        let toml_content = r#"
model = "claude-3"
disable_bypass_permissions = true
"#;

        let settings = parse_toml_config(toml_content);
        assert!(settings.is_ok());

        let settings = settings.unwrap();
        assert!(settings.disable_bypass_permissions);
    }

    #[test]
    fn test_parse_toml_config_invalid_syntax() {
        let toml_content = r#"
model = "claude-3"
invalid syntax here
"#;

        let result = parse_toml_config(toml_content);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.contains("Invalid TOML"));
    }

    #[test]
    fn test_parse_toml_config_empty() {
        let toml_content = "";

        let settings = parse_toml_config(toml_content);
        assert!(settings.is_ok());

        let settings = settings.unwrap();
        assert!(settings.is_empty());
    }

    #[test]
    fn test_parse_toml_config_comprehensive() {
        let toml_content = r#"
model = "claude-3-opus-20240229"
api_url = "https://api.anthropic.com/v1/messages"
timeout_secs = 180
cleanup_period_days = 60
disable_bypass_permissions = true

[env_vars]
PROJECT_NAME = "test-project"
DATABASE_URL = "postgresql://localhost/test"

[permissions.bash]
mode = "ask"
patterns = ["ls", "cat", "git"]

[permissions.edit]
mode = "allow"
patterns = ["*.rs", "*.toml"]

[enabled_plugins]
github = true
gitlab = false
jira = true
"#;

        let settings = parse_toml_config(toml_content);
        assert!(settings.is_ok());

        let settings = settings.unwrap();

        assert_eq!(settings.model, Some("claude-3-opus-20240229".to_string()));
        assert_eq!(
            settings.api_url,
            Some("https://api.anthropic.com/v1/messages".to_string())
        );
        assert_eq!(settings.timeout_secs, Some(180));
        assert_eq!(settings.cleanup_period_days, Some(60));
        assert!(settings.disable_bypass_permissions);

        assert_eq!(settings.env_vars.len(), 2);
        assert_eq!(
            settings.env_vars.get("PROJECT_NAME"),
            Some(&"test-project".to_string())
        );

        assert_eq!(settings.permissions.len(), 2);
        assert!(settings.permissions.contains_key("bash"));
        assert!(settings.permissions.contains_key("edit"));

        assert_eq!(settings.enabled_plugins.len(), 3);
        assert_eq!(settings.enabled_plugins.get("github"), Some(&true));
        assert_eq!(settings.enabled_plugins.get("gitlab"), Some(&false));

        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_parse_toml_config_with_tool_search() {
        let toml_content = r#"
model = "claude-3"
tool_search = "auto:15"
"#;

        let settings = parse_toml_config(toml_content);
        assert!(settings.is_ok());

        let settings = settings.unwrap();
        assert!(settings.tool_search.is_auto());
        assert_eq!(settings.tool_search.threshold_percent(), Some(15));
    }

    #[test]
    fn test_parse_json_config_with_tool_search() {
        let json_content = r#"
{
    "model": "claude-3",
    "tool_search": "auto:20"
}
"#;

        let settings = parse_json_config(json_content);
        assert!(settings.is_ok());

        let settings = settings.unwrap();
        assert!(settings.tool_search.is_auto());
        assert_eq!(settings.tool_search.threshold_percent(), Some(20));
    }

    #[test]
    fn test_parse_json_config_with_spinner_tips_override() {
        let json_content = r#"
{
    "model": "claude-3",
    "spinner_tips_override": ["Reticulating splines...", "Loading cargo..."]
}
"#;

        let settings = parse_json_config(json_content).unwrap();
        assert_eq!(
            settings.spinner_tips_override,
            Some(vec![
                "Reticulating splines...".to_string(),
                "Loading cargo...".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_json_config_with_reduced_motion() {
        let json_content = r#"
{
    "model": "claude-3",
    "reduced_motion": true
}
"#;

        let settings = parse_json_config(json_content).unwrap();
        assert!(settings.reduced_motion);
    }

    #[test]
    fn test_parse_toml_config_with_spinner_tips_override() {
        let toml_content = r#"
model = "claude-3"
spinner_tips_override = ["Arr, loading...", "Swabbing the decks..."]
"#;

        let settings = parse_toml_config(toml_content).unwrap();
        assert_eq!(
            settings.spinner_tips_override,
            Some(vec![
                "Arr, loading...".to_string(),
                "Swabbing the decks...".to_string()
            ])
        );
    }

    #[test]
    fn test_parse_toml_config_with_reduced_motion() {
        let toml_content = r#"
model = "claude-3"
reduced_motion = true
"#;

        let settings = parse_toml_config(toml_content).unwrap();
        assert!(settings.reduced_motion);
    }

    #[test]
    fn test_parse_toml_config_reduced_motion_false_by_default() {
        let toml_content = r#"
model = "claude-3"
"#;

        let settings = parse_toml_config(toml_content).unwrap();
        assert!(!settings.reduced_motion);
        assert_eq!(settings.spinner_tips_override, None);
    }
}
