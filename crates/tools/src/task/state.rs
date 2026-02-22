//! Task state management with dependency validation
//!
//! Session-scoped task store with dependency graph validation.

use super::types::{Task, TaskId};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock, RwLock};

/// Error types for task state operations
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TaskStateError {
    /// Task not found
    #[error("Task not found: {0}")]
    TaskNotFound(TaskId),
    /// Circular dependency detected
    #[error("Circular dependency detected: {0:?}")]
    CircularDependency(Vec<TaskId>),
    /// Dependency references non-existent task
    #[error("Dependency references non-existent task: {0}")]
    InvalidDependency(TaskId),
    /// Task is already deleted
    #[error("Task is deleted: {0}")]
    TaskDeleted(TaskId),
    /// Duplicate task ID
    #[error("Duplicate task ID: {0}")]
    DuplicateTask(TaskId),
}

/// Global session-scoped task state
///
/// Uses OnceLock and RwLock for thread-safe session state, similar to TodoWrite.
static TASK_STATE: OnceLock<Arc<RwLock<HashMap<TaskId, Task>>>> = OnceLock::new();

/// Get or initialize the global task state
fn get_state() -> &'static Arc<RwLock<HashMap<TaskId, Task>>> {
    TASK_STATE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

/// Task store with validation
pub struct TaskStore;

impl TaskStore {
    /// Create a new task
    ///
    /// Validates the proposed dependency graph on a read-only snapshot before
    /// performing any mutations, preventing state corruption on cycle detection.
    pub fn create(task: Task) -> Result<Task, TaskStateError> {
        let mut state = get_state()
            .write()
            .expect("invariant: TASK_STATE lock not poisoned");

        // Check for duplicate ID
        if state.contains_key(&task.id) {
            return Err(TaskStateError::DuplicateTask(task.id));
        }

        // Validate dependencies exist
        Self::validate_dependencies(&task, &state)?;

        // Validate on a snapshot: apply proposed changes to a clone, check for cycles
        // Only check the affected subgraph for performance
        {
            let mut snapshot = state.clone();
            let task_with_deps = Self::sync_bidirectional_deps(task.clone(), &mut snapshot);
            let task_id = task_with_deps.id;
            snapshot.insert(task_id, task_with_deps);
            Self::validate_no_cycles_from(task_id, &snapshot)?;
        }

        // Validation passed -- now apply mutations to real state
        let task_with_deps = Self::sync_bidirectional_deps(task, &mut state);
        state.insert(task_with_deps.id, task_with_deps.clone());

        Ok(task_with_deps)
    }

    /// Update an existing task
    ///
    /// Validates the proposed dependency graph on a read-only snapshot before
    /// performing any mutations, preventing state corruption on cycle detection.
    pub fn update(task: Task) -> Result<Task, TaskStateError> {
        let mut state = get_state()
            .write()
            .expect("invariant: TASK_STATE lock not poisoned");

        // Check task exists
        if !state.contains_key(&task.id) {
            return Err(TaskStateError::TaskNotFound(task.id));
        }

        // Check not deleted
        if let Some(existing) = state.get(&task.id) {
            if existing.deleted {
                return Err(TaskStateError::TaskDeleted(task.id));
            }
        }

        // Validate dependencies exist
        Self::validate_dependencies(&task, &state)?;

        // Validate on a snapshot: apply proposed changes to a clone, check for cycles
        // Only check the affected subgraph for performance
        {
            let mut snapshot = state.clone();
            let old_task_clone = snapshot.get(&task.id).cloned();
            if let Some(old_task) = old_task_clone {
                Self::remove_bidirectional_deps(&old_task, &mut snapshot);
            }
            let task_with_deps = Self::sync_bidirectional_deps(task.clone(), &mut snapshot);
            let task_id = task_with_deps.id;
            snapshot.insert(task_id, task_with_deps);
            Self::validate_no_cycles_from(task_id, &snapshot)?;
        }

        // Validation passed -- now apply mutations to real state
        let old_task_clone = state.get(&task.id).cloned();
        if let Some(old_task) = old_task_clone {
            Self::remove_bidirectional_deps(&old_task, &mut state);
        }
        let task_with_deps = Self::sync_bidirectional_deps(task, &mut state);
        state.insert(task_with_deps.id, task_with_deps.clone());

        Ok(task_with_deps)
    }

