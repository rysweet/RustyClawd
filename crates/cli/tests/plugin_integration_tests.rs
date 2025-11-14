//! Comprehensive Plugin System Integration Tests
//!
//! Tests the complete plugin system including:
//! - Plugin discovery and loading
//! - Command execution

#![allow(unused_variables)]
//! - Skill loading
//! - Agent discovery
//! - MCP server management
//! - Hooks integration
//! - Plugin manager orchestration

use claude_code_cli::hooks::registry::HookRegistry;
use claude_code_cli::plugins::*;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test plugin directory structure
fn create_test_plugin(root: &std::path::Path, plugin_id: &str) -> std::path::PathBuf {
    let plugin_dir = root.join(plugin_id);
    fs::create_dir_all(&plugin_dir).unwrap();

    // Create plugin.json
    let manifest = claude_code_cli::plugins::manifest::PluginManifest {
        id: plugin_id.to_string(),
        name: format!("{} Plugin", plugin_id),
        version: "1.0.0".to_string(),
        description: "Test plugin".to_string(),
        author: "Test".to_string(),
        license: "MIT".to_string(),
        main: "index.js".to_string(),
        commands: vec![],
        skills: vec![],
        hooks: vec![],
        agents: vec![],
        mcp_servers: vec![],
        dependencies: HashMap::new(),
        config_schema: serde_json::json!({}),
    };

    fs::write(
        plugin_dir.join("plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    // Create index.js
    fs::write(plugin_dir.join("index.js"), "console.log('Plugin loaded');").unwrap();

    plugin_dir
}

#[tokio::test]
async fn test_plugin_discovery() {
    let temp_dir = TempDir::new().unwrap();
    let plugins_root = temp_dir.path().join("plugins");
    fs::create_dir(&plugins_root).unwrap();

    // Create test plugins
    create_test_plugin(&plugins_root, "test-plugin-1");
    create_test_plugin(&plugins_root, "test-plugin-2");

    let discovery = PluginDiscovery::new(&plugins_root);
    let plugins = discovery.discover_all().unwrap();

    assert_eq!(plugins.len(), 2);

    let ids: Vec<_> = plugins.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"test-plugin-1"));
    assert!(ids.contains(&"test-plugin-2"));
}

#[tokio::test]
async fn test_plugin_loading() {
    let temp_dir = TempDir::new().unwrap();
    let plugins_root = temp_dir.path().join("plugins");
    fs::create_dir(&plugins_root).unwrap();

    let plugin_dir = create_test_plugin(&plugins_root, "loadable-plugin");

    let discovery = PluginDiscovery::new(&plugins_root);
    let plugins = discovery.discover_all().unwrap();

    let mut loader = PluginLoader::new();
    loader.register(plugins[0].clone());

    assert!(loader.load("loadable-plugin").is_ok());
    assert!(loader.is_loaded("loadable-plugin"));

    assert!(loader.initialize("loadable-plugin").is_ok());
}

#[tokio::test]
async fn test_agent_discovery() {
    let temp_dir = TempDir::new().unwrap();
    let agents_dir = temp_dir.path().join(".claude").join("agents");
    fs::create_dir_all(&agents_dir).unwrap();

    // Create test agent files
    fs::write(
        agents_dir.join("builder.md"),
        "# Builder Agent\n\nBuilds code from specifications.",
    )
    .unwrap();

    fs::write(
        agents_dir.join("tester.md"),
        "# Tester Agent\n\nWrites tests for code.",
    )
    .unwrap();

    let discovery = AgentDiscovery::new(temp_dir.path());
    let agents = discovery.discover_all().unwrap();

    assert_eq!(agents.len(), 2);

    let names: Vec<_> = agents.iter().map(|a| a.name.as_str()).collect();
    assert!(names.contains(&"Builder Agent"));
    assert!(names.contains(&"Tester Agent"));
}

#[tokio::test]
async fn test_mcp_proxy_registration() {
    let mut proxy = McpProxy::new();

    let server_def = claude_code_cli::plugins::manifest::McpServerDefinition {
        id: "test-server".to_string(),
        name: "Test Server".to_string(),
        command: "node".to_string(),
        args: vec!["server.js".to_string()],
        env: HashMap::new(),
        description: Some("Test MCP server".to_string()),
    };

    proxy.register_server(server_def);

    let servers = proxy.list_servers();
    assert_eq!(servers.len(), 1);
    assert!(servers.contains(&"test-server".to_string()));
    assert!(!proxy.is_server_running("test-server"));
}

