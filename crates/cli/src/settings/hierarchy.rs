/// Settings hierarchy and precedence management
use crate::settings::types::{Settings, SettingsLayer};
use std::collections::HashMap;

/// Settings hierarchy manager - manages multiple configuration layers
#[derive(Debug, Clone)]
pub struct SettingsHierarchy {
    layers: HashMap<SettingsLayer, Settings>,
}

impl SettingsHierarchy {
    /// Create a new empty hierarchy
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
        }
    }

    /// Add or replace a settings layer
    pub fn add_layer(&mut self, layer: SettingsLayer, settings: Settings) {
        self.layers.insert(layer, settings);
    }

    /// Remove a settings layer
    pub fn remove_layer(&mut self, layer: SettingsLayer) -> Option<Settings> {
        self.layers.remove(&layer)
    }

    /// Get settings from a specific layer
    pub fn get_layer(&self, layer: SettingsLayer) -> Option<&Settings> {
        self.layers.get(&layer)
    }

    /// Get mutable reference to layer settings
    pub fn get_layer_mut(&mut self, layer: SettingsLayer) -> Option<&mut Settings> {
        self.layers.get_mut(&layer)
    }

    /// Check if a layer is present
    pub fn has_layer(&self, layer: SettingsLayer) -> bool {
        self.layers.contains_key(&layer)
    }

    /// Get list of active layers in priority order
    pub fn active_layers(&self) -> Vec<SettingsLayer> {
        let mut layers: Vec<_> = self.layers.keys().copied().collect();
        layers.sort_by_key(|l| l.priority());
        layers
    }

    /// Merge all layers respecting hierarchy precedence (higher priority overrides lower)
    ///
    /// Precedence order (from lowest to highest priority):
    /// 1. Default (implicit, built into merge result)
    /// 2. User Global (~/.claude/config.toml or equivalent)
    /// 3. Project Shared (.claude/config.toml in project repo)
    /// 4. Project Local (.claude/config.local.toml in project repo)
    /// 5. Command Line (runtime CLI flags)
    /// 6. Enterprise Managed (system administrator enforced)
    pub fn merge(&self) -> Settings {
        let mut result = Settings::default();

        // Sort layers by priority (lower to higher)
        let mut sorted_layers: Vec<_> = self.layers.iter().collect();
        sorted_layers.sort_by_key(|(layer, _)| layer.priority());

        for (_, settings) in sorted_layers {
            // Model override - highest priority wins
            if settings.model.is_some() {
                result.model = settings.model.clone();
            }

            // API URL override - highest priority wins
            if settings.api_url.is_some() {
                result.api_url = settings.api_url.clone();
            }

            // Timeout override - highest priority wins
            if settings.timeout_secs.is_some() {
                result.timeout_secs = settings.timeout_secs;
            }

            // Cleanup period override - highest priority wins
            if settings.cleanup_period_days.is_some() {
                result.cleanup_period_days = settings.cleanup_period_days;
            }

            // Merge permissions - higher layer adds/overrides specific tools
            for (tool, perm) in &settings.permissions {
                result.permissions.insert(tool.clone(), perm.clone());
            }

            // Merge environment variables - higher layer overrides
            for (key, val) in &settings.env_vars {
                result.env_vars.insert(key.clone(), val.clone());
            }

            // Bypass permissions flag - sticky: once set to true, stays true
            if settings.disable_bypass_permissions {
                result.disable_bypass_permissions = true;
            }

            // Merge plugin settings - higher layer overrides
            for (plugin, enabled) in &settings.enabled_plugins {
                result.enabled_plugins.insert(plugin.clone(), *enabled);
            }
        }

        result
    }

    /// Clear all layers
    pub fn clear(&mut self) {
        self.layers.clear();
    }

    /// Get total number of layers
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Check if hierarchy is empty
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Get a debug summary of what's in each layer
    pub fn summary(&self) -> String {
        let mut summary = String::from("Settings Hierarchy Summary:\n");

        let mut layers: Vec<_> = self.layers.iter().collect();
        layers.sort_by_key(|(l, _)| l.priority());

        for (layer, settings) in layers {
            summary.push_str(&format!("  {}: ", layer.name()));

            let mut parts: Vec<String> = Vec::new();
            if settings.model.is_some() {
                parts.push("model".to_string());
            }
            if settings.api_url.is_some() {
                parts.push("api_url".to_string());
            }
            if settings.timeout_secs.is_some() {
                parts.push("timeout".to_string());
            }
            if settings.cleanup_period_days.is_some() {
                parts.push("cleanup".to_string());
            }
            if !settings.permissions.is_empty() {
                parts.push(format!("perms:{}", settings.permissions.len()));
            }
            if !settings.env_vars.is_empty() {
                parts.push(format!("env:{}", settings.env_vars.len()));
            }
            if settings.disable_bypass_permissions {
                parts.push("no_bypass".to_string());
            }
            if !settings.enabled_plugins.is_empty() {
                parts.push(format!("plugins:{}", settings.enabled_plugins.len()));
            }

            if parts.is_empty() {
                summary.push_str("(empty)");
            } else {
                summary.push_str(
                    &parts
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }

            summary.push('\n');
        }

        summary
    }
}