    /// Get a task by ID
    pub fn get(id: &TaskId) -> Result<Task, TaskStateError> {
        let state = get_state()
            .read()
            .expect("invariant: TASK_STATE lock not poisoned");
        state
            .get(id)
            .filter(|t| !t.deleted)
            .cloned()
            .ok_or(TaskStateError::TaskNotFound(*id))
    }

    /// List all tasks (excluding deleted)
    pub fn list() -> Vec<Task> {
        let state = get_state()
            .read()
            .expect("invariant: TASK_STATE lock not poisoned");
        state.values().filter(|t| !t.deleted).cloned().collect()
    }

    /// List all tasks including deleted
    pub fn list_all() -> Vec<Task> {
        let state = get_state()
            .read()
            .expect("invariant: TASK_STATE lock not poisoned");
        state.values().cloned().collect()
    }

    /// Delete a task (soft delete)
    pub fn delete(id: &TaskId) -> Result<(), TaskStateError> {
        let mut state = get_state()
            .write()
            .expect("invariant: TASK_STATE lock not poisoned");

        let task = state.get_mut(id).ok_or(TaskStateError::TaskNotFound(*id))?;

        if task.deleted {
            return Err(TaskStateError::TaskDeleted(*id));
        }

        task.mark_deleted();
        Ok(())
    }

    /// Clear all tasks (for testing)
    #[cfg(test)]
    pub fn clear() {
        let mut state = get_state()
            .write()
            .expect("invariant: TASK_STATE lock not poisoned");
        state.clear();
    }

    /// Validate that all dependencies exist
    fn validate_dependencies(
        task: &Task,
        state: &HashMap<TaskId, Task>,
    ) -> Result<(), TaskStateError> {
        // Check all "blocks" dependencies exist
        for dep_id in &task.dependencies.blocks {
            if !state.contains_key(dep_id) {
                return Err(TaskStateError::InvalidDependency(*dep_id));
            }
        }

        // Check all "blocked_by" dependencies exist
        for dep_id in &task.dependencies.blocked_by {
            if !state.contains_key(dep_id) {
                return Err(TaskStateError::InvalidDependency(*dep_id));
            }
        }

        Ok(())
    }

    /// Sync bidirectional dependencies
    ///
    /// When task A blocks task B, ensure B.blocked_by contains A
    fn sync_bidirectional_deps(task: Task, state: &mut HashMap<TaskId, Task>) -> Task {
        // For each task this blocks, add this to their blocked_by
        for blocked_id in task.dependencies.blocks.clone() {
            if let Some(blocked_task) = state.get_mut(&blocked_id) {
                blocked_task.dependencies.add_blocked_by(task.id);
            }
        }

        // For each task that blocks this, add this to their blocks
        for blocker_id in task.dependencies.blocked_by.clone() {
            if let Some(blocker_task) = state.get_mut(&blocker_id) {
                blocker_task.dependencies.add_blocks(task.id);
            }
        }

        task
    }

    /// Remove bidirectional dependencies when updating a task
    fn remove_bidirectional_deps(task: &Task, state: &mut HashMap<TaskId, Task>) {
        // Remove this from all tasks it blocks
        for blocked_id in &task.dependencies.blocks {
            if let Some(blocked_task) = state.get_mut(blocked_id) {
                blocked_task.dependencies.remove_blocked_by(&task.id);
            }
        }

        // Remove this from all tasks that block it
        for blocker_id in &task.dependencies.blocked_by {
            if let Some(blocker_task) = state.get_mut(blocker_id) {
                blocker_task.dependencies.remove_blocks(&task.id);
            }
        }
    }

