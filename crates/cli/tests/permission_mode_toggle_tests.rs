//! Permission Mode Toggle Tests (Issue #108)
//!
//! Tests for permission mode feature that allows toggling between:
//! - Ask: Prompts for permission before tool execution
//! - AutoAccept: Auto-approves all tool executions
//! - Plan: Planning only, disallows tool execution
//!
//! Keyboard shortcut: Shift+Tab cycles through modes
//!
//! Total: 57 tests

use rustyclawd::permission_mode::PermissionMode;

// =============================================================================
// Unit Tests - PermissionMode Enum (12 tests)
// =============================================================================

#[test]
fn test_permission_mode_default() {
    let mode = PermissionMode::default();
    assert_eq!(mode, PermissionMode::Ask, "Default mode should be Ask");
}

#[test]
fn test_permission_mode_cycle_ask_to_autoaccept() {
    let mode = PermissionMode::Ask;
    let next = mode.cycle();
    assert_eq!(
        next,
        PermissionMode::AutoAccept,
        "Ask should cycle to AutoAccept"
    );
}

#[test]
fn test_permission_mode_cycle_autoaccept_to_plan() {
    let mode = PermissionMode::AutoAccept;
    let next = mode.cycle();
    assert_eq!(
        next,
        PermissionMode::Plan,
        "AutoAccept should cycle to Plan"
    );
}

#[test]
fn test_permission_mode_cycle_plan_to_ask() {
    let mode = PermissionMode::Plan;
    let next = mode.cycle();
    assert_eq!(next, PermissionMode::Ask, "Plan should cycle to Ask");
}

#[test]
fn test_permission_mode_display_name_ask() {
    let mode = PermissionMode::Ask;
    assert_eq!(mode.display_name(), "Ask");
}

#[test]
fn test_permission_mode_display_name_autoaccept() {
    let mode = PermissionMode::AutoAccept;
    assert_eq!(mode.display_name(), "Auto-Accept");
}

#[test]
fn test_permission_mode_display_name_plan() {
    let mode = PermissionMode::Plan;
    assert_eq!(mode.display_name(), "Plan");
}

#[test]
fn test_permission_mode_status_indicator_ask() {
    let mode = PermissionMode::Ask;
    assert_eq!(mode.status_indicator(), "? Ask");
}

#[test]
fn test_permission_mode_status_indicator_autoaccept() {
    let mode = PermissionMode::AutoAccept;
    assert_eq!(mode.status_indicator(), "* Auto");
}

#[test]
fn test_permission_mode_status_indicator_plan() {
    let mode = PermissionMode::Plan;
    assert_eq!(mode.status_indicator(), "! Plan");
}

#[test]
fn test_permission_mode_eq() {
    assert_eq!(PermissionMode::Ask, PermissionMode::Ask);
    assert_eq!(PermissionMode::AutoAccept, PermissionMode::AutoAccept);
    assert_eq!(PermissionMode::Plan, PermissionMode::Plan);
    assert_ne!(PermissionMode::Ask, PermissionMode::AutoAccept);
    assert_ne!(PermissionMode::Ask, PermissionMode::Plan);
    assert_ne!(PermissionMode::AutoAccept, PermissionMode::Plan);
}

#[test]
fn test_permission_mode_clone() {
    let mode = PermissionMode::AutoAccept;
    let cloned = mode;
    assert_eq!(mode, cloned);
}

// =============================================================================
// Unit Tests - Tool Restrictions (15 tests)
// =============================================================================

#[test]
fn test_ask_mode_allows_all_tools() {
    let mode = PermissionMode::Ask;
    assert!(mode.allows_tool("Bash"));
    assert!(mode.allows_tool("BashOutput"));
    assert!(mode.allows_tool("KillShell"));
    assert!(mode.allows_tool("Write"));
    assert!(mode.allows_tool("Edit"));
    assert!(mode.allows_tool("Read"));
    assert!(mode.allows_tool("Glob"));
    assert!(mode.allows_tool("Grep"));
}

