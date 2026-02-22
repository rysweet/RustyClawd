use super::*;
use std::fs;
use tempfile::TempDir;

fn setup_test_agents_dir() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let agents_dir = temp_dir.path().join(".claude").join("agents");
    fs::create_dir_all(&agents_dir).unwrap();
    temp_dir
}

#[test]
fn test_discover_empty_directory() {
    let temp_dir = setup_test_agents_dir();
    let discovery = AgentDiscovery::new(temp_dir.path());

    let agents = discovery.discover_all().unwrap();
    assert_eq!(agents.len(), 0);
}

#[test]
fn test_discover_single_agent() {
    let temp_dir = setup_test_agents_dir();
    let agents_dir = temp_dir.path().join(".claude").join("agents");

    fs::write(
        agents_dir.join("builder.md"),
        "# Builder Agent\n\nBuilds code from specifications.",
    )
    .unwrap();

    let discovery = AgentDiscovery::new(temp_dir.path());
    let agents = discovery.discover_all().unwrap();

    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].id, "builder");
    assert_eq!(agents[0].name, "Builder Agent");
    assert_eq!(agents[0].description, "Builds code from specifications.");
}

#[test]
fn test_discover_multiple_agents() {
    let temp_dir = setup_test_agents_dir();
    let agents_dir = temp_dir.path().join(".claude").join("agents");

    fs::write(agents_dir.join("builder.md"), "# Builder\n\nBuilds things.").unwrap();

    fs::write(agents_dir.join("tester.md"), "# Tester\n\nTests things.").unwrap();

    fs::write(
        agents_dir.join("reviewer.md"),
        "# Reviewer\n\nReviews things.",
    )
    .unwrap();

    let discovery = AgentDiscovery::new(temp_dir.path());
    let agents = discovery.discover_all().unwrap();

    assert_eq!(agents.len(), 3);

    let ids: Vec<_> = agents.iter().map(|a| a.id.as_str()).collect();
    assert!(ids.contains(&"builder"));
    assert!(ids.contains(&"tester"));
    assert!(ids.contains(&"reviewer"));
}

#[test]
fn test_extract_metadata_with_heading() {
    let content = "# My Agent\n\nThis is a test agent.\n\nMore content here.";
    let (name, desc) = AgentDiscovery::extract_metadata(content, "fallback");

    assert_eq!(name, "My Agent");
    assert_eq!(desc, "This is a test agent.");
}

#[test]
fn test_extract_metadata_without_heading() {
    let content = "Just some content without a heading.";
    let (name, desc) = AgentDiscovery::extract_metadata(content, "fallback");

    assert_eq!(name, "fallback");
    assert_eq!(desc, "Agent: fallback");
}

#[test]
fn test_has_agent() {
    let temp_dir = setup_test_agents_dir();
    let agents_dir = temp_dir.path().join(".claude").join("agents");

    fs::write(agents_dir.join("builder.md"), "# Builder\n\nBuilds.").unwrap();

    let discovery = AgentDiscovery::new(temp_dir.path());

    assert!(discovery.has_agent("builder"));
    assert!(!discovery.has_agent("nonexistent"));
}

#[test]
fn test_get_agent() {
    let temp_dir = setup_test_agents_dir();
    let agents_dir = temp_dir.path().join(".claude").join("agents");

    fs::write(
        agents_dir.join("builder.md"),
        "# Builder Agent\n\nBuilds code.",
    )
    .unwrap();

    let discovery = AgentDiscovery::new(temp_dir.path());

    let agent = discovery.get_agent("builder").unwrap().unwrap();
    assert_eq!(agent.id, "builder");
    assert_eq!(agent.name, "Builder Agent");

    assert!(discovery.get_agent("nonexistent").unwrap().is_none());
}

#[test]
fn test_list_agent_ids() {
    let temp_dir = setup_test_agents_dir();
    let agents_dir = temp_dir.path().join(".claude").join("agents");

    fs::write(agents_dir.join("a.md"), "# A\n\nAgent A.").unwrap();
    fs::write(agents_dir.join("b.md"), "# B\n\nAgent B.").unwrap();
    fs::write(agents_dir.join("c.md"), "# C\n\nAgent C.").unwrap();

    let discovery = AgentDiscovery::new(temp_dir.path());
    let ids = discovery.list_agent_ids().unwrap();

    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&"a".to_string()));
    assert!(ids.contains(&"b".to_string()));
    assert!(ids.contains(&"c".to_string()));
}

