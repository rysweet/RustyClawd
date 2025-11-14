//! Plugin System Test Suite
//!
//! Comprehensive tests for Claude Code plugins following TDD approach
//! Tests cover: discovery, loading, execution, and API contract validation
//!
//! Testing Pyramid:
//! - Unit Tests (60%): Individual component functionality
//! - Integration Tests (30%): Plugin system interactions
//! - E2E Tests (10%): Full plugin lifecycle

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// =============================================================================
// TEST HELPERS
// =============================================================================

/// Create a test temporary directory
fn create_test_dir(name: &str) -> PathBuf {
    let base = env::temp_dir().join(format!("plugin-tests-{}", name));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("Failed to create test dir");
    base
}

// =============================================================================
// PLUGIN SYSTEM TYPE DEFINITIONS
// =============================================================================

/// Plugin manifest structure (plugin.json)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub main: String,
    #[serde(default)]
    pub commands: Vec<CommandDefinition>,
    #[serde(default)]
    pub skills: Vec<SkillDefinition>,
    #[serde(default)]
    pub hooks: Vec<HookDefinition>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub config_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandDefinition {
    pub name: String,
    pub description: String,
    pub path: String,
    pub args_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookDefinition {
    pub event: String,
    pub handler: String,
}

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub id: String,
    pub path: PathBuf,
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub load_status: PluginLoadStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PluginLoadStatus {
    Discovered,
    Loaded,
    Failed(String),
    Initialized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExecutionResult {
    pub success: bool,
    pub output: String,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

// =============================================================================
// PLUGIN DISCOVERY - UNIT TESTS (60%)
// =============================================================================

mod discovery {
    use super::*;

    pub struct PluginDiscovery {
        root: PathBuf,
    }

    impl PluginDiscovery {
        pub fn new(root: impl AsRef<Path>) -> Self {
            Self {
                root: root.as_ref().to_path_buf(),
            }
        }

        pub fn discover_all(&self) -> Result<Vec<PluginMetadata>, String> {
            if !self.root.exists() {
                return Ok(Vec::new());
            }

            let mut plugins = Vec::new();
            for entry in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();

                if path.is_dir() {
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

        fn load_plugin_metadata(&self, plugin_path: &Path) -> Result<PluginMetadata, String> {
            let manifest_path = plugin_path.join("plugin.json");
            let manifest_content = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;

            let manifest: PluginManifest =
                serde_json::from_str(&manifest_content).map_err(|e| e.to_string())?;

            Ok(PluginMetadata {
                id: manifest.id.clone(),
                path: plugin_path.to_path_buf(),
                manifest,
                enabled: true,
                load_status: PluginLoadStatus::Discovered,
            })
        }

        pub fn validate_structure(&self, plugin_path: &Path) -> Result<(), String> {
            if !plugin_path.join("plugin.json").exists() {
                return Err("Missing plugin.json".to_string());
            }

            let manifest_content =
                fs::read_to_string(plugin_path.join("plugin.json")).map_err(|e| e.to_string())?;
            let manifest: PluginManifest =
                serde_json::from_str(&manifest_content).map_err(|e| e.to_string())?;

            let main_path = plugin_path.join(&manifest.main);
            if !main_path.exists() {
                return Err(format!("Main entry not found: {}", manifest.main));
            }

            Ok(())
        }
    }

    #[test]
    fn test_discover_empty_directory() {
        let test_dir = create_test_dir("discover_empty");
        let discovery = PluginDiscovery::new(&test_dir);
        let plugins = discovery.discover_all().unwrap();
        assert_eq!(plugins.len(), 0);
    }

    #[test]
    fn test_discover_single_plugin() {
        let test_dir = create_test_dir("discover_single");
        let plugin_dir = test_dir.join("test-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        let manifest = PluginManifest {
            id: "com.test.simple".to_string(),
            name: "Simple".to_string(),
            version: "0.1.0".to_string(),
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

        let manifest_path = plugin_dir.join("plugin.json");
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(plugin_dir.join("index.js"), "").unwrap();

        let discovery = PluginDiscovery::new(&test_dir);
        let plugins = discovery.discover_all().unwrap();

        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].id, "com.test.simple");
    }

    #[test]
    fn test_discover_multiple_plugins() {
        let test_dir = create_test_dir("discover_multiple");

        for i in 0..3 {
            let plugin_dir = test_dir.join(format!("plugin-{}", i));
            fs::create_dir(&plugin_dir).unwrap();

            let manifest = PluginManifest {
                id: format!("com.test.plugin-{}", i),
                name: format!("Plugin {}", i),
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

            let manifest_path = plugin_dir.join("plugin.json");
            fs::write(
                &manifest_path,
                serde_json::to_string_pretty(&manifest).unwrap(),
            )
            .unwrap();
            fs::write(plugin_dir.join("index.js"), "").unwrap();
        }

        let discovery = PluginDiscovery::new(&test_dir);
        let plugins = discovery.discover_all().unwrap();

        assert_eq!(plugins.len(), 3);
    }

    #[test]
    fn test_validate_plugin_success() {
        let test_dir = create_test_dir("validate_success");
        let plugin_dir = test_dir.join("valid-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        let manifest = PluginManifest {
            id: "com.test.valid".to_string(),
            name: "Valid".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            main: "main.js".to_string(),
            commands: vec![],
            skills: vec![],
            hooks: vec![],
            dependencies: HashMap::new(),
            config_schema: serde_json::json!({}),
        };

        fs::write(
            &plugin_dir.join("plugin.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(&plugin_dir.join("main.js"), "").unwrap();

        let discovery = PluginDiscovery::new(&test_dir);
        assert!(discovery.validate_structure(&plugin_dir).is_ok());
    }

    #[test]
    fn test_validate_plugin_missing_manifest() {
        let test_dir = create_test_dir("validate_no_manifest");
        let plugin_dir = test_dir.join("broken-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        let discovery = PluginDiscovery::new(&test_dir);
        let result = discovery.validate_structure(&plugin_dir);

        assert!(result.is_err());
    }

    #[test]
    fn test_validate_plugin_missing_entry_point() {
        let test_dir = create_test_dir("validate_no_entry");
        let plugin_dir = test_dir.join("broken-plugin");
        fs::create_dir(&plugin_dir).unwrap();

        let manifest = PluginManifest {
            id: "com.test.broken".to_string(),
            name: "Broken".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            main: "missing.js".to_string(),
            commands: vec![],
            skills: vec![],
            hooks: vec![],
            dependencies: HashMap::new(),
            config_schema: serde_json::json!({}),
        };

        fs::write(
            &plugin_dir.join("plugin.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let discovery = PluginDiscovery::new(&test_dir);
        let result = discovery.validate_structure(&plugin_dir);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Main entry"));
    }
}

// =============================================================================
// PLUGIN LOADING - UNIT TESTS (60%)
// =============================================================================

mod loading {
    use super::*;

    pub struct PluginLoader {
        plugins: HashMap<String, PluginMetadata>,
    }

    impl PluginLoader {
        pub fn new() -> Self {
            Self {
                plugins: HashMap::new(),
            }
        }

        pub fn register(&mut self, metadata: PluginMetadata) {
            self.plugins.insert(metadata.id.clone(), metadata);
        }

        pub fn load(&mut self, plugin_id: &str) -> Result<(), String> {
            let metadata = self
                .plugins
                .get_mut(plugin_id)
                .ok_or_else(|| "Plugin not found".to_string())?;

            if !metadata.path.exists() {
                metadata.load_status = PluginLoadStatus::Failed("Not found".to_string());
                return Err("Plugin directory not found".to_string());
            }

            let manifest_path = metadata.path.join("plugin.json");
            if !manifest_path.exists() {
                metadata.load_status = PluginLoadStatus::Failed("No manifest".to_string());
                return Err("Missing manifest".to_string());
            }

            let entry_point = metadata.path.join(&metadata.manifest.main);
            if !entry_point.exists() {
                metadata.load_status = PluginLoadStatus::Failed("No entry".to_string());
                return Err("Missing entry point".to_string());
            }

            for cmd in &metadata.manifest.commands {
                let cmd_path = metadata.path.join(&cmd.path);
                if !cmd_path.exists() {
                    return Err(format!("Command not found: {}", cmd.path));
                }
            }

            metadata.load_status = PluginLoadStatus::Loaded;
            Ok(())
        }

        pub fn initialize(&mut self, plugin_id: &str) -> Result<(), String> {
            let metadata = self
                .plugins
                .get_mut(plugin_id)
                .ok_or_else(|| "Plugin not found".to_string())?;

            if metadata.load_status != PluginLoadStatus::Loaded {
                return Err("Not loaded".to_string());
            }

            metadata.load_status = PluginLoadStatus::Initialized;
            Ok(())
        }

        pub fn is_loaded(&self, plugin_id: &str) -> bool {
            self.plugins
                .get(plugin_id)
                .map(|p| p.load_status == PluginLoadStatus::Loaded)
                .unwrap_or(false)
        }
    }

    #[test]
    fn test_load_valid_plugin() {
        let test_dir = create_test_dir("load_valid");
        let plugin_dir = test_dir.join("plugin");
        fs::create_dir(&plugin_dir).unwrap();

        let manifest = PluginManifest {
            id: "com.test.loadable".to_string(),
            name: "Loadable".to_string(),
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

        fs::write(
            &plugin_dir.join("plugin.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(&plugin_dir.join("index.js"), "").unwrap();

        let metadata = PluginMetadata {
            id: manifest.id.clone(),
            path: plugin_dir,
            manifest,
            enabled: true,
            load_status: PluginLoadStatus::Discovered,
        };

        let mut loader = PluginLoader::new();
        loader.register(metadata);

        assert!(loader.load("com.test.loadable").is_ok());
        assert!(loader.is_loaded("com.test.loadable"));
    }

    #[test]
    fn test_load_nonexistent_plugin() {
        let mut loader = PluginLoader::new();
        assert!(loader.load("com.nonexistent").is_err());
    }

    #[test]
    fn test_load_plugin_with_commands() {
        let test_dir = create_test_dir("load_commands");
        let plugin_dir = test_dir.join("plugin");
        fs::create_dir(&plugin_dir).unwrap();
        fs::create_dir(plugin_dir.join("cmds")).unwrap();

        let manifest = PluginManifest {
            id: "com.test.commands".to_string(),
            name: "Commands".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            main: "index.js".to_string(),
            commands: vec![
                CommandDefinition {
                    name: "cmd1".to_string(),
                    description: "First".to_string(),
                    path: "cmds/cmd1.js".to_string(),
                    args_schema: serde_json::json!({}),
                },
                CommandDefinition {
                    name: "cmd2".to_string(),
                    description: "Second".to_string(),
                    path: "cmds/cmd2.js".to_string(),
                    args_schema: serde_json::json!({}),
                },
            ],
            skills: vec![],
            hooks: vec![],
            dependencies: HashMap::new(),
            config_schema: serde_json::json!({}),
        };

        fs::write(
            &plugin_dir.join("plugin.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(&plugin_dir.join("index.js"), "").unwrap();
        fs::write(&plugin_dir.join("cmds/cmd1.js"), "").unwrap();
        fs::write(&plugin_dir.join("cmds/cmd2.js"), "").unwrap();

        let metadata = PluginMetadata {
            id: manifest.id.clone(),
            path: plugin_dir,
            manifest,
            enabled: true,
            load_status: PluginLoadStatus::Discovered,
        };

        let mut loader = PluginLoader::new();
        loader.register(metadata);

        assert!(loader.load("com.test.commands").is_ok());
    }

    #[test]
    fn test_initialize_plugin() {
        let test_dir = create_test_dir("init_plugin");
        let plugin_dir = test_dir.join("plugin");
        fs::create_dir(&plugin_dir).unwrap();

        let manifest = PluginManifest {
            id: "com.test.init".to_string(),
            name: "Init".to_string(),
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

        fs::write(
            &plugin_dir.join("plugin.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(&plugin_dir.join("index.js"), "").unwrap();

        let metadata = PluginMetadata {
            id: manifest.id.clone(),
            path: plugin_dir,
            manifest,
            enabled: true,
            load_status: PluginLoadStatus::Discovered,
        };

        let mut loader = PluginLoader::new();
        loader.register(metadata);

        loader.load("com.test.init").unwrap();
        assert!(loader.initialize("com.test.init").is_ok());
    }
}

// =============================================================================
// PLUGIN EXECUTION - INTEGRATION TESTS (30%)
// =============================================================================

mod execution {
    use super::*;

    pub struct PluginExecutor {
        plugins: HashMap<String, PluginMetadata>,
    }

    impl PluginExecutor {
        pub fn new() -> Self {
            Self {
                plugins: HashMap::new(),
            }
        }

        pub fn register(&mut self, metadata: PluginMetadata) {
            self.plugins.insert(metadata.id.clone(), metadata);
        }

        pub fn execute_command(
            &self,
            plugin_id: &str,
            command_name: &str,
            args: serde_json::Value,
        ) -> Result<PluginExecutionResult, String> {
            let plugin = self
                .plugins
                .get(plugin_id)
                .ok_or_else(|| "Plugin not found".to_string())?;

            let _command = plugin
                .manifest
                .commands
                .iter()
                .find(|c| c.name == command_name)
                .ok_or_else(|| "Command not found".to_string())?;

            Ok(PluginExecutionResult {
                success: true,
                output: format!("Executed: {} with {}", command_name, args),
                errors: vec![],
                duration_ms: 100,
            })
        }

        pub fn execute_skill(
            &self,
            plugin_id: &str,
            skill_id: &str,
        ) -> Result<PluginExecutionResult, String> {
            let plugin = self
                .plugins
                .get(plugin_id)
                .ok_or_else(|| "Plugin not found".to_string())?;

            let _skill = plugin
                .manifest
                .skills
                .iter()
                .find(|s| s.id == skill_id)
                .ok_or_else(|| "Skill not found".to_string())?;

            Ok(PluginExecutionResult {
                success: true,
                output: format!("Executed skill: {}", skill_id),
                errors: vec![],
                duration_ms: 50,
            })
        }
    }

    #[test]
    fn test_execute_plugin_command() {
        let test_dir = create_test_dir("exec_command");
        let plugin_dir = test_dir.join("plugin");
        fs::create_dir(&plugin_dir).unwrap();

        let manifest = PluginManifest {
            id: "com.test.exec".to_string(),
            name: "Exec".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            main: "index.js".to_string(),
            commands: vec![CommandDefinition {
                name: "test-cmd".to_string(),
                description: "Test".to_string(),
                path: "cmd.js".to_string(),
                args_schema: serde_json::json!({}),
            }],
            skills: vec![],
            hooks: vec![],
            dependencies: HashMap::new(),
            config_schema: serde_json::json!({}),
        };

        let metadata = PluginMetadata {
            id: manifest.id.clone(),
            path: plugin_dir,
            manifest,
            enabled: true,
            load_status: PluginLoadStatus::Loaded,
        };

        let mut executor = PluginExecutor::new();
        executor.register(metadata);

        let result = executor
            .execute_command("com.test.exec", "test-cmd", serde_json::json!({}))
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("test-cmd"));
    }

    #[test]
    fn test_execute_skill() {
        let test_dir = create_test_dir("exec_skill");
        let plugin_dir = test_dir.join("plugin");
        fs::create_dir(&plugin_dir).unwrap();

        let manifest = PluginManifest {
            id: "com.test.skill".to_string(),
            name: "Skill".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            main: "index.js".to_string(),
            commands: vec![],
            skills: vec![SkillDefinition {
                id: "skill-1".to_string(),
                name: "Skill 1".to_string(),
                description: "Test skill".to_string(),
                path: "skill1.md".to_string(),
            }],
            hooks: vec![],
            dependencies: HashMap::new(),
            config_schema: serde_json::json!({}),
        };

        let metadata = PluginMetadata {
            id: manifest.id.clone(),
            path: plugin_dir,
            manifest,
            enabled: true,
            load_status: PluginLoadStatus::Loaded,
        };

        let mut executor = PluginExecutor::new();
        executor.register(metadata);

        let result = executor.execute_skill("com.test.skill", "skill-1").unwrap();

        assert!(result.success);
        assert!(result.output.contains("skill-1"));
    }
}

// =============================================================================
// PLUGIN API CONTRACT - INTEGRATION TESTS (30%)
// =============================================================================

mod api_contract {
    use super::*;

    pub struct PluginValidator;

    impl PluginValidator {
        pub fn validate_manifest(manifest: &PluginManifest) -> Result<(), Vec<String>> {
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

        pub fn validate_result(result: &PluginExecutionResult) -> Result<(), Vec<String>> {
            let mut errors = Vec::new();

            if result.success && !result.errors.is_empty() {
                errors.push("Success with errors".to_string());
            }

            if !result.success && result.errors.is_empty() {
                errors.push("Failed without errors".to_string());
            }

            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors)
            }
        }
    }

    #[test]
    fn test_validate_valid_manifest() {
        let manifest = PluginManifest {
            id: "com.example.plugin".to_string(),
            name: "Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Author".to_string(),
            license: "MIT".to_string(),
            main: "index.js".to_string(),
            commands: vec![],
            skills: vec![],
            hooks: vec![],
            dependencies: HashMap::new(),
            config_schema: serde_json::json!({}),
        };

        assert!(PluginValidator::validate_manifest(&manifest).is_ok());
    }

    #[test]
    fn test_validate_missing_fields() {
        let manifest = PluginManifest {
            id: "".to_string(),
            name: "".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Author".to_string(),
            license: "MIT".to_string(),
            main: "index.js".to_string(),
            commands: vec![],
            skills: vec![],
            hooks: vec![],
            dependencies: HashMap::new(),
            config_schema: serde_json::json!({}),
        };

        let result = PluginValidator::validate_manifest(&manifest);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.len() > 0);
    }

    #[test]
    fn test_validate_execution_result_success() {
        let result = PluginExecutionResult {
            success: true,
            output: "Output".to_string(),
            errors: vec![],
            duration_ms: 100,
        };

        assert!(PluginValidator::validate_result(&result).is_ok());
    }

    #[test]
    fn test_validate_execution_result_failure() {
        let result = PluginExecutionResult {
            success: false,
            output: String::new(),
            errors: vec!["Error".to_string()],
            duration_ms: 50,
        };

        assert!(PluginValidator::validate_result(&result).is_ok());
    }

    #[test]
    fn test_validate_inconsistent_result() {
        let result = PluginExecutionResult {
            success: true,
            output: "Output".to_string(),
            errors: vec!["Error".to_string()],
            duration_ms: 100,
        };

        assert!(PluginValidator::validate_result(&result).is_err());
    }
}

// =============================================================================
// E2E TEST - FULL PLUGIN LIFECYCLE
// =============================================================================

#[test]
fn test_full_plugin_lifecycle() {
    let test_dir = create_test_dir("lifecycle");
    let plugin_dir = test_dir.join("plugin");
    fs::create_dir(&plugin_dir).unwrap();

    let manifest = PluginManifest {
        id: "com.test.lifecycle".to_string(),
        name: "Lifecycle".to_string(),
        version: "1.0.0".to_string(),
        description: "Test".to_string(),
        author: "Test".to_string(),
        license: "MIT".to_string(),
        main: "index.js".to_string(),
        commands: vec![CommandDefinition {
            name: "process".to_string(),
            description: "Process".to_string(),
            path: "cmd.js".to_string(),
            args_schema: serde_json::json!({}),
        }],
        skills: vec![],
        hooks: vec![],
        dependencies: HashMap::new(),
        config_schema: serde_json::json!({}),
    };

    fs::write(
        &plugin_dir.join("plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(&plugin_dir.join("index.js"), "").unwrap();
    fs::write(&plugin_dir.join("cmd.js"), "").unwrap();

    // 1. Discovery
    let discovery_result = discovery::PluginDiscovery::new(&test_dir).discover_all();
    assert!(discovery_result.is_ok());
    assert_eq!(discovery_result.unwrap().len(), 1);

    // 2. Validation
    let validation_result = api_contract::PluginValidator::validate_manifest(&manifest);
    assert!(validation_result.is_ok());

    // 3. Loading
    let metadata = PluginMetadata {
        id: manifest.id.clone(),
        path: plugin_dir,
        manifest,
        enabled: true,
        load_status: PluginLoadStatus::Discovered,
    };

    let mut loader = loading::PluginLoader::new();
    loader.register(metadata);
    assert!(loader.load("com.test.lifecycle").is_ok());

    // 4. Execution
    let discovered = discovery::PluginDiscovery::new(&test_dir)
        .discover_all()
        .unwrap();
    let plugin = discovered.into_iter().next().unwrap();
    let mut executor = execution::PluginExecutor::new();
    executor.register(plugin);

    let exec_result =
        executor.execute_command("com.test.lifecycle", "process", serde_json::json!({}));
    assert!(exec_result.is_ok());
    assert!(exec_result.unwrap().success);
}
