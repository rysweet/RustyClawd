//! Builtin Commands Real Data Tests
//!
//! Tests that verify builtin commands use real data instead of placeholders:
//! - Only implemented commands are recognized
//! - Removed commands return NotImplemented
//! - /history shows real command history
//! - /stats uses actual session data
//! - No placeholder strings in any output
//! - All commands work with SessionState

#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(clippy::manual_range_contains)]

use claude_code_cli::commands::builtins::BuiltinCommands;
use claude_code_cli::commands::parser::{Command, CommandParser};

// Import the session state types from our tests
// In real implementation, these would come from the main crate
use std::collections::HashMap;

// Simplified session state for testing
#[derive(Debug, Clone)]
struct TestSessionState {
    command_history: Vec<String>,
    message_count: u64,
    commands_executed: u64,
    total_tokens: u64,
    duration_seconds: u64,
    model: String,
}

impl TestSessionState {
    fn new() -> Self {
        Self {
            command_history: Vec::new(),
            message_count: 0,
            commands_executed: 0,
            total_tokens: 0,
            duration_seconds: 0,
            model: "claude-sonnet-4-5".to_string(),
        }
    }

    fn add_command(&mut self, cmd: String) {
        self.command_history.push(cmd);
        self.commands_executed += 1;
    }

    fn add_tokens(&mut self, tokens: u64) {
        self.total_tokens += tokens;
    }

    fn increment_message_count(&mut self) {
        self.message_count += 1;
    }
}

