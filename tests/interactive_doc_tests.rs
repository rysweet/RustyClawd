//! Interactive Mode Documentation Test Suite
//!
//! Comprehensive tests derived from official Claude Code interactive mode documentation.
//! These tests cover EVERY feature documented at https://code.claude.com/docs/en/interactive-mode
//!
//! Test Strategy (TDD):
//! - All tests written to FAIL initially
//! - Tests drive implementation of features
//! - Follows testing pyramid: 60% unit, 30% integration, 10% E2E
//!
//! Coverage Categories:
//! - Keyboard shortcuts (Ctrl+C, Ctrl+D, Ctrl+L, Ctrl+O, Ctrl+R, Ctrl+V, Up/Down, Esc+Esc, Tab, Shift+Tab)
//! - Multiline input methods (backslash+Enter, Option+Enter, Shift+Enter, Ctrl+J, paste)
//! - Quick command prefixes (#, /, !, @)
//! - Vim editor mode (activation, mode switching, navigation, editing)
//! - Command history features (per-directory, navigation, search, clearing)
//! - Background bash commands (async execution, Ctrl+B, task IDs, output retrieval, auto-cleanup)
//! - Configuration commands (?, /terminal-setup, /config, /clear)

// Mock implementations - no external dependencies needed for TDD tests

// ============================================================================
// UNIT TESTS: Keyboard Shortcuts - General Controls
// ============================================================================

#[test]
fn test_ctrl_c_cancels_current_input() {
    let session = InteractiveSession::new();

    session.start_input("long running query");
    assert_eq!(session.input_in_progress(), true);

    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlC);

    assert_eq!(session.input_in_progress(), false);
    assert_eq!(session.last_input_cancelled(), true);
}

#[test]
fn test_ctrl_c_cancels_generation() {
    let session = InteractiveSession::new();

    session.process_input("generate code").unwrap();
    session.start_generation();
    assert_eq!(session.generation_in_progress(), true);

    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlC);

    assert_eq!(session.generation_in_progress(), false);
    assert_eq!(session.generation_cancelled(), true);
}

#[test]
fn test_ctrl_d_exits_session() {
    let session = InteractiveSession::new();
    assert_eq!(session.status(), SessionStatus::Active);

    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlD);

    assert_eq!(session.status(), SessionStatus::Closed);
}

#[test]
fn test_ctrl_l_clears_screen_preserves_history() {
    let session = InteractiveSession::new();

    session.process_input("message 1").unwrap();
    session.process_input("message 2").unwrap();
    session.process_input("message 3").unwrap();

    assert_eq!(session.history_len(), 3);
    assert_eq!(session.screen_is_clear(), false);

    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlL);

    // Screen cleared but history preserved
    assert_eq!(session.screen_is_clear(), true);
    assert_eq!(session.history_len(), 3);
    assert_eq!(session.conversation_turn_count(), 3);
}

#[test]
fn test_ctrl_o_toggles_verbose_output() {
    let session = InteractiveSession::new();
    assert_eq!(session.verbose_output_enabled(), false);

    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlO);
    assert_eq!(session.verbose_output_enabled(), true);

    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlO);
    assert_eq!(session.verbose_output_enabled(), false);
}

#[test]
fn test_ctrl_o_shows_detailed_tool_usage() {
    let session = InteractiveSession::new();
    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlO); // Enable verbose

    session.process_input("!ls -la").unwrap();
    let output = session.get_last_output();

    // Verbose output should contain tool details
    assert!(output.contains_tool_details());
    assert!(output.contains("Bash"));
    assert!(output.contains("execution"));
}

#[test]
fn test_ctrl_r_activates_reverse_search() {
    let history = CommandHistory::new();
    history.add_command("cargo build");
    history.add_command("cargo test");
    history.add_command("git commit");

    history.handle_keyboard_shortcut(KeyboardShortcut::CtrlR);

    assert_eq!(history.reverse_search_active(), true);
    assert_eq!(history.search_prompt_displayed(), true);
}

#[test]
fn test_ctrl_r_search_with_highlighting() {
    let history = CommandHistory::new();
    history.add_command("grep pattern file.txt");
    history.add_command("grep another search.log");
    history.add_command("find . -name '*.txt'");

    history.handle_keyboard_shortcut(KeyboardShortcut::CtrlR);
    history.type_search_term("grep");

    let results = history.get_search_results();
    assert_eq!(results.len(), 2);
    assert!(results[0].has_highlighting());
    assert!(results[0].highlighted_term() == "grep");
}

#[test]
fn test_ctrl_r_cycles_through_matches() {
    let history = CommandHistory::new();
    history.add_command("echo test1");
    history.add_command("echo test2");
    history.add_command("echo test3");

    history.handle_keyboard_shortcut(KeyboardShortcut::CtrlR);
    history.type_search_term("echo");

    assert_eq!(history.current_match_index(), 0);

    history.handle_keyboard_shortcut(KeyboardShortcut::CtrlR); // Next match
    assert_eq!(history.current_match_index(), 1);

    history.handle_keyboard_shortcut(KeyboardShortcut::CtrlR); // Next match
    assert_eq!(history.current_match_index(), 2);
}

#[test]
fn test_ctrl_v_paste_image_from_clipboard_macos() {
    let session = InteractiveSession::new();
    let clipboard = MockClipboard::new();
    clipboard.set_image_data(vec![0xFF, 0xD8, 0xFF]); // JPEG header

    session.handle_keyboard_shortcut_with_clipboard(
        KeyboardShortcut::CtrlV,
        &clipboard
    );

    assert_eq!(session.has_pending_image(), true);
    assert_eq!(session.pending_image_format(), Some(ImageFormat::JPEG));
}

#[test]
fn test_alt_v_paste_image_from_clipboard_windows() {
    let session = InteractiveSession::new();
    let clipboard = MockClipboard::new();
    clipboard.set_image_data(vec![0x89, 0x50, 0x4E, 0x47]); // PNG header

    session.handle_keyboard_shortcut_with_clipboard(
        KeyboardShortcut::AltV,
        &clipboard
    );

    assert_eq!(session.has_pending_image(), true);
    assert_eq!(session.pending_image_format(), Some(ImageFormat::PNG));
}

