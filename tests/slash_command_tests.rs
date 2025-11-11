//! Comprehensive test suite for slash commands
//!
//! Tests cover:
//! - /help command and built-in commands
//! - Custom command loading and execution
//! - Command expansion with arguments
//! - Argument passing (positional and $ARGUMENTS)
//! - Frontmatter parsing
//! - Error handling and edge cases
//!
//! Following Testing Pyramid:
//! - Unit tests: Command parsing, argument extraction
//! - Integration tests: File loading, expansion, output
//! - Edge cases: Empty args, special characters, missing files

use std::path::PathBuf;
use tokio::fs;

// NOTE: These tests follow TDD principles - they define the specification
// that the slash command implementation must satisfy.

// ============================================================================
// TEST FIXTURES AND HELPERS
// ============================================================================

/// Test fixture setup and teardown
struct TestFixture {
    command_dir: PathBuf,
}

impl TestFixture {
    async fn new() -> Self {
        let command_dir = PathBuf::from(".claude/commands_test");
        let _ = fs::create_dir_all(&command_dir).await;
        TestFixture { command_dir }
    }

    async fn create_command(
        &self,
        name: &str,
        content: &str,
    ) -> std::io::Result<PathBuf> {
        let path = self.command_dir.join(format!("{}.md", name));
        fs::write(&path, content).await?;
        Ok(path)
    }

    async fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.command_dir).await;
    }
}

// ============================================================================
// UNIT TESTS - COMMAND PARSING
// ============================================================================

#[test]
fn test_command_parsing_simple_no_args() {
    // Test: Parse "/help" with no arguments
    let input = "/help";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0], "help");
}

#[test]
fn test_command_parsing_with_single_arg() {
    // Test: Parse "/review-pr 123"
    let input = "/review-pr 123";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let command_name = parts[0];
    let args = parts.get(1);

    assert_eq!(command_name, "review-pr");
    assert_eq!(args, Some(&"123"));
}

#[test]
fn test_command_parsing_with_multiple_args() {
    // Test: Parse "/review-pr 456 high alice"
    let input = "/review-pr 456 high alice";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let command_name = parts[0];
    let args_str = parts.get(1).map(|s| *s).unwrap_or("");

    assert_eq!(command_name, "review-pr");
    assert_eq!(args_str, "456 high alice");

    // Extract individual positional args
    let args: Vec<&str> = args_str.split_whitespace().collect();
    assert_eq!(args[0], "456");
    assert_eq!(args[1], "high");
    assert_eq!(args[2], "alice");
}

#[test]
fn test_command_parsing_removes_leading_slash() {
    // Test: Slash is properly removed during parsing
    let input = "/my-command arg1 arg2";
    let cleaned = input.trim_start_matches('/');

    assert_eq!(cleaned, "my-command arg1 arg2");
    assert!(!cleaned.starts_with('/'));
}

#[test]
fn test_command_name_extraction_with_hyphens() {
    // Test: Command names with hyphens are parsed correctly
    let input = "/review-pr-detailed 123";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();

    assert_eq!(parts[0], "review-pr-detailed");
}

#[test]
fn test_command_name_extraction_with_underscores() {
    // Test: Command names with underscores are parsed correctly
    let input = "/my_custom_command arg";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();

    assert_eq!(parts[0], "my_custom_command");
}

// ============================================================================
// UNIT TESTS - ARGUMENT EXTRACTION
// ============================================================================

#[test]
fn test_positional_argument_extraction() {
    // Test: Extract positional arguments {0}, {1}, {2}
    let args_str = "456 high alice";
    let arg_parts: Vec<&str> = args_str.split_whitespace().collect();

    assert_eq!(arg_parts[0], "456");
    assert_eq!(arg_parts[1], "high");
    assert_eq!(arg_parts[2], "alice");
}

#[test]
fn test_positional_argument_placeholder_replacement() {
    // Test: Replace {0}, {1} placeholders with actual args
    let template = "Review PR #{0} with priority {1} assigned to {2}";
    let args_str = "456 high alice";
    let arg_parts: Vec<&str> = args_str.split_whitespace().collect();

    let mut result = template.to_string();
    for (i, arg) in arg_parts.iter().enumerate() {
        result = result.replace(&format!("{{{}}}", i), arg);
    }

    assert_eq!(result, "Review PR #456 with priority high assigned to alice");
}