// ==================== Runtime Agent Tests ====================

#[test]
fn test_parse_runtime_agents_valid() {
    let json = r#"{
        "code-reviewer": {
            "description": "Reviews code for quality",
            "prompt": "You are a code reviewer. Review the given code.",
            "tools": ["Read", "Grep"],
            "model": "sonnet"
        }
    }"#;

    let agents = parse_runtime_agents(json).unwrap();
    assert_eq!(agents.len(), 1);

    let agent = agents.get("code-reviewer").unwrap();
    assert_eq!(agent.description, "Reviews code for quality");
    assert_eq!(
        agent.prompt,
        "You are a code reviewer. Review the given code."
    );
    assert_eq!(agent.tools, vec!["Read", "Grep"]);
    assert_eq!(agent.model, Some("sonnet".to_string()));
}

#[test]
fn test_parse_runtime_agents_multiple() {
    let json = r#"{
        "writer": {
            "description": "Writes documentation",
            "prompt": "Write docs",
            "tools": ["Write"]
        },
        "tester": {
            "description": "Runs tests",
            "prompt": "Run tests",
            "model": "haiku"
        }
    }"#;

    let agents = parse_runtime_agents(json).unwrap();
    assert_eq!(agents.len(), 2);
    assert!(agents.contains_key("writer"));
    assert!(agents.contains_key("tester"));
}

#[test]
fn test_parse_runtime_agents_minimal() {
    // Only description and prompt are required, tools and model are optional
    let json = r#"{
        "simple": {
            "description": "A simple agent",
            "prompt": "Do something"
        }
    }"#;

    let agents = parse_runtime_agents(json).unwrap();
    let agent = agents.get("simple").unwrap();
    assert_eq!(agent.description, "A simple agent");
    assert_eq!(agent.prompt, "Do something");
    assert!(agent.tools.is_empty());
    assert!(agent.model.is_none());
}

#[test]
fn test_parse_runtime_agents_invalid_json() {
    let json = "not valid json";
    let result = parse_runtime_agents(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid agents JSON"));
}

#[test]
fn test_parse_runtime_agents_missing_required_field() {
    // Missing prompt field
    let json = r#"{
        "incomplete": {
            "description": "Only description"
        }
    }"#;

    let result = parse_runtime_agents(json);
    assert!(result.is_err());
}

#[test]
fn test_validate_runtime_agents_valid() {
    let mut agents = HashMap::new();
    agents.insert(
        "test".to_string(),
        RuntimeAgentDefinition {
            description: "Test agent".to_string(),
            prompt: "Test prompt".to_string(),
            tools: vec![],
            allowed_tools: vec![],
            disallowed_tools: vec![],
            model: None,
        },
    );

    assert!(validate_runtime_agents(&agents).is_ok());
}

#[test]
fn test_validate_runtime_agents_empty_description() {
    let mut agents = HashMap::new();
    agents.insert(
        "test".to_string(),
        RuntimeAgentDefinition {
            description: "".to_string(),
            prompt: "Test prompt".to_string(),
            tools: vec![],
            allowed_tools: vec![],
            disallowed_tools: vec![],
            model: None,
        },
    );

    let result = validate_runtime_agents(&agents);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("empty description")));
}

#[test]
fn test_validate_runtime_agents_empty_prompt() {
    let mut agents = HashMap::new();
    agents.insert(
        "test".to_string(),
        RuntimeAgentDefinition {
            description: "Test description".to_string(),
            prompt: "".to_string(),
            tools: vec![],
            allowed_tools: vec![],
            disallowed_tools: vec![],
            model: None,
        },
    );

    let result = validate_runtime_agents(&agents);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("empty prompt")));
}

