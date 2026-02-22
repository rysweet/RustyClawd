use super::*;
use crate::Tool;
use futures::StreamExt;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn test_todowrite_valid_list() {
    clear_todos();
    let tool = TodoWriteTool;
    let params = TodoWriteParams {
        todos: vec![
            Task {
                content: "Task 1".to_string(),
                status: TaskStatus::Completed,
                active_form: "Completing task 1".to_string(),
            },
            Task {
                content: "Task 2".to_string(),
                status: TaskStatus::InProgress,
                active_form: "Doing task 2".to_string(),
            },
            Task {
                content: "Task 3".to_string(),
                status: TaskStatus::Pending,
                active_form: "Will do task 3".to_string(),
            },
        ],
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events
        .iter()
        .find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        })
        .unwrap();

    assert_eq!(result.tasks_written, 3);
    assert_eq!(result.pending, 1);
    assert_eq!(result.in_progress, 1);
    assert_eq!(result.completed, 1);
}

#[serial]
#[tokio::test]
async fn test_todowrite_invalid_multiple_in_progress() {
    clear_todos();
    let tool = TodoWriteTool;
    let params = TodoWriteParams {
        todos: vec![
            Task {
                content: "Task 1".to_string(),
                status: TaskStatus::InProgress,
                active_form: "Doing 1".to_string(),
            },
            Task {
                content: "Task 2".to_string(),
                status: TaskStatus::InProgress, // Invalid!
                active_form: "Doing 2".to_string(),
            },
        ],
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    // Should have error event
    let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
    assert!(has_error);
}

#[serial]
#[tokio::test]
async fn test_todowrite_invalid_no_in_progress() {
    clear_todos();
    let tool = TodoWriteTool;
    let params = TodoWriteParams {
        todos: vec![
            Task {
                content: "Task 1".to_string(),
                status: TaskStatus::Pending,
                active_form: "Will do 1".to_string(),
            },
            Task {
                content: "Task 2".to_string(),
                status: TaskStatus::Completed,
                active_form: "Did 2".to_string(),
            },
        ],
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    // Should have error event
    let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
    assert!(has_error);
}

#[serial]
#[tokio::test]
async fn test_todowrite_session_state_persistence() {
    clear_todos();
    let tool = TodoWriteTool;

    // First call - write some todos
    let params1 = TodoWriteParams {
        todos: vec![
            Task {
                content: "Task 1".to_string(),
                status: TaskStatus::InProgress,
                active_form: "Doing task 1".to_string(),
            },
            Task {
                content: "Task 2".to_string(),
                status: TaskStatus::Pending,
                active_form: "Will do task 2".to_string(),
            },
        ],
    };
    let ctx = ToolContext::default();
    let stream = tool.execute(params1, &ctx).await.unwrap();
    let _: Vec<_> = stream.collect().await;

    // Verify state was stored
    let stored = get_todos();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].content, "Task 1");
    assert_eq!(stored[1].content, "Task 2");

    // Second call - update todos
    let params2 = TodoWriteParams {
        todos: vec![
            Task {
                content: "Task 1".to_string(),
                status: TaskStatus::Completed,
                active_form: "Completed task 1".to_string(),
            },
            Task {
                content: "Task 2".to_string(),
                status: TaskStatus::InProgress,
                active_form: "Doing task 2".to_string(),
            },
            Task {
                content: "Task 3".to_string(),
                status: TaskStatus::Pending,
                active_form: "Will do task 3".to_string(),
            },
        ],
    };
    let stream = tool.execute(params2, &ctx).await.unwrap();
    let _: Vec<_> = stream.collect().await;

    // Verify state was updated
    let stored = get_todos();
    assert_eq!(stored.len(), 3);
    assert_eq!(stored[0].status, TaskStatus::Completed);
    assert_eq!(stored[1].status, TaskStatus::InProgress);
    assert_eq!(stored[2].status, TaskStatus::Pending);
}

#[serial]
#[tokio::test]
async fn test_todowrite_all_pending_except_one() {
    clear_todos();
    let tool = TodoWriteTool;
    let params = TodoWriteParams {
        todos: vec![
            Task {
                content: "Current task".to_string(),
                status: TaskStatus::InProgress,
                active_form: "Doing current task".to_string(),
            },
            Task {
                content: "Future 1".to_string(),
                status: TaskStatus::Pending,
                active_form: "Will do future 1".to_string(),
            },
            Task {
                content: "Future 2".to_string(),
                status: TaskStatus::Pending,
                active_form: "Will do future 2".to_string(),
            },
        ],
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events
        .iter()
        .find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        })
        .unwrap();

    assert_eq!(result.pending, 2);
    assert_eq!(result.in_progress, 1);
    assert_eq!(result.completed, 0);
}

