//! AskUserQuestion tool - Interactive user prompts
//!
//! Demonstrates:
//! - Terminal interaction with dialoguer
//! - Multi-select and single-select prompts
//! - Input validation
//! - User-friendly interfaces
//! - Automatic "Other" option support
//! - TUI and non-interactive mode handling

use crate::{ExecutionContext, ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect, Select};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single question with options
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Question {
    /// The question text
    pub question: String,

    /// Short header label (max 12 chars)
    pub header: String,

    /// Available options (2-4 options)
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
    pub answers: HashMap<String, String>,
}

/// Output from AskUserQuestion tool
#[derive(Debug, Serialize)]
pub struct AskUserQuestionOutput {
    /// Collected answers keyed by question header
    pub answers: HashMap<String, String>,

    /// Number of questions answered
    pub questions_answered: usize,
}

/// The AskUserQuestion tool
pub struct AskUserQuestionTool;

impl AskUserQuestionTool {
    /// Ask a single-select question in TUI mode
    fn ask_single_select_tui(
        question: &Question,
        debug: bool,
    ) -> Result<String, String> {
        // Add "Other" option automatically
        let mut items: Vec<String> = question
            .options
            .iter()
            .map(|opt| format!("{} - {}", opt.label, opt.description))
            .collect();
        items.push("Other (custom input)".to_string());

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(&question.question)
            .items(&items)
            .default(0)
            .interact()
            .map_err(|e| {
                if debug {
                    tracing::warn!("User cancelled or error: {}", e);
                }
                format!("Question cancelled or error: {}", e)
            })?;

        // Handle "Other" option
        if selection == items.len() - 1 {
            let other: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Please specify")
                .interact_text()
                .map_err(|e| format!("Failed to read input: {}", e))?;

            if other.trim().is_empty() {
                return Err("No input provided".to_string());
            }
            Ok(other.trim().to_string())
        } else {
            Ok(question.options[selection].label.clone())
        }
    }

    /// Ask a multi-select question in TUI mode
    fn ask_multi_select_tui(
        question: &Question,
        debug: bool,
    ) -> Result<String, String> {
        // Add "Other" option automatically
        let mut items: Vec<String> = question
            .options
            .iter()
            .map(|opt| format!("{} - {}", opt.label, opt.description))
            .collect();
        items.push("Other (custom input)".to_string());

        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt(&question.question)
            .items(&items)
            .interact()
            .map_err(|e| {
                if debug {
                    tracing::warn!("User cancelled or error: {}", e);
                }
                format!("Question cancelled or error: {}", e)
            })?;

        if selections.is_empty() {
            return Err("No options selected".to_string());
        }

        let mut selected_labels = Vec::new();

        // Check if "Other" was selected
        let other_selected = selections.contains(&(items.len() - 1));

        // Collect regular selections
        for &idx in &selections {
            if idx < question.options.len() {
                selected_labels.push(question.options[idx].label.clone());
            }
        }

        // Handle "Other" option
        if other_selected {
            let other: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Please specify other option(s)")
                .interact_text()
                .map_err(|e| format!("Failed to read input: {}", e))?;

            if !other.trim().is_empty() {
                selected_labels.push(other.trim().to_string());
            }
        }

        Ok(selected_labels.join(", "))
    }

    /// Ask a question in non-interactive/CLI mode
    fn ask_cli_mode(question: &Question) -> Result<String, String> {
        eprintln!("\n{}", question.question);
        eprintln!("Options:");
        for (i, opt) in question.options.iter().enumerate() {
            eprintln!("  {}. {} - {}", i + 1, opt.label, opt.description);
        }
        eprintln!("  {}. Other (custom input)", question.options.len() + 1);

        if question.multi_select {
            eprintln!("\nEnter selection(s) (comma-separated numbers or text):");
        } else {
            eprintln!("\nEnter selection (number or text):");
        }

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Failed to read input: {}", e))?;

        let input = input.trim();
        if input.is_empty() {
            return Err("No input provided".to_string());
        }

        // Try to parse as number(s)
        if question.multi_select {
            let parts: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
            let mut selected = Vec::new();

            for part in parts {
                if let Ok(num) = part.parse::<usize>() {
                    if num > 0 && num <= question.options.len() {
                        selected.push(question.options[num - 1].label.clone());
                    } else if num == question.options.len() + 1 {
                        // "Other" selected via number
                        eprintln!("Please specify:");
                        let mut other = String::new();
                        std::io::stdin()
                            .read_line(&mut other)
                            .map_err(|e| format!("Failed to read input: {}", e))?;
                        if !other.trim().is_empty() {
                            selected.push(other.trim().to_string());
                        }
                    }
                } else {
                    // Treat as custom text
                    selected.push(part.to_string());
                }
            }

            if selected.is_empty() {
                return Err("No valid selections".to_string());
            }

            Ok(selected.join(", "))
        } else {
            // Single select
            if let Ok(num) = input.parse::<usize>() {
                if num > 0 && num <= question.options.len() {
                    Ok(question.options[num - 1].label.clone())
                } else if num == question.options.len() + 1 {
                    // "Other" selected
                    eprintln!("Please specify:");
                    let mut other = String::new();
                    std::io::stdin()
                        .read_line(&mut other)
                        .map_err(|e| format!("Failed to read input: {}", e))?;
                    let other = other.trim();
                    if other.is_empty() {
                        Err("No input provided".to_string())
                    } else {
                        Ok(other.to_string())
                    }
                } else {
                    Err(format!("Invalid option number: {}", num))
                }
            } else {
                // Treat as custom text
                Ok(input.to_string())
            }
        }
    }