#[test]
fn test_up_arrow_navigates_command_history_backward() {
    let history = CommandHistory::new();
    history.add_command("first");
    history.add_command("second");
    history.add_command("third");

    let result = history.handle_keyboard_shortcut(KeyboardShortcut::UpArrow);
    assert_eq!(result, Some("third".to_string()));

    let result = history.handle_keyboard_shortcut(KeyboardShortcut::UpArrow);
    assert_eq!(result, Some("second".to_string()));

    let result = history.handle_keyboard_shortcut(KeyboardShortcut::UpArrow);
    assert_eq!(result, Some("first".to_string()));

    let result = history.handle_keyboard_shortcut(KeyboardShortcut::UpArrow);
    assert_eq!(result, None); // At beginning
}

#[test]
fn test_down_arrow_navigates_command_history_forward() {
    let history = CommandHistory::new();
    history.add_command("first");
    history.add_command("second");
    history.add_command("third");

    // Navigate backward first
    history.handle_keyboard_shortcut(KeyboardShortcut::UpArrow);
    history.handle_keyboard_shortcut(KeyboardShortcut::UpArrow);
    history.handle_keyboard_shortcut(KeyboardShortcut::UpArrow);

    // Now navigate forward
    let result = history.handle_keyboard_shortcut(KeyboardShortcut::DownArrow);
    assert_eq!(result, Some("second".to_string()));

    let result = history.handle_keyboard_shortcut(KeyboardShortcut::DownArrow);
    assert_eq!(result, Some("third".to_string()));

    let result = history.handle_keyboard_shortcut(KeyboardShortcut::DownArrow);
    assert_eq!(result, None); // At end
}

#[test]
fn test_esc_esc_rewinds_conversation() {
    let session = InteractiveSession::new();

    session.process_input("step 1").unwrap();
    session.process_input("step 2").unwrap();
    session.process_input("step 3").unwrap();
    session.process_input("step 4").unwrap();

    assert_eq!(session.conversation_turn_count(), 4);

    session.handle_keyboard_shortcut(KeyboardShortcut::EscEsc);

    // Should show rewind dialog/prompt
    assert_eq!(session.rewind_dialog_active(), true);
}

#[test]
fn test_esc_esc_rewind_to_selected_state() {
    let session = InteractiveSession::new();

    session.process_input("turn 1").unwrap();
    session.process_input("turn 2").unwrap();
    session.process_input("turn 3").unwrap();

    session.handle_keyboard_shortcut(KeyboardShortcut::EscEsc);
    session.select_rewind_point(1); // Rewind to turn 1

    assert_eq!(session.conversation_turn_count(), 1);
    let messages = session.get_conversation_messages();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].content.contains("turn 1"));
}

#[test]
fn test_tab_toggles_extended_thinking() {
    let session = InteractiveSession::new();
    assert_eq!(session.extended_thinking_enabled(), false);

    session.handle_keyboard_shortcut(KeyboardShortcut::Tab);
    assert_eq!(session.extended_thinking_enabled(), true);

    session.handle_keyboard_shortcut(KeyboardShortcut::Tab);
    assert_eq!(session.extended_thinking_enabled(), false);
}

#[test]
fn test_shift_tab_cycles_permission_modes() {
    let session = InteractiveSession::new();
    assert_eq!(session.permission_mode(), PermissionMode::Normal);

    session.handle_keyboard_shortcut(KeyboardShortcut::ShiftTab);
    assert_eq!(session.permission_mode(), PermissionMode::AutoAccept);

    session.handle_keyboard_shortcut(KeyboardShortcut::ShiftTab);
    assert_eq!(session.permission_mode(), PermissionMode::Plan);

    session.handle_keyboard_shortcut(KeyboardShortcut::ShiftTab);
    assert_eq!(session.permission_mode(), PermissionMode::Normal);
}

#[test]
fn test_alt_m_cycles_permission_modes() {
    let session = InteractiveSession::new();
    assert_eq!(session.permission_mode(), PermissionMode::Normal);

    session.handle_keyboard_shortcut(KeyboardShortcut::AltM);
    assert_eq!(session.permission_mode(), PermissionMode::AutoAccept);

    session.handle_keyboard_shortcut(KeyboardShortcut::AltM);
    assert_eq!(session.permission_mode(), PermissionMode::Plan);

    session.handle_keyboard_shortcut(KeyboardShortcut::AltM);
    assert_eq!(session.permission_mode(), PermissionMode::Normal);
}

// ============================================================================
// UNIT TESTS: Multiline Input Methods
// ============================================================================

#[test]
fn test_backslash_enter_creates_multiline_input() {
    let input_handler = InputHandler::new();

    input_handler.type_text("first line \\");
    input_handler.press_key(Key::Enter);
    input_handler.type_text("second line");

    assert_eq!(input_handler.is_multiline(), true);
    assert_eq!(input_handler.line_count(), 2);
    assert!(input_handler.get_text().contains("first line"));
    assert!(input_handler.get_text().contains("second line"));
}

#[test]
fn test_backslash_enter_works_on_all_terminals() {
    // This should work universally across bash, zsh, fish, etc.
    let terminals = vec!["bash", "zsh", "fish", "pwsh"];

    for terminal in terminals {
        let input_handler = InputHandler::new_for_terminal(terminal);

        input_handler.type_text("line 1 \\");
        input_handler.press_key(Key::Enter);
        input_handler.type_text("line 2");

        assert_eq!(input_handler.is_multiline(), true, "Failed for {}", terminal);
    }
}

#[test]
fn test_option_enter_multiline_macos() {
    let input_handler = InputHandler::new_for_platform(Platform::MacOS);

    input_handler.type_text("first line");
    input_handler.press_key_combination(&[Key::Option, Key::Enter]);
    input_handler.type_text("second line");

    assert_eq!(input_handler.is_multiline(), true);
    assert_eq!(input_handler.line_count(), 2);
}

#[test]
fn test_shift_enter_multiline_after_terminal_setup() {
    let session = InteractiveSession::new();

    // Configure terminal for Shift+Enter
    session.process_input("/terminal-setup").unwrap();

    let input_handler = session.get_input_handler();
    input_handler.type_text("line 1");
    input_handler.press_key_combination(&[Key::Shift, Key::Enter]);
    input_handler.type_text("line 2");

    assert_eq!(input_handler.is_multiline(), true);
}

