//! Claude Code Checkpointing System
//!
//! This module provides session-level recovery by automatically capturing state
//! before edit operations. It implements the complete checkpointing system from
//! Claude Code, including:
//!
//! - **Session state persistence**: Conversations and code changes
//! - **Checkpoint creation**: Automatic and on-demand
//! - **State restoration**: Code, conversation, or both
//! - **Session resuming**: Full context restoration
//!
//! # Architecture
//!
//! The module follows a clean separation of concerns:
//!
//! - `types`: Core data structures (Checkpoint, Session, etc.)
//! - `storage`: File system operations for persistence
//! - `saver`: Session saving and checkpoint creation
//! - `loader`: Session restoration and checkpoint loading
//!
//! # Usage
//!
//! ```rust
//! use rustyclawd::checkpoint::{Session, SessionSaver, SessionLoader, RestoreScope};
//!
//! // Create a session
//! let mut session = Session::new("my-session", 50);
//!
//! // Create checkpoints
//! let checkpoint_id = session.create_checkpoint(Some("Before refactor".to_string()));
//!
//! // Save to disk
//! let saver = SessionSaver::default()?;
//! saver.save_session(&session)?;
//!
//! // Later, resume the session
//! let loader = SessionLoader::default()?;
//! let restored_session = loader.resume_session("my-session", 50)?;
//!
//! // Restore from a specific checkpoint
//! restored_session.restore_checkpoint(&checkpoint_id, RestoreScope::Both)?;
//! ```
//!
//! # Checkpoint Structure
//!
//! Each checkpoint captures:
//! - Conversation messages (user, assistant, system)
//! - File changes with content and hash verification
//! - Session state (working directory, environment variables, active contexts)
//! - Metadata (timestamp, description)
//!
//! # Storage
//!
//! Checkpoints are stored in `.claude/sessions/{session_id}/` as JSON files.
//! Each checkpoint file is named `{checkpoint_id}.json`.

// Public API
pub use loader::SessionLoader;
pub use saver::SessionSaver;
pub use types::Session;

// Module structure
pub mod loader;
pub mod saver;
pub mod storage;
pub mod types;

// Re-export commonly used items
