//! Plugin Execution - Running Plugin Commands and Skills
//!
//! Executes plugin commands and skills with argument validation and result handling.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

use crate::plugins::discovery::PluginMetadata;
use crate::plugins::subprocess::SubprocessExecutor;

/// Plugin execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Output from plugin execution
    pub output: String,
    /// Any errors encountered
    pub errors: Vec<String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

/// Plugin executor for running commands and skills
pub struct PluginExecutor {
    plugins: HashMap<String, PluginMetadata>,
}

impl PluginExecutor {
    /// Create new plugin executor
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Register a plugin for execution
    pub fn register(&mut self, metadata: PluginMetadata) {
        self.plugins.insert(metadata.id.clone(), metadata);
    }

    /// Execute a plugin command
    ///
    /// # Arguments
    /// * `plugin_id` - The plugin identifier
    /// * `command_name` - The command name to execute
    /// * `args` - JSON arguments for the command
    pub fn execute_command(
        &self,
        plugin_id: &str,
        command_name: &str,
        _args: serde_json::Value,
    ) -> Result<PluginExecutionResult, String> {
        // Get plugin
        let plugin = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

        // Check plugin is enabled
        if !plugin.enabled {
            return Err("Plugin is disabled".to_string());
        }

        // Find command definition
        let command = plugin
            .manifest
            .commands
            .iter()
            .find(|c| c.name == command_name)
            .ok_or_else(|| format!("Command not found: {}", command_name))?;

        // Build the full path to the command script
        let command_path = plugin.path.join(&command.path);

        // Detect the interpreter for the command script
        let interpreter = SubprocessExecutor::detect_interpreter(&command_path)
            .map_err(|e| format!("Cannot determine interpreter for '{}': {}", command.path, e))?;

        // Serialize args to pass as a JSON string argument
        let args_str = _args.to_string();

        // Execute via real subprocess with 30-second timeout
        let result = SubprocessExecutor::execute(
            &interpreter,
            &[command_path.to_str().unwrap_or(""), &args_str],
            30_000,
        )
        .map_err(|e| format!("Failed to execute command '{}': {}", command.name, e))?;

        Ok(PluginExecutionResult {
            success: result.success,
            output: result.output,
            errors: if result.stderr.is_empty() {
                vec![]
            } else {
                vec![result.stderr]
            },
            duration_ms: result.duration_ms,
        })
    }

    /// Execute a plugin skill
    ///
    /// # Arguments
    /// * `plugin_id` - The plugin identifier
    /// * `skill_id` - The skill identifier to execute
    pub fn execute_skill(
        &self,
        plugin_id: &str,
        skill_id: &str,
    ) -> Result<PluginExecutionResult, String> {
        let start = Instant::now();

        // Get plugin
        let plugin = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| format!("Plugin not found: {}", plugin_id))?;

        // Check plugin is enabled
        if !plugin.enabled {
            return Err("Plugin is disabled".to_string());
        }

        // Find skill definition
        let skill = plugin
            .manifest
            .skills
            .iter()
            .find(|s| s.id == skill_id)
            .ok_or_else(|| format!("Skill not found: {}", skill_id))?;

        // Load and execute skill
        let duration = start.elapsed().as_millis() as u64;

        // Load skill content from file
        let skill_path = plugin.path.join(&skill.path);
        let skill_content = std::fs::read_to_string(&skill_path)
            .map_err(|e| format!("Failed to load skill from '{}': {}", skill.path, e))?;

        // Return skill execution result with loaded content
        Ok(PluginExecutionResult {
            success: true,
            output: format!(
                "Skill '{}' loaded successfully\n{}",
                skill.id, skill_content
            ),
            errors: vec![],
            duration_ms: duration,
        })
    }

    /// Get plugin by ID
    pub fn get_plugin(&self, plugin_id: &str) -> Option<PluginMetadata> {
        self.plugins.get(plugin_id).cloned()
    }

    /// Get all registered plugins
    pub fn all_plugins(&self) -> Vec<PluginMetadata> {
        self.plugins.values().cloned().collect()
    }

    /// Get all commands from a plugin
    pub fn get_commands(&self, plugin_id: &str) -> Result<Vec<String>, String> {
        let plugin = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| "Plugin not found".to_string())?;

        Ok(plugin
            .manifest
            .commands
            .iter()
            .map(|c| c.name.clone())
            .collect())
    }

    /// Get all skills from a plugin
    pub fn get_skills(&self, plugin_id: &str) -> Result<Vec<String>, String> {
        let plugin = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| "Plugin not found".to_string())?;

        Ok(plugin
            .manifest
            .skills
            .iter()
            .map(|s| s.id.clone())
            .collect())
    }
}

impl Default for PluginExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Validator for plugin execution contracts
pub struct PluginValidator;

impl PluginValidator {
    /// Validate plugin manifest structure
    pub fn validate_manifest(
        manifest: &crate::plugins::manifest::PluginManifest,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if manifest.id.is_empty() {
            errors.push("ID required".to_string());
        }
        if manifest.name.is_empty() {
            errors.push("Name required".to_string());
        }
        if manifest.version.is_empty() {
            errors.push("Version required".to_string());
        }
        if manifest.main.is_empty() {
            errors.push("Main required".to_string());
        }
        if manifest.author.is_empty() {
            errors.push("Author required".to_string());
        }
        if manifest.license.is_empty() {
            errors.push("License required".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate execution result consistency
    pub fn validate_result(result: &PluginExecutionResult) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Success state must match error list
        if result.success && !result.errors.is_empty() {
            errors.push("Success cannot have errors".to_string());
        }

        if !result.success && result.errors.is_empty() {
            errors.push("Failure must have at least one error".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_command_plugin_not_found() {
        let executor = PluginExecutor::new();
        let result = executor.execute_command("com.nonexistent", "test", serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_result_success() {
        let result = PluginExecutionResult {
            success: true,
            output: "Output".to_string(),
            errors: vec![],
            duration_ms: 100,
        };

        assert!(PluginValidator::validate_result(&result).is_ok());
    }

    #[test]
    fn test_validate_result_inconsistent() {
        let result = PluginExecutionResult {
            success: true,
            output: "Output".to_string(),
            errors: vec!["Error".to_string()],
            duration_ms: 100,
        };

        assert!(PluginValidator::validate_result(&result).is_err());
    }
}
