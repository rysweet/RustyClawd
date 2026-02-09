//! Task types and data structures
//!
//! Defines core task types with dependency tracking support.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// Unique identifier for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(Uuid);

impl TaskId {
    /// Create a new random task ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a task ID from a UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is waiting to be started
    Pending,
    /// Task is currently being worked on
    InProgress,
    /// Task has been completed
    Completed,
    /// Task has been cancelled or blocked indefinitely
    Blocked,
}

/// Task dependencies - tracks blocking relationships
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskDependencies {
    /// Tasks that this task blocks (cannot start until this completes)
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub blocks: HashSet<TaskId>,

    /// Tasks that block this task (this cannot start until they complete)
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub blocked_by: HashSet<TaskId>,
}

impl TaskDependencies {
    /// Create empty dependencies
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if this task has any dependencies
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty() && self.blocked_by.is_empty()
    }

    /// Check if this task blocks another task
    pub fn blocks_task(&self, task_id: &TaskId) -> bool {
        self.blocks.contains(task_id)
    }

    /// Check if this task is blocked by another task
    pub fn is_blocked_by(&self, task_id: &TaskId) -> bool {
        self.blocked_by.contains(task_id)
    }

    /// Add a blocking relationship (this blocks other_task)
    pub fn add_blocks(&mut self, other_task: TaskId) {
        self.blocks.insert(other_task);
    }

    /// Remove a blocking relationship
    pub fn remove_blocks(&mut self, other_task: &TaskId) {
        self.blocks.remove(other_task);
    }

    /// Add a blocked-by relationship (this is blocked by other_task)
    pub fn add_blocked_by(&mut self, other_task: TaskId) {
        self.blocked_by.insert(other_task);
    }

    /// Remove a blocked-by relationship
    pub fn remove_blocked_by(&mut self, other_task: &TaskId) {
        self.blocked_by.remove(other_task);
    }
}

/// A task with dependencies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: TaskId,

    /// Task description (what needs to be done)
    pub content: String,

    /// Current status
    pub status: TaskStatus,

    /// Active form (present continuous for in-progress display)
    #[serde(rename = "activeForm")]
    pub active_form: String,

    /// Task dependencies
    #[serde(default, skip_serializing_if = "TaskDependencies::is_empty")]
    pub dependencies: TaskDependencies,

    /// Soft delete flag (true if task has been deleted)
    #[serde(default, skip_serializing_if = "is_false")]
    pub deleted: bool,
}

/// Helper function for serde skip_serializing_if
fn is_false(b: &bool) -> bool {
    !*b
}

impl Task {
    /// Create a new task
    pub fn new(content: String, active_form: String) -> Self {
        Self {
            id: TaskId::new(),
            content,
            status: TaskStatus::Pending,
            active_form,
            dependencies: TaskDependencies::new(),
            deleted: false,
        }
    }

    /// Create a new task with a specific ID (for testing)
    pub fn with_id(id: TaskId, content: String, active_form: String) -> Self {
        Self {
            id,
            content,
            status: TaskStatus::Pending,
            active_form,
            dependencies: TaskDependencies::new(),
            deleted: false,
        }
    }

    /// Check if task is deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    /// Mark task as deleted (soft delete)
    pub fn mark_deleted(&mut self) {
        self.deleted = true;
    }

    /// Check if task is blocked by incomplete dependencies
    pub fn is_blocked(&self, task_store: &[Task]) -> bool {
        // Task is blocked if any of its blocked_by dependencies are not completed or deleted
        self.dependencies.blocked_by.iter().any(|dep_id| {
            task_store
                .iter()
                .find(|t| &t.id == dep_id)
                .map(|t| !t.deleted && t.status != TaskStatus::Completed)
                .unwrap_or(false)
        })
    }

    /// Check if task can be started (not blocked)
    pub fn can_start(&self, task_store: &[Task]) -> bool {
        !self.is_blocked(task_store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_creation() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2); // Should be unique
    }