#[tokio::test]
async fn test_hooks_integration() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("plugin");
    fs::create_dir_all(&plugin_dir).unwrap();

    // Create a test hook
    fs::write(
        plugin_dir.join("test-hook.sh"),
        "#!/bin/bash\necho 'Hook executed'\nexit 0",
    )
    .unwrap();

    let hook_def = claude_code_cli::plugins::manifest::HookDefinition {
        event: "PreToolUse".to_string(),
        handler: "test-hook.sh".to_string(),
    };

    let integrator = PluginHooksIntegrator::new("test-plugin".to_string(), plugin_dir);

    let mut registry = HookRegistry::new();
    let result = integrator.register_hooks(&vec![hook_def], &mut registry);

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_plugin_manager_lifecycle() {
    let temp_dir = TempDir::new().unwrap();
    let plugins_root = temp_dir.path().join("plugins");
    fs::create_dir(&plugins_root).unwrap();

    // Create test plugin with all features
    let plugin_dir = plugins_root.join("full-plugin");
    fs::create_dir_all(&plugin_dir).unwrap();

    // Create commands directory
    let commands_dir = plugin_dir.join("commands");
    fs::create_dir(&commands_dir).unwrap();
    fs::write(commands_dir.join("test.js"), "console.log('Test command');").unwrap();

    // Create skills directory
    let skills_dir = plugin_dir.join("skills");
    fs::create_dir(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("test-skill.md"),
        "# Test Skill\n\nA test skill.",
    )
    .unwrap();

    // Create manifest with command and skill
    let manifest = claude_code_cli::plugins::manifest::PluginManifest {
        id: "full-plugin".to_string(),
        name: "Full Plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "Plugin with all features".to_string(),
        author: "Test".to_string(),
        license: "MIT".to_string(),
        main: "index.js".to_string(),
        commands: vec![claude_code_cli::plugins::manifest::CommandDefinition {
            name: "test-cmd".to_string(),
            description: "Test command".to_string(),
            path: "commands/test.js".to_string(),
            args_schema: serde_json::json!({}),
        }],
        skills: vec![claude_code_cli::plugins::manifest::SkillDefinition {
            id: "test-skill".to_string(),
            name: "Test Skill".to_string(),
            description: "A test skill".to_string(),
            path: "skills/test-skill.md".to_string(),
        }],
        hooks: vec![],
        agents: vec![],
        mcp_servers: vec![],
        dependencies: HashMap::new(),
        config_schema: serde_json::json!({}),
    };

    fs::write(
        plugin_dir.join("plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    fs::write(plugin_dir.join("index.js"), "console.log('Plugin loaded');").unwrap();

    // Test plugin manager
    let mut manager = PluginManager::new(&plugins_root).with_project_root(temp_dir.path());

    let loaded = manager.discover_and_load_all().await.unwrap();
    eprintln!("Loaded plugins: {:?}", loaded);

    let summary = manager.summary();
    eprintln!(
        "Summary: total={}, loaded={}",
        summary.total_plugins, summary.loaded_plugins
    );

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0], "full-plugin");

    // Check summary
    assert_eq!(summary.total_plugins, 1);
    assert_eq!(summary.loaded_plugins, 1);
    assert_eq!(summary.total_commands, 1);
    assert_eq!(summary.total_skills, 1);

    // List commands and skills
    let commands = manager.list_all_commands();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].0, "full-plugin");
    assert_eq!(commands[0].1.len(), 1);

    let skills = manager.list_all_skills();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].0, "full-plugin");
    assert_eq!(skills[0].1.len(), 1);
}

#[tokio::test]
async fn test_plugin_with_agents() {
    let temp_dir = TempDir::new().unwrap();
    let plugins_root = temp_dir.path().join("plugins");
    fs::create_dir(&plugins_root).unwrap();

    // Create agents directory
    let agents_dir = temp_dir.path().join(".claude").join("agents");
    fs::create_dir_all(&agents_dir).unwrap();
    fs::write(agents_dir.join("my-agent.md"), "# My Agent\n\nDoes things.").unwrap();

    // Create plugin with agent reference
    let plugin_dir = plugins_root.join("agent-plugin");
    fs::create_dir_all(&plugin_dir).unwrap();

    let agents_subdir = plugin_dir.join("agents");
    fs::create_dir(&agents_subdir).unwrap();
    fs::write(
        agents_subdir.join("plugin-agent.md"),
        "# Plugin Agent\n\nPlugin-specific agent.",
    )
    .unwrap();

    let manifest = claude_code_cli::plugins::manifest::PluginManifest {
        id: "agent-plugin".to_string(),
        name: "Agent Plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "Plugin with agents".to_string(),
        author: "Test".to_string(),
        license: "MIT".to_string(),
        main: "index.js".to_string(),
        commands: vec![],
        skills: vec![],
        hooks: vec![],
        agents: vec![claude_code_cli::plugins::manifest::AgentDefinition {
            id: "plugin-agent".to_string(),
            name: "Plugin Agent".to_string(),
            description: "Agent defined in plugin".to_string(),
            path: "agents/plugin-agent.md".to_string(),
            model: Some("sonnet".to_string()),
        }],
        mcp_servers: vec![],
        dependencies: HashMap::new(),
        config_schema: serde_json::json!({}),
    };

    fs::write(
        plugin_dir.join("plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    fs::write(plugin_dir.join("index.js"), "console.log('Agent plugin');").unwrap();

    let mut manager = PluginManager::new(&plugins_root).with_project_root(temp_dir.path());

    let loaded = manager.discover_and_load_all().await.unwrap();
    assert_eq!(loaded.len(), 1);

    // Check discovered agents (from .claude/agents/)
    let agents = manager.discover_agents().unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0], "my-agent");

    // Check plugin agents
    let summary = manager.summary();
    assert_eq!(summary.total_agents, 2); // 1 discovered + 1 from plugin
}

