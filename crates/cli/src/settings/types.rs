/// Core types and data structures for the settings/configuration system
use std::collections::HashMap;

use crate::plugins::tool_search_config::ToolSearchConfig;
use crate::settings::validation;

/// Represents permission modes for tool access control
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionMode {
    Allow,
    Ask,
    Deny,
}

impl PermissionMode {
    /// Parse permission mode from string representation
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allow" => Some(PermissionMode::Allow),
            "ask" => Some(PermissionMode::Ask),
            "deny" => Some(PermissionMode::Deny),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &str {
        match self {
            PermissionMode::Allow => "allow",
            PermissionMode::Ask => "ask",
            PermissionMode::Deny => "deny",
        }
    }
}

/// Permission rules for a single tool
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPermission {
    pub mode: PermissionMode,
    pub patterns: Vec<String>, // Prefix patterns for bash commands or file paths
}

impl ToolPermission {
    /// Create a new tool permission
    pub fn new(mode: PermissionMode, patterns: Vec<String>) -> Self {
        Self { mode, patterns }
    }
}

/// Sandbox configuration settings
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SandboxSettings {
    /// Whether sandbox is enabled
    pub enabled: bool,
}

/// Core configuration settings
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Settings {
    /// LLM model to use
    pub model: Option<String>,
    /// API endpoint URL
    pub api_url: Option<String>,
    /// Timeout in seconds for operations
    pub timeout_secs: Option<u64>,
    /// Cleanup period for temporary files in days (None means "not set by this layer")
    pub cleanup_period_days: Option<u32>,
    /// Tool-specific permissions
    pub permissions: HashMap<String, ToolPermission>,
    /// Environment variables to set
    pub env_vars: HashMap<String, String>,
    /// If true, prevent bypassing permissions
    pub disable_bypass_permissions: bool,
    /// Plugin enable/disable settings
    pub enabled_plugins: HashMap<String, bool>,
    /// Sandbox settings
    pub sandbox: Option<SandboxSettings>,
    /// MCP tool search configuration (auto:N syntax)
    pub tool_search: ToolSearchConfig,
}

impl Settings {
    /// Create new default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model
    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    /// Set the API URL
    pub fn with_api_url(mut self, url: String) -> Self {
        self.api_url = Some(url);
        self
    }

    /// Set timeout in seconds
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Set cleanup period in days
    pub fn with_cleanup_period(mut self, days: u32) -> Self {
        self.cleanup_period_days = Some(days);
        self
    }

    /// Add tool permission
    pub fn with_permission(mut self, tool: String, permission: ToolPermission) -> Self {
        self.permissions.insert(tool, permission);
        self
    }

    /// Add environment variable
    pub fn with_env_var(mut self, key: String, value: String) -> Self {
        self.env_vars.insert(key, value);
        self
    }

    /// Enable bypass prevention
    pub fn disable_bypass(mut self) -> Self {
        self.disable_bypass_permissions = true;
        self
    }

    /// Enable/disable a plugin
    pub fn set_plugin(mut self, plugin: String, enabled: bool) -> Self {
        self.enabled_plugins.insert(plugin, enabled);
        self
    }

    /// Set MCP tool search configuration
    pub fn with_tool_search(mut self, config: ToolSearchConfig) -> Self {
        self.tool_search = config;
        self
    }

    /// Validate settings configuration.
    /// Delegates to the shared validation functions in `validation.rs`.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(timeout) = self.timeout_secs {
            validation::validate_timeout(timeout)?;
        }

        if let Some(days) = self.cleanup_period_days {
            validation::validate_cleanup_period(days)?;
        }

        if let Some(ref url) = self.api_url {
            validation::validate_url(url)?;
        }

        Ok(())
    }

    /// Check if settings are effectively empty (only defaults)
    pub fn is_empty(&self) -> bool {
        self.model.is_none()
            && self.api_url.is_none()
            && self.timeout_secs.is_none()
            && self.cleanup_period_days.is_none()
            && self.permissions.is_empty()
            && self.env_vars.is_empty()
            && !self.disable_bypass_permissions
            && self.enabled_plugins.is_empty()
            && self.sandbox.is_none()
            && self.tool_search.is_auto()
    }

    /// Get tool search configuration
    pub fn get_tool_search(&self) -> ToolSearchConfig {
        self.tool_search
    }
}

/// Settings layer identifier for hierarchy tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SettingsLayer {
    Default = 0,
    UserGlobal = 1,
    ProjectShared = 2,
    ProjectLocal = 3,
    CommandLine = 4,
    EnterpriseManaged = 5,
}

impl SettingsLayer {
    /// Get numeric priority (higher = higher priority)
    pub fn priority(&self) -> u32 {
        *self as u32
    }

    /// Get layer name
    pub fn name(&self) -> &str {
        match self {
            SettingsLayer::Default => "default",
            SettingsLayer::UserGlobal => "user_global",
            SettingsLayer::ProjectShared => "project_shared",
            SettingsLayer::ProjectLocal => "project_local",
            SettingsLayer::CommandLine => "command_line",
            SettingsLayer::EnterpriseManaged => "enterprise_managed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_mode_from_str() {
        assert_eq!(PermissionMode::parse("allow"), Some(PermissionMode::Allow));
        assert_eq!(PermissionMode::parse("ask"), Some(PermissionMode::Ask));
        assert_eq!(PermissionMode::parse("deny"), Some(PermissionMode::Deny));
        assert_eq!(PermissionMode::parse("invalid"), None);
    }

    #[test]
    fn test_settings_default() {
        let settings = Settings::new();
        assert_eq!(settings.model, None);
        assert_eq!(settings.cleanup_period_days, None);
        assert!(settings.permissions.is_empty());
    }

    #[test]
    fn test_settings_builder() {
        let settings = Settings::new()
            .with_model("claude-3".to_string())
            .with_timeout(60);

        assert_eq!(settings.model, Some("claude-3".to_string()));
        assert_eq!(settings.timeout_secs, Some(60));
    }

    #[test]
    fn test_layer_priority() {
        assert!(SettingsLayer::Default.priority() < SettingsLayer::UserGlobal.priority());
        assert!(
            SettingsLayer::EnterpriseManaged.priority() > SettingsLayer::CommandLine.priority()
        );
    }
}