impl Default for SettingsHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchy_empty() {
        let hierarchy = SettingsHierarchy::new();
        assert!(hierarchy.is_empty());
        assert_eq!(hierarchy.layer_count(), 0);
        assert_eq!(hierarchy.merge(), Settings::default());
    }

    #[test]
    fn test_add_and_retrieve_layer() {
        let mut hierarchy = SettingsHierarchy::new();
        let settings = Settings::new().with_timeout(60);

        hierarchy.add_layer(SettingsLayer::UserGlobal, settings.clone());

        assert!(hierarchy.has_layer(SettingsLayer::UserGlobal));
        assert_eq!(
            hierarchy.get_layer(SettingsLayer::UserGlobal),
            Some(&settings)
        );
    }

    #[test]
    fn test_remove_layer() {
        let mut hierarchy = SettingsHierarchy::new();
        hierarchy.add_layer(SettingsLayer::UserGlobal, Settings::new().with_timeout(60));

        let removed = hierarchy.remove_layer(SettingsLayer::UserGlobal);
        assert!(removed.is_some());
        assert!(!hierarchy.has_layer(SettingsLayer::UserGlobal));
    }

    #[test]
    fn test_merge_respects_priority() {
        let mut hierarchy = SettingsHierarchy::new();

        hierarchy.add_layer(
            SettingsLayer::UserGlobal,
            Settings::new().with_model("claude-1".to_string()),
        );

        hierarchy.add_layer(
            SettingsLayer::ProjectLocal,
            Settings::new().with_model("claude-2".to_string()),
        );

        let merged = hierarchy.merge();

        // ProjectLocal has higher priority
        assert_eq!(merged.model, Some("claude-2".to_string()));
    }

    #[test]
    fn test_merge_partial_override() {
        let mut hierarchy = SettingsHierarchy::new();

        hierarchy.add_layer(
            SettingsLayer::UserGlobal,
            Settings::new()
                .with_model("claude-1".to_string())
                .with_timeout(60),
        );

        hierarchy.add_layer(
            SettingsLayer::ProjectLocal,
            Settings::new().with_timeout(90), // Only timeout, keeps model from UserGlobal
        );

        let merged = hierarchy.merge();

        assert_eq!(merged.model, Some("claude-1".to_string()));
        assert_eq!(merged.timeout_secs, Some(90));
    }

    #[test]
    fn test_merge_accumulates_permissions() {
        use crate::settings::types::{PermissionMode, ToolPermission};

        let mut hierarchy = SettingsHierarchy::new();

        let bash_perm = ToolPermission {
            mode: PermissionMode::Allow,
            patterns: vec!["ls".to_string()],
        };

        hierarchy.add_layer(
            SettingsLayer::UserGlobal,
            Settings::new().with_permission("bash".to_string(), bash_perm),
        );

        let edit_perm = ToolPermission {
            mode: PermissionMode::Ask,
            patterns: vec![],
        };

        hierarchy.add_layer(
            SettingsLayer::ProjectLocal,
            Settings::new().with_permission("edit".to_string(), edit_perm),
        );

        let merged = hierarchy.merge();

        assert!(merged.permissions.contains_key("bash"));
        assert!(merged.permissions.contains_key("edit"));
    }

    #[test]
    fn test_merge_environment_variables() {
        let mut hierarchy = SettingsHierarchy::new();

        hierarchy.add_layer(
            SettingsLayer::UserGlobal,
            Settings::new().with_env_var("API_KEY".to_string(), "key1".to_string()),
        );

        hierarchy.add_layer(
            SettingsLayer::ProjectLocal,
            Settings::new().with_env_var("DEBUG".to_string(), "true".to_string()),
        );

        let merged = hierarchy.merge();

        assert_eq!(merged.env_vars.get("API_KEY"), Some(&"key1".to_string()));
        assert_eq!(merged.env_vars.get("DEBUG"), Some(&"true".to_string()));
    }

    #[test]
    fn test_merge_env_var_override() {
        let mut hierarchy = SettingsHierarchy::new();

        hierarchy.add_layer(
            SettingsLayer::UserGlobal,
            Settings::new().with_env_var("API_KEY".to_string(), "key1".to_string()),
        );

        hierarchy.add_layer(
            SettingsLayer::ProjectLocal,
            Settings::new().with_env_var("API_KEY".to_string(), "key2".to_string()),
        );

        let merged = hierarchy.merge();

        assert_eq!(merged.env_vars.get("API_KEY"), Some(&"key2".to_string()));
    }

    #[test]
    fn test_bypass_is_sticky() {
        let mut hierarchy = SettingsHierarchy::new();

        hierarchy.add_layer(SettingsLayer::UserGlobal, Settings::new());
        hierarchy.add_layer(
            SettingsLayer::ProjectLocal,
            Settings::new().disable_bypass(),
        );

        let merged = hierarchy.merge();

        assert!(merged.disable_bypass_permissions);
    }

    #[test]
    fn test_enterprise_managed_highest_priority() {
        let mut hierarchy = SettingsHierarchy::new();

        hierarchy.add_layer(
            SettingsLayer::CommandLine,
            Settings::new().with_model("user-model".to_string()),
        );

        hierarchy.add_layer(
            SettingsLayer::EnterpriseManaged,
            Settings::new().with_model("enterprise-model".to_string()),
        );

        let merged = hierarchy.merge();

        assert_eq!(merged.model, Some("enterprise-model".to_string()));
    }

    #[test]
    fn test_active_layers() {
        let mut hierarchy = SettingsHierarchy::new();

        hierarchy.add_layer(SettingsLayer::UserGlobal, Settings::new());
        hierarchy.add_layer(SettingsLayer::CommandLine, Settings::new());

        let active = hierarchy.active_layers();

        assert_eq!(active.len(), 2);
        assert!(active[0] < active[1]); // Sorted by priority
    }
}
