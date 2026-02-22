//! TodoWrite tool - Manage task lists with session-scoped in-memory state
//!
//! Demonstrates:
//! - Session-scoped state management
//! - Task state validation
//! - Structured task management
//! - Atomic state updates

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock, RwLock};

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
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

/// Global session-scoped todo state
///
/// This state persists across TodoWrite tool calls within the same session,
/// providing continuity for task tracking. The state is in-memory and
/// session-scoped (not persisted between CLI invocations).
static TODO_STATE: OnceLock<Arc<RwLock<Vec<Task>>>> = OnceLock::new();

/// Get or initialize the global todo state
fn get_state() -> &'static Arc<RwLock<Vec<Task>>> {
    TODO_STATE.get_or_init(|| Arc::new(RwLock::new(Vec::new())))
}

/// Get the current todo list from session state
pub fn get_todos() -> Vec<Task> {
    get_state().read().unwrap().clone()
}

/// Set the todo list in session state
pub fn set_todos(todos: Vec<Task>) {
    let mut state = get_state().write().unwrap();
    *state = todos;
}

/// Clear the todo list (useful for testing)
#[cfg(test)]
pub fn clear_todos() {
    let mut state = get_state().write().unwrap();
    state.clear();
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

            // Store in session state
            set_todos(todos.clone());

            if debug {
                tracing::debug!(
                    "TodoWrite: Stored {} tasks (pending: {}, in_progress: {}, completed: {})",
                    todos.len(), pending, in_progress, completed
                );
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
#[path = "todo_write_tests.rs"]
mod tests;
