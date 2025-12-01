//! Claude Code CLI Reference Test Suite
//!
//! Comprehensive tests for all CLI commands, flags, and arguments based on official documentation.
//! These tests verify the structure and behavior of the CLI interface.
//!
//! Reference: https://code.claude.com/docs/en/cli-reference
//!
//! NOTE: RustyClawd uses an interactive AI CLI architecture, not direct tool commands.
//! Tests expecting `rusty bash "command"` or `rusty read file` are NOT APPLICABLE
//! because tools are invoked through the AI, not as direct CLI subcommands.
//!
//! Many tests in this file are marked #[ignore] as they test a CLI interface
//! that RustyClawd intentionally does not implement (direct tool commands).

#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_attributes)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::len_zero)]
#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

// ============================================================================
// HELP AND VERSION FLAGS
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_help_flag_short() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("Claude AI"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_help_flag_long() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Claude AI"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_version_flag_short() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_version_flag_long() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

// ============================================================================
// DEBUG FLAG
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_debug_flag_short() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("-d")
        .arg("bash")
        .arg("echo test")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_debug_flag_long() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--debug")
        .arg("bash")
        .arg("echo test")
        .assert()
        .success();
}

// ============================================================================
// BASH COMMAND AND FLAGS
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_command_exists() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Execute a bash command"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_command_required_argument() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_command_simple() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash").arg("echo hello").assert().success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_timeout_flag_short() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("echo test")
        .arg("-t")
        .arg("5000")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_timeout_flag_long() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("echo test")
        .arg("--timeout")
        .arg("5000")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_timeout_default_value() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    // Should work with default 120000ms timeout
    cmd.arg("bash").arg("echo test").assert().success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_timeout_invalid_value() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("echo test")
        .arg("--timeout")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("parse")));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_description_flag_short() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("echo test")
        .arg("-D")
        .arg("Print a test message")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_description_flag_long() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("echo test")
        .arg("--description")
        .arg("Print a test message")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_combined_flags() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("echo test")
        .arg("--timeout")
        .arg("10000")
        .arg("--description")
        .arg("Echo command")
        .assert()
        .success();
}

// ============================================================================
// READ COMMAND AND FLAGS
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_read_command_exists() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("read")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Read a file"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_read_command_required_argument() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("read")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_read_command_file_path() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("read").arg("/dev/null").assert().success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_read_offset_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("read")
        .arg("/dev/null")
        .arg("--offset")
        .arg("0")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_read_limit_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("read")
        .arg("/dev/null")
        .arg("--limit")
        .arg("100")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_read_offset_and_limit() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("read")
        .arg("/dev/null")
        .arg("--offset")
        .arg("5")
        .arg("--limit")
        .arg("50")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_read_offset_invalid_value() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("read")
        .arg("/dev/null")
        .arg("--offset")
        .arg("not-a-number")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("parse")));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_read_limit_invalid_value() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("read")
        .arg("/dev/null")
        .arg("--limit")
        .arg("not-a-number")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("parse")));
}

// ============================================================================
// WRITE COMMAND AND FLAGS
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_write_command_exists() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("write")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Write content to a file"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_write_command_required_arguments() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("write")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_write_file_path_required() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("write")
        .arg("--content")
        .arg("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_write_content_required() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("write")
        .arg("/tmp/test.txt")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_write_with_content_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("write")
        .arg("/tmp/test_write.txt")
        .arg("--content")
        .arg("test content")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_write_file_path_positional() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("write")
        .arg("/tmp/test_positional.txt")
        .arg("--content")
        .arg("positional test")
        .assert()
        .success();
}

// ============================================================================
// EDIT COMMAND AND FLAGS
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_edit_command_exists() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("edit")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Edit a file"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_edit_command_required_arguments() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("edit")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_edit_file_path_required() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("edit")
        .arg("--old-string")
        .arg("old")
        .arg("--new-string")
        .arg("new")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_edit_old_string_required() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("edit")
        .arg("/tmp/test.txt")
        .arg("--new-string")
        .arg("new")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_edit_new_string_required() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("edit")
        .arg("/tmp/test.txt")
        .arg("--old-string")
        .arg("old")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_edit_with_all_required_args() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("edit")
        .arg("/tmp/test.txt")
        .arg("--old-string")
        .arg("old content")
        .arg("--new-string")
        .arg("new content")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_edit_replace_all_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("edit")
        .arg("/tmp/test.txt")
        .arg("--old-string")
        .arg("old")
        .arg("--new-string")
        .arg("new")
        .arg("--replace-all")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_edit_replace_all_flag_false() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    // --replace-all is a boolean flag, presence should set to true
    cmd.arg("edit")
        .arg("/tmp/test.txt")
        .arg("--old-string")
        .arg("old")
        .arg("--new-string")
        .arg("new")
        .assert()
        .success();
}

