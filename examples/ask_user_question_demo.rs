//! Demo of AskUserQuestion tool
//!
//! This example demonstrates how to use the AskUserQuestion tool
//! in both TUI and CLI modes.
//!
//! Run with: cargo run --example ask_user_question_demo

use rustyclawd_tools::{
    ask_user_question::{AskUserQuestionParams, Question, QuestionOption},
    AskUserQuestionTool, ExecutionContext, Tool, ToolContext, ToolEvent,
};
use futures::StreamExt;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    println!("AskUserQuestion Tool Demo");
    println!("=========================\n");

    // Example 1: Single select question
    println!("Example 1: Single Select");
    let params = AskUserQuestionParams {
        questions: vec![Question {
            question: "Which programming language do you prefer?".to_string(),
            header: "lang".to_string(),
            multi_select: false,
            options: vec![
                QuestionOption {
                    label: "Rust".to_string(),
                    description: "Systems programming with safety".to_string(),
                },
                QuestionOption {
                    label: "JavaScript".to_string(),
                    description: "Web development".to_string(),
                },
                QuestionOption {
                    label: "Python".to_string(),
                    description: "Data science and scripting".to_string(),
                },
            ],
        }],
        answers: HashMap::new(),
    };

    let ctx = ToolContext {
        cwd: std::env::current_dir().unwrap_or_default(),
        debug: false,
        metadata: serde_json::Value::Null,
        execution_context: ExecutionContext::Tui, // Change to NonInteractive to test CLI mode
    };

    let tool = AskUserQuestionTool;
    let mut stream = tool.execute(params, &ctx).await.unwrap();

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Progress { step, percentage } => {
                if let Some(pct) = percentage {
                    println!("[{:.0}%] {}", pct, step);
                } else {
                    println!("{}", step);
                }
            }
            ToolEvent::Result(output) => {
                println!("\nResult:");
                println!("  Answers: {:?}", output.answers);
                println!("  Questions answered: {}", output.questions_answered);
            }
            ToolEvent::Error { message } => {
                eprintln!("Error: {}", message);
            }
        }
    }

    println!("\n---\n");

    // Example 2: Multi-select question
    println!("Example 2: Multi-Select");
    let params = AskUserQuestionParams {
        questions: vec![Question {
            question: "Which features do you want to enable?".to_string(),
            header: "features".to_string(),
            multi_select: true,
            options: vec![
                QuestionOption {
                    label: "Authentication".to_string(),
                    description: "User login and session management".to_string(),
                },
                QuestionOption {
                    label: "Database".to_string(),
                    description: "PostgreSQL integration".to_string(),
                },
                QuestionOption {
                    label: "API".to_string(),
                    description: "REST API endpoints".to_string(),
                },
            ],
        }],
        answers: HashMap::new(),
    };

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Progress { step, percentage } => {
                if let Some(pct) = percentage {
                    println!("[{:.0}%] {}", pct, step);
                } else {
                    println!("{}", step);
                }
            }
            ToolEvent::Result(output) => {
                println!("\nResult:");
                println!("  Answers: {:?}", output.answers);
                println!("  Questions answered: {}", output.questions_answered);
            }
            ToolEvent::Error { message } => {
                eprintln!("Error: {}", message);
            }
        }
    }

    println!("\n---\n");

    // Example 3: Multiple questions
    println!("Example 3: Multiple Questions");
    let params = AskUserQuestionParams {
        questions: vec![
            Question {
                question: "Choose your build system?".to_string(),
                header: "build".to_string(),
                multi_select: false,
                options: vec![
                    QuestionOption {
                        label: "Cargo".to_string(),
                        description: "Rust's native build tool".to_string(),
                    },
                    QuestionOption {
                        label: "Bazel".to_string(),
                        description: "Scalable build system".to_string(),
                    },
                ],
            },
            Question {
                question: "Select testing frameworks?".to_string(),
                header: "test".to_string(),
                multi_select: true,
                options: vec![
                    QuestionOption {
                        label: "tokio-test".to_string(),
                        description: "Async testing utilities".to_string(),
                    },
                    QuestionOption {
                        label: "criterion".to_string(),
                        description: "Benchmarking framework".to_string(),
                    },
                ],
            },
        ],
        answers: HashMap::new(),
    };

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Progress { step, percentage } => {
                if let Some(pct) = percentage {
                    println!("[{:.0}%] {}", pct, step);
                } else {
                    println!("{}", step);
                }
            }
            ToolEvent::Result(output) => {
                println!("\nResult:");
                println!("  Answers: {:?}", output.answers);
                println!("  Questions answered: {}", output.questions_answered);
            }
            ToolEvent::Error { message } => {
                eprintln!("Error: {}", message);
            }
        }
    }

    println!("\n---\n");

    // Example 4: Resumption (pre-filled answer)
    println!("Example 4: Resumption (with pre-filled answer)");
    let mut answers = HashMap::new();
    answers.insert("lang".to_string(), "Rust".to_string());

    let params = AskUserQuestionParams {
        questions: vec![Question {
            question: "Which language?".to_string(),
            header: "lang".to_string(),
            multi_select: false,
            options: vec![
                QuestionOption {
                    label: "Rust".to_string(),
                    description: "Already selected".to_string(),
                },
                QuestionOption {
                    label: "Go".to_string(),
                    description: "Alternative".to_string(),
                },
            ],
        }],
        answers,
    };

    let mut stream = tool.execute(params, &ctx).await.unwrap();

    while let Some(event) = stream.next().await {
        match event {
            ToolEvent::Progress { step, percentage } => {
                if let Some(pct) = percentage {
                    println!("[{:.0}%] {}", pct, step);
                } else {
                    println!("{}", step);
                }
            }
            ToolEvent::Result(output) => {
                println!("\nResult:");
                println!("  Answers: {:?}", output.answers);
                println!("  Questions answered: {}", output.questions_answered);
                println!("  (Notice: question was skipped because answer was pre-filled)");
            }
            ToolEvent::Error { message } => {
                eprintln!("Error: {}", message);
            }
        }
    }
}