    /// Validate question structure
    fn validate_question(question: &Question, index: usize) -> Result<(), String> {
        // Check header length
        if question.header.is_empty() {
            return Err(format!("Question {}: header cannot be empty", index + 1));
        }
        if question.header.len() > 12 {
            return Err(format!(
                "Question {}: header '{}' exceeds 12 characters (has {})",
                index + 1,
                question.header,
                question.header.len()
            ));
        }

        // Check options count
        if question.options.len() < 2 || question.options.len() > 4 {
            return Err(format!(
                "Question {}: must have 2-4 options, got {}",
                index + 1,
                question.options.len()
            ));
        }

        // Check question text
        if question.question.is_empty() {
            return Err(format!(
                "Question {}: question text cannot be empty",
                index + 1
            ));
        }

        // Check options
        for (i, opt) in question.options.iter().enumerate() {
            if opt.label.is_empty() {
                return Err(format!(
                    "Question {}, option {}: label cannot be empty",
                    index + 1,
                    i + 1
                ));
            }
            if opt.description.is_empty() {
                return Err(format!(
                    "Question {}, option {}: description cannot be empty",
                    index + 1,
                    i + 1
                ));
            }
        }

        Ok(())
    }
}

#[async_trait]
impl crate::Tool for AskUserQuestionTool {
    type Params = AskUserQuestionParams;
    type Output = AskUserQuestionOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "AskUserQuestion",
            description: "Ask user questions with predefined options during execution",
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
        let execution_context = ctx.execution_context;

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

            // Validate each question
            for (i, question) in questions.iter().enumerate() {
                if let Err(e) = Self::validate_question(question, i) {
                    yield ToolEvent::Error {
                        message: format!("Validation error: {}", e),
                    };
                    return;
                }
            }

            // Ask each question
            for (i, question) in questions.iter().enumerate() {
                if answers.contains_key(&question.header) {
                    if debug {
                        tracing::debug!(
                            header = %question.header,
                            answer = ?answers.get(&question.header),
                            "Question already answered, skipping"
                        );
                    }
                    continue; // Already answered
                }

                yield ToolEvent::Progress {
                    step: format!("Question {}/{}: {}", i + 1, questions.len(), question.header),
                    percentage: Some((i as f32 / questions.len() as f32) * 100.0),
                };

                let selected = match execution_context {
                    ExecutionContext::Tui => {
                        // Interactive TUI mode
                        if question.multi_select {
                            Self::ask_multi_select_tui(question, debug)
                        } else {
                            Self::ask_single_select_tui(question, debug)
                        }
                    }
                    ExecutionContext::NonInteractive => {
                        // Non-interactive CLI mode
                        Self::ask_cli_mode(question)
                    }
                };

                match selected {
                    Ok(answer) => {
                        answers.insert(question.header.clone(), answer.clone());

                        if debug {
                            tracing::debug!(
                                header = %question.header,
                                answer = %answer,
                                "Question answered"
                            );
                        }
                    }
                    Err(e) => {
                        yield ToolEvent::Error {
                            message: format!("Failed to get answer for '{}': {}", question.header, e),
                        };
                        return;
                    }
                }
            }

