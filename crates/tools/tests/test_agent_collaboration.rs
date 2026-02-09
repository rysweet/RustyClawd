//! Integration tests for multi-agent collaboration
//!
//! Tests the complete agent collaboration system including:
//! - Agent memory with scopes (user/project/local)
//! - Hook events (TeammateIdle, TaskCompleted)
//! - Agent registry for tracking agent state
//! - Coordination patterns

use rustyclawd_tools::{
    global_agent_memory, global_agent_registry, AgentMemory, AgentRegistry, AgentStatus,
    MemoryScope,
};
use serde_json::json;

#[tokio::test]
async fn test_agent_collaboration_basic() {
    // Scenario: Two agents collaborating on a task
    // Agent 1 processes data and stores it in shared memory
    // Agent 2 reads from shared memory and continues processing

    let memory = AgentMemory::new();
    let registry = AgentRegistry::new();

    // Agent 1 starts working
    let agent1_id = "agent1_builder".to_string();
    registry
        .register(
            agent1_id.clone(),
            "builder".to_string(),
            "sonnet".to_string(),
        )
        .await
        .unwrap();

    // Agent 1 processes some data and stores result in project scope
    memory
        .set(
            MemoryScope::Project,
            "build_status".to_string(),
            json!({"status": "compiled", "artifacts": ["binary", "docs"]}),
            agent1_id.clone(),
            Some("project123".to_string()),
        )
        .await
        .unwrap();

    // Agent 1 completes work
    registry.mark_completed(&agent1_id).await.unwrap();

    // Agent 2 starts working (triggered by TaskCompleted event)
    let agent2_id = "agent2_tester".to_string();
    registry
        .register(
            agent2_id.clone(),
            "tester".to_string(),
            "sonnet".to_string(),
        )
        .await
        .unwrap();

    // Agent 2 reads build status from shared memory
    let build_status = memory
        .get(
            MemoryScope::Project,
            "build_status",
            &agent2_id,
            Some("project123"),
        )
        .await
        .unwrap()
        .expect("Build status should exist");

    assert_eq!(build_status.value["status"], "compiled");
    assert_eq!(build_status.created_by, "agent1_builder");

    // Agent 2 stores test results in project scope
    memory
        .set(
            MemoryScope::Project,
            "test_results".to_string(),
            json!({"status": "passed", "coverage": 95}),
            agent2_id.clone(),
            Some("project123".to_string()),
        )
        .await
        .unwrap();

    // Verify both agents' work is visible in project scope
    let keys = memory
        .list_keys(MemoryScope::Project, &agent2_id, Some("project123"))
        .await
        .unwrap();

    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"build_status".to_string()));
    assert!(keys.contains(&"test_results".to_string()));
}

#[tokio::test]
async fn test_agent_memory_scopes() {
    // Test that different memory scopes work correctly
    let memory = AgentMemory::new();
    let agent1 = "agent1".to_string();
    let agent2 = "agent2".to_string();
    let project = "test_project".to_string();

    // User scope - shared across all agents
    memory
        .set(
            MemoryScope::User,
            "preference".to_string(),
            json!({"theme": "dark"}),
            agent1.clone(),
            None,
        )
        .await
        .unwrap();

    // Both agents can read user preference
    let pref1 = memory
        .get(MemoryScope::User, "preference", &agent1, None)
        .await
        .unwrap()
        .unwrap();
    let pref2 = memory
        .get(MemoryScope::User, "preference", &agent2, None)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(pref1.value, pref2.value);

    // Project scope - shared within project
    memory
        .set(
            MemoryScope::Project,
            "config".to_string(),
            json!({"debug": true}),
            agent1.clone(),
            Some(project.clone()),
        )
        .await
        .unwrap();

    let config = memory
        .get(MemoryScope::Project, "config", &agent2, Some(&project))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(config.value["debug"], true);

    // Local scope - private to each agent
    memory
        .set(
            MemoryScope::Local,
            "state".to_string(),
            json!({"progress": 50}),
            agent1.clone(),
            None,
        )
        .await
        .unwrap();

    // Agent 1 can read its local state
    let state1 = memory
        .get(MemoryScope::Local, "state", &agent1, None)
        .await
        .unwrap();
    assert!(state1.is_some());

    // Agent 2 cannot read Agent 1's local state
    let state2 = memory
        .get(MemoryScope::Local, "state", &agent2, None)
        .await
        .unwrap();
    assert!(state2.is_none());
}

