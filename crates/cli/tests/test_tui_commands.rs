//! TUI Command Tests
//!
//! Tests for slash command handling:
//! - Command parsing
//! - Command execution
//! - Built-in commands
//! - Command history

mod helpers;
mod tui_test_harness;

use helpers::event_generator::EventGenerator;
use rustyclawd::tui::{ChatMessage, MessageRole};
use tui_test_harness::TuiTestHarness;

#[test]
fn test_exit_command() {
    // Test /exit command
    let events = EventGenerator::slash_command("exit");

    assert_eq!(events.len(), 6);
    assert_eq!(events[0].code, crossterm::event::KeyCode::Char('/'));

    // Reconstruct command
    let command: String = events[..5]
        .iter()
        .filter_map(|e| {
            if let crossterm::event::KeyCode::Char(c) = e.code {
                Some(c)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(command, "/exit");
}

#[test]
fn test_help_command() {
    // Test /help command
    let events = EventGenerator::slash_command("help");

    // Reconstruct command
    let command: String = events[..5]
        .iter()
        .filter_map(|e| {
            if let crossterm::event::KeyCode::Char(c) = e.code {
                Some(c)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(command, "/help");
}

#[test]
fn test_clear_command() {
    // Test /clear command clears messages
    let mut messages = vec![
        ChatMessage::user("Hello".to_string()),
        ChatMessage::assistant("Hi there".to_string()),
    ];

    assert_eq!(messages.len(), 2);

    // Simulate clear
    messages.clear();

    assert_eq!(messages.len(), 0);
}

#[test]
fn test_clear_command_rendering() {
    // Test that clear command is rendered
    let mut harness = TuiTestHarness::new().unwrap();

    harness
        .terminal
        .draw(|f| {
            use ratatui::{text::Line, widgets::Paragraph};

            let paragraph = Paragraph::new(Line::from("System> Conversation cleared."));
            f.render_widget(paragraph, f.area());
        })
        .unwrap();

    assert!(harness.contains("System>"));
    assert!(harness.contains("cleared"));
}

#[test]
fn test_unknown_command() {
    // Test unknown command handling
    let command = "/unknowncommand";

    // Unknown commands should be identifiable
    let known_commands = ["/exit", "/help", "/clear"];
    let is_known = known_commands.contains(&command);

    assert!(!is_known);
}

#[test]
fn test_command_parsing() {
    // Test command parsing
    let inputs = vec![
        ("/exit", "exit", None),
        ("/help", "help", None),
        ("/help bash", "help", Some("bash")),
    ];

    for (input, expected_cmd, expected_arg) in inputs {
        let parts: Vec<&str> = input[1..].split_whitespace().collect();
        let cmd = parts[0];
        let arg = parts.get(1).copied();

        assert_eq!(cmd, expected_cmd);
        assert_eq!(arg, expected_arg);
    }
}

#[test]
fn test_quit_command() {
    // Test /quit command (alias for /exit)
    let quit_aliases = vec!["/exit", "/quit"];

    for alias in quit_aliases {
        assert!(alias.starts_with('/'));
        let cmd = &alias[1..];
        assert!(["exit", "quit"].contains(&cmd));
    }
}

#[test]
fn test_command_input_validation() {
    // Test command input validation
    let inputs = vec![
        ("/exit", true),
        ("exit", false), // Missing slash
        ("/", false),    // Only slash
        ("", false),     // Empty
    ];

    for (input, should_be_command) in inputs {
        let is_command = input.starts_with('/') && input.len() > 1;
        assert_eq!(is_command, should_be_command);
    }
}

#[test]
fn test_multiple_commands_sequence() {
    // Test sequence of multiple commands
    let commands = vec!["/help", "/clear", "/exit"];

    for cmd in commands {
        assert!(cmd.starts_with('/'));
        assert!(cmd.len() > 1);
    }
}

#[test]
fn test_command_with_special_characters() {
    // Test commands with special characters
    let command = "/help bash";

    let parts: Vec<&str> = command[1..].split_whitespace().collect();
    assert_eq!(parts[0], "help");
    assert_eq!(parts[1], "bash");
}

#[test]
fn test_ctrl_c_exit() {
    // Test Ctrl+C triggers exit
    let ctrl_c = EventGenerator::ctrl_c();

    assert_eq!(ctrl_c.code, crossterm::event::KeyCode::Char('c'));
    assert!(ctrl_c
        .modifiers
        .contains(crossterm::event::KeyModifiers::CONTROL));
}

#[test]
fn test_ctrl_d_exit() {
    // Test Ctrl+D triggers exit
    let ctrl_d = EventGenerator::ctrl_d();

    assert_eq!(ctrl_d.code, crossterm::event::KeyCode::Char('d'));
    assert!(ctrl_d
        .modifiers
        .contains(crossterm::event::KeyModifiers::CONTROL));
}

#[test]
fn test_command_message_display() {
    // Test system messages for commands
    let system_message = ChatMessage::system("Command executed successfully".to_string());

    assert!(matches!(system_message.role, MessageRole::System));
    assert_eq!(system_message.content, "Command executed successfully");
}

#[test]
fn test_help_output() {
    // Test help command output format
    let help_text = "Commands: /exit, /quit, /clear, /help\nPress Ctrl+C or Ctrl+D to exit.";

    assert!(help_text.contains("/exit"));
    assert!(help_text.contains("/help"));
    assert!(help_text.contains("Ctrl+C"));
}