            yield ToolEvent::Result(AskUserQuestionOutput {
                questions_answered: questions.len(),
                answers,
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

    fn create_sample_question(multi_select: bool) -> Question {
        Question {
            question: "Choose your language?".to_string(),
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
            multi_select,
        }
    }

    #[test]
    fn test_validate_question_valid() {
        let question = create_sample_question(false);
        assert!(AskUserQuestionTool::validate_question(&question, 0).is_ok());
    }

    #[test]
    fn test_validate_question_header_too_long() {
        let mut question = create_sample_question(false);
        question.header = "ThisIsWayTooLong".to_string(); // 16 chars > 12
        let result = AskUserQuestionTool::validate_question(&question, 0);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("exceeds 12 characters"));
    }

    #[test]
    fn test_validate_question_empty_header() {
        let mut question = create_sample_question(false);
        question.header = "".to_string();
        let result = AskUserQuestionTool::validate_question(&question, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("header cannot be empty"));
    }

    #[test]
    fn test_validate_question_too_few_options() {
        let mut question = create_sample_question(false);
        question.options = vec![QuestionOption {
            label: "Only one".to_string(),
            description: "Not enough".to_string(),
        }];
        let result = AskUserQuestionTool::validate_question(&question, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must have 2-4 options"));
    }

    #[test]
    fn test_validate_question_too_many_options() {
        let mut question = create_sample_question(false);
        question.options = vec![
            QuestionOption {
                label: "One".to_string(),
                description: "First".to_string(),
            },
            QuestionOption {
                label: "Two".to_string(),
                description: "Second".to_string(),
            },
            QuestionOption {
                label: "Three".to_string(),
                description: "Third".to_string(),
            },
            QuestionOption {
                label: "Four".to_string(),
                description: "Fourth".to_string(),
            },
            QuestionOption {
                label: "Five".to_string(),
                description: "Fifth".to_string(),
            },
        ];
        let result = AskUserQuestionTool::validate_question(&question, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must have 2-4 options"));
    }

    #[test]
    fn test_validate_question_empty_label() {
        let mut question = create_sample_question(false);
        question.options[0].label = "".to_string();
        let result = AskUserQuestionTool::validate_question(&question, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("label cannot be empty"));
    }

    #[test]
    fn test_validate_question_empty_description() {
        let mut question = create_sample_question(false);
        question.options[0].description = "".to_string();
        let result = AskUserQuestionTool::validate_question(&question, 0);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("description cannot be empty"));
    }

    #[test]
    fn test_validate_question_empty_text() {
        let mut question = create_sample_question(false);
        question.question = "".to_string();
        let result = AskUserQuestionTool::validate_question(&question, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("question text cannot be empty"));
    }

    #[test]
    fn test_params_deserialization() {
        let json = r#"{
            "questions": [{
                "question": "Test?",
                "header": "test",
                "multiSelect": true,
                "options": [
                    {"label": "A", "description": "First"},
                    {"label": "B", "description": "Second"}
                ]
            }]
        }"#;

        let params: AskUserQuestionParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.questions.len(), 1);
        assert_eq!(params.questions[0].header, "test");
        assert!(params.questions[0].multi_select);
        assert_eq!(params.questions[0].options.len(), 2);
    }

    #[test]
    fn test_params_deserialization_with_answers() {
        let json = r#"{
            "questions": [{
                "question": "Test?",
                "header": "test",
                "multiSelect": false,
                "options": [
                    {"label": "A", "description": "First"},
                    {"label": "B", "description": "Second"}
                ]
            }],
            "answers": {
                "test": "A"
            }
        }"#;

