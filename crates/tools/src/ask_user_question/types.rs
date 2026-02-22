//! AskUserQuestion types - Data models for interactive user prompts
//!
//! Contains question definitions, option structures, parameters, and output types.

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