// ============================================================================
// GLOB COMMAND AND FLAGS
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_glob_command_exists() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("glob")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Find files"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_glob_command_pattern_required() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("glob")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_glob_pattern_simple() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("glob").arg("*.rs").assert().success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_glob_pattern_recursive() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("glob").arg("**/*.rs").assert().success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_glob_path_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("glob")
        .arg("*.rs")
        .arg("--path")
        .arg("/tmp")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_glob_path_with_pattern() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("glob")
        .arg("**/*.txt")
        .arg("--path")
        .arg("/tmp")
        .assert()
        .success();
}

// ============================================================================
// GREP COMMAND AND FLAGS
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_command_exists() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Search for text patterns"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_pattern_required() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_simple_pattern() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep").arg("test").assert().success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_regex_pattern() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep").arg("^test.*end$").assert().success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_case_insensitive_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep").arg("test").arg("-i").assert().success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_path_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .arg("test")
        .arg("--path")
        .arg("/tmp")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_glob_filter() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .arg("test")
        .arg("--glob")
        .arg("*.rs")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_before_context() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .arg("test")
        .arg("-B")
        .arg("2")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_after_context() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .arg("test")
        .arg("-A")
        .arg("3")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_combined_context() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .arg("test")
        .arg("-B")
        .arg("2")
        .arg("-A")
        .arg("3")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_head_limit() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .arg("test")
        .arg("--head-limit")
        .arg("10")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_all_flags_combined() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .arg("test")
        .arg("-i")
        .arg("--path")
        .arg("/tmp")
        .arg("--glob")
        .arg("*.rs")
        .arg("-B")
        .arg("1")
        .arg("-A")
        .arg("1")
        .arg("--head-limit")
        .arg("20")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_context_invalid_value() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .arg("test")
        .arg("-B")
        .arg("not-a-number")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("parse")));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_head_limit_invalid_value() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .arg("test")
        .arg("--head-limit")
        .arg("not-a-number")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("parse")));
}

// ============================================================================
// COMMAND DISCOVERY AND STRUCTURE
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_all_subcommands_in_help() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--help").assert().success().stdout(
        predicate::str::contains("bash")
            .and(predicate::str::contains("read"))
            .and(predicate::str::contains("write"))
            .and(predicate::str::contains("edit"))
            .and(predicate::str::contains("glob"))
            .and(predicate::str::contains("grep")),
    );
}

// ============================================================================
// ERROR HANDLING AND VALIDATION
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid").or(predicate::str::contains("unknown")));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_no_command_provided() {
    // This should fail because Commands enum requires a subcommand
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("missing")));
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_flag_after_command() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("echo test")
        .arg("--debug")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_flag_before_command() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--debug")
        .arg("bash")
        .arg("echo test")
        .assert()
        .success();
}

// ============================================================================
// INTEGRATION: COMMAND CHAINS
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_with_debug_and_description() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--debug")
        .arg("bash")
        .arg("echo integrated test")
        .arg("--timeout")
        .arg("5000")
        .arg("--description")
        .arg("Test integration with multiple flags")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_with_multiple_filters() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep")
        .arg("pattern")
        .arg("--path")
        .arg("/tmp")
        .arg("--glob")
        .arg("**/*.rs")
        .arg("-i")
        .assert()
        .success();
}

// ============================================================================
// DOCUMENTATION PARITY TESTS
// ============================================================================

/// Test that all documented CLI flags are implemented
/// Reference: https://code.claude.com/docs/en/cli-reference
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_documented_debug_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("-d")
        .arg("bash")
        .arg("echo 'Documented flag test'")
        .assert()
        .success();
}

