use super::*;

#[tokio::test]
async fn test_registry_creation() {
    let registry = AgentRegistry::new();
    assert_eq!(registry.list_ids().await.len(), 0);
}

#[tokio::test]
async fn test_generate_id() {
    let id1 = AgentRegistry::generate_id("test");
    let id2 = AgentRegistry::generate_id("test");
    // IDs should have correct format
    assert!(id1.starts_with("agent_test_t"));
    assert!(id2.starts_with("agent_test_t"));
    // Sleep to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
    let id3 = AgentRegistry::generate_id("test");
    assert_ne!(id1, id3);
}

#[tokio::test]
async fn test_register_and_retrieve() {
    let registry = AgentRegistry::new();
    let id = "test_agent_123".to_string();

    // Register agent
    let result = registry
        .register(id.clone(), "builder".to_string(), "sonnet".to_string())
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), id);

    // Check exists
    assert!(registry.exists(&id).await);
    assert!(!registry.exists("nonexistent").await);

    // Get status
    let status = registry.get_status(&id).await.unwrap();
    assert!(matches!(status, AgentStatus::Running));
}

#[tokio::test]
async fn test_append_response() {
    let registry = AgentRegistry::new();
    let id = "test_agent_response".to_string();

    registry
        .register(id.clone(), "builder".to_string(), "sonnet".to_string())
        .await
        .unwrap();

    // Append some response
    registry
        .append_response(&id, "Hello ".to_string())
        .await
        .unwrap();
    registry
        .append_response(&id, "World!".to_string())
        .await
        .unwrap();

    // Get output
    let (response, status, _) = registry.get_output(&id).await.unwrap();
    assert_eq!(response, "Hello World!");
    assert_eq!(status, "running");
}

#[tokio::test]
async fn test_token_usage_update() {
    let registry = AgentRegistry::new();
    let id = "test_agent_tokens".to_string();

    registry
        .register(id.clone(), "builder".to_string(), "sonnet".to_string())
        .await
        .unwrap();

    // Update token usage
    registry.update_token_usage(&id, 100, 50).await.unwrap();

    // Get output and verify tokens
    let (_, _, tokens) = registry.get_output(&id).await.unwrap();
    assert_eq!(tokens.input_tokens, 100);
    assert_eq!(tokens.output_tokens, 50);
}

#[tokio::test]
async fn test_status_transitions() {
    let registry = AgentRegistry::new();
    let id = "test_agent_status".to_string();

    registry
        .register(id.clone(), "builder".to_string(), "sonnet".to_string())
        .await
        .unwrap();

    // Initial status
    let status = registry.get_status(&id).await.unwrap();
    assert!(matches!(status, AgentStatus::Running));

    // Mark completed
    registry.mark_completed(&id).await.unwrap();
    let status = registry.get_status(&id).await.unwrap();
    assert!(matches!(status, AgentStatus::Completed));

    // Verify output shows completed
    let (_, status_str, _) = registry.get_output(&id).await.unwrap();
    assert_eq!(status_str, "completed");
}

#[tokio::test]
async fn test_mark_failed() {
    let registry = AgentRegistry::new();
    let id = "test_agent_failed".to_string();

    registry
        .register(id.clone(), "builder".to_string(), "sonnet".to_string())
        .await
        .unwrap();

    // Mark failed
    registry
        .mark_failed(&id, "API timeout".to_string())
        .await
        .unwrap();

    // Verify status
    let (_, status_str, _) = registry.get_output(&id).await.unwrap();
    assert!(status_str.starts_with("failed:"));
    assert!(status_str.contains("API timeout"));
}

#[tokio::test]
async fn test_remove_agent() {
    let registry = AgentRegistry::new();
    let id = "test_agent_remove".to_string();

    registry
        .register(id.clone(), "builder".to_string(), "sonnet".to_string())
        .await
        .unwrap();

    assert!(registry.exists(&id).await);

    // Remove
    registry.remove(&id).await.unwrap();
    assert!(!registry.exists(&id).await);
}

#[tokio::test]
async fn test_list_ids() {
    let registry = AgentRegistry::new();

    registry
        .register(
            "agent_1".to_string(),
            "builder".to_string(),
            "sonnet".to_string(),
        )
        .await
        .unwrap();
    registry
        .register(
            "agent_2".to_string(),
            "tester".to_string(),
            "haiku".to_string(),
        )
        .await
        .unwrap();

    let ids = registry.list_ids().await;
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"agent_1".to_string()));
    assert!(ids.contains(&"agent_2".to_string()));
}

#[tokio::test]
async fn test_nonexistent_agent_errors() {
    let registry = AgentRegistry::new();

    // All operations on nonexistent agent should fail
    assert!(registry
        .append_response("fake", "text".to_string())
        .await
        .is_err());
    assert!(registry.update_token_usage("fake", 0, 0).await.is_err());
    assert!(registry.get_output("fake").await.is_err());
    assert!(registry.mark_completed("fake").await.is_err());
    assert!(registry
        .mark_failed("fake", "error".to_string())
        .await
        .is_err());
    assert!(registry.get_status("fake").await.is_err());
    assert!(registry.remove("fake").await.is_err());
}

#[tokio::test]
async fn test_task_completed_callback_fires() {
    let registry = AgentRegistry::new();
    let id = "callback_test_agent".to_string();

    // Track callback invocations
    let callback_fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let callback_agent_id = Arc::new(Mutex::new(String::new()));
    let fired_clone = Arc::clone(&callback_fired);
    let id_clone = Arc::clone(&callback_agent_id);

    registry
        .set_on_task_completed(Arc::new(move |info: AgentCompletionInfo| {
            fired_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            // Use try_lock since we're in a sync callback
            if let Ok(mut guard) = id_clone.try_lock() {
                *guard = info.agent_id.clone();
            }
        }))
        .await;

    registry
        .register(id.clone(), "builder".to_string(), "sonnet".to_string())
        .await
        .unwrap();

    // mark_completed should fire the callback
    registry.mark_completed(&id).await.unwrap();

    assert!(callback_fired.load(std::sync::atomic::Ordering::SeqCst));
    let stored_id = callback_agent_id.lock().await;
    assert_eq!(*stored_id, "callback_test_agent");
}

#[tokio::test]
async fn test_mark_completed_without_callback() {
    // Ensure mark_completed works fine when no callback is set
    let registry = AgentRegistry::new();
    let id = "no_callback_agent".to_string();

    registry
        .register(id.clone(), "builder".to_string(), "sonnet".to_string())
        .await
        .unwrap();

    // Should not panic or error
    registry.mark_completed(&id).await.unwrap();

    let status = registry.get_status(&id).await.unwrap();
    assert!(matches!(status, AgentStatus::Completed));
}
