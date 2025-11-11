//! Skill tool - Load and execute skills
//!
//! Demonstrates:
//! - Dynamic skill loading
//! - Skill registry management

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

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

            // Try different skill file locations
            let possible_paths = vec![
                PathBuf::from(format!(".claude/skills/{}.md", skill)),
                PathBuf::from(format!(".claude/skills/{}/skill.md", skill)),
                PathBuf::from(format!(".claude/skills/{}.yaml", skill)),
                PathBuf::from(format!(".claude/skills/{}/skill.yaml", skill)),
            ];

            let mut found = false;
            let mut prompt = String::new();

            for path in possible_paths {
                if path.exists() {
                    match fs::read_to_string(&path).await {
                        Ok(content) => {
                            found = true;

                            // Parse based on file extension
                            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                                // Markdown file - use as-is or parse frontmatter
                                if content.starts_with("---") {
                                    // Has frontmatter, extract content after second ---
                                    if let Some(end) = content[3..].find("---") {
                                        prompt = content[3 + end + 3..].trim().to_string();
                                    } else {
                                        prompt = content;
                                    }
                                } else {
                                    prompt = content;
                                }
                            } else if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                                // YAML file - parse and extract prompt field
                                if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(&content) {
                                    if let Some(prompt_value) = yaml.get("prompt").or_else(|| yaml.get("instructions")) {
                                        prompt = prompt_value.as_str().unwrap_or(&content).to_string();
                                    } else {
                                        prompt = content;
                                    }
                                } else {
                                    prompt = content;
                                }
                            }

                            if debug {
                                tracing::debug!(
                                    skill = %skill,
                                    path = ?path,
                                    prompt_len = prompt.len(),
                                    "Skill loaded successfully"
                                );
                            }
                            break;
                        }
                        Err(e) => {
                            if debug {
                                tracing::warn!("Failed to read skill file {:?}: {}", path, e);
                            }
                            continue;
                        }
                    }
                }
            }

            if !found {
                prompt = format!("Skill '{}' not found. Searched in .claude/skills/", skill);
            }

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
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_skill_loading() {
        // Create temporary skill file for testing
        let skill_dir = PathBuf::from(".claude/skills");
        let _ = fs::create_dir_all(&skill_dir).await;

        let skill_path = skill_dir.join("test-skill.md");
        let test_content = "# Test Skill\n\nThis is a test skill for verifying skill loading.\n";
        let _ = fs::write(&skill_path, test_content).await;

        let tool = SkillTool;
        let params = SkillParams {
            skill: "test-skill".to_string(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        }).unwrap();

        assert_eq!(result.skill, "test-skill");
        assert!(result.found);
        assert!(result.prompt.contains("Test Skill"));

        // Clean up
        let _ = fs::remove_file(&skill_path).await;
    }
}