    /// Validate no circular dependencies in affected subgraph
    ///
    /// More efficient than full graph validation - only checks the subgraph
    /// reachable from the modified task and its transitive dependencies/dependents.
    fn validate_no_cycles_from(
        task_id: TaskId,
        state: &HashMap<TaskId, Task>,
    ) -> Result<(), TaskStateError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        // Only check the subgraph starting from the modified task
        if let Some(cycle) =
            Self::detect_cycle_dfs(task_id, state, &mut visited, &mut rec_stack, &mut path)
        {
            return Err(TaskStateError::CircularDependency(cycle));
        }

        Ok(())
    }

    /// DFS-based cycle detection
    fn detect_cycle_dfs(
        task_id: TaskId,
        state: &HashMap<TaskId, Task>,
        visited: &mut HashSet<TaskId>,
        rec_stack: &mut HashSet<TaskId>,
        path: &mut Vec<TaskId>,
    ) -> Option<Vec<TaskId>> {
        visited.insert(task_id);
        rec_stack.insert(task_id);
        path.push(task_id);

        if let Some(task) = state.get(&task_id) {
            // Check all tasks this blocks (dependencies)
            for &dep_id in &task.dependencies.blocks {
                if !visited.contains(&dep_id) {
                    if let Some(cycle) =
                        Self::detect_cycle_dfs(dep_id, state, visited, rec_stack, path)
                    {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&dep_id) {
                    // Found cycle - return path from dep_id to current
                    let cycle_start = path
                        .iter()
                        .position(|&id| id == dep_id)
                        .expect("invariant: dep_id must be in path when present in rec_stack");
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        path.pop();
        rec_stack.remove(&task_id);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::types::{Task, TaskId};
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_create_task() {
        TaskStore::clear();

        let task = Task::new("Test task".to_string(), "Testing".to_string());
        let result = TaskStore::create(task.clone());

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.content, "Test task");
    }

    #[test]
    #[serial]
    fn test_create_duplicate_task() {
        TaskStore::clear();

        let task = Task::new("Test".to_string(), "Testing".to_string());
        let id = task.id;

        TaskStore::create(task.clone()).unwrap();

        // Try to create same task again
        let result = TaskStore::create(task);
        assert_eq!(result, Err(TaskStateError::DuplicateTask(id)));
    }

    #[test]
    #[serial]
    fn test_get_task() {
        TaskStore::clear();

        let task = Task::new("Test".to_string(), "Testing".to_string());
        let id = task.id;

        TaskStore::create(task).unwrap();

        let retrieved = TaskStore::get(&id);
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[test]
    #[serial]
    fn test_get_nonexistent_task() {
        TaskStore::clear();

        let fake_id = TaskId::new();
        let result = TaskStore::get(&fake_id);
        assert_eq!(result, Err(TaskStateError::TaskNotFound(fake_id)));
    }

    #[test]
    #[serial]
    fn test_list_tasks() {
        TaskStore::clear();

        let task1 = Task::new("Task 1".to_string(), "Working 1".to_string());
        let task2 = Task::new("Task 2".to_string(), "Working 2".to_string());

        TaskStore::create(task1).unwrap();
        TaskStore::create(task2).unwrap();

        let tasks = TaskStore::list();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    #[serial]
    fn test_delete_task() {
        TaskStore::clear();

        let task = Task::new("Test".to_string(), "Testing".to_string());
        let id = task.id;

        TaskStore::create(task).unwrap();
        TaskStore::delete(&id).unwrap();

        // Should not be in list (soft deleted)
        let tasks = TaskStore::list();
        assert_eq!(tasks.len(), 0);

        // But should be in list_all
        let all_tasks = TaskStore::list_all();
        assert_eq!(all_tasks.len(), 1);
        assert!(all_tasks[0].deleted);
    }

    #[test]
    #[serial]
    fn test_delete_nonexistent_task() {
        TaskStore::clear();

        let fake_id = TaskId::new();
        let result = TaskStore::delete(&fake_id);
        assert_eq!(result, Err(TaskStateError::TaskNotFound(fake_id)));
    }

    #[test]
    #[serial]
    fn test_update_task() {
        TaskStore::clear();

        let mut task = Task::new("Original".to_string(), "Working".to_string());
        let id = task.id;

        TaskStore::create(task.clone()).unwrap();

        task.content = "Updated".to_string();
        TaskStore::update(task).unwrap();

        let updated = TaskStore::get(&id).unwrap();
        assert_eq!(updated.content, "Updated");
    }

    #[test]
    #[serial]
    fn test_update_nonexistent_task() {
        TaskStore::clear();

        let task = Task::new("Test".to_string(), "Testing".to_string());
        let result = TaskStore::update(task.clone());
        assert_eq!(result, Err(TaskStateError::TaskNotFound(task.id)));
    }

    #[test]
    #[serial]
    fn test_invalid_dependency() {
        TaskStore::clear();

        let mut task = Task::new("Test".to_string(), "Testing".to_string());
        let fake_dep = TaskId::new();
        task.dependencies.add_blocked_by(fake_dep);

        let result = TaskStore::create(task);
        assert_eq!(result, Err(TaskStateError::InvalidDependency(fake_dep)));
    }

    #[test]
    #[serial]
    fn test_bidirectional_dependencies() {
        TaskStore::clear();

        let task1 = Task::new("Task 1".to_string(), "Working 1".to_string());
        let task2 = Task::new("Task 2".to_string(), "Working 2".to_string());
        let id1 = task1.id;
        let id2 = task2.id;

        TaskStore::create(task1).unwrap();
        TaskStore::create(task2).unwrap();

        // Make task1 block task2
        let mut task1 = TaskStore::get(&id1).unwrap();
        task1.dependencies.add_blocks(id2);
        TaskStore::update(task1).unwrap();

        // Check bidirectional sync
        let task1 = TaskStore::get(&id1).unwrap();
        let task2 = TaskStore::get(&id2).unwrap();

        assert!(task1.dependencies.blocks_task(&id2));
        assert!(task2.dependencies.is_blocked_by(&id1));
    }

    #[test]
    #[serial]
    fn test_circular_dependency_detection() {
        TaskStore::clear();

        let task1 = Task::new("Task 1".to_string(), "Working 1".to_string());
        let task2 = Task::new("Task 2".to_string(), "Working 2".to_string());
        let id1 = task1.id;
        let id2 = task2.id;

        TaskStore::create(task1).unwrap();
        TaskStore::create(task2).unwrap();

        // Task1 blocks Task2
        let mut task1 = TaskStore::get(&id1).unwrap();
        task1.dependencies.add_blocks(id2);
        TaskStore::update(task1).unwrap();

        // Try to make Task2 block Task1 (cycle!)
        let mut task2 = TaskStore::get(&id2).unwrap();
        task2.dependencies.add_blocks(id1);
        let result = TaskStore::update(task2);

        assert!(matches!(result, Err(TaskStateError::CircularDependency(_))));
    }

    #[test]
    #[serial]
    fn test_three_way_circular_dependency() {
        TaskStore::clear();

        let task1 = Task::new("Task 1".to_string(), "Working 1".to_string());
        let task2 = Task::new("Task 2".to_string(), "Working 2".to_string());
        let task3 = Task::new("Task 3".to_string(), "Working 3".to_string());
        let id1 = task1.id;
        let id2 = task2.id;
        let id3 = task3.id;

        TaskStore::create(task1).unwrap();
        TaskStore::create(task2).unwrap();
        TaskStore::create(task3).unwrap();

        // Task1 blocks Task2
        let mut task1 = TaskStore::get(&id1).unwrap();
        task1.dependencies.add_blocks(id2);
        TaskStore::update(task1).unwrap();

        // Task2 blocks Task3
        let mut task2 = TaskStore::get(&id2).unwrap();
        task2.dependencies.add_blocks(id3);
        TaskStore::update(task2).unwrap();

        // Try to make Task3 block Task1 (creates cycle!)
        let mut task3 = TaskStore::get(&id3).unwrap();
        task3.dependencies.add_blocks(id1);
        let result = TaskStore::update(task3);

        assert!(matches!(result, Err(TaskStateError::CircularDependency(_))));
    }

    #[test]
    #[serial]
    fn test_cycle_detection_does_not_corrupt_state() {
        TaskStore::clear();

        // Set up: Task1 blocks Task2 (valid)
        let task1 = Task::new("Task 1".to_string(), "Working 1".to_string());
        let task2 = Task::new("Task 2".to_string(), "Working 2".to_string());
        let id1 = task1.id;
        let id2 = task2.id;

        TaskStore::create(task1).unwrap();
        TaskStore::create(task2).unwrap();

        let mut t1 = TaskStore::get(&id1).unwrap();
        t1.dependencies.add_blocks(id2);
        TaskStore::update(t1).unwrap();

        // Snapshot state before the failed cycle attempt
        let task1_before = TaskStore::get(&id1).unwrap();
        let task2_before = TaskStore::get(&id2).unwrap();

        // Attempt to create a cycle: Task2 blocks Task1 (should fail)
        let mut t2 = TaskStore::get(&id2).unwrap();
        t2.dependencies.add_blocks(id1);
        let result = TaskStore::update(t2);
        assert!(matches!(result, Err(TaskStateError::CircularDependency(_))));

        // Verify state is unchanged after the failed cycle attempt
        let task1_after = TaskStore::get(&id1).unwrap();
        let task2_after = TaskStore::get(&id2).unwrap();

        assert_eq!(
            task1_before.dependencies, task1_after.dependencies,
            "Task1 dependencies should be unchanged after failed cycle check"
        );
        assert_eq!(
            task2_before.dependencies, task2_after.dependencies,
            "Task2 dependencies should be unchanged after failed cycle check"
        );
    }

    #[test]
    #[serial]
    fn test_create_cycle_does_not_corrupt_state() {
        TaskStore::clear();

        // Set up: A blocks B (valid chain)
        let t1 = Task::new("Task A".to_string(), "Working A".to_string());
        let t2 = Task::new("Task B".to_string(), "Working B".to_string());
        let a_id = t1.id;
        let b_id = t2.id;

        TaskStore::create(t1).unwrap();
        TaskStore::create(t2).unwrap();

        let mut ta = TaskStore::get(&a_id).unwrap();
        ta.dependencies.add_blocks(b_id);
        TaskStore::update(ta).unwrap();

        // Snapshot state before failed create
        let count_before = TaskStore::list().len();
        let ta_before = TaskStore::get(&a_id).unwrap();
        let tb_before = TaskStore::get(&b_id).unwrap();

        // Try to create Task C that is blocked_by B and blocks A (cycle: A->B->C->A)
        let mut tc = Task::new("Task C".to_string(), "Working C".to_string());
        tc.dependencies.add_blocked_by(b_id);
        tc.dependencies.add_blocks(a_id);
        let result = TaskStore::create(tc);

        assert!(matches!(result, Err(TaskStateError::CircularDependency(_))));

        // Verify state unchanged: no new task, existing tasks unmodified
        assert_eq!(
            TaskStore::list().len(),
            count_before,
            "Task count should be unchanged after failed create"
        );
        assert_eq!(
            TaskStore::get(&a_id).unwrap().dependencies,
            ta_before.dependencies,
            "Task A dependencies should be unchanged after failed create"
        );
        assert_eq!(
            TaskStore::get(&b_id).unwrap().dependencies,
            tb_before.dependencies,
            "Task B dependencies should be unchanged after failed create"
        );
    }
}
