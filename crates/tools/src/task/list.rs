//! TaskList tool - List all tasks

use crate::task::state::TaskStore;
use crate::task::types::{Task, TaskStatus};
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for TaskList tool
#[derive(Debug, Deserialize, Default)]
pub struct TaskListParams {
    /// Filter by status (optional)
    pub status: Option<TaskStatus>,

    /// Include deleted tasks (default: false)
    #[serde(default)]
    pub include_deleted: bool,
}

/// Output from TaskList tool
#[derive(Debug, Serialize)]
pub struct TaskListOutput {
    /// List of tasks
    pub tasks: Vec<Task>,

    /// Number of tasks returned
    pub count: usize,

    /// Message
    pub message: String,
}

/// The TaskList tool
pub struct TaskListTool;

#[async_trait]
impl crate::Tool for TaskListTool {
    type Params = TaskListParams;
    type Output = TaskListOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "TaskList",
            description: "List all tasks with optional filtering",
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
                step: "Fetching tasks...".to_string(),
                percentage: Some(30.0),
            };

            if debug {
                tracing::debug!(
                    "TaskList: Fetching tasks (status={:?}, include_deleted={})",
                    params.status,
                    params.include_deleted
                );
            }

            // Get tasks
            let mut tasks = if params.include_deleted {
                TaskStore::list_all()
            } else {
                TaskStore::list()
            };

            yield ToolEvent::Progress {
                step: "Filtering tasks...".to_string(),
                percentage: Some(60.0),
            };

            // Filter by status if specified
            if let Some(status) = params.status {
                tasks.retain(|t| t.status == status);
            }

            let count = tasks.len();
            let message = match params.status {
                Some(status) => format!("Found {} task(s) with status {:?}", count, status),
                None => format!("Found {} task(s)", count),
            };

            if debug {
                tracing::debug!("TaskList: {}", message);
            }

            yield ToolEvent::Result(TaskListOutput {
                tasks,
                count,
                message,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true // Only reads task state
    }

    fn is_concurrency_safe(&self) -> bool {
        true // Read-only operation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::create::{TaskCreateParams, TaskCreateTool};
    use crate::task::state::TaskStore;
    use crate::task::types::TaskStatus;
    use crate::Tool;
    use futures::StreamExt;
    use serial_test::serial;

    async fn create_task_with_status(status: TaskStatus) -> Task {
        let tool = TaskCreateTool;
        let params = TaskCreateParams {
            content: format!("Task {:?}", status),
            active_form: format!("Working {:?}", status),
            status: Some(status),
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
    async fn test_list_all_tasks() {
        TaskStore::clear();

        create_task_with_status(TaskStatus::Pending).await;
        create_task_with_status(TaskStatus::InProgress).await;
        create_task_with_status(TaskStatus::Completed).await;

        let tool = TaskListTool;
        let params = TaskListParams::default();
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

        assert_eq!(result.count, 3);
        assert_eq!(result.tasks.len(), 3);
    }

    #[tokio::test]
    #[serial]
    async fn test_list_filtered_by_status() {
        TaskStore::clear();

        create_task_with_status(TaskStatus::Pending).await;
        create_task_with_status(TaskStatus::Pending).await;
        create_task_with_status(TaskStatus::InProgress).await;

        let tool = TaskListTool;
        let params = TaskListParams {
            status: Some(TaskStatus::Pending),
            include_deleted: false,
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

        assert_eq!(result.count, 2);
        assert!(result.tasks.iter().all(|t| t.status == TaskStatus::Pending));
    }

    #[tokio::test]
    #[serial]
    async fn test_list_empty() {
        TaskStore::clear();

        let tool = TaskListTool;
        let params = TaskListParams::default();
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

        assert_eq!(result.count, 0);
        assert_eq!(result.tasks.len(), 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_list_excludes_deleted() {
        TaskStore::clear();

        let task = create_task_with_status(TaskStatus::Pending).await;

        // Delete the task
        TaskStore::delete(&task.id).unwrap();

        let tool = TaskListTool;
        let params = TaskListParams {
            status: None,
            include_deleted: false,
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

        // Should not include deleted task
        assert_eq!(result.count, 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_list_includes_deleted_when_requested() {
        TaskStore::clear();

        let task = create_task_with_status(TaskStatus::Pending).await;

        // Delete the task
        TaskStore::delete(&task.id).unwrap();

        let tool = TaskListTool;
        let params = TaskListParams {
            status: None,
            include_deleted: true,
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

        // Should include deleted task
        assert_eq!(result.count, 1);
        assert!(result.tasks[0].deleted);
    }
}