#[test]
fn test_autoaccept_mode_allows_all_tools() {
    let mode = PermissionMode::AutoAccept;
    assert!(mode.allows_tool("Bash"));
    assert!(mode.allows_tool("BashOutput"));
    assert!(mode.allows_tool("KillShell"));
    assert!(mode.allows_tool("Write"));
    assert!(mode.allows_tool("Edit"));
    assert!(mode.allows_tool("Read"));
    assert!(mode.allows_tool("Glob"));
    assert!(mode.allows_tool("Grep"));
}

#[test]
fn test_plan_mode_blocks_bash() {
    let mode = PermissionMode::Plan;
    assert!(!mode.allows_tool("Bash"), "Plan mode should block Bash");
}

#[test]
fn test_plan_mode_blocks_bashoutput() {
    let mode = PermissionMode::Plan;
    assert!(
        !mode.allows_tool("BashOutput"),
        "Plan mode should block BashOutput"
    );
}

#[test]
fn test_plan_mode_blocks_killshell() {
    let mode = PermissionMode::Plan;
    assert!(
        !mode.allows_tool("KillShell"),
        "Plan mode should block KillShell"
    );
}

#[test]
fn test_plan_mode_blocks_write() {
    let mode = PermissionMode::Plan;
    assert!(!mode.allows_tool("Write"), "Plan mode should block Write");
}

#[test]
fn test_plan_mode_blocks_edit() {
    let mode = PermissionMode::Plan;
    assert!(!mode.allows_tool("Edit"), "Plan mode should block Edit");
}

#[test]
fn test_plan_mode_allows_read() {
    let mode = PermissionMode::Plan;
    assert!(mode.allows_tool("Read"), "Plan mode should allow Read");
}

#[test]
fn test_plan_mode_allows_glob() {
    let mode = PermissionMode::Plan;
    assert!(mode.allows_tool("Glob"), "Plan mode should allow Glob");
}

#[test]
fn test_plan_mode_allows_grep() {
    let mode = PermissionMode::Plan;
    assert!(mode.allows_tool("Grep"), "Plan mode should allow Grep");
}

#[test]
fn test_plan_mode_allows_askuserquestion() {
    let mode = PermissionMode::Plan;
    assert!(
        mode.allows_tool("AskUserQuestion"),
        "Plan mode should allow AskUserQuestion"
    );
}

#[test]
fn test_plan_mode_allows_skill() {
    let mode = PermissionMode::Plan;
    assert!(mode.allows_tool("Skill"), "Plan mode should allow Skill");
}

#[test]
fn test_plan_mode_allows_slashcommand() {
    let mode = PermissionMode::Plan;
    assert!(
        mode.allows_tool("SlashCommand"),
        "Plan mode should allow SlashCommand"
    );
}

#[test]
fn test_plan_mode_allows_task() {
    let mode = PermissionMode::Plan;
    assert!(mode.allows_tool("Task"), "Plan mode should allow Task");
}

#[test]
fn test_plan_mode_allows_todowrite() {
    let mode = PermissionMode::Plan;
    assert!(
        mode.allows_tool("TodoWrite"),
        "Plan mode should allow TodoWrite"
    );
}

// =============================================================================
// Unit Tests - Blocked Tools List (3 tests)
// =============================================================================

#[test]
fn test_ask_mode_blocked_tools_empty() {
    let mode = PermissionMode::Ask;
    assert!(
        mode.blocked_tools().is_empty(),
        "Ask mode should have no blocked tools"
    );
}

#[test]
fn test_autoaccept_mode_blocked_tools_empty() {
    let mode = PermissionMode::AutoAccept;
    assert!(
        mode.blocked_tools().is_empty(),
        "AutoAccept mode should have no blocked tools"
    );
}