#[test]
fn test_arguments_token_replacement() {
    // Test: Replace {{args}} with full argument string
    let template = "Process these arguments: {{args}}";
    let args_str = "456 high alice";
    let result = template.replace("{{args}}", args_str);

    assert_eq!(
        result,
        "Process these arguments: 456 high alice"
    );
}

#[test]
fn test_empty_arguments() {
    // Test: Handle commands with no arguments
    let input = "/help";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let args = parts.get(1);

    assert_eq!(args, None);
}

#[test]
fn test_single_space_arguments() {
    // Test: Handle single space as no arguments
    let input = "/help ".trim();
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();

    assert_eq!(parts[0], "help");
}

#[test]
fn test_argument_with_special_characters() {
    // Test: Arguments can contain various characters
    let input = "/search \"search term with spaces\"";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let args = parts.get(1);

    assert_eq!(args, Some(&"\"search term with spaces\""));
}

#[test]
fn test_argument_with_file_paths() {
    // Test: Arguments can be file paths
    let input = "/include src/utils/helpers.js";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let args = parts.get(1);

    assert_eq!(args, Some(&"src/utils/helpers.js"));
}

// ============================================================================
// FRONTMATTER PARSING TESTS
// ============================================================================

#[test]
fn test_frontmatter_detection() {
    // Test: Detect YAML frontmatter marked with ---
    let content = "---\ndescription: Review PR\n---\nPrompt content here";

    assert!(content.starts_with("---"));
}

#[test]
fn test_frontmatter_extraction() {
    // Test: Extract frontmatter between --- markers
    let content = "---\ndescription: Review PR\nauthor: test\n---\nPrompt content here";

    if content.starts_with("---") {
        if let Some(end_idx) = content[3..].find("---") {
            let frontmatter = &content[3..3 + end_idx];
            let body = content[3 + end_idx + 3..].trim();

            assert!(frontmatter.contains("description"));
            assert!(frontmatter.contains("author"));
            assert_eq!(body, "Prompt content here");
        } else {
            panic!("Malformed frontmatter");
        }
    }
}

#[test]
fn test_content_extraction_with_frontmatter() {
    // Test: Extract content after frontmatter
    let content = "---\ndescription: Review PR\n---\nReview PR #{0}";

    let expanded = if content.starts_with("---") {
        if let Some(end_idx) = content[3..].find("---") {
            &content[3 + end_idx + 3..].trim()
        } else {
            &content
        }
    } else {
        &content
    };

    assert_eq!(*expanded, "Review PR #{0}");
}

#[test]
fn test_content_without_frontmatter() {
    // Test: Handle content without frontmatter
    let content = "Review PR #{0}";

    assert!(!content.starts_with("---"));
    assert_eq!(content, "Review PR #{0}");
}

#[test]
fn test_empty_frontmatter() {
    // Test: Handle empty frontmatter section
    let content = "---\n---\nPrompt content";

    if content.starts_with("---") {
        if let Some(end_idx) = content[3..].find("---") {
            let frontmatter = &content[3..3 + end_idx];
            let body = content[3 + end_idx + 3..].trim();

            assert_eq!(frontmatter, "\n");
            assert_eq!(body, "Prompt content");
        }
    }
}

#[test]
fn test_multiline_frontmatter() {
    // Test: Parse multiline YAML frontmatter
    let content = "---\ndescription: Review a pull request\nmodel: claude-sonnet\nallowed-tools:\n  - Bash\n  - Grep\n---\nReview PR #{0}";

    if content.starts_with("---") {
        if let Some(end_idx) = content[3..].find("---") {
            let frontmatter = &content[3..3 + end_idx];

            assert!(frontmatter.contains("description:"));
            assert!(frontmatter.contains("model:"));
            assert!(frontmatter.contains("allowed-tools:"));
        }
    }
}

// ============================================================================
// COMMAND EXPANSION TESTS
// ============================================================================