#[serial]
#[tokio::test]
async fn test_todowrite_all_completed_except_one() {
    clear_todos();
    let tool = TodoWriteTool;
    let params = TodoWriteParams {
        todos: vec![
            Task {
                content: "Done 1".to_string(),
                status: TaskStatus::Completed,
                active_form: "Completed 1".to_string(),
            },
            Task {
                content: "Done 2".to_string(),
                status: TaskStatus::Completed,
                active_form: "Completed 2".to_string(),
            },
            Task {
                content: "Current".to_string(),
                status: TaskStatus::InProgress,
                active_form: "Doing current".to_string(),
            },
        ],
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events
        .iter()
        .find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        })
        .unwrap();

    assert_eq!(result.pending, 0);
    assert_eq!(result.in_progress, 1);
    assert_eq!(result.completed, 2);
}

#[serial]
#[tokio::test]
async fn test_todowrite_single_task() {
    clear_todos();
    let tool = TodoWriteTool;
    let params = TodoWriteParams {
        todos: vec![Task {
            content: "Only task".to_string(),
            status: TaskStatus::InProgress,
            active_form: "Doing only task".to_string(),
        }],
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events
        .iter()
        .find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        })
        .unwrap();

    assert_eq!(result.tasks_written, 1);
    assert_eq!(result.in_progress, 1);
}

#[serial]
#[tokio::test]
async fn test_todowrite_empty_list_invalid() {
    clear_todos();
    let tool = TodoWriteTool;
    let params = TodoWriteParams { todos: vec![] };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    // Should have error - no in_progress task
    let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
    assert!(has_error);
}

#[serial]
#[tokio::test]
async fn test_todowrite_large_task_list() {
    clear_todos();
    let tool = TodoWriteTool;

    // Create 50 tasks - 1 in progress, 25 pending, 24 completed
    let mut todos = vec![];
    for i in 0..24 {
        todos.push(Task {
            content: format!("Completed task {}", i),
            status: TaskStatus::Completed,
            active_form: format!("Completed {}", i),
        });
    }
    todos.push(Task {
        content: "Current task".to_string(),
        status: TaskStatus::InProgress,
        active_form: "Doing current".to_string(),
    });
    for i in 0..25 {
        todos.push(Task {
            content: format!("Future task {}", i),
            status: TaskStatus::Pending,
            active_form: format!("Will do {}", i),
        });
    }

    let params = TodoWriteParams { todos };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events
        .iter()
        .find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        })
        .unwrap();

    assert_eq!(result.tasks_written, 50);
    assert_eq!(result.pending, 25);
    assert_eq!(result.in_progress, 1);
    assert_eq!(result.completed, 24);
}

#[serial]
#[tokio::test]
async fn test_todowrite_output_message_format() {
    clear_todos();
    let tool = TodoWriteTool;
    let params = TodoWriteParams {
        todos: vec![Task {
            content: "Task".to_string(),
            status: TaskStatus::InProgress,
            active_form: "Doing task".to_string(),
        }],
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events
        .iter()
        .find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        })
        .unwrap();

    // Verify message format matches expected pattern
    assert!(result
        .message
        .contains("Todos have been modified successfully"));
    assert!(result.message.contains("0 pending"));
    assert!(result.message.contains("1 in progress"));
    assert!(result.message.contains("0 completed"));
}

#[serial]
#[tokio::test]
async fn test_todowrite_progress_events() {
    clear_todos();
    let tool = TodoWriteTool;
    let params = TodoWriteParams {
        todos: vec![Task {
            content: "Task".to_string(),
            status: TaskStatus::InProgress,
            active_form: "Doing task".to_string(),
        }],
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    // Should have progress events
    let has_validation_progress = events
        .iter()
        .any(|e| matches!(e, ToolEvent::Progress { step, .. } if step.contains("Validating")));
    let has_writing_progress = events
        .iter()
        .any(|e| matches!(e, ToolEvent::Progress { step, .. } if step.contains("Writing")));

    assert!(has_validation_progress);
    assert!(has_writing_progress);
}

#[serial]
#[test]
fn test_get_set_todos_api() {
    clear_todos();

    // Initially empty
    assert_eq!(get_todos().len(), 0);

    // Set some todos
    let todos = vec![Task {
        content: "Test".to_string(),
        status: TaskStatus::InProgress,
        active_form: "Testing".to_string(),
    }];
    set_todos(todos.clone());

    // Verify they were stored
    let retrieved = get_todos();
    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].content, "Test");

    // Clean up for other tests
    clear_todos();
}

#[test]
fn test_task_status_serialization() {
    // Test that TaskStatus serializes to lowercase strings
    let pending = TaskStatus::Pending;
    let in_progress = TaskStatus::InProgress;
    let completed = TaskStatus::Completed;

    let pending_json = serde_json::to_string(&pending).unwrap();
    let in_progress_json = serde_json::to_string(&in_progress).unwrap();
    let completed_json = serde_json::to_string(&completed).unwrap();

    assert_eq!(pending_json, "\"pending\"");
    assert_eq!(in_progress_json, "\"in_progress\""); // Note: serde(rename_all = "snake_case") keeps underscore
    assert_eq!(completed_json, "\"completed\"");
}