#[test]
fn test_ctrl_j_line_feed_multiline() {
    let input_handler = InputHandler::new();

    input_handler.type_text("first");
    input_handler.press_key_combination(&[Key::Ctrl, Key::J]); // Line feed
    input_handler.type_text("second");

    assert_eq!(input_handler.is_multiline(), true);
    assert_eq!(input_handler.line_count(), 2);
}

#[test]
fn test_direct_paste_code_blocks() {
    let input_handler = InputHandler::new();
    let clipboard = MockClipboard::new();

    let code_block = "function test() {\n  console.log('hello');\n  return true;\n}";
    clipboard.set_text(code_block);

    input_handler.paste_from_clipboard(&clipboard);

    assert_eq!(input_handler.is_multiline(), true);
    assert_eq!(input_handler.line_count(), 4);
    assert!(input_handler.get_text().contains("function test()"));
}

#[test]
fn test_direct_paste_log_output() {
    let input_handler = InputHandler::new();
    let clipboard = MockClipboard::new();

    let log_output = "[ERROR] Connection failed\n[INFO] Retrying...\n[ERROR] Timeout";
    clipboard.set_text(log_output);

    input_handler.paste_from_clipboard(&clipboard);

    assert_eq!(input_handler.is_multiline(), true);
    assert!(input_handler.get_text().contains("[ERROR]"));
    assert!(input_handler.get_text().contains("[INFO]"));
}

// ============================================================================
// UNIT TESTS: Quick Command Prefixes
// ============================================================================

#[test]
fn test_hash_prefix_memory_shortcut() {
    let session = InteractiveSession::new();

    let parsed = session.process_input("#Remember this important pattern").unwrap();

    assert_eq!(parsed.input_type, InputType::MemoryShortcut);
    assert_eq!(parsed.content, "Remember this important pattern");
}

#[test]
fn test_hash_prefix_prompts_file_selection() {
    let session = InteractiveSession::new();

    session.process_input("#Save this for later").unwrap();

    // Should prompt user to select which file (CLAUDE.md by default)
    assert_eq!(session.file_selection_prompt_active(), true);
    assert_eq!(session.suggested_file(), Some("CLAUDE.md".to_string()));
}

#[test]
fn test_hash_prefix_appends_to_claude_md() {
    let session = InteractiveSession::new();
    let fs = MockFileSystem::new();
    fs.create_file("CLAUDE.md", "# Existing content\n");

    session.process_input("#New memory item").unwrap();
    session.confirm_file_selection("CLAUDE.md");

    let content = fs.read_file("CLAUDE.md");
    assert!(content.contains("Existing content"));
    assert!(content.contains("New memory item"));
}

#[test]
fn test_slash_prefix_accesses_slash_commands() {
    let session = InteractiveSession::new();

    let parsed = session.process_input("/clear").unwrap();

    assert_eq!(parsed.input_type, InputType::SlashCommand);
    assert_eq!(parsed.command_name, Some("clear".to_string()));
}

#[test]
fn test_slash_command_with_arguments() {
    let session = InteractiveSession::new();

    let parsed = session.process_input("/terminal-setup bash").unwrap();

    assert_eq!(parsed.input_type, InputType::SlashCommand);
    assert_eq!(parsed.command_name, Some("terminal-setup".to_string()));
    assert_eq!(parsed.arguments, vec!["bash"]);
}

#[test]
fn test_exclamation_prefix_bash_mode() {
    let session = InteractiveSession::new();

    let parsed = session.process_input("!ls -la").unwrap();

    assert_eq!(parsed.input_type, InputType::BashCommand);
    assert_eq!(parsed.content, "ls -la");
}

#[test]
fn test_bash_mode_executes_command_directly() {
    let session = InteractiveSession::new();

    session.process_input("!echo 'direct execution'").unwrap();
    let output = session.get_last_command_output();

    assert!(output.is_some());
    assert!(output.unwrap().contains("direct execution"));
}

#[test]
fn test_bash_mode_integrates_output_into_conversation() {
    let session = InteractiveSession::new();

    session.process_input("!date").unwrap();

    let conversation = session.get_conversation_context();
    let messages = conversation.get_all_messages();

    // Output should be part of conversation context
    assert!(messages.iter().any(|m| m.content.contains("date") || m.is_tool_output()));
}

#[test]
fn test_at_symbol_triggers_file_autocomplete() {
    let session = InteractiveSession::new();
    let fs = MockFileSystem::new();
    fs.create_file("src/main.rs", "");
    fs.create_file("src/lib.rs", "");
    fs.create_file("tests/test.rs", "");

    let input_handler = session.get_input_handler();
    input_handler.type_text("@src/");

    assert_eq!(input_handler.autocomplete_active(), true);
    let suggestions = input_handler.get_autocomplete_suggestions();
    assert!(suggestions.contains(&"src/main.rs".to_string()));
    assert!(suggestions.contains(&"src/lib.rs".to_string()));
}

#[test]
fn test_at_symbol_autocomplete_filters_as_typing() {
    let session = InteractiveSession::new();
    let fs = MockFileSystem::new();
    fs.create_file("src/main.rs", "");
    fs.create_file("src/lib.rs", "");
    fs.create_file("src/utils.rs", "");

    let input_handler = session.get_input_handler();
    input_handler.type_text("@src/m");

    let suggestions = input_handler.get_autocomplete_suggestions();
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0], "src/main.rs");
}

// ============================================================================
// UNIT TESTS: Vim Editor Mode
// ============================================================================

#[test]
fn test_vim_mode_activation_via_slash_command() {
    let session = InteractiveSession::new();
    assert_eq!(session.vim_mode_enabled(), false);

    session.process_input("/vim").unwrap();

    assert_eq!(session.vim_mode_enabled(), true);
    assert_eq!(session.vim_current_mode(), VimMode::Normal);
}

#[test]
fn test_vim_mode_activation_via_config() {
    let session = InteractiveSession::new();

    session.process_input("/config set editor vim").unwrap();

    assert_eq!(session.vim_mode_enabled(), true);
}

#[test]
fn test_vim_mode_permanent_via_config() {
    let config = SessionConfig::with_setting("editor.mode", "vim");
    config.save();

    // New session should have vim mode enabled
    let session = InteractiveSession::new_with_saved_config();
    assert_eq!(session.vim_mode_enabled(), true);
}

