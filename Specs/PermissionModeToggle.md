# Permission Mode Toggle Specification

## Overview

Implements Claude Code's permission mode toggle feature that allows users to switch between different tool permission levels using Shift+Tab keyboard shortcut.

## Requirements (from Issue #108)

1. Add `Plan` variant to `PermissionMode` enum
2. Implement Shift+Tab keyboard shortcut for cycling modes
3. Match Claude Code's behavior exactly

## Permission Modes

### Mode Definitions

| Mode       | Description                                          | Tool Execution              |
|------------|-----------------------------------------------------|----------------------------|
| `Ask`      | Default mode - prompts for permission before tools  | User confirms each tool    |
| `AutoAccept` | Auto-approves all tool executions                 | No prompts, all tools run  |
| `Plan`     | Planning only - disallows tool execution            | All tools blocked          |

### Mode Cycling Order

Shift+Tab cycles through modes in this order:
```
Ask -> AutoAccept -> Plan -> Ask -> ...
```

### Keyboard Shortcut

- **Shift+Tab**: Cycles to next permission mode
- Works in TUI input mode
- Displays mode change notification

## Tool Restrictions in Plan Mode

When `Plan` mode is active, these tools are BLOCKED:
- `Bash` - Shell command execution
- `BashOutput` - Reading shell output
- `KillShell` - Terminating shells
- `Write` - File writing
- `Edit` - File editing

These tools remain ALLOWED in Plan mode (read-only):
- `Read` - File reading
- `Glob` - File pattern matching
- `Grep` - Content searching
- `AskUserQuestion` - User interaction
- `Skill` - Skill execution
- `SlashCommand` - Command execution
- `Task` - Agent delegation
- `TodoWrite` - Todo management

## API

### PermissionMode Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    #[default]
    Ask,
    AutoAccept,
    Plan,
}
```

### Methods

```rust
impl PermissionMode {
    /// Cycle to the next mode: Ask -> AutoAccept -> Plan -> Ask
    pub fn cycle(&self) -> Self;

    /// Get display name for TUI
    pub fn display_name(&self) -> &'static str;

    /// Get short indicator for status bar (icon + abbreviated name)
    pub fn status_indicator(&self) -> &'static str;

    /// Check if tool execution is allowed in current mode
    pub fn allows_tool(&self, tool_name: &str) -> bool;

