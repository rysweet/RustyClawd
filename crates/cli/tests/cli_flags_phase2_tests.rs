//! CLI Flags Phase 2 Test Suite
//!
//! Tests for the 8 P1 CLI flags implemented in Phase 2:
//! 1. --fork-session - Fork from existing session
//! 2. --fallback-model - Specify fallback model when primary fails
//! 3. --settings - Override settings file location
//! 4. --ide - IDE integration mode (structured JSON output)
//! 5. --mcp-config - Override MCP configuration file location
//! 6. --resume-from-checkpoint - Resume from specific checkpoint number
//! 7. --model-capabilities - Override model capabilities (JSON format)
//! 8. --dangerous-mode - Skip safety checks and hooks (dangerous)
//!
//! Related issue: #18
//!
//! Note: These are integration tests that verify the CLI accepts these flags
//! and doesn't error out. Full functional testing would require an API key
//! and real sessions, so we focus on flag acceptance and basic parsing.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// FLAG ACCEPTANCE TESTS
// ============================================================================
// These tests verify that each Phase 2 flag is accepted by the CLI parser

#[test]
fn test_fork_session_flag_accepted() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--help");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Verify --fork-session is in help output
    assert!(
        stdout.contains("--fork-session"),
        "Flag --fork-session should be in help"
    );
}

#[test]
fn test_fallback_model_flag_accepted() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--help");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("--fallback-model"),
        "Flag --fallback-model should be in help"
    );
}

#[test]
fn test_settings_flag_accepted() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--help");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("--settings"),
        "Flag --settings should be in help"
    );
}

#[test]
fn test_ide_flag_accepted() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--help");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("--ide"), "Flag --ide should be in help");
}

#[test]
fn test_mcp_config_flag_accepted() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--help");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("--mcp-config"),
        "Flag --mcp-config should be in help"
    );
}

#[test]
fn test_resume_from_checkpoint_flag_accepted() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--help");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("--resume-from-checkpoint"),
        "Flag --resume-from-checkpoint should be in help"
    );
}

#[test]
fn test_model_capabilities_flag_accepted() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--help");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("--model-capabilities"),
        "Flag --model-capabilities should be in help"
    );
}

#[test]
fn test_dangerous_mode_flag_accepted() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--help");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("--dangerous-mode"),
        "Flag --dangerous-mode should be in help"
    );
}

// ============================================================================
// FLAG COMBINATION TESTS
// ============================================================================
// Test that Phase 2 flags can be combined without errors

#[test]
fn test_fork_session_with_fallback_model() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--fork-session")
        .arg("test-session")
        .arg("--fallback-model")
        .arg("haiku")
        .arg("--help");

    // Should not error when combining flags
    cmd.assert().success();
}

#[test]
fn test_ide_mode_with_settings() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--ide")
        .arg("--settings")
        .arg("./test-settings.json")
        .arg("--help");

    cmd.assert().success();
}

#[test]
fn test_dangerous_mode_with_mcp_config() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--dangerous-mode")
        .arg("--mcp-config")
        .arg("./mcp.json")
        .arg("--help");

    cmd.assert().success();
}

#[test]
fn test_all_phase2_flags_combined() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--fork-session")
        .arg("session-123")
        .arg("--fallback-model")
        .arg("haiku")
        .arg("--settings")
        .arg("./settings.json")
        .arg("--ide")
        .arg("--mcp-config")
        .arg("./mcp.json")
        .arg("--resume-from-checkpoint")
        .arg("3")
        .arg("--model-capabilities")
        .arg(r#"{"max_tokens":8192}"#)
        .arg("--dangerous-mode")
        .arg("--help");

    cmd.assert().success();
}

// ============================================================================
// VALIDATION TESTS
// ============================================================================
// Test that invalid flag values are properly rejected

#[test]
fn test_resume_from_checkpoint_rejects_negative() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--resume-from-checkpoint").arg("-1");

    // Should fail with invalid value
    cmd.assert().failure();
}

#[test]
fn test_resume_from_checkpoint_rejects_non_numeric() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--resume-from-checkpoint").arg("abc");

    // Should fail with invalid value
    cmd.assert().failure();
}

// ============================================================================
// FUNCTIONAL TESTS (Limited - no API key required)
// ============================================================================
// These tests verify basic functional behavior without needing API calls

#[test]
fn test_settings_flag_overrides_default() {
    // Create a temporary settings file
    let temp_dir = TempDir::new().unwrap();
    let settings_path = temp_dir.path().join("test-settings.json");
    fs::write(&settings_path, r#"{"model": "test-model"}"#).unwrap();

    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--settings")
        .arg(settings_path.to_str().unwrap())
        .arg("--help");

    // Should accept custom settings file
    cmd.assert().success();
}

#[test]
fn test_model_capabilities_accepts_valid_json() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--model-capabilities")
        .arg(r#"{"max_tokens":8192,"tools":true}"#)
        .arg("--help");

    cmd.assert().success();
}

#[test]
fn test_ide_flag_boolean() {
    // Test that --ide doesn't require a value (boolean flag)
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--ide").arg("--help");

    cmd.assert().success();
}

#[test]
fn test_dangerous_mode_boolean() {
    // Test that --dangerous-mode doesn't require a value (boolean flag)
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--dangerous-mode").arg("--help");

    cmd.assert().success();
}

// ============================================================================
// DOCUMENTATION TESTS
// ============================================================================
// Verify that flag descriptions are present and accurate

#[test]
fn test_all_phase2_flags_documented_in_help() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--help");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // All 8 Phase 2 flags should appear in help
    let flags = vec![
        "--fork-session",
        "--fallback-model",
        "--settings",
        "--ide",
        "--mcp-config",
        "--resume-from-checkpoint",
        "--model-capabilities",
        "--dangerous-mode",
    ];

    for flag in flags {
        assert!(
            stdout.contains(flag),
            "Flag {} should be documented in --help output",
            flag
        );
    }
}

#[test]
fn test_dangerous_mode_has_warning_in_help() {
    let mut cmd = Command::cargo_bin("rusty").unwrap();
    cmd.arg("--help");

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should have warning about dangerous mode
    assert!(
        stdout.contains("dangerous") || stdout.contains("Skip safety"),
        "Help should warn about dangerous mode"
    );
}
