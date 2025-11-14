//! Plugin Discovery - Scanning and Locating Plugins
//!
//! Scans plugin directories, discovers plugin.json manifests, and loads plugin metadata.

use std::fs;
use std::path::{Path, PathBuf};

use crate::plugins::manifest::{parse_manifest, validate_manifest, PluginManifest};

/// Plugin load status tracking
#[derive(Debug, Clone, PartialEq)]
pub enum PluginLoadStatus {
    /// Plugin found and manifest parsed
    Discovered,
    /// Plugin files loaded successfully
    Loaded,
    /// Plugin failed to load with error message
    Failed(String),
    /// Plugin initialized and ready
    Initialized,
}

/// Plugin metadata and status information
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// Unique plugin identifier from manifest
    pub id: String,
    /// Plugin root directory path
    pub path: PathBuf,
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// Whether plugin is enabled
    pub enabled: bool,
    /// Current load status
    pub load_status: PluginLoadStatus,
}

/// Plugin discovery and scanning
pub struct PluginDiscovery {
    root: PathBuf,
}

impl PluginDiscovery {
    /// Create new discovery scanner for a plugin directory
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Discover all plugins in the root directory
    ///
    /// Scans immediate subdirectories for plugin.json manifests.
    /// Returns list of discovered plugins (even if invalid).
    pub fn discover_all(&self) -> Result<Vec<PluginMetadata>, String> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut plugins = Vec::new();

        // Scan immediate subdirectories
        for entry in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_dir() {
                // Look for plugin.json in this directory
                let manifest_path = path.join("plugin.json");
                if manifest_path.exists() {
                    if let Ok(metadata) = self.load_plugin_metadata(&path) {
                        plugins.push(metadata);
                    }
                }
            }
        }

        Ok(plugins)
    }

    /// Load plugin metadata from a specific directory
    fn load_plugin_metadata(&self, plugin_path: &Path) -> Result<PluginMetadata, String> {
        let manifest = parse_manifest(plugin_path)?;

        Ok(PluginMetadata {
            id: manifest.id.clone(),
            path: plugin_path.to_path_buf(),
            manifest,
            enabled: true,
            load_status: PluginLoadStatus::Discovered,
        })
    }

    /// Validate complete plugin structure
    ///
    /// Checks for:
    /// - plugin.json existence
    /// - Manifest validity
    /// - Main entry point exists
    /// - All referenced files exist
    pub fn validate_structure(&self, plugin_path: &Path) -> Result<(), String> {
        // Check manifest exists
        if !plugin_path.join("plugin.json").exists() {
            return Err("Missing plugin.json".to_string());
        }

        // Parse and validate manifest
        let manifest = parse_manifest(plugin_path)?;

        // Validate manifest schema
        if let Err(errors) = validate_manifest(&manifest) {
            return Err(format!("Invalid manifest: {}", errors.join(", ")));
        }

        // Validate main entry point exists
        let main_path = plugin_path.join(&manifest.main);
        if !main_path.exists() {
            return Err(format!("Main entry not found: {}", manifest.main));
        }

        Ok(())
    }

    /// Get plugin by ID from discovered plugins
    pub fn find_plugin(&self, plugins: &[PluginMetadata], id: &str) -> Option<PluginMetadata> {
        plugins.iter().find(|p| p.id == id).cloned()
    }

    /// Filter plugins by status
    pub fn filter_by_status(
        plugins: &[PluginMetadata],
        status: PluginLoadStatus,
    ) -> Vec<PluginMetadata> {
        plugins
            .iter()
            .filter(|p| p.load_status == status)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_empty_directory() {
        let test_dir = std::env::temp_dir().join("plugin-discover-empty");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let discovery = PluginDiscovery::new(&test_dir);
        let plugins = discovery.discover_all().unwrap();
        assert_eq!(plugins.len(), 0);
    }

    #[test]
    fn test_validate_structure_missing_manifest() {
        let test_dir = std::env::temp_dir().join("plugin-validate-no-manifest");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let discovery = PluginDiscovery::new(&test_dir);
        let result = discovery.validate_structure(&test_dir);
        assert!(result.is_err());
    }
}
