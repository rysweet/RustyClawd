//! Tool system for Claude Code
//!
//! This module defines the Tool trait and common types used by all tools.
//! Tools execute operations (file I/O, shell commands, etc.) and stream results.

pub mod agent;
pub mod ask_user_question;
pub mod bash;
pub mod bash_output;
pub mod edit;
pub mod error;
pub mod glob_tool;
pub mod grep;
pub mod kill_shell;
pub mod notebook_edit;
pub mod process_isolation;
pub mod process_registry;
pub mod read;
pub mod skill;
pub mod slash_command;
pub mod todo_write;
pub mod types;
pub mod web_fetch;
pub mod web_fetch_phase2;
pub mod web_search;
pub mod web_search_phase2;
pub mod write;

pub use agent::AgentTool;
pub use ask_user_question::AskUserQuestionTool;
pub use bash::BashTool;
pub use bash_output::BashOutputTool;
pub use edit::EditTool;
pub use error::{ToolError, ToolResult};
pub use glob_tool::GlobTool;
pub use grep::GrepTool;
pub use kill_shell::KillShellTool;
pub use notebook_edit::NotebookEditTool;
pub use process_isolation::{apply_isolation, spawn_with_isolation, ProcessSpawnConfig};
pub use process_registry::{global_registry, ProcessHandle, ProcessRegistry, ProcessStatus};
pub use read::ReadTool;
pub use skill::SkillTool;
pub use slash_command::SlashCommandTool;
pub use todo_write::TodoWriteTool;
pub use types::{ExecutionContext, ToolContext, ToolEvent, ToolMetadata, ToolStream};
pub use web_fetch::WebFetchTool;
pub use web_fetch_phase2::WebFetchToolPhase2;
pub use web_search::WebSearchTool;
pub use web_search_phase2::WebSearchToolPhase2;
pub use write::WriteTool;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// The core Tool trait
///
/// All tools must implement this trait. The trait uses associated types
/// for compile-time type safety of parameters and outputs.
///
/// # Example
///
/// ```ignore
/// use async_trait::async_trait;
///
/// struct MyTool;
///
/// #[async_trait]
/// impl Tool for MyTool {
///     type Params = MyParams;
///     type Output = MyOutput;
///
///     fn metadata(&self) -> ToolMetadata {
///         ToolMetadata {
///             name: "MyTool",
///             description: "Does something useful",
///         }
///     }
///
///     async fn execute(&self, params: Self::Params, ctx: &ToolContext)
///         -> ToolResult<ToolStream<Self::Output>>
///     {
///         Ok(Box::pin(stream! {
///             yield ToolEvent::Progress { step: "working".into() };
///             yield ToolEvent::Result(output);
///         }))
///     }
/// }
/// ```
#[async_trait]
pub trait Tool: Send + Sync {
    /// Parameter type for this tool
    type Params: DeserializeOwned + Send;

    /// Output type for this tool
    type Output: Serialize + Send;

    /// Get tool metadata (name, description, schema)
    fn metadata(&self) -> ToolMetadata;

    /// Execute the tool with given parameters
    ///
    /// Returns a stream of events (progress updates and final result).
    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>>;

    /// Check if tool is read-only (doesn't modify system state)
    fn is_read_only(&self) -> bool {
        false
    }

    /// Check if tool can be run concurrently
    fn is_concurrency_safe(&self) -> bool {
        true
    }
}
