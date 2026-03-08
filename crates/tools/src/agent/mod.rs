//! Agent/Task tool - Enables agent orchestration through Claude API
//!
//! This is the CRITICAL tool that enables multi-agent workflows. It:
//! - Invokes sub-agents with specialized prompts
//! - Forks context for agent isolation
//! - Streams agent responses in real-time
//! - Supports model selection (haiku/sonnet/opus)
//! - Allows resuming previous agent executions
//! - Supports background execution (run_in_background)
//!
//! # Module structure
//!
//! - `frontmatter` - AgentFrontmatter parsing from agent definition YAML headers
//! - `types` - AgentParams, AgentOutput, TokenUsage type definitions
//! - `execute` - AgentTool struct and Tool trait implementation

mod execute;
mod frontmatter;
mod types;

#[cfg(test)]
mod tests;

pub use execute::AgentTool;
pub use frontmatter::{AgentFrontmatter, AgentIsolation};
pub use types::{AgentOutput, AgentParams, TokenUsage};
