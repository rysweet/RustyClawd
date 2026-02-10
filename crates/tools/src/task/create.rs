//! TaskCreate tool - Create new tasks with dependencies

use crate::task::state::TaskStore;
use crate::task::types::{Task, TaskDependencies, TaskStatus};
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for TaskCreate tool
#[derive(Debug, Deserialize)]
pub struct TaskCreateParams {
    /// Task description
    pub content: String,

    /// Active form (present continuous)
    #[serde(rename = "activeForm")]
    pub active_form: String,

    /// Initial status (defaults to Pending)
    #[serde(default)]
    pub status: Option<TaskStatus>,

    /// Task dependencies
    #[serde(default)]
    pub dependencies: Option<TaskDependencies>,
}

/// Output from TaskCreate tool
#[derive(Debug, Serialize)]
pub struct TaskCreateOutput {
    /// The created task
    pub task: Task,

    /// Success message
    pub message: String,
}

/// The TaskCreate tool
pub struct TaskCreateTool;

#[async_trait]
impl crate::Tool for TaskCreateTool {
    type Params = TaskCreateParams;
    type Output = TaskCreateOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "TaskCreate",
            description: "Create a new task with optional dependencies",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: "Creating task...".to_string(),
                percentage: Some(30.0),
            };

            // Create task
            let mut task = Task::new(params.content.clone(), params.active_form.clone());

            // Set status if provided
            if let Some(status) = params.status {
                task.status = status;
            }

            // Set dependencies if provided
            if let Some(deps) = params.dependencies {
                task.dependencies = deps;
            }

            if debug {
                tracing::debug!("TaskCreate: Creating task with id={}", task.id);
            }

            yield ToolEvent::Progress {
                step: "Validating dependencies...".to_string(),
                percentage: Some(60.0),
            };

            // Store task (validates dependencies)
            match TaskStore::create(task) {
                Ok(created_task) => {
                    let message = format!("Task created successfully: {}", created_task.id);

                    if debug {
                        tracing::debug!("TaskCreate: {}", message);
                    }

                    yield ToolEvent::Result(TaskCreateOutput {
                        task: created_task,
                        message,
                    });
                }
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to create task: {}", e),
                    };
                }
            }
        }))
    }

    fn is_read_only(&self) -> bool {
        false // Creates task state
    }

    fn is_concurrency_safe(&self) -> bool {
        false // Writing task state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::state::TaskStore;
    use crate::task::types::TaskId;
    use crate::Tool;
    use futures::StreamExt;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_create_simple_task() {
        TaskStore::clear();

        let tool = TaskCreateTool;
        let params = TaskCreateParams {
            content: "Test task".to_string(),
            active_form: "Testing".to_string(),
            status: None,
            dependencies: None,
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

        assert_eq!(result.task.content, "Test task");
        assert_eq!(result.task.status, TaskStatus::Pending);
    }

    #[tokio::test]
    #[serial]
    async fn test_create_task_with_status() {
        TaskStore::clear();

        let tool = TaskCreateTool;
        let params = TaskCreateParams {
            content: "In progress task".to_string(),
            active_form: "Working".to_string(),
            status: Some(TaskStatus::InProgress),
            dependencies: None,
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

        assert_eq!(result.task.status, TaskStatus::InProgress);
    }

    #[tokio::test]
    #[serial]
    async fn test_create_task_with_dependencies() {
        TaskStore::clear();

        // Create first task
        let tool = TaskCreateTool;
        let params1 = TaskCreateParams {
            content: "Task 1".to_string(),
            active_form: "Working 1".to_string(),
            status: None,
            dependencies: None,
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params1, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let task1 = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output.task.clone()),
                _ => None,
            })
            .unwrap();

        // Create second task that depends on first
        let mut deps = TaskDependencies::new();
        deps.add_blocked_by(task1.id);

        let params2 = TaskCreateParams {
            content: "Task 2".to_string(),
            active_form: "Working 2".to_string(),
            status: None,
            dependencies: Some(deps),
        };

        let stream = tool.execute(params2, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert!(result.task.dependencies.is_blocked_by(&task1.id));
    }

    #[tokio::test]
    #[serial]
    async fn test_create_task_with_invalid_dependency() {
        TaskStore::clear();

        let tool = TaskCreateTool;

        // Try to create task with non-existent dependency
        let mut deps = TaskDependencies::new();
        deps.add_blocked_by(TaskId::new());

        let params = TaskCreateParams {
            content: "Task".to_string(),
            active_form: "Working".to_string(),
            status: None,
            dependencies: Some(deps),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should have error event
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
    }

    #[test]
    fn test_params_deserialization_full() {
        // Test full JSON with all fields
        let json = serde_json::json!({
            "content": "Fix bug",
            "activeForm": "Fixing bug",
            "status": "in_progress",
            "dependencies": {
                "blocks": [],
                "blocked_by": []
            }
        });

        let params: TaskCreateParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.content, "Fix bug");
        assert_eq!(params.active_form, "Fixing bug");
        assert_eq!(params.status, Some(TaskStatus::InProgress));
        assert!(params.dependencies.is_some());
    }

    #[test]
    fn test_params_deserialization_minimal() {
        // Test minimal JSON with only required fields
        let json = serde_json::json!({
            "content": "Fix bug",
            "activeForm": "Fixing bug"
        });

        let params: TaskCreateParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.content, "Fix bug");
        assert_eq!(params.active_form, "Fixing bug");
        assert!(params.status.is_none());
        assert!(params.dependencies.is_none());
    }

    #[test]
    fn test_params_deserialization_active_form_rename() {
        // Verify the serde rename attribute works correctly
        let json_with_camel = serde_json::json!({
            "content": "Task",
            "activeForm": "Working"
        });

        let params: TaskCreateParams = serde_json::from_value(json_with_camel).unwrap();
        assert_eq!(params.active_form, "Working");

        // Verify snake_case fails (wrong format)
        let json_with_snake = serde_json::json!({
            "content": "Task",
            "active_form": "Working"
        });

        let result: Result<TaskCreateParams, _> = serde_json::from_value(json_with_snake);
        assert!(result.is_err(), "Should fail with snake_case field name");
    }
}