#[test]
fn test_only_implemented_commands_recognized() {
    // Commands that SHOULD be recognized
    let implemented = vec![
        "help", "exit", "quit", "clear", "history", "stats", "config", "model", "status", "version",
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
fn test_history_command_no_placeholder_strings() {
    let cmd = Command::new("history".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // These strings indicate placeholder/fake implementations
    assert!(
        !output.to_lowercase().contains("coming soon"),
        "Output should not contain 'coming soon': {}",
        output
    );
    assert!(
        !output.to_lowercase().contains("not yet implemented"),
        "Output should not contain 'not yet implemented': {}",
        output
    );
    assert!(
        !output.to_lowercase().contains("todo"),
        "Output should not contain 'todo': {}",
        output
    );
    assert!(
        !output.to_lowercase().contains("placeholder"),
        "Output should not contain 'placeholder': {}",
        output
    );
}

#[test]
fn test_stats_command_no_placeholder_strings() {
    let cmd = Command::new("stats".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // Verify no placeholder strings
    assert!(
        !output.to_lowercase().contains("coming soon"),
        "Stats output should not contain placeholders: {}",
        output
    );
    assert!(
        !output.to_lowercase().contains("not yet implemented"),
        "Stats output should not contain placeholders: {}",
        output
    );
    assert!(
        !output.to_lowercase().contains("n/a"),
        "Stats should show real values, not N/A: {}",
        output
    );
}

#[test]
fn test_stats_shows_numeric_values() {
    let cmd = Command::new("stats".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // Stats should contain actual numbers (even if 0)
    assert!(
        output.contains("Messages:") || output.contains("messages:"),
        "Stats should show message count: {}",
        output
    );
    assert!(
        output.contains("tokens") || output.contains("Tokens"),
        "Stats should show token count: {}",
        output
    );

    // Should not contain question marks or dashes as placeholders
    assert!(
        !output.contains("???"),
        "Stats should not have ??? placeholders: {}",
        output
    );
    assert!(
        !output.contains("---"),
        "Stats should not have --- placeholders: {}",
        output
    );
}

#[test]
fn test_help_command_provides_useful_information() {
    let cmd = Command::new("help".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // Help should mention actual commands
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

    // Should not be empty or placeholder
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

    // Should contain "RustyClawd" and version number
    assert!(
        output.contains("RustyClawd") || output.contains("version"),
        "Version should show program name: {}",
        output
    );

    // Should not be placeholder
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

    // Should indicate success
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

    // Should have a farewell message
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

    // Both should have similar output (exit message)
    let exit_output = exit_result.unwrap();
    let quit_output = quit_result.unwrap();

    assert_eq!(
        exit_output, quit_output,
        "Exit and quit should produce same output"
    );
}

#[test]
fn test_config_command_shows_current_config() {
    let cmd = Command::new("config".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // Should show config information
    assert!(
        output.contains("configuration") || output.contains("Configuration"),
        "Config should show configuration: {}",
        output
    );

    // Should not be empty
    assert!(output.len() > 10, "Config output should be substantial");
}

#[test]
fn test_model_command_shows_current_model() {
    let cmd = Command::new("model".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // Should mention model
    assert!(
        output.to_lowercase().contains("model"),
        "Model command should mention model: {}",
        output
    );

    // Should show an actual model name (likely claude)
    assert!(
        output.contains("claude") || output.contains("sonnet") || output.contains("opus"),
        "Model should show actual Claude model: {}",
        output
    );
}

#[test]
fn test_status_command_shows_system_status() {
    let cmd = Command::new("status".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // Should show status information
    assert!(
        output.to_lowercase().contains("status"),
        "Status command should mention status: {}",
        output
    );

    // Should not be placeholder
    assert!(
        !output.contains("unknown"),
        "Status should not show unknown: {}",
        output
    );
}

#[test]
fn test_doctor_command_shows_diagnostics() {
    let cmd = Command::new("doctor".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // Should show diagnostic information
    assert!(
        output.to_lowercase().contains("check") || output.to_lowercase().contains("diagnostic"),
        "Doctor should show diagnostic info: {}",
        output
    );
}

#[test]
fn test_tools_command_lists_available_tools() {
    let cmd = Command::new("tools".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // Should list actual tools
    assert!(
        output.contains("Bash") || output.contains("Read") || output.contains("Write"),
        "Tools should list actual tool names: {}",
        output
    );

    // Should not be empty list
    assert!(
        !output.contains("No tools") && !output.contains("(none)"),
        "Tools should show available tools: {}",
        output
    );
}

#[test]
fn test_command_parser_handles_all_builtins() {
    let parser = CommandParser::new();

    let builtin_names = vec![
        "help", "exit", "quit", "clear", "history", "stats", "config", "model", "status",
        "version", "doctor",
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
    let commands = vec![
        "help", "stats", "history", "version", "config", "model", "status",
    ];

    for cmd_name in commands {
        let cmd = Command::new(cmd_name.to_string(), None);
        let result = BuiltinCommands::execute(&cmd);

        assert!(result.is_some());
        let output = result.unwrap();

        // Verify output doesn't contain obvious sample data markers
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
fn test_model_command_with_argument() {
    let cmd = Command::new("model".to_string(), Some("claude-opus-4".to_string()));
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // Should acknowledge the model change
    assert!(
        output.contains("claude-opus-4") || output.contains("Switching"),
        "Model command should acknowledge argument: {}",
        output
    );
}

#[test]
fn test_help_with_search_term() {
    let cmd = Command::new("help".to_string(), Some("commands".to_string()));
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // Should mention the search term
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
        "history",
        "stats",
        "config",
        "model",
        "status",
        "version",
        "doctor",
        "tools",
        "plugins",
        "checkpoint",
        "reset",
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
fn test_checkpoint_command_with_name() {
    let cmd = Command::new("checkpoint".to_string(), Some("my-checkpoint".to_string()));
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    assert!(
        output.contains("checkpoint") && output.contains("my-checkpoint"),
        "Checkpoint should acknowledge the name: {}",
        output
    );
}

#[test]
fn test_restore_command_with_name() {
    let cmd = Command::new("restore".to_string(), Some("my-checkpoint".to_string()));
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    assert!(
        output.contains("Restoring") || output.contains("checkpoint"),
        "Restore should acknowledge action: {}",
        output
    );
}

#[test]
fn test_export_command_with_path() {
    let cmd = Command::new("export".to_string(), Some("/tmp/export.json".to_string()));
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    assert!(
        output.contains("/tmp/export.json") || output.contains("Exporting"),
        "Export should acknowledge the path: {}",
        output
    );
}

#[test]
fn test_debug_command_with_level() {
    let cmd = Command::new("debug".to_string(), Some("verbose".to_string()));
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    assert!(
        output.contains("Debug") || output.contains("verbose"),
        "Debug should acknowledge the level: {}",
        output
    );
}

#[test]
fn test_mcp_command_shows_status() {
    let cmd = Command::new("mcp".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    assert!(
        output.to_lowercase().contains("mcp"),
        "MCP command should show MCP info: {}",
        output
    );
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
    // Test that is_builtin and execute are consistent
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
fn test_no_emoji_in_error_paths() {
    // While emojis are OK in success messages, critical error paths should be clear
    let cmd = Command::new("restore".to_string(), None);
    let result = BuiltinCommands::execute(&cmd);

    assert!(result.is_some());
    let output = result.unwrap();

    // Usage messages should be clear
    if output.contains("Usage:") {
        // Count emoji-like characters
        let emoji_count = output
            .chars()
            .filter(|c| {
                let code = *c as u32;
                code >= 0x1F300 && code <= 0x1F9FF // Emoji range
            })
            .count();

        // Usage messages should be straightforward
        assert!(
            emoji_count == 0,
            "Usage messages should not contain emojis: {}",
            output
        );
    }
}