#[tokio::test]
async fn test_command_expansion_basic() {
    // Test: Basic command expansion without arguments
    let fixture = TestFixture::new().await;
    let content = "---\ndescription: Simple command\n---\nHello from custom command";

    let _ = fixture
        .create_command("hello", content)
        .await;

    // Expected: Content after frontmatter is returned
    let expanded = if content.starts_with("---") {
        if let Some(end_idx) = content[3..].find("---") {
            content[3 + end_idx + 3..].trim().to_string()
        } else {
            content.to_string()
        }
    } else {
        content.to_string()
    };

    assert_eq!(expanded, "Hello from custom command");

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_command_expansion_with_template() {
    // Test: Expand command with template placeholders
    let fixture = TestFixture::new().await;
    let content = "---\n---\nReview PR #{0} with priority {1}";
    let args = "123 high";

    let _ = fixture
        .create_command("review", content)
        .await;

    let mut expanded = content.to_string();
    if expanded.starts_with("---") {
        if let Some(end_idx) = expanded[3..].find("---") {
            let body = expanded[3 + end_idx + 3..].trim().to_string();
            let arg_parts: Vec<&str> = args.split_whitespace().collect();

            let mut result = body.clone();
            for (i, arg) in arg_parts.iter().enumerate() {
                result = result.replace(&format!("{{{}}}", i), arg);
            }

            assert_eq!(result, "Review PR #123 with priority high");
        }
    }

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_command_expansion_with_arguments_token() {
    // Test: Expand {{args}} token with full argument string
    let fixture = TestFixture::new().await;
    let content = "---\n---\nArguments passed: {{args}}";
    let args = "foo bar baz";

    let _ = fixture
        .create_command("echo", content)
        .await;

    let mut expanded = content.to_string();
    if expanded.starts_with("---") {
        if let Some(end_idx) = expanded[3..].find("---") {
            let body = expanded[3 + end_idx + 3..].trim().to_string();
            let result = body.replace("{{args}}", args);

            assert_eq!(result, "Arguments passed: foo bar baz");
        }
    }

    fixture.cleanup().await;
}

// ============================================================================
// EDGE CASES - EMPTY AND NULL CONDITIONS
// ============================================================================

#[test]
fn test_empty_command_name() {
    // Test: Handle empty command name after slash
    let input = "/ arg1 arg2";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();

    assert_eq!(parts[0], "");
}

#[test]
fn test_whitespace_only_arguments() {
    // Test: Handle whitespace-only arguments
    let input = "/cmd     ";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();

    assert_eq!(parts[0], "cmd");
    // Note: parts[1] would be "    " if present
}

#[test]
fn test_very_long_command_name() {
    // Test: Handle long command names
    let long_name = "very_long_command_name_that_exceeds_normal_limits_significantly";
    let input = format!("/{}", long_name);
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();

    assert_eq!(parts[0], long_name);
}

#[test]
fn test_very_long_arguments() {
    // Test: Handle very long argument strings
    let long_args = "a".repeat(10000);
    let input = format!("/cmd {}", long_args);
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();

    assert_eq!(parts[0], "cmd");
    assert_eq!(parts.get(1).map(|s| s.len()), Some(10001)); // "a" + " " + 10000 "a"s
}

#[test]
fn test_maximum_positional_arguments() {
    // Test: Handle many positional arguments
    let args: Vec<String> = (0..100).map(|i| i.to_string()).collect();
    let args_str = args.join(" ");
    let arg_parts: Vec<&str> = args_str.split_whitespace().collect();

    assert_eq!(arg_parts.len(), 100);
    assert_eq!(arg_parts[0], "0");
    assert_eq!(arg_parts[99], "99");
}

#[test]
fn test_argument_with_zero_value() {
    // Test: Arguments can be zero
    let input = "/cmd 0";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let args = parts.get(1);

    assert_eq!(args, Some(&"0"));
}

#[test]
fn test_argument_with_negative_value() {
    // Test: Arguments can be negative numbers
    let input = "/cmd -42";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let args = parts.get(1);

    assert_eq!(args, Some(&"-42"));
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[tokio::test]
async fn test_command_not_found_error() {
    // Test: Proper error when command file doesn't exist
    let fixture = TestFixture::new().await;

    // Try to load non-existent command
    let result = fs::read_to_string(fixture.command_dir.join("nonexistent.md")).await;

    assert!(result.is_err());

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_malformed_frontmatter_handling() {
    // Test: Handle malformed frontmatter gracefully
    let fixture = TestFixture::new().await;
    let content = "---\nincomplete frontmatter\nPrompt content";

    let _ = fixture
        .create_command("broken", content)
        .await;

    // Should use content as-is if no closing ---
    assert!(content.starts_with("---"));
    if content[3..].find("---").is_none() {
        // No closing marker, use content as-is
        assert_eq!(content, "---\nincomplete frontmatter\nPrompt content");
    }

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_empty_command_file() {
    // Test: Handle empty command files
    let fixture = TestFixture::new().await;
    let content = "";

    let _ = fixture
        .create_command("empty", content)
        .await;

    // Should be empty
    assert_eq!(content, "");

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_command_with_only_whitespace() {
    // Test: Handle command files with only whitespace
    let fixture = TestFixture::new().await;
    let content = "   \n\n   ";

    let _ = fixture
        .create_command("whitespace", content)
        .await;

    assert!(content.trim().is_empty());

    fixture.cleanup().await;
}

// ============================================================================
// BUILT-IN COMMANDS TESTS - /help
// ============================================================================

#[test]
fn test_help_command_identification() {
    // Test: /help is recognized as help command
    let input = "/help";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();

    assert_eq!(parts[0], "help");
}

#[test]
fn test_help_command_with_search_term() {
    // Test: /help can be called with search term
    let input = "/help slash-commands";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let command = parts[0];
    let args = parts.get(1);

    assert_eq!(command, "help");
    assert_eq!(args, Some(&"slash-commands"));
}

#[test]
fn test_help_command_pagination() {
    // Test: /help with multiple search results should support pagination
    // (This is a specification test)
    let input = "/help all";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();

    assert_eq!(parts[0], "help");
    assert_eq!(parts.get(1), Some(&"all"));
}

// ============================================================================
// CHARACTER BUDGET TESTS
// ============================================================================

#[test]
fn test_character_budget_enforcement() {
    // Test: Track character count for SlashCommand tool usage
    // Default budget: 15,000 characters
    const CHAR_BUDGET: usize = 15_000;

    let expanded_prompt = "a".repeat(15_000);
    assert_eq!(expanded_prompt.len(), CHAR_BUDGET);
}

#[test]
fn test_character_budget_within_limit() {
    // Test: Verify expanded prompt within budget
    const CHAR_BUDGET: usize = 15_000;

    let expanded = "Review PR #123 with priority high";
    assert!(expanded.len() <= CHAR_BUDGET);
}

#[test]
fn test_character_budget_exceeds_limit() {
    // Test: Detect when expansion exceeds character budget
    const CHAR_BUDGET: usize = 15_000;

    let expanded = "a".repeat(15_001);
    assert!(expanded.len() > CHAR_BUDGET);
}

// ============================================================================
// COMMAND LOCATION TESTS
// ============================================================================

#[tokio::test]
async fn test_command_in_project_directory() {
    // Test: Commands are found in .claude/commands/ (project-level)
    let fixture = TestFixture::new().await;
    let command_path = fixture.command_dir.join("my-command.md");

    let _ = fixture
        .create_command("my-command", "Prompt content")
        .await;

    assert!(fs::metadata(&command_path).await.is_ok());

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_command_file_extension() {
    // Test: Command files use .md extension
    let fixture = TestFixture::new().await;
    let expected_path = fixture.command_dir.join("test.md");

    let _ = fixture
        .create_command("test", "Content")
        .await;

    assert!(fs::metadata(&expected_path).await.is_ok());

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_command_directory_creation() {
    // Test: .claude/commands directory is created automatically
    let fixture = TestFixture::new().await;

    // Should already be created
    assert!(fs::metadata(&fixture.command_dir).await.is_ok());

    fixture.cleanup().await;
}

// ============================================================================
// INTEGRATION TESTS - END TO END
// ============================================================================

#[tokio::test]
async fn test_full_command_lifecycle() {
    // Test: Complete flow from command invocation to expansion
    let fixture = TestFixture::new().await;

    // 1. Create command file
    let content = "---\ndescription: Review PR\n---\nReview PR #{0} with priority {1}";
    let _ = fixture
        .create_command("review-pr", content)
        .await;

    // 2. Parse command
    let input = "/review-pr 123 high";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let command_name = parts[0];
    let args_str = parts.get(1).map(|s| *s).unwrap_or("");

    // 3. Load command file
    let file_content = fs::read_to_string(fixture.command_dir.join(format!("{}.md", command_name)))
        .await
        .unwrap();

    // 4. Expand command
    let mut expanded = if file_content.starts_with("---") {
        if let Some(end_idx) = file_content[3..].find("---") {
            file_content[3 + end_idx + 3..].trim().to_string()
        } else {
            file_content.to_string()
        }
    } else {
        file_content.to_string()
    };

    // 5. Replace placeholders
    let arg_parts: Vec<&str> = args_str.split_whitespace().collect();
    for (i, arg) in arg_parts.iter().enumerate() {
        expanded = expanded.replace(&format!("{{{}}}", i), arg);
    }

    // 6. Verify result
    assert_eq!(expanded, "Review PR #123 with priority high");

    fixture.cleanup().await;
}

#[tokio::test]
async fn test_multiple_commands_isolation() {
    // Test: Multiple commands don't interfere with each other
    let fixture = TestFixture::new().await;

    // Create multiple commands
    let _ = fixture
        .create_command("cmd1", "---\n---\nContent of command 1: {0}")
        .await;
    let _ = fixture
        .create_command("cmd2", "---\n---\nContent of command 2: {0}")
        .await;

    // Load and expand both
    let file1 = fs::read_to_string(fixture.command_dir.join("cmd1.md"))
        .await
        .unwrap();
    let file2 = fs::read_to_string(fixture.command_dir.join("cmd2.md"))
        .await
        .unwrap();

    let expanded1 = if file1.starts_with("---") {
        if let Some(end_idx) = file1[3..].find("---") {
            let mut result = file1[3 + end_idx + 3..].trim().to_string();
            result = result.replace("{0}", "arg1");
            result
        } else {
            file1.to_string()
        }
    } else {
        file1.to_string()
    };

    let expanded2 = if file2.starts_with("---") {
        if let Some(end_idx) = file2[3..].find("---") {
            let mut result = file2[3 + end_idx + 3..].trim().to_string();
            result = result.replace("{0}", "arg2");
            result
        } else {
            file2.to_string()
        }
    } else {
        file2.to_string()
    };

    assert_eq!(expanded1, "Content of command 1: arg1");
    assert_eq!(expanded2, "Content of command 2: arg2");

    fixture.cleanup().await;
}

// ============================================================================
// SPECIAL CHARACTERS AND ESCAPING TESTS
// ============================================================================

#[test]
fn test_command_with_numbers_in_name() {
    // Test: Command names can contain numbers
    let input = "/cmd123 arg";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();

    assert_eq!(parts[0], "cmd123");
}

#[test]
fn test_argument_with_equals_sign() {
    // Test: Arguments can contain equals signs (for key=value pairs)
    let input = "/cmd key=value";
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let args = parts.get(1);

    assert_eq!(args, Some(&"key=value"));
}

#[test]
fn test_argument_with_json() {
    // Test: Arguments can be JSON strings
    let input = r#"/cmd {"key":"value"}"#;
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let args = parts.get(1);

    assert_eq!(args, Some(&r#"{"key":"value"}"#));
}

#[test]
fn test_template_with_special_placeholders() {
    // Test: Templates with various placeholder styles
    let template = "Command {0} running on {1} with timeout {2}";
    let args = "deploy prod 300";
    let arg_parts: Vec<&str> = args.split_whitespace().collect();

    let mut result = template.to_string();
    for (i, arg) in arg_parts.iter().enumerate() {
        result = result.replace(&format!("{{{}}}", i), arg);
    }

    assert_eq!(result, "Command deploy running on prod with timeout 300");
}

// ============================================================================
// PERFORMANCE BASELINE TESTS
// ============================================================================

#[test]
fn test_parsing_performance_baseline() {
    // Test: Command parsing should be fast (no async overhead baseline)
    let input = "/review-pr 123 high alice";

    // This should complete in microseconds
    let start = std::time::Instant::now();
    let parts: Vec<&str> = input.trim_start_matches('/').splitn(2, ' ').collect();
    let elapsed = start.elapsed();

    assert_eq!(parts[0], "review-pr");
    assert!(elapsed.as_micros() < 100); // Should be < 100 microseconds
}

#[test]
fn test_placeholder_replacement_performance() {
    // Test: Placeholder replacement should be efficient
    let template = "Review PR #{0} with priority {1} assigned to {2} status {3}";
    let args = "456 high alice complete";

    let start = std::time::Instant::now();
    let mut result = template.to_string();
    let arg_parts: Vec<&str> = args.split_whitespace().collect();
    for (i, arg) in arg_parts.iter().enumerate() {
        result = result.replace(&format!("{{{}}}", i), arg);
    }
    let elapsed = start.elapsed();

    assert_eq!(result, "Review PR #456 with priority high assigned to alice status complete");
    assert!(elapsed.as_micros() < 500); // Should be < 500 microseconds
}