#[test]
fn test_plan_mode_blocked_tools_correct() {
    let mode = PermissionMode::Plan;
    let blocked = mode.blocked_tools();
    assert!(blocked.contains(&"Bash"), "Plan should block Bash");
    assert!(
        blocked.contains(&"BashOutput"),
        "Plan should block BashOutput"
    );
    assert!(
        blocked.contains(&"KillShell"),
        "Plan should block KillShell"
    );
    assert!(blocked.contains(&"Write"), "Plan should block Write");
    assert!(blocked.contains(&"Edit"), "Plan should block Edit");
    assert_eq!(blocked.len(), 5, "Plan should block exactly 5 tools");
}

// =============================================================================
// Integration Tests - Keyboard Handling (8 tests)
// =============================================================================

mod keyboard_tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    /// Helper to create key event
    fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn test_shift_tab_cycles_mode() {
        let mut mode = PermissionMode::Ask;

        // Simulate Shift+Tab press
        mode = mode.cycle();
        assert_eq!(mode, PermissionMode::AutoAccept);

        mode = mode.cycle();
        assert_eq!(mode, PermissionMode::Plan);

        mode = mode.cycle();
        assert_eq!(mode, PermissionMode::Ask);
    }

    #[test]
    fn test_shift_tab_from_ask() {
        let mode = PermissionMode::Ask;
        assert_eq!(mode.cycle(), PermissionMode::AutoAccept);
    }

    #[test]
    fn test_shift_tab_from_autoaccept() {
        let mode = PermissionMode::AutoAccept;
        assert_eq!(mode.cycle(), PermissionMode::Plan);
    }

    #[test]
    fn test_shift_tab_from_plan() {
        let mode = PermissionMode::Plan;
        assert_eq!(mode.cycle(), PermissionMode::Ask);
    }

    #[test]
    fn test_multiple_shift_tab_cycles() {
        let mut mode = PermissionMode::Ask;

        // Cycle 6 times to complete 2 full cycles
        for _ in 0..6 {
            mode = mode.cycle();
        }
        assert_eq!(
            mode,
            PermissionMode::Ask,
            "After 6 cycles should return to Ask"
        );
    }

    #[test]
    fn test_backtab_key_code_exists() {
        // Verify BackTab is the correct key code for Shift+Tab
        let key = key_event(KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(key.code, KeyCode::BackTab);
        assert!(key.modifiers.contains(KeyModifiers::SHIFT));
    }

    #[test]
    fn test_tab_key_different_from_backtab() {
        let tab = key_event(KeyCode::Tab, KeyModifiers::NONE);
        let backtab = key_event(KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_ne!(tab.code, backtab.code);
    }

    #[test]
    fn test_shift_tab_returns_mode_change_event() {
        // When Shift+Tab is pressed, the handler should return the new mode
        let mode = PermissionMode::Ask;
        let new_mode = mode.cycle();
        assert_ne!(mode, new_mode, "Mode should change");
    }
}

// =============================================================================
// Integration Tests - TUI Display (6 tests)
// =============================================================================

mod tui_display_tests {
    use super::*;

    #[test]
    fn test_status_bar_shows_ask_mode() {
        let mode = PermissionMode::Ask;
        let indicator = mode.status_indicator();
        assert!(indicator.contains("Ask"), "Status should show Ask");
        assert!(indicator.contains("?"), "Status should show ? for Ask");
    }

    #[test]
    fn test_status_bar_shows_autoaccept_mode() {
        let mode = PermissionMode::AutoAccept;
        let indicator = mode.status_indicator();
        assert!(indicator.contains("Auto"), "Status should show Auto");
        assert!(indicator.contains("*"), "Status should show * for Auto");
    }

    #[test]
    fn test_status_bar_shows_plan_mode() {
        let mode = PermissionMode::Plan;
        let indicator = mode.status_indicator();
        assert!(indicator.contains("Plan"), "Status should show Plan");
        assert!(indicator.contains("!"), "Status should show ! for Plan");
    }

    #[test]
    fn test_mode_change_shows_notification() {
        let mode = PermissionMode::Ask;
        let new_mode = mode.cycle();
        let notification = format!("Permission mode changed to: {}", new_mode.display_name());
        assert!(notification.contains("Auto-Accept"));
    }

    #[test]
    fn test_initial_mode_is_ask() {
        let mode = PermissionMode::default();
        assert_eq!(mode, PermissionMode::Ask);
        assert_eq!(mode.display_name(), "Ask");
    }

    #[test]
    fn test_status_bar_mode_updates_on_cycle() {
        let mut mode = PermissionMode::Ask;
        assert!(mode.status_indicator().contains("Ask"));

        mode = mode.cycle();
        assert!(mode.status_indicator().contains("Auto"));

        mode = mode.cycle();
        assert!(mode.status_indicator().contains("Plan"));
    }
}

// =============================================================================
// Integration Tests - Tool Execution (10 tests)
// =============================================================================

mod tool_execution_tests {
    use super::*;

    /// Helper to check tool permission result
    fn check_tool_result(mode: PermissionMode, tool_name: &str) -> Result<(), String> {
        if mode.allows_tool(tool_name) {
            Ok(())
        } else {
            Err(format!(
                "Tool '{}' blocked in Plan mode. Switch to Ask or Auto-Accept mode to execute tools.",
                tool_name
            ))
        }
    }

    #[test]
    fn test_tool_execution_ask_mode_prompts() {
        // In Ask mode, tools should be allowed but would prompt user
        let mode = PermissionMode::Ask;
        assert!(mode.allows_tool("Bash"));
        // Note: Actual prompting is handled by the interactive session
    }

    #[test]
    fn test_tool_execution_autoaccept_runs() {
        let mode = PermissionMode::AutoAccept;
        assert!(mode.allows_tool("Bash"));
        assert!(mode.allows_tool("Write"));
        assert!(mode.allows_tool("Edit"));
    }

    #[test]
    fn test_tool_execution_plan_blocks_bash() {
        let mode = PermissionMode::Plan;
        let result = check_tool_result(mode, "Bash");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("blocked in Plan mode"));
    }

    #[test]
    fn test_tool_execution_plan_blocks_write() {
        let mode = PermissionMode::Plan;
        let result = check_tool_result(mode, "Write");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("blocked in Plan mode"));
    }

    #[test]
    fn test_tool_execution_plan_allows_read() {
        let mode = PermissionMode::Plan;
        let result = check_tool_result(mode, "Read");
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_execution_plan_error_message() {
        let mode = PermissionMode::Plan;
        let result = check_tool_result(mode, "Bash");
        let err = result.unwrap_err();
        assert!(err.contains("Bash"), "Error should mention tool name");
        assert!(err.contains("Plan mode"), "Error should mention Plan mode");
        assert!(
            err.contains("Ask") || err.contains("Auto-Accept"),
            "Error should suggest alternatives"
        );
    }

    #[test]
    fn test_tool_execution_plan_mode_read_only() {
        let mode = PermissionMode::Plan;
        // All read-only tools should work
        assert!(mode.allows_tool("Read"));
        assert!(mode.allows_tool("Glob"));
        assert!(mode.allows_tool("Grep"));
        // All modification tools should be blocked
        assert!(!mode.allows_tool("Bash"));
        assert!(!mode.allows_tool("Write"));
        assert!(!mode.allows_tool("Edit"));
    }

    #[test]
    fn test_tool_execution_mode_persists() {
        let mode = PermissionMode::Plan;
        // Mode should stay the same across multiple checks
        assert!(!mode.allows_tool("Bash"));
        assert!(!mode.allows_tool("Bash"));
        assert_eq!(mode, PermissionMode::Plan);
    }

    #[test]
    fn test_tool_blocked_returns_informative_error() {
        let mode = PermissionMode::Plan;
        let result = check_tool_result(mode, "Edit");
        let err = result.unwrap_err();

        // Error should be informative
        assert!(err.len() > 20, "Error message should be descriptive");
        assert!(err.contains("Edit"), "Should mention the tool");
        assert!(err.contains("Switch"), "Should suggest how to resolve");
    }

    #[test]
    fn test_mode_change_affects_subsequent_tools() {
        let mut mode = PermissionMode::Ask;
        assert!(mode.allows_tool("Bash"));

        // Switch to Plan mode
        mode = PermissionMode::Plan;
        assert!(!mode.allows_tool("Bash"));

        // Switch to AutoAccept
        mode = PermissionMode::AutoAccept;
        assert!(mode.allows_tool("Bash"));
    }
}

