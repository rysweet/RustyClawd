//! Builtin Commands Real Data Tests
//!
//! Tests that verify builtin commands use real data instead of placeholders:
//! - Only implemented commands are recognized
//! - Removed stub commands return None
//! - No placeholder strings in any output
//! - All commands work with real logic

#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(clippy::manual_range_contains)]

use rustyclawd::commands::builtins::BuiltinCommands;
use rustyclawd::commands::parser::{Command, CommandParser};

#[test]
fn test_only_implemented_commands_recognized() {
    let implemented = vec![
        "help",
        "exit",
        "quit",
        "clear",
        "version",
        "permissions",
        "login",
        "bug",
        "add-dir",
        "fast",
    ];

    for cmd in implemented {
        assert!(
            BuiltinCommands::is_builtin(cmd),
            "Command '{}' should be recognized as builtin",
            cmd
        );
    }
}

#[test]
fn test_nonexistent_commands_not_recognized() {
    let nonexistent = vec!["nonexistent", "fake-command", "not-a-command", "made-up"];

    for cmd in nonexistent {
        assert!(
            !BuiltinCommands::is_builtin(cmd),
            "Command '{}' should NOT be recognized",
            cmd
        );
    }
}

#[test]
fn test_removed_stubs_not_recognized() {
    // These were removed because they only returned static/placeholder text
    let removed = vec![
        "bashes",
        "context",
        "cost",
        "todos",
        "usage",
        "logout",
        "compact",
        "rewind",
        "config",
        "model", // handled in interactive.rs session layer, not as a builtin
        "status",
        "review",
        "sandbox",
        "doctor",
        "export",
        "memory",
        "mcp",
        "agents",
        "hooks",
        "init",
        "debug",
        "trace",
        "log",
        "checkpoint",
        "restore",
        "tools",
        "plugins",
        "save",
        "load",
        "reset",
        "undo",
        "redo",
        "history",
        "stats",
        "output-style",
        "privacy-settings",
        "statusline",
        "terminal-setup",
        "vim",
        "pr_comments",
    ];

    for cmd in removed {
        assert!(
            !BuiltinCommands::is_builtin(cmd),
            "Removed stub '{}' should NOT be recognized as builtin",
            cmd
        );
    }
}

