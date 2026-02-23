use super::*;

#[test]
fn test_valid_semver() {
    assert!(is_valid_semver("1.0.0"));
    assert!(is_valid_semver("0.1.0"));
    assert!(is_valid_semver("10.20.30"));
    assert!(!is_valid_semver("1.0"));
    assert!(!is_valid_semver("1.a.0"));
}

#[test]
fn test_validate_manifest_valid() {
    let manifest = PluginManifest {
        id: "com.example.test".to_string(),
        name: "Test".to_string(),
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

    assert!(validate_manifest(&manifest).is_ok());
}

#[test]
fn test_validate_manifest_missing_id() {
    let manifest = PluginManifest {
        id: "".to_string(),
        name: "Test".to_string(),
        version: "1.0.0".to_string(),
        description: "Test".to_string(),
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

    let result = validate_manifest(&manifest);
    assert!(result.is_err());
}

#[test]
fn test_mcp_transport_backward_compatibility() {
    // Old format using command/args
    let server = McpServerDefinition {
        id: "test".to_string(),
        name: "Test".to_string(),
        transport: None,
        command: Some("node".to_string()),
        args: vec!["server.js".to_string()],
        env: HashMap::new(),
        description: None,
        startup_timeout: None,
    };

    let transport = server.get_transport().unwrap();
    assert!(matches!(transport, McpTransportConfig::Stdio { .. }));
}

#[test]
fn test_mcp_transport_new_stdio_format() {
    // New format using transport field
    let server = McpServerDefinition {
        id: "test".to_string(),
        name: "Test".to_string(),
        transport: Some(McpTransportConfig::Stdio {
            command: "node".to_string(),
            args: vec!["server.js".to_string()],
        }),
        command: None,
        args: vec![],
        env: HashMap::new(),
        description: None,
        startup_timeout: None,
    };

    let transport = server.get_transport().unwrap();
    assert!(matches!(transport, McpTransportConfig::Stdio { .. }));
}

#[test]
fn test_mcp_transport_http() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer token".to_string());

    let server = McpServerDefinition {
        id: "test".to_string(),
        name: "Test".to_string(),
        transport: Some(McpTransportConfig::Http {
            url: "http://localhost:8080/mcp".to_string(),
            headers: Some(headers),
        }),
        command: None,
        args: vec![],
        env: HashMap::new(),
        description: None,
        startup_timeout: None,
    };

    let transport = server.get_transport().unwrap();
    assert!(matches!(transport, McpTransportConfig::Http { .. }));
}

#[test]
fn test_agent_definition_with_disallowed_tools() {
    let json = r#"{
        "id": "secure-agent",
        "name": "Secure Agent",
        "description": "Read-only agent",
        "path": "agents/secure.md",
        "disallowedTools": ["Write", "Edit", "Bash"]
    }"#;

    let agent: AgentDefinition = serde_json::from_str(json).unwrap();

    assert_eq!(agent.id, "secure-agent");
    assert_eq!(agent.name, "Secure Agent");
    assert_eq!(agent.disallowed_tools, vec!["Write", "Edit", "Bash"]);
}

#[test]
fn test_agent_definition_disallowed_tools_default_empty() {
    let json = r#"{
        "id": "basic-agent",
        "name": "Basic Agent",
        "description": "Basic agent",
        "path": "agents/basic.md"
    }"#;

    let agent: AgentDefinition = serde_json::from_str(json).unwrap();

    // disallowedTools should default to empty vec when not specified
    assert!(agent.disallowed_tools.is_empty());
}

#[test]
fn test_agent_definition_serialization_skips_empty_disallowed_tools() {
    let agent = AgentDefinition {
        id: "test".to_string(),
        name: "Test".to_string(),
        description: "Test agent".to_string(),
        path: "test.md".to_string(),
        model: None,
        allowed_tools: None,
        disallowed_tools: vec![],
        isolation: None,
        background: false,
        memory: None,
    };

    let json = serde_json::to_string(&agent).unwrap();
    // Empty disallowed_tools should not appear in serialized JSON
    assert!(!json.contains("disallowedTools"));
}