    /// Get list of blocked tools in current mode
    pub fn blocked_tools(&self) -> &'static [&'static str];
}
```

### Display Names

| Mode       | `display_name()` | `status_indicator()` |
|------------|------------------|---------------------|
| Ask        | "Ask"            | "? Ask"             |
| AutoAccept | "Auto-Accept"    | "* Auto"            |
| Plan       | "Plan"           | "! Plan"            |

## TUI Integration

### Status Bar Display

The current permission mode should be displayed in the TUI status bar:
- Format: `[Mode: {indicator}]`
- Example: `[Mode: ? Ask]`
- Position: Right side of status bar

### Mode Change Notification

When mode changes via Shift+Tab:
1. Cycle to next mode
2. Update status bar display
3. Show brief notification message in messages area:
   - Format: `Permission mode changed to: {display_name}`

### Keyboard Handling

In `handle_key_event`:
```rust
// Shift+Tab cycles permission mode
(KeyCode::BackTab, KeyModifiers::SHIFT) => {
    self.permission_mode = self.permission_mode.cycle();
    // Notification handled by caller
}
```

Note: `KeyCode::BackTab` is crossterm's code for Shift+Tab.

## Tool Executor Integration

### Pre-execution Check

Before executing any tool in `execute_tool_with_hooks`:
```rust
// Check permission mode
if !permission_mode.allows_tool(&tool_name) {
    return Err(ClientError::Api(format!(
        "Tool '{}' blocked in Plan mode. Switch to Ask or Auto-Accept mode to execute tools.",
        tool_name
    )));
}
```

### Permission Prompt (Ask Mode)

In Ask mode, before tool execution:
1. Display tool call details
2. Prompt: "Allow this tool execution? [y/n/a]"
   - `y` = yes, allow this time
   - `n` = no, deny this time
   - `a` = allow all (switch to AutoAccept mode)

## Test Coverage (57 Tests)

### Unit Tests - PermissionMode Enum (12 tests)

1. `test_permission_mode_default` - Default is Ask
2. `test_permission_mode_cycle_ask_to_autoaccept`
3. `test_permission_mode_cycle_autoaccept_to_plan`
4. `test_permission_mode_cycle_plan_to_ask`
5. `test_permission_mode_display_name_ask`
6. `test_permission_mode_display_name_autoaccept`
7. `test_permission_mode_display_name_plan`
8. `test_permission_mode_status_indicator_ask`
9. `test_permission_mode_status_indicator_autoaccept`
10. `test_permission_mode_status_indicator_plan`
11. `test_permission_mode_eq`
12. `test_permission_mode_clone`

### Unit Tests - Tool Restrictions (15 tests)

13. `test_ask_mode_allows_all_tools`
14. `test_autoaccept_mode_allows_all_tools`
15. `test_plan_mode_blocks_bash`
16. `test_plan_mode_blocks_bashoutput`
17. `test_plan_mode_blocks_killshell`
18. `test_plan_mode_blocks_write`
19. `test_plan_mode_blocks_edit`
20. `test_plan_mode_allows_read`
21. `test_plan_mode_allows_glob`
22. `test_plan_mode_allows_grep`
23. `test_plan_mode_allows_askuserquestion`
24. `test_plan_mode_allows_skill`
25. `test_plan_mode_allows_slashcommand`
26. `test_plan_mode_allows_task`
27. `test_plan_mode_allows_todowrite`

### Unit Tests - Blocked Tools List (3 tests)

28. `test_ask_mode_blocked_tools_empty`
29. `test_autoaccept_mode_blocked_tools_empty`
30. `test_plan_mode_blocked_tools_correct`

### Integration Tests - Keyboard Handling (8 tests)

31. `test_shift_tab_cycles_mode`
32. `test_shift_tab_from_ask`
33. `test_shift_tab_from_autoaccept`
34. `test_shift_tab_from_plan`
35. `test_multiple_shift_tab_cycles`
36. `test_backtab_without_shift_ignored`
37. `test_tab_key_not_cycle`
38. `test_shift_tab_returns_mode_change_event`

### Integration Tests - TUI Display (6 tests)

39. `test_status_bar_shows_ask_mode`
40. `test_status_bar_shows_autoaccept_mode`
41. `test_status_bar_shows_plan_mode`
42. `test_mode_change_shows_notification`
43. `test_initial_mode_is_ask`
44. `test_status_bar_mode_updates_on_cycle`

### Integration Tests - Tool Execution (10 tests)

45. `test_tool_execution_ask_mode_prompts`
46. `test_tool_execution_autoaccept_runs`
47. `test_tool_execution_plan_blocks_bash`
48. `test_tool_execution_plan_blocks_write`
49. `test_tool_execution_plan_allows_read`
50. `test_tool_execution_plan_error_message`
51. `test_tool_execution_plan_mode_read_only`
52. `test_tool_execution_mode_persists`
53. `test_tool_blocked_returns_informative_error`
54. `test_mode_change_affects_subsequent_tools`

### E2E Tests - Full Workflow (3 tests)

55. `test_e2e_permission_mode_full_cycle`
56. `test_e2e_plan_mode_prevents_modifications`
57. `test_e2e_mode_change_mid_session`

## Implementation Files

1. `crates/cli/src/permission_mode.rs` - New module with PermissionMode enum
2. `crates/cli/src/tui/ui.rs` - Keyboard handling and display updates
3. `crates/cli/src/interactive.rs` - Session state and tool execution integration
4. `crates/cli/src/lib.rs` - Module exports
5. `crates/cli/tests/permission_mode_toggle_tests.rs` - Test file

## Success Criteria

1. All 57 tests pass
2. Shift+Tab cycles modes in correct order
3. Plan mode blocks only modification tools
4. Status bar displays current mode
5. Mode change notification appears
6. Error messages are clear and actionable