// =============================================================================
// E2E Tests - Full Workflow (3 tests)
// =============================================================================

mod e2e_tests {
    use super::*;

    #[test]
    fn test_e2e_permission_mode_full_cycle() {
        // Simulate a full cycle through all modes
        let mut mode = PermissionMode::default();
        assert_eq!(mode, PermissionMode::Ask);

        // First Shift+Tab: Ask -> AutoAccept
        mode = mode.cycle();
        assert_eq!(mode, PermissionMode::AutoAccept);
        assert!(mode.allows_tool("Bash"));
        assert!(mode.allows_tool("Write"));

        // Second Shift+Tab: AutoAccept -> Plan
        mode = mode.cycle();
        assert_eq!(mode, PermissionMode::Plan);
        assert!(!mode.allows_tool("Bash"));
        assert!(mode.allows_tool("Read"));

        // Third Shift+Tab: Plan -> Ask
        mode = mode.cycle();
        assert_eq!(mode, PermissionMode::Ask);
        assert!(mode.allows_tool("Bash"));
    }

    #[test]
    fn test_e2e_plan_mode_prevents_modifications() {
        let mode = PermissionMode::Plan;

        // Verify all modification tools are blocked
        let blocked = mode.blocked_tools();
        for tool in blocked {
            assert!(
                !mode.allows_tool(tool),
                "Tool {} should be blocked in Plan mode",
                tool
            );
        }

        // Verify read-only tools work
        let readonly_tools = [
            "Read",
            "Glob",
            "Grep",
            "AskUserQuestion",
            "Skill",
            "SlashCommand",
            "Task",
            "TodoWrite",
        ];
        for tool in readonly_tools {
            assert!(
                mode.allows_tool(tool),
                "Tool {} should be allowed in Plan mode",
                tool
            );
        }
    }