#[test]
fn test_agent_definition_serialization_includes_non_empty_disallowed_tools() {
    let agent = AgentDefinition {
        id: "test".to_string(),
        name: "Test".to_string(),
        description: "Test agent".to_string(),
        path: "test.md".to_string(),
        model: None,
        allowed_tools: None,
        disallowed_tools: vec!["Bash".to_string()],
        isolation: None,
        background: false,
        memory: None,
    };

    let json = serde_json::to_string(&agent).unwrap();
    // Non-empty disallowed_tools should appear in serialized JSON
    assert!(json.contains("disallowedTools"));
    assert!(json.contains("Bash"));
}

// Tests for allowed_tools feature

#[test]
fn test_agent_definition_with_allowed_tools() {
    let json = r#"{
        "id": "restricted-agent",
        "name": "Restricted Agent",
        "description": "Agent with limited tool access",
        "path": "agents/restricted.md",
        "allowedTools": ["Read", "Grep", "Glob"]
    }"#;

    let agent: AgentDefinition = serde_json::from_str(json).unwrap();

    assert_eq!(agent.id, "restricted-agent");
    assert_eq!(agent.name, "Restricted Agent");
    assert_eq!(
        agent.allowed_tools,
        Some(vec![
            "Read".to_string(),
            "Grep".to_string(),
            "Glob".to_string()
        ])
    );
}

#[test]
fn test_agent_definition_allowed_tools_default_none() {
    let json = r#"{
        "id": "basic-agent",
        "name": "Basic Agent",
        "description": "Basic agent",
        "path": "agents/basic.md"
    }"#;

    let agent: AgentDefinition = serde_json::from_str(json).unwrap();

    // allowedTools should default to None when not specified
    assert!(agent.allowed_tools.is_none());
}

#[test]
fn test_agent_definition_serialization_skips_none_allowed_tools() {
    let agent = AgentDefinition {
        id: "test".to_string(),
        name: "Test".to_string(),
        description: "Test agent".to_string(),
        path: "test.md".to_string(),
        model: None,
        disallowed_tools: vec![],
        allowed_tools: None,
        isolation: None,
        background: false,
        memory: None,
    };

    let json = serde_json::to_string(&agent).unwrap();
    // None allowed_tools should not appear in serialized JSON
    assert!(!json.contains("allowedTools"));
}

#[test]
fn test_agent_definition_serialization_includes_non_empty_allowed_tools() {
    let agent = AgentDefinition {
        id: "test".to_string(),
        name: "Test".to_string(),
        description: "Test agent".to_string(),
        path: "test.md".to_string(),
        model: None,
        disallowed_tools: vec![],
        allowed_tools: Some(vec!["Read".to_string(), "Grep".to_string()]),
        isolation: None,
        background: false,
        memory: None,
    };

    let json = serde_json::to_string(&agent).unwrap();
    // Non-empty allowed_tools should appear in serialized JSON
    assert!(json.contains("allowedTools"));
    assert!(json.contains("Read"));
    assert!(json.contains("Grep"));
}

#[test]
fn test_agent_definition_with_both_allowed_and_disallowed_tools() {
    let json = r#"{
        "id": "complex-agent",
        "name": "Complex Agent",
        "description": "Agent with both allowed and disallowed tools",
        "path": "agents/complex.md",
        "allowedTools": ["Read", "Write", "Bash"],
        "disallowedTools": ["Bash"]
    }"#;

    let agent: AgentDefinition = serde_json::from_str(json).unwrap();

    assert_eq!(agent.id, "complex-agent");
    // Both fields should be preserved (filtering happens at execution time)
    assert_eq!(
        agent.allowed_tools,
        Some(vec![
            "Read".to_string(),
            "Write".to_string(),
            "Bash".to_string()
        ])
    );
    assert_eq!(agent.disallowed_tools, vec!["Bash".to_string()]);
}
