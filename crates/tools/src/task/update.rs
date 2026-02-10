//! TaskUpdate tool - Update existing tasks

use crate::task::state::TaskStore;
use crate::task::types::{Task, TaskDependencies, TaskId, TaskStatus};
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for TaskUpdate tool
#[derive(Debug, Deserialize)]
pub struct TaskUpdateParams {
    /// Task ID to update
    pub id: TaskId,

    /// New content (optional)
    pub content: Option<String>,

    /// New active form (optional)
    #[serde(rename = "activeForm")]
    pub active_form: Option<String>,

    /// New status (optional)
    pub status: Option<TaskStatus>,

    /// New dependencies (optional, replaces existing)
    pub dependencies: Option<TaskDependencies>,
}

/// Output from TaskUpdate tool
#[derive(Debug, Serialize)]
pub struct TaskUpdateOutput {
    /// The updated task
    pub task: Task,

    /// Success message
    pub message: String,
}

/// The TaskUpdate tool
pub struct TaskUpdateTool;

#[async_trait]
impl crate::Tool for TaskUpdateTool {
    type Params = TaskUpdateParams;
    type Output = TaskUpdateOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "TaskUpdate",
            description: "Update an existing task",
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
                step: "Fetching task...".to_string(),
                percentage: Some(20.0),
            };

            // Get existing task
            let mut task = match TaskStore::get(&params.id) {
                Ok(t) => t,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to fetch task: {}", e),
                    };
                    return;
                }
            };

            if debug {
                tracing::debug!("TaskUpdate: Updating task id={}", task.id);
            }

            yield ToolEvent::Progress {
                step: "Updating fields...".to_string(),
                percentage: Some(40.0),
            };

            // Update fields if provided
            if let Some(content) = params.content {
                task.content = content;
            }
            if let Some(active_form) = params.active_form {
                task.active_form = active_form;
            }
            if let Some(status) = params.status {
                task.status = status;
            }
            if let Some(deps) = params.dependencies {
                task.dependencies = deps;
            }

            yield ToolEvent::Progress {
                step: "Validating dependencies...".to_string(),
                percentage: Some(70.0),
            };

            // Update task (validates dependencies)
            match TaskStore::update(task) {
                Ok(updated_task) => {
                    let message = format!("Task updated successfully: {}", updated_task.id);

                    if debug {
                        tracing::debug!("TaskUpdate: {}", message);
                    }

                    yield ToolEvent::Result(TaskUpdateOutput {
                        task: updated_task,
                        message,
                    });
                }
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to update task: {}", e),
                    };
                }
            }
        }))
    }

    fn is_read_only(&self) -> bool {
        false // Updates task state
    }

    fn is_concurrency_safe(&self) -> bool {
        false // Writing task state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::create::{TaskCreateParams, TaskCreateTool};
    use crate::task::state::TaskStore;
    use crate::Tool;
    use futures::StreamExt;
    use serial_test::serial;

    async fn create_test_task() -> Task {
        let tool = TaskCreateTool;
        let params = TaskCreateParams {
            content: "Original content".to_string(),
            active_form: "Working".to_string(),
            status: None,
            dependencies: None,
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output.task.clone()),
                _ => None,
            })
            .unwrap()
    }

    #[tokio::test]
    #[serial]
    async fn test_update_task_content() {
        TaskStore::clear();

        let task = create_test_task().await;
        let task_id = task.id;

        let tool = TaskUpdateTool;
        let params = TaskUpdateParams {
            id: task_id,
            content: Some("Updated content".to_string()),
            active_form: None,
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

        assert_eq!(result.task.content, "Updated content");
        assert_eq!(result.task.active_form, "Working"); // Unchanged
    }

    #[tokio::test]
    #[serial]
    async fn test_update_task_status() {
        TaskStore::clear();

        let task = create_test_task().await;
        let task_id = task.id;

        let tool = TaskUpdateTool;
        let params = TaskUpdateParams {
            id: task_id,
            content: None,
            active_form: None,
            status: Some(TaskStatus::Completed),
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

        assert_eq!(result.task.status, TaskStatus::Completed);
    }

    #[tokio::test]
    #[serial]
    async fn test_update_task_dependencies() {
        TaskStore::clear();

        let task1 = create_test_task().await;
        let task2 = create_test_task().await;

        let mut deps = TaskDependencies::new();
        deps.add_blocked_by(task1.id);

        let tool = TaskUpdateTool;
        let params = TaskUpdateParams {
            id: task2.id,
            content: None,
            active_form: None,
            status: None,
            dependencies: Some(deps),
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

        assert!(result.task.dependencies.is_blocked_by(&task1.id));
    }

    #[tokio::test]
    #[serial]
    async fn test_update_nonexistent_task() {
        TaskStore::clear();

        let tool = TaskUpdateTool;
        let params = TaskUpdateParams {
            id: TaskId::new(),
            content: Some("Updated".to_string()),
            active_form: None,
            status: None,
            dependencies: None,
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should have error event
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
    }

    #[tokio::test]
    #[serial]
    async fn test_update_multiple_fields() {
        TaskStore::clear();

        let task = create_test_task().await;
        let task_id = task.id;

        let tool = TaskUpdateTool;
        let params = TaskUpdateParams {
            id: task_id,
            content: Some("New content".to_string()),
            active_form: Some("New form".to_string()),
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

        assert_eq!(result.task.content, "New content");
        assert_eq!(result.task.active_form, "New form");
        assert_eq!(result.task.status, TaskStatus::InProgress);
    }

    #[test]
    fn test_params_deserialization_partial() {
        // Test partial update with only content
        let task_id = TaskId::new();
        let json = serde_json::json!({
            "id": task_id,
            "content": "Updated content"
        });

        let params: TaskUpdateParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.id, task_id);
        assert_eq!(params.content, Some("Updated content".to_string()));
        assert!(params.active_form.is_none());
        assert!(params.status.is_none());
        assert!(params.dependencies.is_none());
    }

    #[test]
    fn test_params_deserialization_active_form_rename() {
        // Verify the serde rename attribute works correctly
        let task_id = TaskId::new();
        let json_with_camel = serde_json::json!({
            "id": task_id,
            "activeForm": "Updated form"
        });

        let params: TaskUpdateParams = serde_json::from_value(json_with_camel).unwrap();
        assert_eq!(params.active_form, Some("Updated form".to_string()));

        // Verify snake_case fails (only camelCase works with rename)
        let json_with_snake = serde_json::json!({
            "id": task_id,
            "active_form": "Updated form"
        });

        let params: TaskUpdateParams = serde_json::from_value(json_with_snake).unwrap();
        assert!(
            params.active_form.is_none(),
            "snake_case should be ignored by serde rename"
        );
    }

    #[test]
    fn test_params_deserialization_all_optional_fields() {
        // Test that omitting all optional fields works
        let task_id = TaskId::new();
        let json = serde_json::json!({
            "id": task_id
        });

        let params: TaskUpdateParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.id, task_id);
        assert!(params.content.is_none());
        assert!(params.active_form.is_none());
        assert!(params.status.is_none());
        assert!(params.dependencies.is_none());
    }
}
