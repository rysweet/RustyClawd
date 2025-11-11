//! Skill tool - Load and execute skills
//!
//! Demonstrates:
//! - Dynamic skill loading
//! - Skill registry management

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameters for Skill tool
#[derive(Debug, Deserialize)]
pub struct SkillParams {
    /// Skill name to execute
    pub skill: String,
}

/// Output from Skill tool
#[derive(Debug, Serialize)]
pub struct SkillOutput {
    /// Skill that was loaded
    pub skill: String,

    /// Skill prompt/instructions
    pub prompt: String,

    /// Whether skill was found
    pub found: bool,
}

/// The Skill tool
pub struct SkillTool;

#[async_trait]
impl crate::Tool for SkillTool {
    type Params = SkillParams;
    type Output = SkillOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "Skill",
            description: "Loads and executes skills from the skill registry",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let skill = params.skill.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Loading skill: {}", skill),
                percentage: None,
            };

            // In full implementation:
            // 1. Check if skill exists in registry
            // 2. Load skill definition
            // 3. Return skill prompt/instructions

            // Simplified: Simulate skill loading
            let found = !skill.is_empty();
            let prompt = if found {
                format!("Skill '{}' loaded and ready to execute", skill)
            } else {
                "Skill not found".to_string()
            };

            if debug {
                tracing::debug!(
                    skill = %skill,
                    found = found,
                    "Skill loading complete"
                );
            }

            yield ToolEvent::Result(SkillOutput {
                skill: params.skill.clone(),
                prompt,
                found,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true // Loading skills doesn't modify state
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_skill_loading() {
        let tool = SkillTool;
        let params = SkillParams {
            skill: "test-skill".to_string(),
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        }).unwrap();

        assert_eq!(result.skill, "test-skill");
        assert!(result.found);
    }
}