#[test]
fn test_vim_esc_enters_normal_mode() {
    let vim_input = VimInputHandler::new();
    vim_input.enter_insert_mode();
    assert_eq!(vim_input.current_mode(), VimMode::Insert);

    vim_input.press_key(Key::Esc);

    assert_eq!(vim_input.current_mode(), VimMode::Normal);
}

#[test]
fn test_vim_i_enters_insert_mode_at_cursor() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world");
    vim_input.move_cursor(6); // After "hello "

    vim_input.press_key(Key::Char('i'));

    assert_eq!(vim_input.current_mode(), VimMode::Insert);
    assert_eq!(vim_input.cursor_position(), 6);
}

#[test]
fn test_vim_capital_i_enters_insert_mode_at_line_start() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world");
    vim_input.move_cursor(6);

    vim_input.press_key(Key::Char('I'));

    assert_eq!(vim_input.current_mode(), VimMode::Insert);
    assert_eq!(vim_input.cursor_position(), 0);
}

#[test]
fn test_vim_a_enters_insert_mode_after_cursor() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello");
    vim_input.move_cursor(0); // At 'h'

    vim_input.press_key(Key::Char('a'));

    assert_eq!(vim_input.current_mode(), VimMode::Insert);
    assert_eq!(vim_input.cursor_position(), 1); // After 'h'
}

#[test]
fn test_vim_capital_a_enters_insert_mode_at_line_end() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello");
    vim_input.move_cursor(0);

    vim_input.press_key(Key::Char('A'));

    assert_eq!(vim_input.current_mode(), VimMode::Insert);
    assert_eq!(vim_input.cursor_position(), 5); // After "hello"
}

#[test]
fn test_vim_o_opens_line_below() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("line 1\nline 3");
    vim_input.move_cursor_to_line(0);

    vim_input.press_key(Key::Char('o'));

    assert_eq!(vim_input.current_mode(), VimMode::Insert);
    assert_eq!(vim_input.current_line_number(), 1);
    assert_eq!(vim_input.line_count(), 3);
}

#[test]
fn test_vim_capital_o_opens_line_above() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("line 1\nline 2");
    vim_input.move_cursor_to_line(1);

    vim_input.press_key(Key::Char('O'));

    assert_eq!(vim_input.current_mode(), VimMode::Insert);
    assert_eq!(vim_input.current_line_number(), 1);
    assert_eq!(vim_input.line_count(), 3);
}

#[test]
fn test_vim_hjkl_navigation() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("abc\ndef\nghi");
    vim_input.move_cursor_to_line(1); // "def"
    vim_input.move_cursor(1); // At 'e'

    // h - left
    vim_input.press_key(Key::Char('h'));
    assert_eq!(vim_input.cursor_column(), 0);

    // l - right
    vim_input.press_key(Key::Char('l'));
    assert_eq!(vim_input.cursor_column(), 1);

    // j - down
    vim_input.press_key(Key::Char('j'));
    assert_eq!(vim_input.current_line_number(), 2);

    // k - up
    vim_input.press_key(Key::Char('k'));
    assert_eq!(vim_input.current_line_number(), 1);
}

#[test]
fn test_vim_w_word_forward() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world test");
    vim_input.move_cursor(0); // At 'h'

    vim_input.press_key(Key::Char('w'));
    assert_eq!(vim_input.cursor_position(), 6); // At 'w' in "world"

    vim_input.press_key(Key::Char('w'));
    assert_eq!(vim_input.cursor_position(), 12); // At 't' in "test"
}

#[test]
fn test_vim_e_word_end() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world");
    vim_input.move_cursor(0);

    vim_input.press_key(Key::Char('e'));
    assert_eq!(vim_input.cursor_position(), 4); // At 'o' in "hello"

    vim_input.press_key(Key::Char('e'));
    assert_eq!(vim_input.cursor_position(), 10); // At 'd' in "world"
}

#[test]
fn test_vim_b_word_backward() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world test");
    vim_input.move_cursor(12); // At 't' in "test"

    vim_input.press_key(Key::Char('b'));
    assert_eq!(vim_input.cursor_position(), 6); // At 'w' in "world"

    vim_input.press_key(Key::Char('b'));
    assert_eq!(vim_input.cursor_position(), 0); // At 'h' in "hello"
}

#[test]
fn test_vim_0_line_start() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("  hello world");
    vim_input.move_cursor(10);

    vim_input.press_key(Key::Char('0'));

    assert_eq!(vim_input.cursor_column(), 0);
}

#[test]
fn test_vim_dollar_line_end() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world");
    vim_input.move_cursor(0);

    vim_input.press_key(Key::Char('$'));

    assert_eq!(vim_input.cursor_position(), 10); // Last char
}

#[test]
fn test_vim_gg_document_start() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("line 1\nline 2\nline 3\nline 4");
    vim_input.move_cursor_to_line(3);

    vim_input.press_key(Key::Char('g'));
    vim_input.press_key(Key::Char('g'));

    assert_eq!(vim_input.current_line_number(), 0);
}

#[test]
fn test_vim_capital_g_document_end() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("line 1\nline 2\nline 3");
    vim_input.move_cursor_to_line(0);

    vim_input.press_key(Key::Char('G'));

    assert_eq!(vim_input.current_line_number(), 2);
}

#[test]
fn test_vim_x_delete_character() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello");
    vim_input.move_cursor(1); // At 'e'

    vim_input.press_key(Key::Char('x'));

    assert_eq!(vim_input.get_text(), "hllo");
    assert_eq!(vim_input.cursor_position(), 1);
}

#[test]
fn test_vim_dd_delete_line() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("line 1\nline 2\nline 3");
    vim_input.move_cursor_to_line(1);

    vim_input.press_key(Key::Char('d'));
    vim_input.press_key(Key::Char('d'));

    assert_eq!(vim_input.get_text(), "line 1\nline 3");
    assert_eq!(vim_input.line_count(), 2);
}

#[test]
fn test_vim_capital_d_delete_to_end_of_line() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world");
    vim_input.move_cursor(6); // At 'w'

    vim_input.press_key(Key::Char('D'));

    assert_eq!(vim_input.get_text(), "hello ");
}