#[test]
fn test_help_command_provides_useful_information() {
    let cmd = Command::new("help".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    assert!(
        output.contains("/help") || output.contains("help"),
        "Help should mention help command: {}",
        output
    );
    assert!(
        output.contains("/exit") || output.contains("exit"),
        "Help should mention exit command: {}",
        output
    );

    assert!(output.len() > 50, "Help output should be substantial");
    assert!(
        !output.contains("TODO"),
        "Help should not contain TODOs: {}",
        output
    );
}

#[test]
fn test_version_command_shows_real_version() {
    let cmd = Command::new("version".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    assert!(
        output.contains("RustyClawd") || output.contains("version"),
        "Version should show program name: {}",
        output
    );
    assert!(
        !output.contains("0.0.0"),
        "Version should not be placeholder 0.0.0: {}",
        output
    );
    assert!(
        !output.contains("x.y.z"),
        "Version should not be placeholder x.y.z: {}",
        output
    );
}

#[test]
fn test_clear_command_returns_success() {
    let cmd = Command::new("clear".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    assert!(
        output.contains("clear") || output.contains("Clear"),
        "Clear should indicate action: {}",
        output
    );
    assert!(!output.is_empty(), "Clear should return a message");
}

#[test]
fn test_exit_command_returns_goodbye() {
    let cmd = Command::new("exit".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    assert!(
        output.to_lowercase().contains("exit")
            || output.to_lowercase().contains("goodbye")
            || output.to_lowercase().contains("bye"),
        "Exit should have farewell: {}",
        output
    );
}

#[test]
fn test_quit_command_same_as_exit() {
    let exit_cmd = Command::new("exit".to_string(), None);
    let quit_cmd = Command::new("quit".to_string(), None);

    let exit_result = BuiltinCommands::execute(&exit_cmd);
    let quit_result = BuiltinCommands::execute(&quit_cmd);

    assert!(exit_result.is_some());
    assert!(quit_result.is_some());

    let exit_output = exit_result.unwrap();
    let quit_output = quit_result.unwrap();

    assert_eq!(
        exit_output, quit_output,
        "Exit and quit should produce same output"
    );
}

#[test]
fn test_command_parser_handles_all_builtins() {
    let parser = CommandParser::new();

    let builtin_names = vec![
        "help", "exit", "quit", "clear", "version", "login", "bug", "fast",
    ];

    for name in builtin_names {
        let input = format!("/{}", name);
        let result = parser.parse(&input);

        assert!(result.is_ok(), "Parser should handle /{}", name);
        let cmd = result.unwrap();
        assert_eq!(cmd.name, name);
        assert!(BuiltinCommands::is_builtin(&cmd.name));
    }
}

#[test]
fn test_no_command_returns_hardcoded_sample_data() {
    let commands = vec!["help", "version"];

    for cmd_name in commands {
        let cmd = Command::new(cmd_name.to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();

        assert!(
            !output.contains("Lorem ipsum"),
            "Command {} should not have Lorem ipsum: {}",
            cmd_name,
            output
        );
        assert!(
            !output.contains("Sample data"),
            "Command {} should not have 'Sample data': {}",
            cmd_name,
            output
        );
        assert!(
            !output.contains("Example output"),
            "Command {} should not have 'Example output': {}",
            cmd_name,
            output
        );
    }
}

#[test]
fn test_builtin_command_with_args_parsing() {
    let parser = CommandParser::new();

    let cmd = parser.parse("/help search").expect("Should parse");
    assert_eq!(cmd.name, "help");
    assert_eq!(cmd.args_str, Some("search".to_string()));

    let result = BuiltinCommands::execute(&cmd);
    assert!(result.is_some());
}

#[test]
fn test_help_with_search_term() {
    let cmd = Command::new("help".to_string(), Some("commands".to_string()));
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    assert!(
        output.contains("commands"),
        "Help should include search term: {}",
        output
    );
}

#[test]
fn test_all_builtins_return_non_empty_output() {
    let builtins = vec![
        "help",
        "exit",
        "quit",
        "clear",
        "version",
        "permissions",
        "login",
        "bug",
        "fast",
    ];

    for builtin in builtins {
        let cmd = Command::new(builtin.to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(
            result.is_some(),
            "Builtin '{}' should return output",
            builtin
        );
        let output = result.unwrap();
        assert!(
            !output.is_empty(),
            "Builtin '{}' should return non-empty output",
            builtin
        );
        assert!(
            output.len() > 5,
            "Builtin '{}' should return substantial output, got: {}",
            builtin,
            output
        );
    }
}

#[test]
fn test_unknown_command_returns_none() {
    let cmd = Command::new("totally-unknown-command".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(
        result.is_none(),
        "Unknown commands should return None, not fake output"
    );
}

#[test]
fn test_builtin_commands_are_consistent() {
    let test_commands = vec![
        ("help", true),
        ("exit", true),
        ("notacommand", false),
        ("fake", false),
    ];

    for (cmd, should_be_builtin) in test_commands {
        let is_builtin = BuiltinCommands::is_builtin(cmd);
        let command = Command::new(cmd.to_string(), None);
        let result = BuiltinCommands::execute(&command);

        if should_be_builtin {
            assert!(is_builtin, "Command '{}' should be builtin", cmd);
            assert!(result.is_some(), "Command '{}' should return output", cmd);
        } else {
            assert!(!is_builtin, "Command '{}' should not be builtin", cmd);
            assert!(result.is_none(), "Command '{}' should return None", cmd);
        }
    }
}

#[test]
fn test_login_command_checks_api_key() {
    let cmd = Command::new("login".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();
    assert!(output.contains("ANTHROPIC_API_KEY"));
}

#[test]
fn test_bug_command_shows_github_url() {
    let cmd = Command::new("bug".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();
    assert!(output.contains("rysweet/RustyClawd"));
    assert!(output.contains("github.com"));
}

#[test]
fn test_add_dir_validates_directory() {
    let cmd = Command::new(
        "add-dir".to_string(),
        Some("/nonexistent/path/xyz".to_string()),
    );
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();
    assert!(output.contains("Error"));
    assert!(output.contains("does not exist"));
}
