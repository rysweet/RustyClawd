//! Task Management System with dependency tracking
//!
//! This module provides a complete task management system with:
//! - Task dependencies (blocks/blockedBy relationships)
//! - CRUD operations via tools
//! - Soft delete for referential integrity
//! - Session-scoped validation
//!
//! ## Architecture
//!
//! The task system follows the "bricks & studs" modular design:
//!
//! - **types.rs**: Core data structures (Task, TaskId, TaskDependencies)
//! - **state.rs**: Session-scoped state with dependency validation
//! - **create.rs**: TaskCreate tool
//! - **update.rs**: TaskUpdate tool
//! - **get.rs**: TaskGet tool
//! - **list.rs**: TaskList tool
//!
//! ## Example Usage
//!
//! ```ignore
//! use crate::task::{TaskCreateTool, TaskCreateParams};
//! use crate::Tool;
//!
//! let tool = TaskCreateTool;
//! let params = TaskCreateParams {
//!     content: "Implement feature X".to_string(),
//!     active_form: "Implementing feature X".to_string(),
//!     status: None,
//!     dependencies: None,
//! };
//!
//! let stream = tool.execute(params, &context).await?;
//! // Handle result stream...
//! ```
//!
//! ## Philosophy Alignment
//!
//! This module follows RustyClawd's core principles:
//! - **Ruthless Simplicity**: Session-scoped state, no external dependencies
//! - **Zero-BS**: All functions work, no stubs or placeholders
//! - **Modular Design**: Self-contained brick with clear public API
//! - **Regeneratable**: Can be rebuilt from this specification

pub mod create;
pub mod get;
pub mod list;
pub mod state;
pub mod types;
pub mod update;

// Re-export public API (the "studs")
pub use create::{TaskCreateOutput, TaskCreateParams, TaskCreateTool};
pub use get::{TaskGetOutput, TaskGetParams, TaskGetTool};
pub use list::{TaskListOutput, TaskListParams, TaskListTool};
pub use state::{TaskStateError, TaskStore};
pub use types::{Task, TaskDependencies, TaskId, TaskStatus};
pub use update::{TaskUpdateOutput, TaskUpdateParams, TaskUpdateTool};