#[test]
fn test_vim_dw_delete_word() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world test");
    vim_input.move_cursor(0);

    vim_input.press_key(Key::Char('d'));
    vim_input.press_key(Key::Char('w'));

    assert_eq!(vim_input.get_text(), "world test");
}

#[test]
fn test_vim_de_delete_to_end_of_word() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world");
    vim_input.move_cursor(0);

    vim_input.press_key(Key::Char('d'));
    vim_input.press_key(Key::Char('e'));

    assert_eq!(vim_input.get_text(), " world");
}

#[test]
fn test_vim_db_delete_word_backward() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world");
    vim_input.move_cursor(10); // At 'd'

    vim_input.press_key(Key::Char('d'));
    vim_input.press_key(Key::Char('b'));

    assert_eq!(vim_input.get_text(), "hello d");
}

#[test]
fn test_vim_cc_change_line() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("old line");
    vim_input.move_cursor(0);

    vim_input.press_key(Key::Char('c'));
    vim_input.press_key(Key::Char('c'));

    assert_eq!(vim_input.current_mode(), VimMode::Insert);
    assert_eq!(vim_input.get_text(), "");
}

#[test]
fn test_vim_capital_c_change_to_end_of_line() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world");
    vim_input.move_cursor(6);

    vim_input.press_key(Key::Char('C'));

    assert_eq!(vim_input.current_mode(), VimMode::Insert);
    assert_eq!(vim_input.get_text(), "hello ");
}

#[test]
fn test_vim_cw_change_word() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("hello world");
    vim_input.move_cursor(0);

    vim_input.press_key(Key::Char('c'));
    vim_input.press_key(Key::Char('w'));

    assert_eq!(vim_input.current_mode(), VimMode::Insert);
    assert!(vim_input.get_text().starts_with(" "));
}

#[test]
fn test_vim_dot_repeats_last_edit() {
    let vim_input = VimInputHandler::new();
    vim_input.set_text("test test test");
    vim_input.move_cursor(0);

    // Delete first word
    vim_input.press_key(Key::Char('d'));
    vim_input.press_key(Key::Char('w'));
    assert_eq!(vim_input.get_text(), "test test");

    // Repeat with .
    vim_input.press_key(Key::Char('.'));
    assert_eq!(vim_input.get_text(), "test");
}

// ============================================================================
// UNIT TESTS: Command History Features
// ============================================================================

#[test]
fn test_history_stored_per_working_directory() {
    let history1 = CommandHistory::new_in_dir("/project1");
    let history2 = CommandHistory::new_in_dir("/project2");

    history1.add_command("command for project1");
    history2.add_command("command for project2");

    assert_eq!(history1.get_commands().len(), 1);
    assert_eq!(history2.get_commands().len(), 1);
    assert!(history1.get_commands()[0].contains("project1"));
    assert!(history2.get_commands()[0].contains("project2"));
}

#[test]
fn test_history_persists_across_sessions() {
    let dir = "/test/project";
    let storage = MockHistoryStorage::new();

    {
        let history = CommandHistory::new_in_dir_with_storage(dir, &storage);
        history.add_command("persistent command");
        history.save();
    }

    // New session in same directory
    let history = CommandHistory::new_in_dir_with_storage(dir, &storage);
    history.load();

    assert!(history.get_commands().contains(&"persistent command".to_string()));
}

#[test]
fn test_history_cleared_with_slash_clear() {
    let session = InteractiveSession::new();
    let history = session.get_command_history();

    history.add_command("command 1");
    history.add_command("command 2");
    history.add_command("command 3");

    assert_eq!(history.get_commands().len(), 3);

    session.process_input("/clear").unwrap();

    assert_eq!(history.get_commands().len(), 0);
}

#[test]
fn test_history_expansion_disabled_by_default() {
    let session = InteractiveSession::new();

    // History expansion (!!) should not work by default
    let parsed = session.process_input("!!").unwrap();

    // Should be treated as literal text, not expansion
    assert_eq!(parsed.content, "!!");
    assert_eq!(parsed.input_type, InputType::BashCommand);
}

// ============================================================================
// INTEGRATION TESTS: Background Bash Commands
// ============================================================================

#[test]
fn test_background_command_async_execution() {
    let session = InteractiveSession::new();

    let task_id = session.execute_background_command("sleep 10").unwrap();

    assert!(task_id.len() > 0);
    assert_eq!(session.is_task_running(&task_id), true);
    assert_eq!(session.status(), SessionStatus::Active); // Session not blocked
}

#[test]
fn test_background_command_continues_while_responding() {
    let session = InteractiveSession::new();

    let task_id = session.execute_background_command("find / -name '*.txt'").unwrap();

    // Session should handle new input while background task runs
    let result = session.process_input("What is Rust?").unwrap();
    assert!(result.is_ok());
    assert_eq!(session.is_task_running(&task_id), true);
}

#[test]
fn test_ctrl_b_moves_command_to_background() {
    let session = InteractiveSession::new();

    session.start_command("cargo build");
    assert_eq!(session.command_in_progress(), true);

    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlB);

    let task_id = session.get_last_background_task_id();
    assert!(task_id.is_some());
    assert_eq!(session.is_task_running(&task_id.unwrap()), true);
    assert_eq!(session.command_in_progress(), false);
}

#[test]
fn test_ctrl_b_twice_for_tmux_users() {
    let session = InteractiveSession::new();
    session.configure_for_tmux(true);

    session.start_command("npm install");

    // First Ctrl+B should be ignored (for tmux)
    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlB);
    assert_eq!(session.command_in_progress(), true);

    // Second Ctrl+B should background the command
    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlB);
    assert!(session.get_last_background_task_id().is_some());
}

#[test]
fn test_background_task_has_unique_id() {
    let session = InteractiveSession::new();

    let task1 = session.execute_background_command("task 1").unwrap();
    let task2 = session.execute_background_command("task 2").unwrap();
    let task3 = session.execute_background_command("task 3").unwrap();

    assert_ne!(task1, task2);
    assert_ne!(task2, task3);
    assert_ne!(task1, task3);
}

