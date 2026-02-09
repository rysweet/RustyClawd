#  Task Management System with Dependency Tracking

This module implements a complete task management system with dependency tracking for RustyClawd.

## Overview

The task management system extends the existing TodoWrite pattern to provide:

- **Task dependencies**: Support for `blocks` and `blockedBy` relationships
- **CRUD operations**: TaskCreate, TaskUpdate, TaskList, TaskGet tools
- **Soft delete**: Maintains referential integrity when deleting tasks
- **Session-scoped validation**: Validates dependency graph on each operation
- **Zero-BS implementation**: All functions work, no stubs or placeholders

## Architecture

This module follows the "bricks & studs" modular design philosophy:

### Module Structure

```
task/
├── mod.rs              # Public API exports
├── README.md          # This file
├── types.rs           # Core data structures
├── state.rs           # Session-scoped state with validation
├── create.rs          # TaskCreate tool
├── update.rs          # TaskUpdate tool
├── get.rs             # TaskGet tool
└── list.rs            # TaskList tool
```

### Core Components

#### types.rs - Data Structures

- **Task**: Main task type with id, content, status, dependencies, and soft delete flag
- **TaskId**: UUID-based unique identifier
- **TaskStatus**: Pending, InProgress, Completed, Blocked
- **TaskDependencies**: Tracks `blocks` and `blockedBy` relationships

#### state.rs - State Management

- **TaskStore**: Session-scoped state using `OnceLock` and `RwLock`
- **Dependency validation**: Ensures dependencies exist and prevents circular dependencies
- **Bidirectional sync**: When task A blocks task B, automatically updates B's `blockedBy`
- **Soft delete**: Tasks are marked as deleted rather than removed

#### Tools (create, update, get, list)

Each tool implements the `Tool` trait and provides streaming results with progress updates.

## Public API

The module exports the following public interface (the "studs"):

```rust
// Types
pub use types::{Task, TaskId, TaskStatus, TaskDependencies};

// State management
pub use state::{TaskStore, TaskStateError};

// Tools
pub use create::{TaskCreateTool, TaskCreateParams, TaskCreateOutput};
pub use update::{TaskUpdateTool, TaskUpdateParams, TaskUpdateOutput};
pub use get::{TaskGetTool, TaskGetParams, TaskGetOutput};
pub use list::{TaskListTool, TaskListParams, TaskListOutput};
```

## Usage Examples

### Creating a Task

```rust
use rustyclawd_tools::task::{TaskCreateTool, TaskCreateParams};
use rustyclawd_tools::{Tool, ToolContext};

let tool = TaskCreateTool;
let params = TaskCreateParams {
    content: "Implement feature X".to_string(),
    active_form: "Implementing feature X".to_string(),
    status: None,  // Defaults to Pending
    dependencies: None,
};

let context = ToolContext::default();
let stream = tool.execute(params, &context).await?;

// Handle result stream...
```

### Creating Tasks with Dependencies

```rust
use rustyclawd_tools::task::{TaskCreateTool, TaskCreateParams, TaskDependencies};

// Create first task
let task1 = create_task("Design API".into(), "Designing API".into()).await?;

// Create second task that depends on first
let mut deps = TaskDependencies::new();
deps.add_blocked_by(task1.id);

let params = TaskCreateParams {
    content: "Implement API".to_string(),
    active_form: "Implementing API".to_string(),
    status: None,
    dependencies: Some(deps),
};

let task2 = tool.execute(params, &context).await?;
// task2 cannot start until task1 is completed
```

### Updating a Task

```rust
use rustyclawd_tools::task::{TaskUpdateTool, TaskUpdateParams, TaskStatus};

let params = TaskUpdateParams {
    id: task_id,
    content: Some("Updated content".to_string()),
    active_form: None,  // Keep existing value
    status: Some(TaskStatus::Completed),
    dependencies: None,  // Keep existing dependencies
};

let stream = tool.execute(params, &context).await?;
```

### Listing Tasks

```rust
use rustyclawd_tools::task::{TaskListTool, TaskListParams, TaskStatus};

// List all pending tasks
let params = TaskListParams {
    status: Some(TaskStatus::Pending),
    include_deleted: false,
};

let stream = tool.execute(params, &context).await?;
```

### Getting a Single Task

```rust
use rustyclawd_tools::task::{TaskGetTool, TaskGetParams};

let params = TaskGetParams { id: task_id };
let stream = tool.execute(params, &context).await?;
```

## Key Features

### Dependency Tracking

- **Bidirectional relationships**: When task A blocks task B, both tasks are updated automatically
- **Circular dependency detection**: Uses DFS algorithm to prevent cycles
- **Dependency validation**: Ensures all referenced tasks exist
- **Deleted task handling**: Tasks blocked by deleted tasks become unblocked

### State Management

- **Session-scoped**: State persists for the session duration
- **Thread-safe**: Uses `RwLock` for concurrent access
- **Atomic operations**: All state modifications are validated before committing

### Error Handling

All operations return clear, actionable errors:

- `TaskNotFound`: Requested task doesn't exist
- `TaskDeleted`: Attempt to modify deleted task
- `DuplicateTask`: Task with same ID already exists
- `InvalidDependency`: Dependency references non-existent task
- `CircularDependency`: Dependency would create a cycle

## Testing

The module includes comprehensive tests following the 60/30/10 pyramid:

- **60% Unit tests**: Fast tests of individual functions and types
- **30% Integration tests**: Tool execution and state interactions
- **10% E2E tests**: Complete workflows

All tests use `#[serial]` attribute to ensure proper state isolation.

Run tests with:

```bash
cargo test --package rustyclawd-tools --lib task
```

## Philosophy Alignment

This module strictly follows RustyClawd's core principles:

### Ruthless Simplicity

- Session-scoped state (no external database)
- Simple HashMap storage
- Direct dependency tracking (no complex graph library)

### Zero-BS Implementation

- Every function works (no stubs or TODOs)
- No fake implementations or mocked services
- All tests verify actual behavior

### Modular Design

- Self-contained module with clear public API
- All code, tests, and docs in single directory
- Can be regenerated from this specification

### Regeneratable

This entire module can be rebuilt from this README and the requirements:

1. Read this specification
2. Implement types matching the public API
3. Implement state management with validation
4. Implement tools following the Tool trait
5. Write tests verifying the contract

## Future Enhancements

Potential future enhancements (not currently implemented):

- **Task priorities**: Add priority field to tasks
- **Due dates**: Track task deadlines
- **Task labels/tags**: Categorize tasks
- **Task assignments**: Assign tasks to users
- **Dependency visualization**: Generate dependency graphs
- **Persistent storage**: Optional database backend

These would be added only when needed, following the principle of ruthless simplicity.

## License

This module is part of RustyClawd and follows the same license (MIT OR Apache-2.0).
