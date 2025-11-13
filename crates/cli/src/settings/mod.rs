/// Settings and configuration system for Claude Code
///
/// Implements a 5-tier configuration hierarchy with proper precedence:
/// 1. Default - Built-in defaults
/// 2. User Global - User's personal configuration (~/.claude/config)
/// 3. Project Shared - Project-wide configuration (.claude/config)
/// 4. Project Local - Local project overrides (.claude/config.local)
/// 5. Command Line - Runtime CLI flags and CLAUDE_* environment variables
/// 6. Enterprise Managed - System administrator enforced settings
///
/// # Example
///
/// ```ignore
/// use settings::{SettingsLoader, SettingsHierarchy};
///
/// let loader = SettingsLoader::new();
/// let hierarchy = loader.load_hierarchy()?;
/// let merged = hierarchy.merge();
///
/// // merged now contains the effective configuration
/// if let Some(model) = merged.model {
///     println!("Using model: {}", model);
/// }
/// ```

pub mod types;
pub mod validation;
pub mod hierarchy;
pub mod loader;

// Re-export public API
pub use types::Settings;
pub use loader::SettingsLoader;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use super::hierarchy::SettingsHierarchy;
    use super::types::{PermissionMode, SettingsLayer};

    #[test]
    fn test_full_hierarchy_workflow() {
        // Create a test hierarchy
        let mut hierarchy = SettingsHierarchy::new();

        // Add default layer
        hierarchy.add_layer(SettingsLayer::Default, Settings::default());

        // Add user settings
        hierarchy.add_layer(
            SettingsLayer::UserGlobal,
            Settings::new()
                .with_model("claude-2".to_string())
                .with_timeout(60),
        );

        // Add project settings that override some user settings
        hierarchy.add_layer(
            SettingsLayer::ProjectLocal,
            Settings::new()
                .with_model("claude-3".to_string())
                .with_env_var("PROJECT_ID".to_string(), "proj-123".to_string()),
        );

        // Merge all layers
        let merged = hierarchy.merge();

        // Verify effective configuration
        assert_eq!(merged.model, Some("claude-3".to_string())); // From ProjectLocal
        assert_eq!(merged.timeout_secs, Some(60)); // From UserGlobal
        assert_eq!(
            merged.env_vars.get("PROJECT_ID"),
            Some(&"proj-123".to_string())
        );

        // Verify it validates
        assert!(merged.validate().is_ok());
    }

    #[test]
    fn test_enterprise_override_precedence() {
        let mut hierarchy = SettingsHierarchy::new();

        // User tries to set different model
        hierarchy.add_layer(
            SettingsLayer::CommandLine,
            Settings::new().with_model("user-choice".to_string()),
        );

        // Enterprise enforces a model
        hierarchy.add_layer(
            SettingsLayer::EnterpriseManaged,
            Settings::new()
                .with_model("enterprise-enforced".to_string())
                .disable_bypass(),
        );

        let merged = hierarchy.merge();

        // Enterprise wins
        assert_eq!(merged.model, Some("enterprise-enforced".to_string()));
        assert!(merged.disable_bypass_permissions);
    }

    #[test]
    fn test_env_variable_parsing_and_override() {
        let mut overrides = std::collections::HashMap::new();
        overrides.insert("model".to_string(), "claude-3-opus".to_string());
        overrides.insert("timeout_secs".to_string(), "180".to_string());

        let settings = SettingsLoader::parse_env_overrides(&overrides);

        assert_eq!(settings.model, Some("claude-3-opus".to_string()));
        assert_eq!(settings.timeout_secs, Some(180));
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_permissions_accumulate_across_layers() {
        use super::types::{ToolPermission, PermissionMode, SettingsLayer};
        use super::hierarchy::SettingsHierarchy;

        let mut hierarchy = SettingsHierarchy::new();

        // User global: allows bash
        let bash_perm = ToolPermission::new(PermissionMode::Allow, vec!["ls".to_string()]);
        hierarchy.add_layer(
            SettingsLayer::UserGlobal,
            Settings::new().with_permission("bash".to_string(), bash_perm),
        );

        // Project adds edit permissions
        let edit_perm = ToolPermission::new(PermissionMode::Ask, vec![]);
        hierarchy.add_layer(
            SettingsLayer::ProjectLocal,
            Settings::new().with_permission("edit".to_string(), edit_perm),
        );

        let merged = hierarchy.merge();

        // Both permissions should be present
        assert_eq!(merged.permissions.len(), 2);
        assert!(merged.permissions.contains_key("bash"));
        assert!(merged.permissions.contains_key("edit"));
    }
}