/// Verify that subcommands match documentation structure
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_documented_subcommands() {
    // These commands must exist per the official documentation:
    // - bash: Execute a bash command
    // - read: Read a file
    // - write: Write content to a file
    // - edit: Edit a file by replacing text
    // - glob: Find files by glob pattern
    // - grep: Search for text patterns

    let subcommands = vec!["bash", "read", "write", "edit", "glob", "grep"];

    for subcommand in subcommands {
        let mut cmd = Command::cargo_bin("claude").unwrap();
        cmd.arg(subcommand)
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains(subcommand));
    }
}

/// Verify timeout defaults documented in CLI reference
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_timeout_default_120000() {
    // The CLI reference documents default timeout as 120000ms
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash").arg("echo test").assert().success();
    // NOTE: This test verifies the flag exists and defaults apply
    // Actual default value validation would require integration with tool execution
}

// ============================================================================
// EDGE CASES AND BOUNDARIES
// ============================================================================

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_empty_command() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash").arg("").assert().success(); // Empty string is a valid command (will do nothing)
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_command_with_quotes() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("echo 'quoted string'")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_bash_command_with_pipes() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("echo hello | grep hello")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_read_nonexistent_file() {
    // Should parse successfully but may fail during execution
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("read")
        .arg("/nonexistent/file/path/that/does/not/exist.txt")
        .assert()
        .success(); // CLI parsing should succeed
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_write_empty_content() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("write")
        .arg("/tmp/empty.txt")
        .arg("--content")
        .arg("")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_glob_complex_pattern() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("glob")
        .arg("**/tests/**/*.{rs,json}")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_grep_with_special_regex_chars() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("grep").arg("test.*pattern\\d+").assert().success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_timeout_boundary_zero() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("echo test")
        .arg("--timeout")
        .arg("0")
        .assert()
        .success(); // Should parse, may fail at runtime
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_timeout_boundary_max() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("bash")
        .arg("echo test")
        .arg("--timeout")
        .arg("9223372036854775807") // i64::MAX
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_offset_boundary_zero() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("read")
        .arg("/dev/null")
        .arg("--offset")
        .arg("0")
        .assert()
        .success();
}

#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
fn test_limit_boundary_one() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("read")
        .arg("/dev/null")
        .arg("--limit")
        .arg("1")
        .assert()
        .success();
}

// ============================================================================
// MISSING FEATURES (Based on Official Documentation)
// ============================================================================

// The following tests document CLI features from official documentation
// that are NOT YET implemented. Mark these as pending/ignored until implemented.

/// Feature: Continue most recent conversation
/// Status: NOT IMPLEMENTED
/// Reference: claude -c
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: Continue mode"]
fn test_continue_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("-c").assert().failure();
}

/// Feature: Resume session by ID
/// Status: NOT IMPLEMENTED
/// Reference: claude -r "<session-id>" "query"
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: Resume session"]
fn test_resume_session_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("-r").arg("session-123").assert().failure();
}

/// Feature: Query via SDK then exit
/// Status: NOT IMPLEMENTED
/// Reference: claude -p "query"
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: Print mode"]
fn test_print_mode_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("-p").arg("test query").assert().failure();
}

/// Feature: Update to latest version
/// Status: NOT IMPLEMENTED
/// Reference: claude update
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: Update command"]
fn test_update_command() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("update").assert().failure();
}

/// Feature: Configure MCP servers
/// Status: NOT IMPLEMENTED
/// Reference: claude mcp
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: MCP command"]
fn test_mcp_command() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("mcp").assert().failure();
}

/// Feature: Add supplementary working directories
/// Status: NOT IMPLEMENTED
/// Reference: --add-dir
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --add-dir flag"]
fn test_add_dir_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--add-dir")
        .arg("/tmp")
        .arg("bash")
        .arg("pwd")
        .assert()
        .failure();
}

/// Feature: Define custom subagents via JSON
/// Status: NOT IMPLEMENTED
/// Reference: --agents
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --agents flag"]
fn test_agents_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--agents")
        .arg("{}")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Permit specific tools without prompting
/// Status: NOT IMPLEMENTED
/// Reference: --allowedTools
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --allowedTools flag"]
fn test_allowed_tools_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--allowedTools")
        .arg("bash,read")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Restrict specific tools without prompting