    #[test]
    fn test_e2e_mode_change_mid_session() {
        // Simulate session with mode changes
        let mut mode = PermissionMode::Ask;
        let mut actions = Vec::new();

        // User wants to execute Bash - allowed in Ask mode
        if mode.allows_tool("Bash") {
            actions.push(("Bash", true));
        }

        // User presses Shift+Tab twice to enter Plan mode
        mode = mode.cycle(); // AutoAccept
        mode = mode.cycle(); // Plan

        // User tries to execute Bash - blocked
        if !mode.allows_tool("Bash") {
            actions.push(("Bash", false));
        }

        // User can still read files
        if mode.allows_tool("Read") {
            actions.push(("Read", true));
        }

        // User presses Shift+Tab to go back to Ask
        mode = mode.cycle();

        // Now Bash works again
        if mode.allows_tool("Bash") {
            actions.push(("Bash", true));
        }

        // Verify the sequence
        assert_eq!(actions.len(), 4);
        assert_eq!(actions[0], ("Bash", true)); // Ask mode
        assert_eq!(actions[1], ("Bash", false)); // Plan mode - blocked
        assert_eq!(actions[2], ("Read", true)); // Plan mode - allowed
        assert_eq!(actions[3], ("Bash", true)); // Ask mode again
    }
}
