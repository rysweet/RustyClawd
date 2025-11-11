//! AskUserQuestion tool - Interactive user prompts
//!
//! Demonstrates:
//! - Terminal interaction with dialoguer
//! - Multi-select and single-select prompts
//! - Input validation
//! - User-friendly interfaces

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use dialoguer::{Select, MultiSelect, Input};
use serde::{Deserialize, Serialize};

/// A single question with options
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Question {
    /// The question text
    pub question: String,

    /// Short header label
    pub header: String,

    /// Available options
    pub options: Vec<QuestionOption>,

    /// Allow multiple selections
    #[serde(rename = "multiSelect")]
    pub multi_select: bool,
}

/// A single option for a question
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuestionOption {
    /// Display label
    pub label: String,

    /// Description/explanation
    pub description: String,
}

/// Parameters for AskUserQuestion tool
#[derive(Debug, Deserialize)]
pub struct AskUserQuestionParams {
    /// Questions to ask (1-4 questions)
    pub questions: Vec<Question>,

    /// Previously collected answers (for resumption)
    #[serde(default)]
    pub answers: std::collections::HashMap<String, String>,
}

/// Output from AskUserQuestion tool
#[derive(Debug, Serialize)]
pub struct AskUserQuestionOutput {
    /// Collected answers keyed by question header
    pub answers: std::collections::HashMap<String, String>,

    /// Number of questions answered
    pub questions_answered: usize,
}

/// The AskUserQuestion tool
pub struct AskUserQuestionTool;

#[async_trait]
impl crate::Tool for AskUserQuestionTool {
    type Params = AskUserQuestionParams;
    type Output = AskUserQuestionOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "AskUserQuestion",
            description: "Asks user questions with predefined options during execution",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let questions = params.questions.clone();
        let mut answers = params.answers.clone();
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: format!("Preparing {} question(s)...", questions.len()),
                percentage: None,
            };

            // Validate question count
            if questions.is_empty() || questions.len() > 4 {
                yield ToolEvent::Error {
                    message: format!("Must have 1-4 questions, got {}", questions.len()),
                };
                return;
            }

            // Ask each question
            for (i, question) in questions.iter().enumerate() {
                if answers.contains_key(&question.header) {
                    continue; // Already answered
                }

                yield ToolEvent::Progress {
                    step: format!("Question {}/{}: {}", i + 1, questions.len(), question.header),
                    percentage: Some((i as f32 / questions.len() as f32) * 100.0),
                };

                // Build menu items with descriptions
                let items: Vec<String> = question.options.iter()
                    .map(|opt| format!("{} - {}", opt.label, opt.description))
                    .collect();

                let selected = if question.multi_select {
                    // Multi-select mode
                    let selections = MultiSelect::new()
                        .with_prompt(&question.question)
                        .items(&items)
                        .interact();

                    match selections {
                        Ok(indices) => {
                            let labels: Vec<String> = indices.iter()
                                .map(|&idx| question.options[idx].label.clone())
                                .collect();
                            labels.join(", ")
                        }
                        Err(e) => {
                            if debug {
                                tracing::warn!("User cancelled or error: {}", e);
                            }
                            yield ToolEvent::Error {
                                message: format!("Question cancelled or error: {}", e),
                            };
                            return;
                        }
                    }
                } else {
                    // Single select mode
                    let selection = Select::new()
                        .with_prompt(&question.question)
                        .items(&items)
                        .default(0)
                        .interact();

                    match selection {
                        Ok(idx) => question.options[idx].label.clone(),
                        Err(e) => {
                            if debug {
                                tracing::warn!("User cancelled or error: {}", e);
                            }
                            // Allow "Other" as fallback
                            let other: String = Input::new()
                                .with_prompt("Other (please specify)")
                                .interact_text()
                                .unwrap_or_else(|_| "".to_string());

                            if other.is_empty() {
                                yield ToolEvent::Error {
                                    message: format!("Question cancelled or no input: {}", e),
                                };
                                return;
                            }
                            other
                        }
                    }
                };

                answers.insert(question.header.clone(), selected);

                if debug {
                    tracing::debug!(
                        header = %question.header,
                        selected = ?answers.get(&question.header),
                        "Question answered"
                    );
                }
            }

            yield ToolEvent::Result(AskUserQuestionOutput {
                answers,
                questions_answered: questions.len(),
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        true // Reading user input doesn't modify files
    }

    fn is_concurrency_safe(&self) -> bool {
        false // Terminal interaction can't be concurrent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;

    #[tokio::test]
    #[ignore]
    async fn test_ask_question_single_select() {
        // This test requires terminal interaction via dialoguer which is not available in test environments.
        // Terminal-based interactive tests should be run manually or with a terminal-aware test harness.
        // In CI/CD environments, this test is properly ignored.
        let tool = AskUserQuestionTool;
        let params = AskUserQuestionParams {
            questions: vec![Question {
                question: "Choose your language".to_string(),
                header: "lang".to_string(),
                options: vec![
                    QuestionOption {
                        label: "Rust".to_string(),
                        description: "Systems programming".to_string(),
                    },
                    QuestionOption {
                        label: "JavaScript".to_string(),
                        description: "Web development".to_string(),
                    },
                ],
                multi_select: false,
            }],
            answers: std::collections::HashMap::new(),
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let _events: Vec<_> = stream.collect().await;

        // Terminal interaction cannot be tested without an actual TTY.
        // Manual testing is recommended for this tool.
    }
}