#[tokio::test]
async fn test_complete_plugin_system_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let plugins_root = temp_dir.path().join("plugins");
    fs::create_dir(&plugins_root).unwrap();

    // Create a complete plugin
    let plugin_dir = plugins_root.join("workflow-plugin");
    fs::create_dir_all(&plugin_dir).unwrap();

    // Create all subdirectories
    fs::create_dir(plugin_dir.join("commands")).unwrap();
    fs::create_dir(plugin_dir.join("skills")).unwrap();
    fs::create_dir(plugin_dir.join("hooks")).unwrap();
    fs::create_dir(plugin_dir.join("agents")).unwrap();

    // Create files
    fs::write(
        plugin_dir.join("commands/cmd.js"),
        "console.log('Command');",
    )
    .unwrap();
    fs::write(
        plugin_dir.join("skills/skill.md"),
        "# Skill\n\nSkill content.",
    )
    .unwrap();
    fs::write(plugin_dir.join("hooks/hook.sh"), "#!/bin/bash\necho 'Hook'").unwrap();
    fs::write(
        plugin_dir.join("agents/agent.md"),
        "# Agent\n\nAgent content.",
    )
    .unwrap();

    let manifest = claude_code_cli::plugins::manifest::PluginManifest {
        id: "workflow-plugin".to_string(),
        name: "Workflow Plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "Complete workflow test".to_string(),
        author: "Test".to_string(),
        license: "MIT".to_string(),
        main: "index.js".to_string(),
        commands: vec![claude_code_cli::plugins::manifest::CommandDefinition {
            name: "cmd".to_string(),
            description: "Test".to_string(),
            path: "commands/cmd.js".to_string(),
            args_schema: serde_json::json!({}),
        }],
        skills: vec![claude_code_cli::plugins::manifest::SkillDefinition {
            id: "skill".to_string(),
            name: "Skill".to_string(),
            description: "Test".to_string(),
            path: "skills/skill.md".to_string(),
        }],
        hooks: vec![claude_code_cli::plugins::manifest::HookDefinition {
            event: "PreToolUse".to_string(),
            handler: "hooks/hook.sh".to_string(),
        }],
        agents: vec![claude_code_cli::plugins::manifest::AgentDefinition {
            id: "agent".to_string(),
            name: "Agent".to_string(),
            description: "Test".to_string(),
            path: "agents/agent.md".to_string(),
            model: None,
        }],
        mcp_servers: vec![claude_code_cli::plugins::manifest::McpServerDefinition {
            id: "mcp".to_string(),
            name: "MCP".to_string(),
            command: "node".to_string(),
            args: vec!["server.js".to_string()],
            env: HashMap::new(),
            description: None,
        }],
        dependencies: HashMap::new(),
        config_schema: serde_json::json!({}),
    };

    fs::write(
        plugin_dir.join("plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    fs::write(plugin_dir.join("index.js"), "console.log('OK');").unwrap();

    // Full workflow
    let mut manager = PluginManager::new(&plugins_root).with_project_root(temp_dir.path());

    // 1. Discover and load
    let loaded = manager.discover_and_load_all().await.unwrap();
    assert_eq!(loaded.len(), 1);

    // 2. Register hooks
    let mut hooks_registry = HookRegistry::new();
    let hooks_result = manager.register_all_hooks(&mut hooks_registry);
    assert!(hooks_result.is_ok());

    // 3. Check MCP servers
    let mcp_servers = manager.mcp_proxy.list_servers();
    assert_eq!(mcp_servers.len(), 1);

    // 4. Get summary
    let summary = manager.summary();
    assert_eq!(summary.loaded_plugins, 1);
    assert_eq!(summary.total_commands, 1);
    assert_eq!(summary.total_skills, 1);
    assert_eq!(summary.total_hooks, 1);
    assert_eq!(summary.total_agents, 1);
    assert_eq!(summary.total_mcp_servers, 1);

    println!("Complete workflow test passed!\n{}", summary);
}
