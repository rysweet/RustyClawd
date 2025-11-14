//! Integration tests for the slash command system

use rustyclawd::commands::*;
use std::path::PathBuf;
use tokio::fs;

#[tokio::test]
async fn test_command_parser_simple() {
    let parser = CommandParser::new();
    let cmd = parser.parse("/help").unwrap();

    assert_eq!(cmd.name, "help");
    assert!(cmd.args.is_empty());
}

#[tokio::test]
async fn test_command_parser_with_args() {
    let parser = CommandParser::new();
    let cmd = parser.parse("/review-pr 123").unwrap();

    assert_eq!(cmd.name, "review-pr");
    assert_eq!(cmd.get_arg(0), Some("123"));
}

#[tokio::test]
async fn test_registry_creation() {
    let registry = Registry::new(PathBuf::from(".test_commands"));
    assert_eq!(registry.command_count(), 0);
}

#[tokio::test]
async fn test_registry_register_and_retrieve() {
    let mut registry = Registry::new(PathBuf::from(".test"));

    let cmd = rustyclawd::commands::loader::LoadedCommand {
        name: "hello".to_string(),
        frontmatter: rustyclawd::commands::loader::FrontMatter::default(),
        content: "Hello world".to_string(),
    };

    registry.register(cmd).unwrap();
    assert!(registry.has_command("hello"));

    let retrieved = registry.get("hello").unwrap();
    assert_eq!(retrieved.content, "Hello world");
}

#[tokio::test]
async fn test_executor_builtin_command() {
    let executor = Executor::new();
    let registry = Registry::new(PathBuf::from(".test"));
    let cmd = Command::new("help".to_string(), None);

    let result = executor.execute(&cmd, &registry).await.unwrap();

    assert!(result.is_builtin);
    assert_eq!(result.command_name, "help");
    assert!(!result.expanded_prompt.is_empty());
}

#[tokio::test]
async fn test_executor_custom_command() {
    let executor = Executor::new();
    let mut registry = Registry::new(PathBuf::from(".test"));

    let cmd_obj = rustyclawd::commands::loader::LoadedCommand {
        name: "review".to_string(),
        frontmatter: rustyclawd::commands::loader::FrontMatter::default(),
        content: "Review PR #{0}".to_string(),
    };

    registry.register(cmd_obj).unwrap();

    let cmd = Command::new("review".to_string(), Some("456".to_string()));
    let result = executor.execute(&cmd, &registry).await.unwrap();

    assert!(!result.is_builtin);
    assert_eq!(result.expanded_prompt, "Review PR #456");
    assert!(result.is_within_budget());
}

#[tokio::test]
async fn test_template_expansion_multiple_args() {
    let executor = Executor::new();
    let mut registry = Registry::new(PathBuf::from(".test"));

    let cmd_obj = rustyclawd::commands::loader::LoadedCommand {
        name: "analyze".to_string(),
        frontmatter: rustyclawd::commands::loader::FrontMatter::default(),
        content: "Analyze {0} with priority {1} assigned to {2}".to_string(),
    };

    registry.register(cmd_obj).unwrap();

    let cmd = Command::new(
        "analyze".to_string(),
        Some("PR-123 high alice".to_string()),
    );
    let result = executor.execute(&cmd, &registry).await.unwrap();

    assert_eq!(result.expanded_prompt, "Analyze PR-123 with priority high assigned to alice");
    assert_eq!(result.arguments.len(), 3);
}

#[test]
fn test_command_result_budget_tracking() {
    let result = CommandResult {
        command_name: "test".to_string(),
        expanded_prompt: "x".repeat(5000),
        is_builtin: false,
        arguments: vec![],
    };

    assert!(result.is_within_budget());
    assert_eq!(result.char_count(), 5000);
    assert!(result.budget_percentage() < 50.0);
}

#[tokio::test]
async fn test_executor_character_limit() {
    let executor = Executor::new();
    let mut registry = Registry::new(PathBuf::from(".test"));

    let cmd_obj = rustyclawd::commands::loader::LoadedCommand {
        name: "huge".to_string(),
        frontmatter: rustyclawd::commands::loader::FrontMatter::default(),
        content: "x".repeat(15_001),
    };

    registry.register(cmd_obj).unwrap();

    let cmd = Command::new("huge".to_string(), None);
    let result = executor.execute(&cmd, &registry).await;

    assert!(result.is_err());
}

#[test]
fn test_slash_commands_constants() {
    assert_eq!(commands::DEFAULT_COMMANDS_DIR, ".claude/commands");
    assert_eq!(commands::MAX_EXPANDED_CHARS, 15_000);
}

#[test]
fn test_builtin_command_help() {
    use rustyclawd::commands::builtins::BuiltinCommands;

    let cmd = Command::new("help".to_string(), None);
    let output = BuiltinCommands::execute(&cmd);

    assert!(output.is_some());
    assert!(output.unwrap().contains("Help"));
}

#[test]
fn test_builtin_command_exit() {
    use rustyclawd::commands::builtins::BuiltinCommands;

    let cmd = Command::new("exit".to_string(), None);
    let output = BuiltinCommands::execute(&cmd);

    assert!(output.is_some());
    assert!(output.unwrap().contains("Exiting"));
}

#[test]
fn test_builtin_command_clear() {
    use rustyclawd::commands::builtins::BuiltinCommands;

    let cmd = Command::new("clear".to_string(), None);
    let output = BuiltinCommands::execute(&cmd);

    assert!(output.is_some());
    assert!(output.unwrap().contains("cleared"));
}

#[test]
fn test_parser_command_with_namespace() {
    let parser = CommandParser::new();
    let cmd = parser.parse("/amplihack:ultrathink test").unwrap();

    assert_eq!(cmd.name, "amplihack:ultrathink");
    assert_eq!(cmd.get_arg(0), Some("test"));
}

#[test]
fn test_parser_invalid_input() {
    let parser = CommandParser::new();
    let result = parser.parse("no-slash");

    assert!(result.is_err());
}

#[test]
fn test_parser_empty_command() {
    let parser = CommandParser::new();
    let result = parser.parse("/");

    assert!(result.is_err());
}
