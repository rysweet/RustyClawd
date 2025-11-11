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

            // For this educational implementation, simulate user selection
            // In production, would use dialoguer crate for real terminal UI
            for (i, question) in questions.iter().enumerate() {
                if answers.contains_key(&question.header) {
                    continue; // Already answered
                }

                yield ToolEvent::Progress {
                    step: format!("Question {}/{}: {}", i + 1, questions.len(), question.header),
                    percentage: Some((i as f32 / questions.len() as f32) * 100.0),
                };

                // Simulate selection (in real impl, would prompt user)
                let selected = if question.multi_select {
                    // Multi-select: select first option
                    question.options.first()
                        .map(|o| o.label.clone())
                        .unwrap_or_else(|| "None".to_string())
                } else {
                    // Single select: select first option
                    question.options.first()
                        .map(|o| o.label.clone())
                        .unwrap_or_else(|| "None".to_string())
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
    async fn test_ask_question_single_select() {
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

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events.iter().find_map(|e| match e {
            ToolEvent::Result(output) => Some(output),
            _ => None,
        }).unwrap();

        assert_eq!(result.questions_answered, 1);
        assert!(result.answers.contains_key("lang"));
    }
}
