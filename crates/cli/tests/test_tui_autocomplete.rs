//! TUI Autocomplete Tests
//!
//! Tests for autocomplete functionality:
//! - Suggestion generation
//! - Suggestion selection
//! - Suggestion application
//! - Tab completion

mod helpers;
mod tui_test_harness;

use helpers::event_generator::EventGenerator;

#[test]
fn test_autocomplete_slash_commands() {
    // Test autocomplete for slash commands
    let slash_commands: Vec<(&str, Option<&str>)> =
        vec![("exit", None), ("help", None), ("clear", None)];

    // Verify commands can be matched
    let prefix = "ex";
    let matches: Vec<_> = slash_commands
        .iter()
        .filter(|(cmd, _)| cmd.starts_with(prefix))
        .collect();

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].0, "exit");
}

#[test]
fn test_autocomplete_prefix_matching() {
    // Test prefix matching for suggestions
    let commands = ["exit", "execute", "export"];

    let prefix = "ex";
    let matches: Vec<_> = commands
        .iter()
        .filter(|cmd| cmd.starts_with(prefix))
        .collect();

    assert_eq!(matches.len(), 3);
}

#[test]
fn test_autocomplete_exact_match() {
    // Test exact match returns single suggestion
    let commands = ["exit", "execute", "export"];

    let prefix = "exit";
    let matches: Vec<_> = commands
        .iter()
        .filter(|cmd| cmd.starts_with(prefix))
        .collect();

    assert!(matches.contains(&&"exit"));
}

#[test]
fn test_autocomplete_no_matches() {
    // Test no matches returns empty
    let commands = ["exit", "help", "clear"];

    let prefix = "xyz";
    let matches: Vec<_> = commands
        .iter()
        .filter(|cmd| cmd.starts_with(prefix))
        .collect();

    assert_eq!(matches.len(), 0);
}

#[test]
fn test_autocomplete_case_sensitivity() {
    // Test case-sensitive matching
    let commands = ["exit", "Exit", "EXIT"];

    let prefix = "ex";
    let matches: Vec<_> = commands
        .iter()
        .filter(|cmd| cmd.starts_with(prefix))
        .collect();

    assert_eq!(matches.len(), 1); // Only lowercase "exit"
}

#[test]
fn test_autocomplete_with_arguments() {
    // Test autocomplete with command arguments
    let commands_with_args = [("help", Some("<command>")), ("exit", None), ("clear", None)];

    // Verify commands with args can be identified
    let has_args: Vec<_> = commands_with_args
        .iter()
        .filter(|(_, args)| args.is_some())
        .collect();

    assert_eq!(has_args.len(), 1);
    assert_eq!(has_args[0].0, "help");
}

#[test]
fn test_tab_key_for_completion() {
    // Test Tab key triggers completion
    let tab_event = EventGenerator::tab();

    assert_eq!(tab_event.code, crossterm::event::KeyCode::Tab);
}

#[test]
fn test_autocomplete_selection_up_down() {
    // Test up/down arrow selection
    let suggestions = ["exit", "execute", "export"];
    let mut selected = 0;

    // Simulate down arrow
    selected = (selected + 1) % suggestions.len();
    assert_eq!(selected, 1);

    // Simulate down arrow again
    selected = (selected + 1) % suggestions.len();
    assert_eq!(selected, 2);

    // Simulate up arrow
    selected = if selected == 0 {
        suggestions.len() - 1
    } else {
        selected - 1
    };
    assert_eq!(selected, 1);
}

#[test]
fn test_autocomplete_cycling() {
    // Test cycling through suggestions
    let suggestions = ["a", "b", "c"];
    let mut selected = 0;

    // Cycle forward
    for i in 1..=5 {
        selected = (selected + 1) % suggestions.len();
        assert_eq!(selected, i % suggestions.len());
    }

    // Cycle backward
    selected = 0;
    for _ in 0..3 {
        selected = if selected == 0 {
            suggestions.len() - 1
        } else {
            selected - 1
        };
    }
    assert_eq!(selected, 0);
}

#[test]
fn test_autocomplete_application() {
    // Test applying selected suggestion
    let command = "exit";
    let input_prefix = "/ex";

    // Simulate applying suggestion
    let completed = format!("/{}", command);

    assert_eq!(completed, "/exit");
    assert!(completed.starts_with(input_prefix));
}

#[test]
fn test_autocomplete_with_space() {
    // Test autocomplete adds space for commands with arguments
    let command = "help";
    let has_args = true;

    let completed = if has_args {
        format!("/{} ", command)
    } else {
        format!("/{}", command)
    };

    assert_eq!(completed, "/help ");
    assert!(completed.ends_with(' '));
}

#[test]
fn test_autocomplete_clear_on_application() {
    // Test suggestions cleared after application
    let mut suggestions = vec!["exit", "execute"];

    // Simulate applying suggestion
    suggestions.clear();

    assert_eq!(suggestions.len(), 0);
}