#[test]
fn test_background_task_output_retrieval_via_bash_output_tool() {
    let session = InteractiveSession::new();

    let task_id = session.execute_background_command("echo 'background output'").unwrap();

    // Wait for task to complete (in real impl would be async)
    std::thread::sleep(std::time::Duration::from_millis(100));

    let output = session.get_task_output(&task_id);
    assert!(output.is_some());
    assert!(output.unwrap().contains("background output"));
}

#[test]
fn test_background_task_output_buffered() {
    let session = InteractiveSession::new();
    let task_id = session.execute_background_command("long running").unwrap();

    // Simulate partial output
    session.buffer_task_output(&task_id, "chunk 1\n");
    session.buffer_task_output(&task_id, "chunk 2\n");
    session.buffer_task_output(&task_id, "chunk 3\n");

    let output = session.get_task_output(&task_id);
    assert!(output.is_some());
    let out = output.unwrap();
    assert!(out.contains("chunk 1"));
    assert!(out.contains("chunk 2"));
    assert!(out.contains("chunk 3"));
}

#[test]
fn test_background_tasks_cleaned_on_exit() {
    let session = InteractiveSession::new();

    let _task1 = session.execute_background_command("task 1").unwrap();
    let _task2 = session.execute_background_command("task 2").unwrap();

    assert_eq!(session.active_background_tasks().len(), 2);

    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlD); // Exit

    assert_eq!(session.status(), SessionStatus::Closed);
    assert_eq!(session.active_background_tasks().len(), 0);
}

#[test]
fn test_background_ideal_for_build_tools() {
    let session = InteractiveSession::new();

    let commands = vec![
        "cargo build --release",
        "npm run build",
        "make all",
        "gradle build",
    ];

    for cmd in commands {
        let task_id = session.execute_background_command(cmd).unwrap();
        assert!(session.is_task_running(&task_id));
    }
}

#[test]
fn test_background_ideal_for_package_managers() {
    let session = InteractiveSession::new();

    let task = session.execute_background_command("npm install").unwrap();

    assert!(session.is_task_running(&task));

    // Can continue working
    session.process_input("explain package.json").unwrap();
    assert_eq!(session.status(), SessionStatus::Active);
}

#[test]
fn test_background_ideal_for_test_runners() {
    let session = InteractiveSession::new();

    let task = session.execute_background_command("cargo test --all").unwrap();

    assert!(session.is_task_running(&task));
}

#[test]
fn test_background_ideal_for_dev_servers() {
    let session = InteractiveSession::new();

    let task = session.execute_background_command("npm run dev").unwrap();

    assert!(session.is_task_running(&task));

    // Server runs in background while we work
    session.process_input("update the homepage").unwrap();
}

// ============================================================================
// UNIT TESTS: Configuration & Help Commands
// ============================================================================

#[test]
fn test_question_mark_displays_shortcuts() {
    let session = InteractiveSession::new();

    session.process_input("?").unwrap();

    let output = session.get_last_output();
    assert!(output.contains_shortcut_help());
    assert!(output.contains("Ctrl+C"));
    assert!(output.contains("Ctrl+D"));
    assert!(output.contains("Ctrl+L"));
}

#[test]
fn test_question_mark_shows_environment_specific_shortcuts() {
    let session_mac = InteractiveSession::new_for_platform(Platform::MacOS);
    session_mac.process_input("?").unwrap();
    let output_mac = session_mac.get_last_output();
    assert!(output_mac.contains("Option+Enter"));

    let session_win = InteractiveSession::new_for_platform(Platform::Windows);
    session_win.process_input("?").unwrap();
    let output_win = session_win.get_last_output();
    assert!(output_win.contains("Alt+V"));
}

#[test]
fn test_terminal_setup_command() {
    let session = InteractiveSession::new();

    session.process_input("/terminal-setup").unwrap();

    assert!(session.terminal_setup_dialog_shown());
}

#[test]
fn test_terminal_setup_configures_line_breaks() {
    let session = InteractiveSession::new();

    session.process_input("/terminal-setup").unwrap();
    session.select_terminal_option("bash");

    // Shift+Enter should now work for multiline
    let handler = session.get_input_handler();
    handler.press_key_combination(&[Key::Shift, Key::Enter]);
    assert!(handler.multiline_break_configured());
}

#[test]
fn test_config_command_shows_settings() {
    let session = InteractiveSession::new();

    session.process_input("/config").unwrap();

    let output = session.get_last_output();
    assert!(output.contains_config_options());
}

#[test]
fn test_config_command_sets_permanent_settings() {
    let session = InteractiveSession::new();

    session.process_input("/config set editor vim").unwrap();

    let config = session.get_config();
    assert_eq!(config.get("editor"), Some("vim".to_string()));
}

#[test]
fn test_config_permanent_across_sessions() {
    let config_file = TempConfigFile::new();

    {
        let session = InteractiveSession::new_with_config_file(&config_file);
        session.process_input("/config set theme dark").unwrap();
    }

    // New session
    let session = InteractiveSession::new_with_config_file(&config_file);
    let config = session.get_config();
    assert_eq!(config.get("theme"), Some("dark".to_string()));
}

#[test]
fn test_clear_command_removes_history() {
    let session = InteractiveSession::new();

    session.process_input("message 1").unwrap();
    session.process_input("message 2").unwrap();
    session.process_input("message 3").unwrap();

    let history = session.get_command_history();
    assert_eq!(history.get_commands().len(), 3);

    session.process_input("/clear").unwrap();

    assert_eq!(history.get_commands().len(), 0);
}

// ============================================================================
// INTEGRATION TESTS: Complex Feature Interactions
// ============================================================================

#[test]
fn test_multiline_input_with_bash_command() {
    let session = InteractiveSession::new();

    session.process_input("!echo 'line 1' \\\n&& echo 'line 2' \\\n&& echo 'line 3'").unwrap();

    let output = session.get_last_command_output();
    assert!(output.is_some());
    let out = output.unwrap();
    assert!(out.contains("line 1"));
    assert!(out.contains("line 2"));
    assert!(out.contains("line 3"));
}

#[test]
fn test_vim_mode_with_multiline_editing() {
    let session = InteractiveSession::new();
    session.process_input("/vim").unwrap();

    let vim_input = session.get_vim_input_handler();
    vim_input.enter_insert_mode();
    vim_input.type_text("line 1");
    vim_input.press_key(Key::Enter);
    vim_input.type_text("line 2");

    assert_eq!(vim_input.line_count(), 2);
}

