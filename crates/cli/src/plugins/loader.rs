//! Plugin Loading - File and Metadata Loading
//!
//! Handles loading plugin files, validating references, and tracking load state.

use std::collections::HashMap;

use crate::plugins::discovery::{PluginLoadStatus, PluginMetadata};
use crate::plugins::manifest::validate_references;

/// Plugin loader managing plugin lifecycle
pub struct PluginLoader {
    plugins: HashMap<String, PluginMetadata>,
}

impl PluginLoader {
    /// Create new plugin loader
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Register a discovered plugin
    pub fn register(&mut self, metadata: PluginMetadata) {
        self.plugins.insert(metadata.id.clone(), metadata);
    }

    /// Load plugin - validate files and update status
    ///
    /// Performs complete validation:
    /// - Plugin directory exists
    /// - Manifest exists and is valid
    /// - Entry point exists
    /// - All command/skill files exist
    pub fn load(&mut self, plugin_id: &str) -> Result<(), String> {
        let metadata = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| "Plugin not found".to_string())?;

        // Check plugin directory exists
        if !metadata.path.exists() {
            metadata.load_status = PluginLoadStatus::Failed("Directory not found".to_string());
            return Err("Plugin directory not found".to_string());
        }

        // Check manifest exists
        let manifest_path = metadata.path.join("plugin.json");
        if !manifest_path.exists() {
            metadata.load_status = PluginLoadStatus::Failed("No manifest".to_string());
            return Err("Missing plugin.json manifest".to_string());
        }

        // Check entry point exists
        let entry_point = metadata.path.join(&metadata.manifest.main);
        if !entry_point.exists() {
            metadata.load_status = PluginLoadStatus::Failed("No entry point".to_string());
            return Err(format!("Missing entry point: {}", metadata.manifest.main));
        }

        // Validate all file references
        if let Err(errors) = validate_references(&metadata.manifest, &metadata.path) {
            metadata.load_status =
                PluginLoadStatus::Failed(format!("Invalid references: {}", errors.join(", ")));
            return Err(format!("File reference validation failed: {}", errors.join(", ")));
        }

        // Mark as loaded
        metadata.load_status = PluginLoadStatus::Loaded;
        Ok(())
    }

    /// Initialize plugin after loading
    ///
    /// Must be called after load() succeeds.
    /// Transitions plugin to Initialized status.
    pub fn initialize(&mut self, plugin_id: &str) -> Result<(), String> {
        let metadata = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| "Plugin not found".to_string())?;

        if metadata.load_status != PluginLoadStatus::Loaded {
            return Err(format!(
                "Cannot initialize: plugin is in {:?} state",
                metadata.load_status
            ));
        }

        metadata.load_status = PluginLoadStatus::Initialized;
        Ok(())
    }

    /// Check if plugin is loaded
    pub fn is_loaded(&self, plugin_id: &str) -> bool {
        self.plugins
            .get(plugin_id)
            .map(|p| p.load_status == PluginLoadStatus::Loaded)
            .unwrap_or(false)
    }

    /// Get plugin metadata
    pub fn get(&self, plugin_id: &str) -> Option<PluginMetadata> {
        self.plugins.get(plugin_id).cloned()
    }

    /// Get all loaded plugins
    pub fn loaded_plugins(&self) -> Vec<PluginMetadata> {
        self.plugins
            .values()
            .filter(|p| p.load_status == PluginLoadStatus::Loaded)
            .cloned()
            .collect()
    }

    /// Get all plugins
    pub fn all_plugins(&self) -> Vec<PluginMetadata> {
        self.plugins.values().cloned().collect()
    }

    /// Disable plugin (prevent execution)
    pub fn disable(&mut self, plugin_id: &str) -> Result<(), String> {
        self.plugins
            .get_mut(plugin_id)
            .ok_or_else(|| "Plugin not found".to_string())?
            .enabled = false;
        Ok(())
    }

    /// Enable plugin (allow execution)
    pub fn enable(&mut self, plugin_id: &str) -> Result<(), String> {
        self.plugins
            .get_mut(plugin_id)
            .ok_or_else(|| "Plugin not found".to_string())?
            .enabled = true;
        Ok(())
    }

    /// Unload plugin
    pub fn unload(&mut self, plugin_id: &str) -> Result<(), String> {
        self.plugins
            .remove(plugin_id)
            .ok_or_else(|| "Plugin not found".to_string())?;
        Ok(())
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manifest::PluginManifest;
    use std::collections::HashMap;

    #[test]
    fn test_load_nonexistent_plugin() {
        let mut loader = PluginLoader::new();
        let result = loader.load("com.nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_register_and_get() {
        let manifest = PluginManifest {
            id: "com.test.basic".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            main: "index.js".to_string(),
            commands: vec![],
            skills: vec![],
            hooks: vec![],
            dependencies: HashMap::new(),
            config_schema: serde_json::json!({}),
        };

        let metadata = PluginMetadata {
            id: "com.test.basic".to_string(),
            path: std::env::temp_dir().join("test-plugin"),
            manifest,
            enabled: true,
            load_status: PluginLoadStatus::Discovered,
        };

        let mut loader = PluginLoader::new();
        loader.register(metadata.clone());

        let retrieved = loader.get("com.test.basic");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "com.test.basic");
    }
}
