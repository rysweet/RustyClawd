/// Settings loader - coordinator that orchestrates discovery and parsing.
///
/// Loads configuration from multiple sources (files, environment variables)
/// and assembles them into a [`SettingsHierarchy`].
use crate::plugins::tool_search_config::ToolSearchConfig;
use crate::settings::discovery;
use crate::settings::hierarchy::SettingsHierarchy;
use crate::settings::types::{Settings, SettingsLayer};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

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
        let path = PathBuf::from(settings_path);
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Settings file not found: {}",
                settings_path
            ));
        }

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
                    if let Ok(config) = ToolSearchConfig::parse(value) {
                        settings = settings.with_tool_search(config);
                    }
                }
                "reduced_motion" => {
                    if value.to_lowercase() == "true" || value == "1" {
                        settings = settings.with_reduced_motion(true);
                    }
                }
                _ if !key.starts_with("_") => {
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
        if let Ok(user_settings) = discovery::load_user_global_settings() {
            hierarchy.add_layer(SettingsLayer::UserGlobal, user_settings);
        }

        // Try to load project settings
        if let Some(ref project_root) = self.project_root {
            if let Ok(shared) = discovery::load_project_shared_settings(project_root) {
                hierarchy.add_layer(SettingsLayer::ProjectShared, shared);
            }

            if let Ok(local) = discovery::load_project_local_settings(project_root) {
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
        if let Ok(enterprise) = discovery::load_enterprise_settings() {
            hierarchy.add_layer(SettingsLayer::EnterpriseManaged, enterprise);
        }

        Ok(hierarchy)
    }

    /// Get the user config directory (delegates to discovery module)
    pub fn get_user_config_dir() -> Result<PathBuf, String> {
        discovery::get_user_config_dir()
    }

    /// Get the user config file path (delegates to discovery module)
    pub fn get_user_config_path() -> Result<PathBuf, String> {
        discovery::get_user_config_path()
    }

    /// Parse JSON configuration (delegates to parser module)
    pub fn parse_json_config(content: &str) -> Result<Settings, String> {
        crate::settings::parser::parse_json_config(content)
    }

    /// Parse TOML configuration (delegates to parser module)
    pub fn parse_toml_config(content: &str) -> Result<Settings, String> {
        crate::settings::parser::parse_toml_config(content)
    }

    /// Parse YAML configuration (delegates to parser module)
    pub fn parse_yaml_config(content: &str) -> Result<Settings, String> {
        crate::settings::parser::parse_yaml_config(content)
    }

    /// Create an in-memory settings hierarchy for testing
    pub fn create_test_hierarchy() -> SettingsHierarchy {
        let mut hierarchy = SettingsHierarchy::new();
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
}
