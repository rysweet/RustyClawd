//! Plugin Hooks Integration - Registers plugin hooks with the hooks system
//!
//! Provides integration between plugin-defined hooks and the CLI hooks system.

use std::path::PathBuf;

use crate::hooks::registry::HookRegistry;
use crate::hooks::types::{Hook, HookConfig, HookEvent, HookMatcher, HookType};
use crate::plugins::manifest::HookDefinition;

/// Plugin hooks integrator
pub struct PluginHooksIntegrator {
    plugin_id: String,
    plugin_path: PathBuf,
}

impl PluginHooksIntegrator {
    /// Create new integrator for a plugin
    pub fn new(plugin_id: String, plugin_path: PathBuf) -> Self {
        Self {
            plugin_id,
            plugin_path,
        }
    }

    /// Register plugin hooks with the hooks system
    pub fn register_hooks(
        &self,
        hook_definitions: &[HookDefinition],
        registry: &mut HookRegistry,
    ) -> Result<(), String> {
        for hook_def in hook_definitions {
            self.register_hook(hook_def, registry)?;
        }
        Ok(())
    }

    /// Register a single hook
    fn register_hook(
        &self,
        hook_def: &HookDefinition,
        registry: &mut HookRegistry,
    ) -> Result<(), String> {
        // Parse event type
        let event = self.parse_event(&hook_def.event)?;

        // Resolve handler path
        let handler_path = self.plugin_path.join(&hook_def.handler);
        if !handler_path.exists() {
            return Err(format!(
                "Hook handler not found: {} (plugin: {})",
                hook_def.handler, self.plugin_id
            ));
        }

        // Determine hook type based on file extension
        let hook_type = if handler_path.extension().and_then(|s| s.to_str()) == Some("js")
            || handler_path.extension().and_then(|s| s.to_str()) == Some("sh")
        {
            HookType::Command
        } else {
            HookType::Prompt
        };

        // Create command to execute the handler
        let command = match hook_type {
            HookType::Command => {
                // For JS files, use node; for sh files, use bash
                if handler_path.extension().and_then(|s| s.to_str()) == Some("js") {
                    format!("node {}", handler_path.display())
                } else {
                    format!("bash {}", handler_path.display())
                }
            }
            HookType::Prompt => {
                // For prompt-based hooks, read the file content
                std::fs::read_to_string(&handler_path)
                    .map_err(|e| format!("Failed to read hook prompt: {}", e))?
            }
        };

        // Create hook configuration
        let hook = match hook_type {
            HookType::Command => Hook::command(command, Some(60000)), // 60s timeout
            HookType::Prompt => Hook::prompt(Some(command), Some(60000)),
        };

        // Parse matcher from hook definition
        // Default to "*" if not specified (backward compatibility)
        let matcher_str = hook_def.matcher.as_deref().unwrap_or("*");

        // Determine if it's a regex pattern (contains regex metacharacters)
        // Simple heuristic: if it contains |, ^, $, [, ], (, ), or other regex chars, treat as regex
        let matcher = if matcher_str.contains('|')
            || matcher_str.contains('^')
            || matcher_str.contains('$')
            || matcher_str.contains('[')
            || matcher_str.contains('(')
        {
            HookMatcher::Regex(matcher_str.to_string())
        } else {
            HookMatcher::Exact(matcher_str.to_string())
        };

        // Create hook config with parsed matcher
        let hook_config = HookConfig {
            matcher,
            hooks: vec![hook],
        };

        // Register with the hooks system
        registry.register_hook(event, hook_config);

        Ok(())
    }

    /// Parse event string into HookEvent enum
    fn parse_event(&self, event_str: &str) -> Result<HookEvent, String> {
        match event_str {
            "onLoad" | "SessionStart" => Ok(HookEvent::SessionStart),
            "onUnload" | "SessionEnd" => Ok(HookEvent::SessionEnd),
            "PreToolUse" => Ok(HookEvent::PreToolUse),
            "PostToolUse" => Ok(HookEvent::PostToolUse),
            "UserPromptSubmit" => Ok(HookEvent::UserPromptSubmit),
            "Stop" => Ok(HookEvent::Stop),
            "SubagentStop" => Ok(HookEvent::SubagentStop),
            "Notification" => Ok(HookEvent::Notification),
            "PreCompact" => Ok(HookEvent::PreCompact),
            _ => Err(format!("Unknown hook event: {}", event_str)),
        }
    }
}

