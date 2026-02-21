use crate::plugins::tool_search_config::ToolSearchConfig;
use crate::settings::hierarchy::SettingsHierarchy;
/// Configuration loading from various sources (files, environment variables)
use crate::settings::types::{PermissionMode, Settings, SettingsLayer, ToolPermission};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

/// Settings loader - loads configuration from multiple sources
pub struct SettingsLoader {
    project_root: Option<PathBuf>,
}

impl SettingsLoader {
    /// Create a new settings loader
    pub fn new() -> Self {
        Self { project_root: None }
    }

    /// Create loader with specific project root
    pub fn with_project_root(project_root: PathBuf) -> Self {
        Self {
            project_root: Some(project_root),
        }
    }

    /// Create loader with custom settings file path
    pub fn with_custom_path(settings_path: &str) -> Result<Self, anyhow::Error> {
        // Parse the settings file path
        let path = PathBuf::from(settings_path);
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Settings file not found: {}",
                settings_path
            ));
        }

        // Use the parent directory as project root
        let project_root = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid settings path: {}", settings_path))?
            .to_path_buf();

        Ok(Self {
            project_root: Some(project_root),
        })
    }

    /// Set the project root
    pub fn set_project_root(&mut self, root: PathBuf) {
        self.project_root = Some(root);
    }

    /// Load all environment variables starting with CLAUDE_
    pub fn load_env_overrides(&self) -> HashMap<String, String> {
        let mut overrides = HashMap::new();

        for (key, value) in env::vars() {
            if key.starts_with("CLAUDE_") {
                // Convert CLAUDE_API_URL -> api_url
                let setting_key = key.strip_prefix("CLAUDE_").unwrap_or(&key).to_lowercase();

                overrides.insert(setting_key, value);
            }
        }

        overrides
    }

    /// Parse environment variable overrides into Settings
    pub fn parse_env_overrides(overrides: &HashMap<String, String>) -> Settings {
        let mut settings = Settings::new();

        for (key, value) in overrides {
            match key.as_str() {
                "model" => {
                    settings = settings.with_model(value.clone());
                }
                "api_url" => {
                    settings = settings.with_api_url(value.clone());
                }
                "timeout_secs" | "timeout" => {
                    if let Ok(timeout) = value.parse::<u64>() {
                        settings = settings.with_timeout(timeout);
                    }
                }
                "cleanup_period_days" | "cleanup_period" => {
                    if let Ok(days) = value.parse::<u32>() {
                        settings = settings.with_cleanup_period(days);
                    }
                }
                "disable_bypass_permissions" | "disable_bypass" => {
                    if value.to_lowercase() == "true" || value == "1" {
                        settings = settings.disable_bypass();
                    }
                }
                "enable_tool_search" | "tool_search" => {
                    // Parse auto:N syntax for MCP tool search configuration
                    if let Ok(config) = ToolSearchConfig::parse(value) {
                        settings = settings.with_tool_search(config);
                    }
                }
                // Additional env variables not directly mapped become env_vars
                _ if !key.starts_with("_") => {
                    // Store as environment variable if not a known setting
                    settings = settings.with_env_var(key.clone(), value.clone());
                }
                _ => {}
            }
        }

        settings
    }

    /// Load settings from default locations
    pub fn load_hierarchy(&self) -> Result<SettingsHierarchy, String> {
        let mut hierarchy = SettingsHierarchy::new();

        // Default settings are always present (implicit)
        hierarchy.add_layer(SettingsLayer::Default, Settings::default());

        // Try to load user global settings
        if let Ok(user_settings) = self.load_user_global_settings() {
            hierarchy.add_layer(SettingsLayer::UserGlobal, user_settings);
        }

        // Try to load project settings
        if let Some(ref project_root) = self.project_root {
            if let Ok(shared) = self.load_project_shared_settings(project_root) {
                hierarchy.add_layer(SettingsLayer::ProjectShared, shared);
            }

            if let Ok(local) = self.load_project_local_settings(project_root) {
                hierarchy.add_layer(SettingsLayer::ProjectLocal, local);
            }
        }

        // Load environment variable overrides
        let env_overrides = self.load_env_overrides();
        if !env_overrides.is_empty() {
            let env_settings = Self::parse_env_overrides(&env_overrides);
            hierarchy.add_layer(SettingsLayer::CommandLine, env_settings);
        }

        // Try to load enterprise settings
        if let Ok(enterprise) = self.load_enterprise_settings() {
            hierarchy.add_layer(SettingsLayer::EnterpriseManaged, enterprise);
        }

        Ok(hierarchy)
    }

    /// Load user global settings from ~/.claude/config
    fn load_user_global_settings(&self) -> Result<Settings, String> {
        let config_path = Self::get_user_config_path()?;

        if !config_path.exists() {
            return Err(format!("User config not found: {:?}", config_path));
        }

        self.load_settings_from_file(&config_path)
    }

    /// Load project shared settings from .claude/config
    fn load_project_shared_settings(&self, project_root: &Path) -> Result<Settings, String> {
        let config_path = project_root.join(".claude").join("config");

        if !config_path.exists() {
            return Err(format!(
                "Project shared config not found: {:?}",
                config_path
            ));
        }

        self.load_settings_from_file(&config_path)
    }

    /// Load project local settings from .claude/config.local
    fn load_project_local_settings(&self, project_root: &Path) -> Result<Settings, String> {
        let config_path = project_root.join(".claude").join("config.local");

        if !config_path.exists() {
            return Err(format!("Project local config not found: {:?}", config_path));
        }

        self.load_settings_from_file(&config_path)
    }

    /// Load enterprise settings from /etc/claude/config
    fn load_enterprise_settings(&self) -> Result<Settings, String> {
        #[cfg(unix)]
        let config_path = Path::new("/etc/claude/config");

        #[cfg(windows)]
        let config_path = Path::new("C:\\ProgramData\\Claude\\config");

        if !config_path.exists() {
            return Err(format!("Enterprise config not found: {:?}", config_path));
        }

        self.load_settings_from_file(config_path)
    }

    /// Load settings from a file
    fn load_settings_from_file(&self, path: &Path) -> Result<Settings, String> {
        use std::fs;

        // Read file contents
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file {:?}: {}", path, e))?;

        // Determine format based on file extension or content
        let extension = path.extension().and_then(|s| s.to_str());

        match extension {
            Some("toml") => {
                // Parse as TOML (highest priority)
                Self::parse_toml_config(&content)
            }
            Some("yaml") | Some("yml") => Self::parse_yaml_config(&content),
            Some("json") => {
                // Parse as JSON
                Self::parse_json_config(&content)
            }
            _ => {
                // No extension or unknown extension
                // Try formats in priority order: TOML > YAML > JSON
                if let Ok(settings) = Self::parse_toml_config(&content) {
                    Ok(settings)
                } else if let Ok(settings) = Self::parse_yaml_config(&content) {
                    Ok(settings)
                } else if let Ok(settings) = Self::parse_json_config(&content) {
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
    fn parse_json_config(content: &str) -> Result<Settings, String> {
        let json_value: serde_json::Value =
            serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {}", e))?;

        let mut settings = Settings::new();

        // Parse known settings fields
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
        }

        Ok(settings)
    }

    /// Parse YAML configuration into Settings
    fn parse_yaml_config(content: &str) -> Result<Settings, String> {
        // Parse YAML into a serde_json::Value via serde_yaml
        let yaml_value: serde_yaml::Value =
            serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;

        // Convert serde_yaml::Value to serde_json::Value for uniform handling
        let json_str = serde_json::to_string(&yaml_value)
            .map_err(|e| format!("Failed to convert YAML to JSON: {}", e))?;

        Self::parse_json_config(&json_str)
    }

    /// Parse TOML configuration into Settings
    fn parse_toml_config(content: &str) -> Result<Settings, String> {
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

        // Parse known settings fields
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
        }

        Ok(settings)
    }

    /// Get the user config directory
    pub fn get_user_config_dir() -> Result<PathBuf, String> {
        if let Ok(config_home) = env::var("XDG_CONFIG_HOME") {
            Ok(PathBuf::from(config_home).join("claude"))
        } else if let Ok(home) = env::var("HOME") {
            Ok(PathBuf::from(home).join(".config").join("claude"))
        } else {
            #[cfg(windows)]
            if let Ok(appdata) = env::var("APPDATA") {
                Ok(PathBuf::from(appdata).join("Claude"))
            } else {
                Err("Could not determine config directory".to_string())
            }

            #[cfg(not(windows))]
            Err("Could not determine config directory".to_string())
        }
    }

    /// Get the user config file path
    pub fn get_user_config_path() -> Result<PathBuf, String> {
        let config_dir = Self::get_user_config_dir()?;
        Ok(config_dir.join("config"))
    }

    /// Create an in-memory settings hierarchy for testing
    pub fn create_test_hierarchy() -> SettingsHierarchy {
        let mut hierarchy = SettingsHierarchy::new();

        // Add some test defaults
        hierarchy.add_layer(SettingsLayer::Default, Settings::default());

        hierarchy
    }
}

impl Default for SettingsLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_env_overrides() {
        let loader = SettingsLoader::new();
        let overrides = loader.load_env_overrides();

        // Will vary based on environment, but should be a HashMap
        // At minimum it should be empty or contain env vars
        // Length is always >= 0 for HashMap, so just verify it's a valid HashMap
        let _count = overrides.len();
    }

    #[test]
    fn test_parse_env_overrides_model() {
        let mut overrides = HashMap::new();
        overrides.insert("model".to_string(), "claude-3".to_string());

        let settings = SettingsLoader::parse_env_overrides(&overrides);
        assert_eq!(settings.model, Some("claude-3".to_string()));
    }

    #[test]
    fn test_parse_env_overrides_timeout() {
        let mut overrides = HashMap::new();
        overrides.insert("timeout_secs".to_string(), "60".to_string());

        let settings = SettingsLoader::parse_env_overrides(&overrides);
        assert_eq!(settings.timeout_secs, Some(60));
    }

    #[test]
    fn test_parse_env_overrides_disable_bypass() {
        let mut overrides = HashMap::new();
        overrides.insert("disable_bypass_permissions".to_string(), "true".to_string());

        let settings = SettingsLoader::parse_env_overrides(&overrides);
        assert!(settings.disable_bypass_permissions);
    }

    #[test]
    fn test_parse_env_overrides_multiple() {
        let mut overrides = HashMap::new();
        overrides.insert("model".to_string(), "claude-3".to_string());
        overrides.insert("timeout_secs".to_string(), "120".to_string());
        overrides.insert("api_url".to_string(), "https://api.example.com".to_string());

        let settings = SettingsLoader::parse_env_overrides(&overrides);

        assert_eq!(settings.model, Some("claude-3".to_string()));
        assert_eq!(settings.timeout_secs, Some(120));
        assert_eq!(
            settings.api_url,
            Some("https://api.example.com".to_string())
        );
    }

    #[test]
    fn test_get_user_config_dir() {
        let result = SettingsLoader::get_user_config_dir();
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(
            path.to_string_lossy().contains("claude") || path.to_string_lossy().contains("Claude")
        );
    }

    #[test]
    fn test_parse_toml_config_basic() {
        let toml_content = r#"
model = "claude-3-opus"
api_url = "https://api.example.com"
timeout_secs = 90
cleanup_period_days = 45
"#;

        let settings = SettingsLoader::parse_toml_config(toml_content);
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

        let settings = SettingsLoader::parse_toml_config(toml_content);
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

        let settings = SettingsLoader::parse_toml_config(toml_content);
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

        let settings = SettingsLoader::parse_toml_config(toml_content);
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

        let settings = SettingsLoader::parse_toml_config(toml_content);
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

        let result = SettingsLoader::parse_toml_config(toml_content);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.contains("Invalid TOML"));
    }

    #[test]
    fn test_parse_toml_config_empty() {
        let toml_content = "";

        let settings = SettingsLoader::parse_toml_config(toml_content);
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

        let settings = SettingsLoader::parse_toml_config(toml_content);
        assert!(settings.is_ok());

        let settings = settings.unwrap();

        // Verify all fields
        assert_eq!(settings.model, Some("claude-3-opus-20240229".to_string()));
        assert_eq!(
            settings.api_url,
            Some("https://api.anthropic.com/v1/messages".to_string())
        );
        assert_eq!(settings.timeout_secs, Some(180));
        assert_eq!(settings.cleanup_period_days, Some(60));
        assert!(settings.disable_bypass_permissions);

        // Verify env_vars
        assert_eq!(settings.env_vars.len(), 2);
        assert_eq!(
            settings.env_vars.get("PROJECT_NAME"),
            Some(&"test-project".to_string())
        );

        // Verify permissions
        assert_eq!(settings.permissions.len(), 2);
        assert!(settings.permissions.contains_key("bash"));
        assert!(settings.permissions.contains_key("edit"));

        // Verify plugins
        assert_eq!(settings.enabled_plugins.len(), 3);
        assert_eq!(settings.enabled_plugins.get("github"), Some(&true));
        assert_eq!(settings.enabled_plugins.get("gitlab"), Some(&false));

        // Verify validation passes
        assert!(settings.validate().is_ok());
    }

    // ===========================================
    // Tool Search (auto:N) Configuration Tests
    // ===========================================

    #[test]
    fn test_parse_env_overrides_tool_search_auto() {
        let mut overrides = HashMap::new();
        overrides.insert("enable_tool_search".to_string(), "auto".to_string());

        let settings = SettingsLoader::parse_env_overrides(&overrides);
        assert!(settings.tool_search.is_auto());
        assert_eq!(settings.tool_search.threshold_percent(), Some(10));
    }

    #[test]
    fn test_parse_env_overrides_tool_search_auto_with_threshold() {
        let mut overrides = HashMap::new();
        overrides.insert("enable_tool_search".to_string(), "auto:5".to_string());

        let settings = SettingsLoader::parse_env_overrides(&overrides);
        assert!(settings.tool_search.is_auto());
        assert_eq!(settings.tool_search.threshold_percent(), Some(5));
    }

    #[test]
    fn test_parse_env_overrides_tool_search_enabled() {
        let mut overrides = HashMap::new();
        overrides.insert("enable_tool_search".to_string(), "true".to_string());

        let settings = SettingsLoader::parse_env_overrides(&overrides);
        assert!(settings.tool_search.is_always_enabled());
    }

    #[test]
    fn test_parse_env_overrides_tool_search_disabled() {
        let mut overrides = HashMap::new();
        overrides.insert("enable_tool_search".to_string(), "false".to_string());

        let settings = SettingsLoader::parse_env_overrides(&overrides);
        assert!(settings.tool_search.is_disabled());
    }

    #[test]
    fn test_parse_toml_config_with_tool_search() {
        let toml_content = r#"
model = "claude-3"
tool_search = "auto:15"
"#;

        let settings = SettingsLoader::parse_toml_config(toml_content);
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

        let settings = SettingsLoader::parse_json_config(json_content);
        assert!(settings.is_ok());

        let settings = settings.unwrap();
        assert!(settings.tool_search.is_auto());
        assert_eq!(settings.tool_search.threshold_percent(), Some(20));
    }
}
