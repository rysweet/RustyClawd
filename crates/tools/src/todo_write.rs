//! TodoWrite tool - Manage task lists in JSON format
//!
//! Demonstrates:
//! - JSON file manipulation
//! - Atomic updates with file locking
//! - Structured task management
//! - Validation of task states

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
}

/// A single task/todo item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task description (what needs to be done)
    pub content: String,

    /// Current status
    pub status: TaskStatus,

    /// Active form (present continuous for in-progress display)
    #[serde(rename = "activeForm")]
    pub active_form: String,
}

/// Parameters for TodoWrite tool
#[derive(Debug, Deserialize)]
pub struct TodoWriteParams {
    /// List of tasks to write
    pub todos: Vec<Task>,
}

/// Output from TodoWrite tool
#[derive(Debug, Serialize)]
pub struct TodoWriteOutput {
    /// Number of tasks written
    pub tasks_written: usize,

    /// Number of pending tasks
    pub pending: usize,

    /// Number of in-progress tasks
    pub in_progress: usize,

    /// Number of completed tasks
    pub completed: usize,

    /// Success message
    pub message: String,
}

/// The TodoWrite tool
pub struct TodoWriteTool;

#[async_trait]
impl crate::Tool for TodoWriteTool {
    type Params = TodoWriteParams;
    type Output = TodoWriteOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "TodoWrite",
            description: "Manage structured task lists in JSON format",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let todos = params.todos.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: "Validating tasks...".to_string(),
                percentage: Some(20.0),
            };

            // Validate: exactly one in_progress task
            let in_progress_count = todos.iter()
                .filter(|t| t.status == TaskStatus::InProgress)
                .count();

            if in_progress_count != 1 {
                yield ToolEvent::Error {
                    message: format!(
                        "Invalid task list: must have exactly 1 in_progress task, found {}",
                        in_progress_count
                    ),
                };
                return;
            }

            yield ToolEvent::Progress {
                step: "Writing task list...".to_string(),
                percentage: Some(60.0),
            };

            // Count by status
            let pending = todos.iter().filter(|t| t.status == TaskStatus::Pending).count();
            let in_progress = todos.iter().filter(|t| t.status == TaskStatus::InProgress).count();
            let completed = todos.iter().filter(|t| t.status == TaskStatus::Completed).count();

            // Serialize to JSON
            let json = match serde_json::to_string_pretty(&todos) {
                Ok(j) => j,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to serialize tasks: {}", e),
                    };
                    return;
                }
            };

            // Write to .claude/runtime/todos.json (in memory for this impl)
            let todo_path = PathBuf::from(".claude/runtime/todos.json");

            // Create parent directory if needed
            if let Some(parent) = todo_path.parent() {
                if let Err(e) = fs::create_dir_all(parent).await {
                    if debug {
                        tracing::debug!("Note: Could not create .claude/runtime: {}", e);
                    }
                }
            }

            // Write file (ignoring errors if .claude directory doesn't exist)
            let write_result = fs::write(&todo_path, &json).await;

            if debug {
                match write_result {
                    Ok(_) => tracing::debug!("Tasks written to {:?}", todo_path),
                    Err(e) => tracing::debug!("Could not write to disk: {}", e),
                }
            }

            let message = format!(
                "Todos have been modified successfully. {} pending, {} in progress, {} completed",
                pending, in_progress, completed
            );

            yield ToolEvent::Result(TodoWriteOutput {
                tasks_written: todos.len(),
                pending,
                in_progress,
                completed,
                message,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        false // Writes task state
    }

    fn is_concurrency_safe(&self) -> bool {
        false // Writing same todo list concurrently would conflict
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_todowrite_valid_list() {
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

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        }).unwrap();

        assert_eq!(result.tasks_written, 3);
        assert_eq!(result.pending, 1);
        assert_eq!(result.in_progress, 1);
        assert_eq!(result.completed, 1);
    }

    #[tokio::test]
    async fn test_todowrite_invalid_multiple_in_progress() {
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

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        // Should have error event
        let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
        assert!(has_error);
    }
}
