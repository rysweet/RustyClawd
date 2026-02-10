//! TaskDelete tool - Soft delete a task by ID

use crate::task::state::TaskStore;
use crate::task::types::TaskId;
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for TaskDelete tool
#[derive(Debug, Deserialize)]
pub struct TaskDeleteParams {
    /// Task ID to delete
    pub id: TaskId,
}

/// Output from TaskDelete tool
#[derive(Debug, Serialize)]
pub struct TaskDeleteOutput {
    /// The ID of the deleted task
    pub id: TaskId,

    /// Success message
    pub message: String,
}

/// The TaskDelete tool
pub struct TaskDeleteTool;

#[async_trait]
impl crate::Tool for TaskDeleteTool {
    type Params = TaskDeleteParams;
    type Output = TaskDeleteOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "TaskDelete",
            description: "Soft delete a task by ID",
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
                step: "Deleting task...".to_string(),
                percentage: Some(50.0),
            };

            if debug {
                tracing::debug!("TaskDelete: Deleting task id={}", params.id);
            }

            match TaskStore::delete(&params.id) {
                Ok(()) => {
                    let message = format!("Task deleted successfully: {}", params.id);

                    if debug {
                        tracing::debug!("TaskDelete: {}", message);
                    }

                    yield ToolEvent::Result(TaskDeleteOutput {
                        id: params.id,
                        message,
                    });
                }
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to delete task: {}", e),
                    };
                }
            }
        }))
    }

    fn is_read_only(&self) -> bool {
        false // Modifies task state
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
    use crate::task::types::Task;
    use crate::Tool;
    use futures::StreamExt;
    use serial_test::serial;

    async fn create_test_task() -> Task {
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
    async fn test_delete_task() {
        TaskStore::clear();

        let task = create_test_task().await;
        let task_id = task.id;

        let tool = TaskDeleteTool;
        let params = TaskDeleteParams { id: task_id };
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

        assert_eq!(result.id, task_id);

        // Verify task is no longer in normal list
        let tasks = TaskStore::list();
        assert!(tasks.iter().all(|t| t.id != task_id));

        // But is in list_all as deleted
        let all = TaskStore::list_all();
        let deleted = all.iter().find(|t| t.id == task_id).unwrap();
        assert!(deleted.deleted);
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_nonexistent_task() {
        TaskStore::clear();

        let tool = TaskDeleteTool;
        let params = TaskDeleteParams { id: TaskId::new() };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_already_deleted_task() {
        TaskStore::clear();

        let task = create_test_task().await;
        let task_id = task.id;

        // Delete once
        TaskStore::delete(&task_id).unwrap();

        // Try to delete again via tool
        let tool = TaskDeleteTool;
        let params = TaskDeleteParams { id: task_id };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
    }
}