        let params: AskUserQuestionParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.answers.len(), 1);
        assert_eq!(params.answers.get("test"), Some(&"A".to_string()));
    }

    #[test]
    fn test_output_serialization() {
        let mut answers = HashMap::new();
        answers.insert("q1".to_string(), "answer1".to_string());
        answers.insert("q2".to_string(), "answer2".to_string());

        let output = AskUserQuestionOutput {
            answers,
            questions_answered: 2,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("answer1"));
        assert!(json.contains("answer2"));
        assert!(json.contains("questions_answered"));
    }

    #[tokio::test]
    async fn test_tool_metadata() {
        let tool = AskUserQuestionTool;
        let metadata = tool.metadata();
        assert_eq!(metadata.name, "AskUserQuestion");
        assert!(!metadata.description.is_empty());
    }

    #[tokio::test]
    async fn test_tool_properties() {
        let tool = AskUserQuestionTool;
        assert!(tool.is_read_only());
        assert!(!tool.is_concurrency_safe());
    }

    #[tokio::test]
    async fn test_validation_too_many_questions() {
        let tool = AskUserQuestionTool;
        let params = AskUserQuestionParams {
            questions: vec![
                create_sample_question(false),
                create_sample_question(false),
                create_sample_question(false),
                create_sample_question(false),
                create_sample_question(false), // 5 questions > 4
            ],
            answers: HashMap::new(),
        };

        let ctx = ToolContext::default();
        let mut stream = tool.execute(params, &ctx).await.unwrap();

        // Should get error about too many questions
        let mut got_error = false;
        while let Some(event) = stream.next().await {
            if let ToolEvent::Error { message } = event {
                assert!(message.contains("Must have 1-4 questions"));
                got_error = true;
                break;
            }
        }
        assert!(got_error);
    }

    #[tokio::test]
    async fn test_validation_no_questions() {
        let tool = AskUserQuestionTool;
        let params = AskUserQuestionParams {
            questions: vec![],
            answers: HashMap::new(),
        };

        let ctx = ToolContext::default();
        let mut stream = tool.execute(params, &ctx).await.unwrap();

        let mut got_error = false;
        while let Some(event) = stream.next().await {
            if let ToolEvent::Error { message } = event {
                assert!(message.contains("Must have 1-4 questions"));
                got_error = true;
                break;
            }
        }
        assert!(got_error);
    }

    #[tokio::test]
    async fn test_validation_invalid_header() {
        let tool = AskUserQuestionTool;
        let mut question = create_sample_question(false);
        question.header = "VeryLongHeader123".to_string();

        let params = AskUserQuestionParams {
            questions: vec![question],
            answers: HashMap::new(),
        };

        let ctx = ToolContext::default();
        let mut stream = tool.execute(params, &ctx).await.unwrap();

        let mut got_error = false;
        while let Some(event) = stream.next().await {
            if let ToolEvent::Error { message } = event {
                assert!(message.contains("Validation error"));
                assert!(message.contains("exceeds 12 characters"));
                got_error = true;
                break;
            }
        }
        assert!(got_error);
    }

    #[tokio::test]
    async fn test_skips_already_answered() {
        let tool = AskUserQuestionTool;
        let question1 = Question {
            question: "First?".to_string(),
            header: "first".to_string(),
            options: vec![
                QuestionOption {
                    label: "A".to_string(),
                    description: "Option A".to_string(),
                },
                QuestionOption {
                    label: "B".to_string(),
                    description: "Option B".to_string(),
                },
            ],
            multi_select: false,
        };

        let mut answers = HashMap::new();
        answers.insert("first".to_string(), "A".to_string());

        let params = AskUserQuestionParams {
            questions: vec![question1],
            answers,
        };

        let ctx = ToolContext {
            debug: true,
            ..Default::default()
        };

        // This should complete immediately since the question is already answered
        // Without requiring any user input
        let mut stream = tool.execute(params, &ctx).await.unwrap();

        let mut got_result = false;
        while let Some(event) = stream.next().await {
            if let ToolEvent::Result(output) = event {
                assert_eq!(output.answers.get("first"), Some(&"A".to_string()));
                assert_eq!(output.questions_answered, 1);
                got_result = true;
                break;
            }
        }
        assert!(got_result);
    }

    #[test]
    fn test_question_option_clone() {
        let opt = QuestionOption {
            label: "Test".to_string(),
            description: "Test desc".to_string(),
        };
        let cloned = opt.clone();
        assert_eq!(opt.label, cloned.label);
        assert_eq!(opt.description, cloned.description);
    }

    #[test]
    fn test_question_clone() {
        let question = create_sample_question(true);
        let cloned = question.clone();
        assert_eq!(question.question, cloned.question);
        assert_eq!(question.header, cloned.header);
        assert_eq!(question.multi_select, cloned.multi_select);
        assert_eq!(question.options.len(), cloned.options.len());
    }

    // Interactive tests below are ignored by default as they require user input
    // Run with: cargo test -- --ignored --nocapture

    #[tokio::test]
    #[ignore]
    async fn test_interactive_single_select() {
        // This test requires terminal interaction via dialoguer.
        // Run manually with: cargo test test_interactive_single_select -- --ignored --nocapture
        let tool = AskUserQuestionTool;
        let params = AskUserQuestionParams {
            questions: vec![create_sample_question(false)],
            answers: HashMap::new(),
        };

        let ctx = ToolContext {
            execution_context: ExecutionContext::Tui,
            ..Default::default()
        };

        let mut stream = tool.execute(params, &ctx).await.unwrap();

        while let Some(event) = stream.next().await {
            match event {
                ToolEvent::Progress { step, .. } => {
                    println!("Progress: {}", step);
                }
                ToolEvent::Result(output) => {
                    println!("Result: {:?}", output);
                    assert_eq!(output.questions_answered, 1);
                }
                ToolEvent::Error { message } => {
                    panic!("Error: {}", message);
                }
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_interactive_multi_select() {
        let tool = AskUserQuestionTool;
        let params = AskUserQuestionParams {
            questions: vec![create_sample_question(true)],
            answers: HashMap::new(),
        };

        let ctx = ToolContext {
            execution_context: ExecutionContext::Tui,
            ..Default::default()
        };

        let mut stream = tool.execute(params, &ctx).await.unwrap();

        while let Some(event) = stream.next().await {
            match event {
                ToolEvent::Progress { step, .. } => {
                    println!("Progress: {}", step);
                }
                ToolEvent::Result(output) => {
                    println!("Result: {:?}", output);
                    assert_eq!(output.questions_answered, 1);
                }
                ToolEvent::Error { message } => {
                    panic!("Error: {}", message);
                }
            }
        }
    }
}