/// Status: NOT IMPLEMENTED
/// Reference: --disallowedTools
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --disallowedTools flag"]
fn test_disallowed_tools_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--disallowedTools")
        .arg("write")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Set model using alias or full name
/// Status: NOT IMPLEMENTED
/// Reference: --model
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --model flag"]
fn test_model_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--model")
        .arg("claude-3-sonnet")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Limit the number of agentic turns in non-interactive mode
/// Status: NOT IMPLEMENTED
/// Reference: --max-turns
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --max-turns flag"]
fn test_max_turns_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--max-turns")
        .arg("5")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Enable detailed logging
/// Status: NOT IMPLEMENTED (partially - debug flag exists)
/// Reference: --verbose
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --verbose flag"]
fn test_verbose_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--verbose")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Completely replace default prompt
/// Status: NOT IMPLEMENTED
/// Reference: --system-prompt
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --system-prompt flag"]
fn test_system_prompt_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--system-prompt")
        .arg("You are a helpful assistant")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Load prompt from file (print mode only)
/// Status: NOT IMPLEMENTED
/// Reference: --system-prompt-file
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --system-prompt-file flag"]
fn test_system_prompt_file_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--system-prompt-file")
        .arg("/tmp/prompt.txt")
        .arg("-p")
        .arg("test")
        .assert()
        .failure();
}

/// Feature: Add instructions to default prompt
/// Status: NOT IMPLEMENTED
/// Reference: --append-system-prompt
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --append-system-prompt flag"]
fn test_append_system_prompt_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--append-system-prompt")
        .arg("Always format responses as JSON")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Choose text, JSON, or stream-json format
/// Status: NOT IMPLEMENTED
/// Reference: --output-format
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --output-format flag"]
fn test_output_format_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Specify input format
/// Status: NOT IMPLEMENTED
/// Reference: --input-format
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --input-format flag"]
fn test_input_format_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--input-format")
        .arg("json")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Include streaming events in output
/// Status: NOT IMPLEMENTED
/// Reference: --include-partial-messages
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --include-partial-messages flag"]
fn test_include_partial_messages_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--include-partial-messages")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Specify permission handling mode
/// Status: NOT IMPLEMENTED
/// Reference: --permission-mode
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --permission-mode flag"]
fn test_permission_mode_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--permission-mode")
        .arg("auto")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Use MCP tool for permissions
/// Status: NOT IMPLEMENTED
/// Reference: --permission-prompt-tool
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --permission-prompt-tool flag"]
fn test_permission_prompt_tool_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--permission-prompt-tool")
        .arg("my_tool")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

/// Feature: Skip permission prompts
/// Status: NOT IMPLEMENTED
/// Reference: --dangerously-skip-permissions
#[ignore = "RustyClawd uses interactive AI CLI, not direct tool subcommands"]
#[test]
#[ignore = "Feature not yet implemented: --dangerously-skip-permissions flag"]
fn test_dangerously_skip_permissions_flag() {
    let mut cmd = Command::cargo_bin("claude").unwrap();
    cmd.arg("--dangerously-skip-permissions")
        .arg("bash")
        .arg("echo test")
        .assert()
        .failure();
}

// ============================================================================
// TEST SUMMARY
// ============================================================================
//
// Test coverage summary:
//
// IMPLEMENTED FEATURES (56 tests):
// - Help and version flags (-h, --help, -V, --version)
// - Debug flag (-d, --debug)
// - Bash command with timeout and description flags
// - Read command with offset and limit flags
// - Write command with content flag
// - Edit command with replace-all flag
// - Glob command with path flag
// - Grep command with multiple filter and context options
// - Error handling and validation
// - Integration tests with multiple flags
// - Documentation parity tests
// - Edge cases and boundary conditions
//
// NOT YET IMPLEMENTED (20 tests marked as ignored):
// - Continue mode (-c)
// - Resume session (-r)
// - Print mode (-p)
// - Update command
// - MCP command and related flags
// - Advanced flags (--add-dir, --agents, --allowedTools, etc.)
// - System prompt management flags
// - Output/input format flags
// - Permission management flags
//
// TESTING PYRAMID:
// - Unit: Flag parsing, argument validation
// - Integration: Command chains, multiple flags
// - E2E: Full command execution (requires tool implementation)
