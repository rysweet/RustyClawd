//! TaskGet tool - Get a single task by ID

use crate::task::state::TaskStore;
use crate::task::types::{Task, TaskId};
use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for TaskGet tool
#[derive(Debug, Deserialize)]
pub struct TaskGetParams {
    /// Task ID to retrieve
    pub id: TaskId,
}

/// Output from TaskGet tool
#[derive(Debug, Serialize)]
pub struct TaskGetOutput {
    /// The retrieved task
    pub task: Task,
}

/// The TaskGet tool
pub struct TaskGetTool;

#[async_trait]
impl crate::Tool for TaskGetTool {
    type Params = TaskGetParams;
    type Output = TaskGetOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "TaskGet",
            description: "Get a single task by ID",
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
                percentage: Some(50.0),
            };

            if debug {
                tracing::debug!("TaskGet: Fetching task id={}", params.id);
            }

            match TaskStore::get(&params.id) {
                Ok(task) => {
                    if debug {
                        tracing::debug!("TaskGet: Found task id={}", task.id);
                    }

                    yield ToolEvent::Result(TaskGetOutput { task });
                }
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to get task: {}", e),
                    };
                }
            }
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
    async fn test_get_task() {
        TaskStore::clear();

        let created = create_test_task().await;

        let tool = TaskGetTool;
        let params = TaskGetParams { id: created.id };
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

        assert_eq!(result.task.id, created.id);
        assert_eq!(result.task.content, "Test task");
    }

    #[tokio::test]
    #[serial]
    async fn test_get_nonexistent_task() {
        TaskStore::clear();

        let tool = TaskGetTool;
        let params = TaskGetParams { id: TaskId::new() };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should have error event
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
    }
}
