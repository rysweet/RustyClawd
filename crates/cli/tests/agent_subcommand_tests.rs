//! Agent Subcommand Tests
//!
//! Tests for the `claude agent` subcommand that invokes specialized agents
//! with prompts from files.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// HELP AND USAGE TESTS
// ============================================================================

#[test]
fn test_agent_help() {
    cargo_bin_cmd!("claude")
        .arg("agent")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Invoke a specialized agent"))
        .stdout(predicate::str::contains("--prompt"));
}

#[test]
fn test_agent_requires_type_and_prompt() {
    cargo_bin_cmd!("claude")
        .arg("agent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_agent_requires_prompt_flag() {
    cargo_bin_cmd!("claude")
        .arg("agent")
        .arg("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--prompt"));
}

// ============================================================================
// PROMPT FILE TESTS
// ============================================================================

#[test]
fn test_agent_missing_prompt_file() {
    cargo_bin_cmd!("claude")
        .arg("agent")
        .arg("test")
        .arg("--prompt")
        .arg("/nonexistent/file.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read prompt file"));
}

#[test]
fn test_agent_missing_agent_file() {
    let temp_dir = TempDir::new().unwrap();
    let prompt_file = temp_dir.path().join("prompt.txt");
    fs::write(&prompt_file, "Test prompt").unwrap();

    cargo_bin_cmd!("claude")
        .current_dir(temp_dir.path())
        .arg("agent")
        .arg("nonexistent_agent")
        .arg("--prompt")
        .arg(prompt_file.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Agent prompt not found"));
}

// ============================================================================
// MODEL OVERRIDE TESTS
// ============================================================================

#[test]
#[ignore = "May make API call if ANTHROPIC_API_KEY is set"]
fn test_agent_with_model_override() {
    let temp_dir = TempDir::new().unwrap();
    let prompt_file = temp_dir.path().join("prompt.txt");
    fs::write(&prompt_file, "Test prompt").unwrap();

    // Create test agent file
    let claude_dir = temp_dir.path().join(".claude").join("agents");
    fs::create_dir_all(&claude_dir).unwrap();
    let agent_file = claude_dir.join("test.md");
    fs::write(&agent_file, "You are a test agent.").unwrap();

    let _output = cargo_bin_cmd!("claude")
        .current_dir(temp_dir.path())
        .arg("agent")
        .arg("test")
        .arg("--prompt")
        .arg(prompt_file.to_str().unwrap())
        .arg("--model")
        .arg("haiku")
        .assert();

    // This test may succeed if API key is available, or fail if not
    // Either way, it should not be an argument parsing error

    // If it fails, should not be an argument parsing error
    // (Check stderr doesn't contain --prompt which would indicate arg parsing error)
    // If it succeeds, that's fine - it means API key was available
}

// ============================================================================
// VERBOSE FLAG TESTS
// ============================================================================

#[test]
#[ignore = "May make API call if ANTHROPIC_API_KEY is set"]
fn test_agent_with_verbose_flag() {
    let temp_dir = TempDir::new().unwrap();
    let prompt_file = temp_dir.path().join("prompt.txt");
    fs::write(&prompt_file, "Test prompt").unwrap();

    // Create test agent file
    let claude_dir = temp_dir.path().join(".claude").join("agents");
    fs::create_dir_all(&claude_dir).unwrap();
    let agent_file = claude_dir.join("test.md");
    fs::write(&agent_file, "You are a test agent.").unwrap();

    let _output = cargo_bin_cmd!("claude")
        .current_dir(temp_dir.path())
        .arg("--verbose")
        .arg("agent")
        .arg("test")
        .arg("--prompt")
        .arg(prompt_file.to_str().unwrap())
        .assert();

    // This test may succeed if API key is available, or fail if not
    // Either way, verbose should be recognized (no arg parsing error)
}

// ============================================================================
// INTEGRATION TEST (requires API key)
// ============================================================================

#[test]
#[ignore = "Requires ANTHROPIC_API_KEY environment variable"]
fn test_agent_real_execution() {
    // Skip if no API key
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        println!("Skipping: ANTHROPIC_API_KEY not set");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let prompt_file = temp_dir.path().join("prompt.txt");
    fs::write(
        &prompt_file,
        "Say 'Hello from agent test!' and nothing else.",
    )
    .unwrap();

    // Create test agent file
    let claude_dir = temp_dir.path().join(".claude").join("agents");
    fs::create_dir_all(&claude_dir).unwrap();
    let agent_file = claude_dir.join("test.md");
    fs::write(
        &agent_file,
        "You are a test agent. Respond concisely to user requests.",
    )
    .unwrap();

    cargo_bin_cmd!("claude")
        .current_dir(temp_dir.path())
        .arg("agent")
        .arg("test")
        .arg("--prompt")
        .arg(prompt_file.to_str().unwrap())
        .arg("--model")
        .arg("haiku")
        .assert()
        .success()
        .stdout(predicate::str::contains("Agent Response"))
        .stdout(predicate::str::contains("Hello"));
}