    #[test]
    fn test_task_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = TaskId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), &uuid);
    }

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test task".to_string(), "Testing".to_string());
        assert_eq!(task.content, "Test task");
        assert_eq!(task.active_form, "Testing");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(!task.deleted);
        assert!(task.dependencies.is_empty());
    }

    #[test]
    fn test_task_soft_delete() {
        let mut task = Task::new("Test".to_string(), "Testing".to_string());
        assert!(!task.is_deleted());

        task.mark_deleted();
        assert!(task.is_deleted());
    }

    #[test]
    fn test_dependencies_blocks() {
        let mut deps = TaskDependencies::new();
        let task_id = TaskId::new();

        assert!(!deps.blocks_task(&task_id));
        deps.add_blocks(task_id);
        assert!(deps.blocks_task(&task_id));

        deps.remove_blocks(&task_id);
        assert!(!deps.blocks_task(&task_id));
    }

    #[test]
    fn test_dependencies_blocked_by() {
        let mut deps = TaskDependencies::new();
        let task_id = TaskId::new();

        assert!(!deps.is_blocked_by(&task_id));
        deps.add_blocked_by(task_id);
        assert!(deps.is_blocked_by(&task_id));

        deps.remove_blocked_by(&task_id);
        assert!(!deps.is_blocked_by(&task_id));
    }

    #[test]
    fn test_task_is_blocked() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();

        let mut task1 = Task::with_id(id1, "Task 1".to_string(), "Working on 1".to_string());
        let task2 = Task::with_id(id2, "Task 2".to_string(), "Working on 2".to_string());

        // Task 1 is blocked by Task 2
        task1.dependencies.add_blocked_by(id2);

        let store = vec![task1.clone(), task2.clone()];

        // Task 1 should be blocked because Task 2 is pending
        assert!(task1.is_blocked(&store));
        assert!(!task1.can_start(&store));
    }

    #[test]
    fn test_task_not_blocked_when_dependency_completed() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();

        let mut task1 = Task::with_id(id1, "Task 1".to_string(), "Working on 1".to_string());
        let mut task2 = Task::with_id(id2, "Task 2".to_string(), "Working on 2".to_string());

        // Task 1 is blocked by Task 2
        task1.dependencies.add_blocked_by(id2);

        // Task 2 is completed
        task2.status = TaskStatus::Completed;

        let store = vec![task1.clone(), task2];

        // Task 1 should not be blocked
        assert!(!task1.is_blocked(&store));
        assert!(task1.can_start(&store));
    }

    #[test]
    fn test_task_not_blocked_when_dependency_deleted() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();

        let mut task1 = Task::with_id(id1, "Task 1".to_string(), "Working on 1".to_string());
        let mut task2 = Task::with_id(id2, "Task 2".to_string(), "Working on 2".to_string());

        // Task 1 is blocked by Task 2
        task1.dependencies.add_blocked_by(id2);

        // Task 2 is deleted
        task2.mark_deleted();

        let store = vec![task1.clone(), task2];

        // Task 1 should not be blocked
        assert!(!task1.is_blocked(&store));
        assert!(task1.can_start(&store));
    }

    #[test]
    fn test_task_status_serialization() {
        let pending = TaskStatus::Pending;
        let in_progress = TaskStatus::InProgress;
        let completed = TaskStatus::Completed;
        let blocked = TaskStatus::Blocked;

        assert_eq!(serde_json::to_string(&pending).unwrap(), "\"pending\"");
        assert_eq!(serde_json::to_string(&in_progress).unwrap(), "\"in_progress\"");
        assert_eq!(serde_json::to_string(&completed).unwrap(), "\"completed\"");
        assert_eq!(serde_json::to_string(&blocked).unwrap(), "\"blocked\"");
    }

    #[test]
    fn test_dependencies_is_empty() {
        let deps = TaskDependencies::new();
        assert!(deps.is_empty());

        let mut deps = TaskDependencies::new();
        deps.add_blocks(TaskId::new());
        assert!(!deps.is_empty());
    }
}