#[tokio::test]
async fn test_team_coordination_via_registry() {
    // Scenario: Team of 3 agents coordinating work
    let registry = AgentRegistry::new();

    // Spawn 3 agents
    let agents = vec![
        ("agent1", "architect"),
        ("agent2", "builder"),
        ("agent3", "tester"),
    ];

    for (id, agent_type) in &agents {
        registry
            .register(id.to_string(), agent_type.to_string(), "sonnet".to_string())
            .await
            .unwrap();
    }

    // All agents should be in the registry
    let active_agents = registry.list_ids().await;
    assert_eq!(active_agents.len(), 3);

    // Agent 1 completes its work
    registry.mark_completed("agent1").await.unwrap();

    // Check agent 1 status
    let status = registry.get_status("agent1").await.unwrap();
    assert!(matches!(status, AgentStatus::Completed));

    // Agent 2 and 3 still running
    let status2 = registry.get_status("agent2").await.unwrap();
    let status3 = registry.get_status("agent3").await.unwrap();
    assert!(matches!(status2, AgentStatus::Running));
    assert!(matches!(status3, AgentStatus::Running));

    // Complete remaining agents
    registry.mark_completed("agent2").await.unwrap();
    registry.mark_completed("agent3").await.unwrap();

    // All agents should be completed
    for id in &["agent1", "agent2", "agent3"] {
        let status = registry.get_status(id).await.unwrap();
        assert!(matches!(status, AgentStatus::Completed));
    }
}

#[tokio::test]
async fn test_agent_handoff_pattern() {
    // Pattern: Agent A completes work, stores result, Agent B picks up
    let memory = AgentMemory::new();
    let registry = AgentRegistry::new();
    let project = "handoff_test".to_string();

    // Agent A (Architect) designs the solution
    let agent_a = "agent_architect".to_string();
    registry
        .register(
            agent_a.clone(),
            "architect".to_string(),
            "sonnet".to_string(),
        )
        .await
        .unwrap();

    memory
        .set(
            MemoryScope::Project,
            "design".to_string(),
            json!({
                "modules": ["auth", "api", "storage"],
                "architecture": "microservices"
            }),
            agent_a.clone(),
            Some(project.clone()),
        )
        .await
        .unwrap();

    registry.mark_completed(&agent_a).await.unwrap();

    // Agent B (Builder) reads the design and implements
    let agent_b = "agent_builder".to_string();
    registry
        .register(agent_b.clone(), "builder".to_string(), "sonnet".to_string())
        .await
        .unwrap();

    let design = memory
        .get(MemoryScope::Project, "design", &agent_b, Some(&project))
        .await
        .unwrap()
        .expect("Design should exist");

    assert_eq!(design.value["architecture"], "microservices");
    assert_eq!(design.created_by, "agent_architect");

    // Builder stores implementation status
    memory
        .set(
            MemoryScope::Project,
            "implementation".to_string(),
            json!({
                "completed_modules": ["auth", "api"],
                "pending_modules": ["storage"]
            }),
            agent_b.clone(),
            Some(project.clone()),
        )
        .await
        .unwrap();

    registry.mark_completed(&agent_b).await.unwrap();

    // Verify handoff chain is recorded in memory
    let keys = memory
        .list_keys(MemoryScope::Project, &agent_b, Some(&project))
        .await
        .unwrap();

    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&"design".to_string()));
    assert!(keys.contains(&"implementation".to_string()));
}

#[tokio::test]
async fn test_global_singletons() {
    // Verify global memory and registry singletons work correctly
    let memory1 = global_agent_memory();
    let memory2 = global_agent_memory();

    // Should be the same instance
    assert!(std::sync::Arc::ptr_eq(&memory1, &memory2));

    let registry1 = global_agent_registry();
    let registry2 = global_agent_registry();

    // Should be the same instance
    assert!(std::sync::Arc::ptr_eq(&registry1, &registry2));
}

#[tokio::test]
async fn test_memory_cleanup() {
    // Test that memory can be cleared per scope
    let memory = AgentMemory::new();
    let agent = "cleanup_agent".to_string();
    let project = "cleanup_project".to_string();

    // Add data to all scopes
    memory
        .set(
            MemoryScope::User,
            "user_key".to_string(),
            json!(1),
            agent.clone(),
            None,
        )
        .await
        .unwrap();

    memory
        .set(
            MemoryScope::Project,
            "project_key".to_string(),
            json!(2),
            agent.clone(),
            Some(project.clone()),
        )
        .await
        .unwrap();

    memory
        .set(
            MemoryScope::Local,
            "local_key".to_string(),
            json!(3),
            agent.clone(),
            None,
        )
        .await
        .unwrap();

    // Clear local scope
    memory
        .clear(MemoryScope::Local, &agent, None)
        .await
        .unwrap();

    // Local should be empty, others should still have data
    assert_eq!(
        memory
            .list_keys(MemoryScope::Local, &agent, None)
            .await
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        memory
            .list_keys(MemoryScope::User, &agent, None)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        memory
            .list_keys(MemoryScope::Project, &agent, Some(&project))
            .await
            .unwrap()
            .len(),
        1
    );
}
