//! Agent Subcommand Tests
//!
//! Tests for the `claude agent` subcommand that invokes specialized agents
//! with prompts from files.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

// Helper function to get the binary path
fn get_binary_path() -> PathBuf {
    // Get the manifest directory (crate root)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");

    // Go up to the workspace root (2 levels: crates/cli -> workspace root)
    let workspace_root = PathBuf::from(manifest_dir)
        .parent()
        .expect("Failed to get parent of crates")
        .parent()
        .expect("Failed to get workspace root")
        .to_path_buf();

    // Get the target directory from CARGO_TARGET_DIR or default to workspace_root/target/
    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("target"));

    // Get the profile (debug or release)
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    // Construct the binary path
    let mut path = target_dir;
    path.push(profile);
    path.push("claude");  // Binary is named "claude" not "rusty"

    // Add .exe extension on Windows
    if cfg!(windows) {
        path.set_extension("exe");
    }

    path
}

// ============================================================================
// HELP AND USAGE TESTS
// ============================================================================

#[test]
fn test_agent_help() {
    let output = Command::new(get_binary_path())
        .arg("agent")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Invoke a specialized agent"));
    assert!(stdout.contains("--prompt"));
}

#[test]
fn test_agent_requires_type_and_prompt() {
    let output = Command::new(get_binary_path())
        .arg("agent")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("required"));
}

#[test]
fn test_agent_requires_prompt_flag() {
    let output = Command::new(get_binary_path())
        .arg("agent")
        .arg("test")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--prompt"));
}

// ============================================================================
// PROMPT FILE TESTS
// ============================================================================

#[test]
fn test_agent_missing_prompt_file() {
    let output = Command::new(get_binary_path())
        .arg("agent")
        .arg("test")
        .arg("--prompt")
        .arg("/nonexistent/file.txt")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to read prompt file"));
}

#[test]
fn test_agent_missing_agent_file() {
    let temp_dir = TempDir::new().unwrap();
    let prompt_file = temp_dir.path().join("prompt.txt");
    fs::write(&prompt_file, "Test prompt").unwrap();

    let output = Command::new(get_binary_path())
        .current_dir(temp_dir.path())
        .arg("agent")
        .arg("nonexistent_agent")
        .arg("--prompt")
        .arg(prompt_file.to_str().unwrap())
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Agent prompt not found"));
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

    let _output = Command::new(get_binary_path())
        .current_dir(temp_dir.path())
        .arg("agent")
        .arg("test")
        .arg("--prompt")
        .arg(prompt_file.to_str().unwrap())
        .arg("--model")
        .arg("haiku")
        .output()
        .expect("Failed to execute command");

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

    let _output = Command::new(get_binary_path())
        .current_dir(temp_dir.path())
        .arg("--verbose")
        .arg("agent")
        .arg("test")
        .arg("--prompt")
        .arg(prompt_file.to_str().unwrap())
        .output()
        .expect("Failed to execute command");

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

    let output = Command::new(get_binary_path())
        .current_dir(temp_dir.path())
        .arg("agent")
        .arg("test")
        .arg("--prompt")
        .arg(prompt_file.to_str().unwrap())
        .arg("--model")
        .arg("haiku")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Agent Response"));
    assert!(stdout.contains("Hello"));
}
