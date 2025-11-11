//! Tool system for Claude Code
//!
//! This module defines the Tool trait and common types used by all tools.
//! Tools execute operations (file I/O, shell commands, etc.) and stream results.

pub mod bash;
pub mod read;
pub mod write;
pub mod edit;
pub mod glob_tool;
pub mod grep;
pub mod todo_write;
pub mod web_fetch;
pub mod web_search;
pub mod bash_output;
pub mod kill_shell;
pub mod ask_user_question;
pub mod notebook_edit;
pub mod slash_command;
pub mod skill;
pub mod error;
pub mod types;

pub use bash::BashTool;
pub use read::ReadTool;
pub use write::WriteTool;
pub use edit::EditTool;
pub use glob_tool::GlobTool;
pub use grep::GrepTool;
pub use todo_write::TodoWriteTool;
pub use web_fetch::WebFetchTool;
pub use web_search::WebSearchTool;
pub use bash_output::BashOutputTool;
pub use kill_shell::KillShellTool;
pub use ask_user_question::AskUserQuestionTool;
pub use notebook_edit::NotebookEditTool;
pub use slash_command::SlashCommandTool;
pub use skill::SkillTool;
pub use error::{ToolError, ToolResult};
pub use types::{ToolContext, ToolEvent, ToolMetadata, ToolStream};

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