#[test]
fn test_agent_discovery_with_runtime_agents() {
    let temp_dir = setup_test_agents_dir();
    let agents_dir = temp_dir.path().join(".claude").join("agents");

    // Create file-based agent
    fs::write(
        agents_dir.join("file-agent.md"),
        "# File Agent\n\nFrom file.",
    )
    .unwrap();

    // Create runtime agent
    let mut runtime_agents = HashMap::new();
    runtime_agents.insert(
        "runtime-agent".to_string(),
        RuntimeAgentDefinition {
            description: "Runtime agent".to_string(),
            prompt: "Runtime prompt".to_string(),
            tools: vec!["Read".to_string()],
            allowed_tools: vec![],
            disallowed_tools: vec![],
            model: Some("sonnet".to_string()),
        },
    );

    let discovery = AgentDiscovery::new(temp_dir.path()).with_runtime_agents(runtime_agents);

    // Test all_agents returns both
    let all = discovery.all_agents().unwrap();
    assert_eq!(all.len(), 2);

    // Check file-based agent
    let file_agent = all.iter().find(|a| a.id == "file-agent").unwrap();
    assert_eq!(file_agent.name, "File Agent");
    assert!(!file_agent.path.starts_with("runtime:"));

    // Check runtime agent
    let runtime = all.iter().find(|a| a.id == "runtime-agent").unwrap();
    assert_eq!(runtime.description, "Runtime agent");
    assert_eq!(runtime.path, "runtime:runtime-agent");
    assert_eq!(runtime.model, Some("sonnet".to_string()));
}

#[test]
fn test_is_runtime_agent() {
    let temp_dir = setup_test_agents_dir();

    let mut runtime_agents = HashMap::new();
    runtime_agents.insert(
        "runtime-agent".to_string(),
        RuntimeAgentDefinition {
            description: "Runtime".to_string(),
            prompt: "Prompt".to_string(),
            tools: vec![],
            allowed_tools: vec![],
            disallowed_tools: vec![],
            model: None,
        },
    );

    let discovery = AgentDiscovery::new(temp_dir.path()).with_runtime_agents(runtime_agents);

    assert!(discovery.is_runtime_agent("runtime-agent"));
    assert!(!discovery.is_runtime_agent("file-agent"));
}

#[test]
fn test_get_runtime_agent() {
    let temp_dir = setup_test_agents_dir();

    let mut runtime_agents = HashMap::new();
    runtime_agents.insert(
        "my-agent".to_string(),
        RuntimeAgentDefinition {
            description: "My agent".to_string(),
            prompt: "My prompt".to_string(),
            tools: vec!["Bash".to_string()],
            allowed_tools: vec![],
            disallowed_tools: vec![],
            model: Some("opus".to_string()),
        },
    );

    let discovery = AgentDiscovery::new(temp_dir.path()).with_runtime_agents(runtime_agents);

    let agent = discovery.get_runtime_agent("my-agent").unwrap();
    assert_eq!(agent.description, "My agent");
    assert_eq!(agent.prompt, "My prompt");
    assert_eq!(agent.tools, vec!["Bash"]);
    assert_eq!(agent.model, Some("opus".to_string()));

    assert!(discovery.get_runtime_agent("nonexistent").is_none());
}

#[test]
fn test_add_runtime_agent() {
    let temp_dir = setup_test_agents_dir();
    let mut discovery = AgentDiscovery::new(temp_dir.path());

    assert_eq!(discovery.runtime_agent_ids().len(), 0);

    discovery.add_runtime_agent(
        "added-agent".to_string(),
        RuntimeAgentDefinition {
            description: "Added".to_string(),
            prompt: "Added prompt".to_string(),
            tools: vec![],
            allowed_tools: vec![],
            disallowed_tools: vec![],
            model: None,
        },
    );

    assert_eq!(discovery.runtime_agent_ids().len(), 1);
    assert!(discovery.is_runtime_agent("added-agent"));
}

#[test]
fn test_parse_runtime_agent_with_disallowed_tools() {
    let json = r#"{
        "secure-agent": {
            "description": "Secure read-only agent",
            "prompt": "You can only read files, never modify them.",
            "tools": ["Read", "Grep", "Glob"],
            "disallowedTools": ["Write", "Edit", "Bash"]
        }
    }"#;

    let result = parse_runtime_agents(json).unwrap();
    let agent = result.get("secure-agent").unwrap();

    assert_eq!(agent.description, "Secure read-only agent");
    assert_eq!(agent.tools, vec!["Read", "Grep", "Glob"]);
    assert_eq!(agent.disallowed_tools, vec!["Write", "Edit", "Bash"]);
}