/// Register all hooks from multiple plugins
pub fn register_plugin_hooks(
    plugins: &[(String, PathBuf, Vec<HookDefinition>)],
    registry: &mut HookRegistry,
) -> Result<(), String> {
    for (plugin_id, plugin_path, hook_defs) in plugins {
        let integrator = PluginHooksIntegrator::new(plugin_id.clone(), plugin_path.clone());
        integrator.register_hooks(hook_defs, registry)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::types::HookContext;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_event_variations() {
        let integrator = PluginHooksIntegrator::new("test".to_string(), PathBuf::from("/tmp/test"));

        assert!(matches!(
            integrator.parse_event("onLoad").unwrap(),
            HookEvent::SessionStart
        ));
        assert!(matches!(
            integrator.parse_event("SessionStart").unwrap(),
            HookEvent::SessionStart
        ));
        assert!(matches!(
            integrator.parse_event("PreToolUse").unwrap(),
            HookEvent::PreToolUse
        ));
        assert!(integrator.parse_event("InvalidEvent").is_err());
    }

    #[test]
    fn test_register_hooks_missing_handler() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().to_path_buf();

        let integrator = PluginHooksIntegrator::new("test".to_string(), plugin_path);

        let hook_def = HookDefinition {
            event: "onLoad".to_string(),
            handler: "nonexistent.js".to_string(),
            matcher: None,
        };

        let mut registry = HookRegistry::new();
        let result = integrator.register_hook(&hook_def, &mut registry);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_register_js_hook() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().to_path_buf();

        // Create a JS handler file
        fs::write(
            plugin_path.join("handler.js"),
            "console.log('Hook executed');",
        )
        .unwrap();

        let integrator = PluginHooksIntegrator::new("test".to_string(), plugin_path);

        let hook_def = HookDefinition {
            event: "PreToolUse".to_string(),
            handler: "handler.js".to_string(),
            matcher: None,
        };

        let mut registry = HookRegistry::new();
        let result = integrator.register_hook(&hook_def, &mut registry);

        assert!(result.is_ok());
    }

    #[test]
    fn test_register_multiple_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().to_path_buf();

        // Create multiple handler files
        fs::write(plugin_path.join("onload.js"), "console.log('Load');").unwrap();
        fs::write(plugin_path.join("pretool.sh"), "echo 'PreTool'").unwrap();

        let integrator = PluginHooksIntegrator::new("test".to_string(), plugin_path);

        let hook_defs = vec![
            HookDefinition {
                event: "onLoad".to_string(),
                handler: "onload.js".to_string(),
                matcher: None,
            },
            HookDefinition {
                event: "PreToolUse".to_string(),
                handler: "pretool.sh".to_string(),
                matcher: None,
            },
        ];

        let mut registry = HookRegistry::new();
        let result = integrator.register_hooks(&hook_defs, &mut registry);

        assert!(result.is_ok());
    }

    #[test]
    fn test_register_plugin_hooks_batch() {
        let temp_dir = TempDir::new().unwrap();

        // Create plugin 1
        let plugin1_path = temp_dir.path().join("plugin1");
        fs::create_dir(&plugin1_path).unwrap();
        fs::write(plugin1_path.join("hook1.js"), "console.log('P1');").unwrap();

        // Create plugin 2
        let plugin2_path = temp_dir.path().join("plugin2");
        fs::create_dir(&plugin2_path).unwrap();
        fs::write(plugin2_path.join("hook2.js"), "console.log('P2');").unwrap();

        let plugins = vec![
            (
                "plugin1".to_string(),
                plugin1_path,
                vec![HookDefinition {
                    event: "onLoad".to_string(),
                    handler: "hook1.js".to_string(),
                    matcher: None, // Use default wildcard
                }],
            ),
            (
                "plugin2".to_string(),
                plugin2_path,
                vec![HookDefinition {
                    event: "PreToolUse".to_string(),
                    handler: "hook2.js".to_string(),
                    matcher: None, // Use default wildcard
                }],
            ),
        ];

        let mut registry = HookRegistry::new();
        let result = register_plugin_hooks(&plugins, &mut registry);

        assert!(result.is_ok());
    }

    #[test]
    fn test_register_hook_with_custom_matcher() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().to_path_buf();

        // Create a JS handler file
        fs::write(
            plugin_path.join("handler.js"),
            "console.log('Hook executed');",
        )
        .unwrap();

        let integrator = PluginHooksIntegrator::new("test".to_string(), plugin_path);

        let hook_def = HookDefinition {
            event: "PreToolUse".to_string(),
            handler: "handler.js".to_string(),
            matcher: Some("Write".to_string()), // Custom matcher for Write tool only
        };

        let mut registry = HookRegistry::new();
        let result = integrator.register_hook(&hook_def, &mut registry);

        assert!(result.is_ok());

        // Verify the hook was registered with the correct matcher using HookContext
        let context_write = HookContext::for_tool(
            "test-session".to_string(),
            "/tmp/transcript".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "auto".to_string(),
            HookEvent::PreToolUse,
            "Write".to_string(),
            None,
        );

        let context_read = HookContext::for_tool(
            "test-session".to_string(),
            "/tmp/transcript".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "auto".to_string(),
            HookEvent::PreToolUse,
            "Read".to_string(),
            None,
        );

        let hooks_write = registry.get_hooks_for_event(&HookEvent::PreToolUse, &context_write);
        assert!(!hooks_write.is_empty(), "Hook should match 'Write' tool");

        let hooks_read = registry.get_hooks_for_event(&HookEvent::PreToolUse, &context_read);
        assert!(hooks_read.is_empty(), "Hook should NOT match 'Read' tool");
    }

    #[test]
    fn test_register_hook_with_regex_matcher() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().to_path_buf();

        // Create a JS handler file
        fs::write(
            plugin_path.join("handler.js"),
            "console.log('Hook executed');",
        )
        .unwrap();

        let integrator = PluginHooksIntegrator::new("test".to_string(), plugin_path);

        let hook_def = HookDefinition {
            event: "PreToolUse".to_string(),
            handler: "handler.js".to_string(),
            matcher: Some("Write|Edit|Read".to_string()), // Regex pattern
        };

        let mut registry = HookRegistry::new();
        let result = integrator.register_hook(&hook_def, &mut registry);

        assert!(result.is_ok());

        // Create contexts for different tools
        let context_write = HookContext::for_tool(
            "test-session".to_string(),
            "/tmp/transcript".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "auto".to_string(),
            HookEvent::PreToolUse,
            "Write".to_string(),
            None,
        );

        let context_read = HookContext::for_tool(
            "test-session".to_string(),
            "/tmp/transcript".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "auto".to_string(),
            HookEvent::PreToolUse,
            "Read".to_string(),
            None,
        );

        let context_bash = HookContext::for_tool(
            "test-session".to_string(),
            "/tmp/transcript".to_string(),
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            "auto".to_string(),
            HookEvent::PreToolUse,
            "Bash".to_string(),
            None,
        );

        // Verify the hook matches multiple tools via regex
        let hooks_write = registry.get_hooks_for_event(&HookEvent::PreToolUse, &context_write);
        assert!(
            !hooks_write.is_empty(),
            "Hook should match 'Write' via regex"
        );

        let hooks_read = registry.get_hooks_for_event(&HookEvent::PreToolUse, &context_read);
        assert!(!hooks_read.is_empty(), "Hook should match 'Read' via regex");

        let hooks_bash = registry.get_hooks_for_event(&HookEvent::PreToolUse, &context_bash);
        assert!(
            hooks_bash.is_empty(),
            "Hook should NOT match 'Bash' (not in regex)"
        );
    }
}