#[test]
fn test_background_command_with_verbose_output() {
    let session = InteractiveSession::new();
    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlO); // Enable verbose

    let task_id = session.execute_background_command("ls -la").unwrap();

    let output = session.get_task_output(&task_id);
    assert!(output.is_some());
    // Verbose mode should show additional details
}

#[test]
fn test_rewind_preserves_vim_mode_setting() {
    let session = InteractiveSession::new();
    session.process_input("/vim").unwrap();
    session.process_input("step 1").unwrap();
    session.process_input("step 2").unwrap();

    session.handle_keyboard_shortcut(KeyboardShortcut::EscEsc);
    session.select_rewind_point(1);

    // Vim mode should still be enabled
    assert_eq!(session.vim_mode_enabled(), true);
}

#[test]
fn test_file_autocomplete_with_vim_mode() {
    let session = InteractiveSession::new();
    let fs = MockFileSystem::new();
    fs.create_file("test.rs", "");

    session.process_input("/vim").unwrap();
    let vim_input = session.get_vim_input_handler();
    vim_input.enter_insert_mode();
    vim_input.type_text("@test");

    assert!(vim_input.autocomplete_active());
}

// ============================================================================
// E2E TESTS: Complete Workflows
// ============================================================================

#[test]
fn test_full_interactive_session_with_all_features() {
    let session = InteractiveSession::new();

    // Enable vim mode
    session.process_input("/vim").unwrap();
    assert_eq!(session.vim_mode_enabled(), true);

    // Use memory shortcut
    session.process_input("#Remember to use TDD").unwrap();

    // Execute bash command
    session.process_input("!ls").unwrap();

    // Background task
    let task = session.execute_background_command("cargo test").unwrap();
    assert!(session.is_task_running(&task));

    // Enable verbose output
    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlO);
    assert_eq!(session.verbose_output_enabled(), true);

    // Toggle extended thinking
    session.handle_keyboard_shortcut(KeyboardShortcut::Tab);
    assert_eq!(session.extended_thinking_enabled(), true);

    // Navigate history
    let history = session.get_command_history();
    let prev = history.handle_keyboard_shortcut(KeyboardShortcut::UpArrow);
    assert!(prev.is_some());

    // Clear screen (preserves history)
    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlL);
    assert_eq!(session.screen_is_clear(), true);
    assert!(session.history_len() > 0);

    // Exit cleanly
    session.handle_keyboard_shortcut(KeyboardShortcut::CtrlD);
    assert_eq!(session.status(), SessionStatus::Closed);
    assert_eq!(session.active_background_tasks().len(), 0);
}

#[test]
fn test_developer_workflow_with_background_builds() {
    let session = InteractiveSession::new();

    // Start a long build
    let build_task = session.execute_background_command("cargo build --release").unwrap();

    // Continue working
    session.process_input("explain the main function").unwrap();
    session.process_input("how can I optimize this?").unwrap();

    // Check build output
    let output = session.get_task_output(&build_task);
    assert!(output.is_some());

    // Start tests in background
    let _test_task = session.execute_background_command("cargo test").unwrap();

    // Keep working
    session.process_input("refactor this function").unwrap();

    // Both tasks should be tracked
    assert_eq!(session.active_background_tasks().len(), 2);
}

// ============================================================================
// MOCK TYPES FOR TESTING
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum KeyboardShortcut {
    CtrlC,
    CtrlD,
    CtrlL,
    CtrlO,
    CtrlR,
    CtrlV,
    AltV,
    CtrlB,
    UpArrow,
    DownArrow,
    EscEsc,
    Tab,
    ShiftTab,
    AltM,
}

#[derive(Debug, Clone, PartialEq)]
enum Key {
    Esc,
    Enter,
    Tab,
    Char(char),
    Ctrl,
    Shift,
    Option,
    Alt,
    J,
}

#[derive(Debug, Clone, PartialEq)]
enum SessionStatus {
    Active,
    Closed,
}

#[derive(Debug, Clone, PartialEq)]
enum PermissionMode {
    Normal,
    AutoAccept,
    Plan,
}

#[derive(Debug, Clone, PartialEq)]
enum InputType {
    Prompt,
    BashCommand,
    SlashCommand,
    MemoryShortcut,
    FileMention,
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
enum VimMode {
    Normal,
    Insert,
    Visual,
}

#[derive(Debug, Clone, PartialEq)]
enum ImageFormat {
    PNG,
    JPEG,
    GIF,
}

#[derive(Debug, Clone, PartialEq)]
enum Platform {
    MacOS,
    Linux,
    Windows,
}

#[derive(Clone)]
struct ParsedInput {
    input_type: InputType,
    content: String,
    command_name: Option<String>,
    arguments: Vec<String>,
}

impl ParsedInput {
    fn is_ok(&self) -> bool {
        true
    }
}

struct InteractiveSession {
    // Mock implementation - real impl would have actual fields
}

impl InteractiveSession {
    fn new() -> Self {
        Self {}
    }

    fn new_for_platform(_platform: Platform) -> Self {
        Self {}
    }

    fn new_with_config_file(_config: &TempConfigFile) -> Self {
        Self {}
    }

    fn new_with_saved_config() -> Self {
        Self {}
    }