#[test]
fn test_runtime_agent_disallowed_tools_inherited_to_agent_definition() {
    let temp_dir = setup_test_agents_dir();

    let mut runtime_agents = HashMap::new();
    runtime_agents.insert(
        "restricted-agent".to_string(),
        RuntimeAgentDefinition {
            description: "Restricted agent".to_string(),
            prompt: "Restricted prompt".to_string(),
            tools: vec!["Read".to_string()],
            allowed_tools: vec![],
            disallowed_tools: vec!["Bash".to_string(), "Write".to_string()],
            model: None,
        },
    );

    let discovery = AgentDiscovery::new(temp_dir.path()).with_runtime_agents(runtime_agents);

    let all = discovery.all_agents().unwrap();
    let restricted = all.iter().find(|a| a.id == "restricted-agent").unwrap();

    assert_eq!(
        restricted.disallowed_tools,
        vec!["Bash".to_string(), "Write".to_string()]
    );
}

#[test]
fn test_runtime_agent_disallowed_tools_default_empty() {
    let json = r#"{
        "basic-agent": {
            "description": "Basic agent",
            "prompt": "Basic prompt"
        }
    }"#;

    let result = parse_runtime_agents(json).unwrap();
    let agent = result.get("basic-agent").unwrap();

    // disallowedTools should default to empty vec when not specified
    assert!(agent.disallowed_tools.is_empty());
}

#[test]
fn test_get_runtime_agent_includes_disallowed_tools() {
    let temp_dir = setup_test_agents_dir();

    let mut runtime_agents = HashMap::new();
    runtime_agents.insert(
        "my-agent".to_string(),
        RuntimeAgentDefinition {
            description: "My agent".to_string(),
            prompt: "My prompt".to_string(),
            tools: vec!["Read".to_string()],
            allowed_tools: vec![],
            disallowed_tools: vec!["Bash".to_string()],
            model: None,
        },
    );

    let discovery = AgentDiscovery::new(temp_dir.path()).with_runtime_agents(runtime_agents);

    let agent = discovery.get_runtime_agent("my-agent").unwrap();
    assert_eq!(agent.disallowed_tools, vec!["Bash".to_string()]);
}

// Tests for allowed_tools feature in RuntimeAgentDefinition

#[test]
fn test_parse_runtime_agents_with_allowed_tools() {
    let json = r#"{
        "restricted-agent": {
            "description": "Agent with limited tool access",
            "prompt": "You can only use specific tools",
            "allowedTools": ["Read", "Grep", "Glob"]
        }
    }"#;

    let result = parse_runtime_agents(json).unwrap();
    let agent = result.get("restricted-agent").unwrap();

    // allowedTools should be parsed from JSON
    assert_eq!(agent.allowed_tools, vec!["Read", "Grep", "Glob"]);
}

#[test]
fn test_runtime_agent_allowed_tools_default_empty() {
    let json = r#"{
        "basic-agent": {
            "description": "Basic agent",
            "prompt": "Basic prompt"
        }
    }"#;

    let result = parse_runtime_agents(json).unwrap();
    let agent = result.get("basic-agent").unwrap();

    // allowedTools should default to empty vec when not specified
    assert!(agent.allowed_tools.is_empty());
}

#[test]
fn test_runtime_agent_with_both_allowed_and_disallowed_tools() {
    let json = r#"{
        "complex-agent": {
            "description": "Agent with both allowed and disallowed",
            "prompt": "Complex restrictions",
            "allowedTools": ["Read", "Write", "Bash"],
            "disallowedTools": ["Bash"]
        }
    }"#;

    let result = parse_runtime_agents(json).unwrap();
    let agent = result.get("complex-agent").unwrap();

    // Both fields should be preserved (filtering happens at execution time)
    assert_eq!(agent.allowed_tools, vec!["Read", "Write", "Bash"]);
    assert_eq!(agent.disallowed_tools, vec!["Bash"]);
}

#[test]
fn test_get_runtime_agent_includes_allowed_tools() {
    let temp_dir = setup_test_agents_dir();

    let mut runtime_agents = HashMap::new();
    runtime_agents.insert(
        "my-agent".to_string(),
        RuntimeAgentDefinition {
            description: "My agent".to_string(),
            prompt: "My prompt".to_string(),
            tools: vec![],
            allowed_tools: vec!["Read".to_string(), "Grep".to_string()],
            disallowed_tools: vec![],
            model: None,
        },
    );

    let discovery = AgentDiscovery::new(temp_dir.path()).with_runtime_agents(runtime_agents);

    let agent = discovery.get_runtime_agent("my-agent").unwrap();
    assert_eq!(agent.allowed_tools, vec!["Read", "Grep"]);
}