    fn start_input(&self, _input: &str) {}
    fn input_in_progress(&self) -> bool { false }
    fn last_input_cancelled(&self) -> bool { false }
    fn process_input(&self, _input: &str) -> Result<ParsedInput, String> {
        Ok(ParsedInput {
            input_type: InputType::Prompt,
            content: String::new(),
            command_name: None,
            arguments: vec![],
        })
    }
    fn start_generation(&self) {}
    fn generation_in_progress(&self) -> bool { false }
    fn generation_cancelled(&self) -> bool { false }
    fn status(&self) -> SessionStatus { SessionStatus::Active }
    fn handle_keyboard_shortcut(&self, _shortcut: KeyboardShortcut) {}
    fn handle_keyboard_shortcut_with_clipboard(&self, _shortcut: KeyboardShortcut, _clipboard: &MockClipboard) {}
    fn history_len(&self) -> usize { 0 }
    fn screen_is_clear(&self) -> bool { false }
    fn conversation_turn_count(&self) -> usize { 0 }
    fn verbose_output_enabled(&self) -> bool { false }
    fn get_last_output(&self) -> SessionOutput { SessionOutput {} }
    fn rewind_dialog_active(&self) -> bool { false }
    fn select_rewind_point(&self, _turn: usize) {}
    fn get_conversation_messages(&self) -> Vec<ConversationMessage> { vec![] }
    fn extended_thinking_enabled(&self) -> bool { false }
    fn permission_mode(&self) -> PermissionMode { PermissionMode::Normal }
    fn get_input_handler(&self) -> InputHandler { InputHandler {} }
    fn file_selection_prompt_active(&self) -> bool { false }
    fn suggested_file(&self) -> Option<String> { None }
    fn confirm_file_selection(&self, _file: &str) {}
    fn has_pending_image(&self) -> bool { false }
    fn pending_image_format(&self) -> Option<ImageFormat> { None }
    fn get_command_history(&self) -> CommandHistory { CommandHistory::new() }
    fn vim_mode_enabled(&self) -> bool { false }
    fn vim_current_mode(&self) -> VimMode { VimMode::Normal }
    fn get_vim_input_handler(&self) -> VimInputHandler { VimInputHandler {} }
    fn terminal_setup_dialog_shown(&self) -> bool { false }
    fn select_terminal_option(&self, _option: &str) {}
    fn get_config(&self) -> SessionConfig { SessionConfig {} }
    fn execute_background_command(&self, _cmd: &str) -> Result<String, String> {
        Ok("task-123".to_string())
    }
    fn is_task_running(&self, _task_id: &str) -> bool { false }
    fn get_last_background_task_id(&self) -> Option<String> { None }
    fn command_in_progress(&self) -> bool { false }
    fn start_command(&self, _cmd: &str) {}
    fn configure_for_tmux(&self, _enabled: bool) {}
    fn get_task_output(&self, _task_id: &str) -> Option<String> { None }
    fn buffer_task_output(&self, _task_id: &str, _chunk: &str) {}
    fn active_background_tasks(&self) -> Vec<String> { vec![] }
    fn get_last_command_output(&self) -> Option<String> { None }
    fn get_conversation_context(&self) -> ConversationContext { ConversationContext {} }
}

struct CommandHistory {}

impl CommandHistory {
    fn new() -> Self { Self {} }
    fn new_in_dir(_dir: &str) -> Self { Self {} }
    fn new_in_dir_with_storage(_dir: &str, _storage: &MockHistoryStorage) -> Self { Self {} }
    fn add_command(&self, _cmd: &str) {}
    fn handle_keyboard_shortcut(&self, _shortcut: KeyboardShortcut) -> Option<String> { None }
    fn reverse_search_active(&self) -> bool { false }
    fn search_prompt_displayed(&self) -> bool { false }
    fn type_search_term(&self, _term: &str) {}
    fn get_search_results(&self) -> Vec<SearchResult> { vec![] }
    fn current_match_index(&self) -> usize { 0 }
    fn get_commands(&self) -> Vec<String> { vec![] }
    fn save(&self) {}
    fn load(&self) {}
}

struct InputHandler {}

impl InputHandler {
    fn new() -> Self { Self {} }
    fn new_for_terminal(_terminal: &str) -> Self { Self {} }
    fn new_for_platform(_platform: Platform) -> Self { Self {} }
    fn type_text(&self, _text: &str) {}
    fn press_key(&self, _key: Key) {}
    fn press_key_combination(&self, _keys: &[Key]) {}
    fn is_multiline(&self) -> bool { false }
    fn line_count(&self) -> usize { 0 }
    fn get_text(&self) -> String { String::new() }
    fn paste_from_clipboard(&self, _clipboard: &MockClipboard) {}
    fn autocomplete_active(&self) -> bool { false }
    fn get_autocomplete_suggestions(&self) -> Vec<String> { vec![] }
    fn multiline_break_configured(&self) -> bool { false }
}

struct VimInputHandler {}

impl VimInputHandler {
    fn new() -> Self { Self {} }
    fn enter_insert_mode(&self) {}
    fn current_mode(&self) -> VimMode { VimMode::Normal }
    fn set_text(&self, _text: &str) {}
    fn move_cursor(&self, _pos: usize) {}
    fn press_key(&self, _key: Key) {}
    fn cursor_position(&self) -> usize { 0 }
    fn cursor_column(&self) -> usize { 0 }
    fn current_line_number(&self) -> usize { 0 }
    fn line_count(&self) -> usize { 0 }
    fn move_cursor_to_line(&self, _line: usize) {}
    fn get_text(&self) -> String { String::new() }
    fn type_text(&self, _text: &str) {}
    fn autocomplete_active(&self) -> bool { false }
}

struct MockClipboard {}

impl MockClipboard {
    fn new() -> Self { Self {} }
    fn set_image_data(&self, _data: Vec<u8>) {}
    fn set_text(&self, _text: &str) {}
}

struct MockFileSystem {}

impl MockFileSystem {
    fn new() -> Self { Self {} }
    fn create_file(&self, _path: &str, _content: &str) {}
    fn read_file(&self, _path: &str) -> String { String::new() }
}

struct SessionOutput {}

impl SessionOutput {
    fn contains_tool_details(&self) -> bool { false }
    fn contains(&self, _text: &str) -> bool { false }
    fn contains_shortcut_help(&self) -> bool { false }
    fn contains_config_options(&self) -> bool { false }
}

struct SearchResult {}

impl SearchResult {
    fn has_highlighting(&self) -> bool { false }
    fn highlighted_term(&self) -> &str { "" }
}

struct ConversationMessage {
    content: String,
}

impl ConversationMessage {
    fn is_tool_output(&self) -> bool { false }
}

struct ConversationContext {}

impl ConversationContext {
    fn get_all_messages(&self) -> Vec<ConversationMessage> { vec![] }
}

struct SessionConfig {}

impl SessionConfig {
    fn get(&self, _key: &str) -> Option<String> { None }
    fn with_setting(_key: &str, _value: &str) -> Self { Self {} }
    fn save(&self) {}
}

struct TempConfigFile {}

impl TempConfigFile {
    fn new() -> Self { Self {} }
}

struct MockHistoryStorage {}

impl MockHistoryStorage {
    fn new() -> Self { Self {} }
}
