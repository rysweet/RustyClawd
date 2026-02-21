//! TUI App Regression Test Suite (Pre-Refactor Baseline)
//!
//! This suite validates the App state machine BEFORE refactoring begins.
//! Any breakage during extraction will be caught immediately.
//!
//! Tests cover the full public API surface of App:
//! - Initialization defaults
//! - Text input (insert, backspace, clear, set)
//! - Input submission
//! - Streaming lifecycle (start, append, finish)
//! - Tool execution (begin, finalize)
//! - Scrolling (up, down, follow-bottom)
//! - Debug panel (toggle, push, clear)
//! - Modals (memory, permissions)
//! - Autocomplete (activate, nav, select, clear)
//! - Dirty flag tracking
//! - Message operations (add, list)
//! - Event handling (Ctrl+C, Enter)

use rustyclawd::permission_mode::PermissionMode;
use rustyclawd::tui::{App, CompletionItem, EventResult, MemoryDestination, Message};

// ============================================================
// 1. App Initialization
// ============================================================

#[test]
fn test_app_new_defaults() {
    let app = App::new(PermissionMode::default());

    assert!(app.messages().is_empty(), "messages should start empty");
    assert_eq!(app.input(), "", "input buffer should start empty");
    assert_eq!(app.cursor_pos(), (0, 0), "cursor should be at origin");
    assert!(!app.should_exit(), "should_exit should be false");
    assert!(app.error().is_none(), "no error on init");
    assert!(!app.is_streaming(), "not streaming on init");
    assert!(app.is_dirty(), "app starts dirty for initial render");
    assert!(!app.debug_visible(), "debug panel hidden by default");
    assert!(!app.menu_open(), "menu closed by default");
    assert_eq!(
        app.permission_mode(),
        PermissionMode::Ask,
        "default permission mode is Ask"
    );
    assert!(app.follow_bottom(), "scroll follows bottom by default");
    assert_eq!(app.scroll_offset(), 0, "scroll offset starts at 0");
    assert!(!app.autocomplete_active(), "no autocomplete on init");
    assert!(!app.memory_modal_active(), "no memory modal on init");
    assert!(
        !app.permissions_modal_active(),
        "no permissions modal on init"
    );
    assert!(app.mouse_mode_enabled(), "mouse mode enabled by default");
    assert_eq!(app.input_line_count(), 1, "single line on init");
    assert!(!app.has_multi_line_input(), "not multi-line on init");
}

#[test]
fn test_app_new_with_auto_accept() {
    let app = App::new(PermissionMode::AutoAccept);
    assert_eq!(app.permission_mode(), PermissionMode::AutoAccept);
}

#[test]
fn test_app_new_with_plan() {
    let app = App::new(PermissionMode::Plan);
    assert_eq!(app.permission_mode(), PermissionMode::Plan);
}

// ============================================================
// 2. Text Input
// ============================================================

#[test]
fn test_insert_char_basic() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char('h');
    app.insert_char('e');
    app.insert_char('l');
    app.insert_char('l');
    app.insert_char('o');
    assert_eq!(app.input(), "hello");
    assert_eq!(app.cursor_pos().1, 5);
}

#[test]
fn test_insert_char_unicode() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char('R');
    app.insert_char('u');
    app.insert_char('s');
    app.insert_char('t');
    assert_eq!(app.input(), "Rust");
}

#[test]
fn test_backspace_removes_last_char() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char('a');
    app.insert_char('b');
    app.insert_char('c');
    app.backspace();
    assert_eq!(app.input(), "ab");
}

#[test]
fn test_backspace_on_empty_input() {
    let mut app = App::new(PermissionMode::default());
    // Should not panic on empty input
    app.backspace();
    assert_eq!(app.input(), "");
}

#[test]
fn test_clear_input() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char('h');
    app.insert_char('i');
    assert_eq!(app.input(), "hi");

    app.clear_input();
    assert_eq!(app.input(), "");
    assert_eq!(app.cursor_pos(), (0, 0));
}

#[test]
fn test_set_input() {
    let mut app = App::new(PermissionMode::default());
    app.set_input("hello world");
    assert_eq!(app.input(), "hello world");
}

#[test]
fn test_set_input_overwrites_existing() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char('x');
    app.set_input("new text");
    assert_eq!(app.input(), "new text");
}

#[test]
fn test_insert_newline() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char('a');
    app.insert_newline();
    app.insert_char('b');
    assert_eq!(app.input(), "a\nb");
    assert_eq!(app.input_line_count(), 2);
    assert!(app.has_multi_line_input());
}

// ============================================================
// 3. Input Submission
// ============================================================

#[test]
fn test_submit_input_returns_content() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char('h');
    app.insert_char('i');

    let result = app.submit_input();
    assert_eq!(result, Some("hi".to_string()));
}

#[test]
fn test_submit_input_clears_buffer() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char('h');
    app.insert_char('i');

    let _ = app.submit_input();
    assert_eq!(app.input(), "");
    assert_eq!(app.cursor_pos(), (0, 0));
}

#[test]
fn test_submit_empty_returns_none() {
    let mut app = App::new(PermissionMode::default());
    assert_eq!(app.submit_input(), None);
}

#[test]
fn test_submit_whitespace_only_returns_none() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char(' ');
    app.insert_char(' ');
    assert_eq!(app.submit_input(), None);
}

#[test]
fn test_submit_multi_line_input() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char('a');
    app.insert_newline();
    app.insert_char('b');

    let result = app.submit_input();
    assert_eq!(result, Some("a\nb".to_string()));
}

// ============================================================
// 4. Streaming Lifecycle
// ============================================================

#[test]
fn test_start_streaming_response() {
    let mut app = App::new(PermissionMode::default());

    let idx = app.start_streaming_response();
    assert_eq!(idx, 0);
    assert!(app.is_streaming());
    assert!(app.is_thinking(), "should be thinking before first token");
    assert_eq!(app.messages().len(), 1);
    assert!(
        app.messages()[0].streaming,
        "message should be marked streaming"
    );
}

#[test]
fn test_append_streaming_content() {
    let mut app = App::new(PermissionMode::default());
    app.start_streaming_response();

    app.append_streaming_content("Hello");
    assert_eq!(app.messages()[0].content, "Hello");

    app.append_streaming_content(" world");
    assert_eq!(app.messages()[0].content, "Hello world");
}

#[test]
fn test_finish_streaming() {
    let mut app = App::new(PermissionMode::default());
    app.start_streaming_response();

    app.append_streaming_content("Complete response");
    app.finish_streaming();

    assert!(!app.is_streaming());
    assert_eq!(app.messages()[0].content, "Complete response");
    assert!(
        !app.messages()[0].streaming,
        "message should no longer be streaming"
    );
}

#[test]
fn test_streaming_index_with_prior_messages() {
    let mut app = App::new(PermissionMode::default());
    app.add_message(Message::user("question".to_string()));

    let idx = app.start_streaming_response();
    assert_eq!(
        idx, 1,
        "streaming index should be 1 after one existing message"
    );
    assert_eq!(app.messages().len(), 2);
}

#[test]
fn test_finish_streaming_without_start_is_noop() {
    let mut app = App::new(PermissionMode::default());
    app.finish_streaming(); // Should not panic
    assert!(!app.is_streaming());
}

#[test]
fn test_complete_last_streaming() {
    let mut app = App::new(PermissionMode::default());
    app.start_streaming_response();
    app.append_streaming_content("partial");
    app.complete_last_streaming();

    // Message should still exist, streaming is marked complete
    assert_eq!(app.messages()[0].content, "partial");
    assert!(!app.messages()[0].streaming);
}

#[test]
fn test_mark_last_message_error() {
    let mut app = App::new(PermissionMode::default());
    app.add_message(Message::assistant("oops".to_string()));
    app.mark_last_message_error();

    // MessageStatus is not re-exported, so verify via observable side effects:
    // mark_error clears the streaming flag
    assert!(!app.messages().last().unwrap().streaming);
}

#[test]
fn test_token_count_during_streaming() {
    let mut app = App::new(PermissionMode::default());

    // No token count when not streaming
    assert!(app.token_count().is_none());

    app.start_streaming_response();
    assert!(app.is_thinking());

    app.update_token_count(10, 5);
    let tc = app.token_count().unwrap();
    assert!(tc.total() > 0);
    assert!(
        !app.is_thinking(),
        "should stop thinking after tokens arrive"
    );
}

// ============================================================
// 5. Tool Execution
// ============================================================

#[test]
fn test_begin_tool_message() {
    let mut app = App::new(PermissionMode::default());

    let idx = app.begin_tool_message(
        "tool-1".to_string(),
        "Bash".to_string(),
        serde_json::json!({"command": "echo hello"}),
    );

    assert_eq!(idx, 0);
    assert_eq!(app.messages().len(), 1);
    assert!(app.has_active_tools());
    assert_eq!(app.active_tool_name(), Some("Bash".to_string()));
}

#[test]
fn test_finalize_tool_message() {
    let mut app = App::new(PermissionMode::default());

    app.begin_tool_message(
        "tool-1".to_string(),
        "Bash".to_string(),
        serde_json::json!({}),
    );

    let result = rustyclawd::tui::ToolResult {
        exit_code: Some(0),
        stdout: "output".to_string(),
        stderr: String::new(),
        is_error: false,
        raw_content: "output".to_string(),
        structured_content: None,
    };

    app.finalize_tool_message("tool-1", result);
    assert!(!app.has_active_tools(), "tool should be marked complete");
    assert!(app.active_tool_name().is_none());
}

#[test]
fn test_tool_message_by_index() {
    let mut app = App::new(PermissionMode::default());

    let idx = app.begin_tool_message(
        "tool-x".to_string(),
        "Write".to_string(),
        serde_json::json!({}),
    );

    let found = app.tool_message_by_index(idx);
    assert!(found.is_some());
    let (tool_id, state) = found.unwrap();
    assert_eq!(tool_id, "tool-x");
    assert_eq!(state.tool_name, "Write");
    assert!(!state.completed);
}

#[test]
fn test_get_tool_message_state() {
    let mut app = App::new(PermissionMode::default());

    app.begin_tool_message("t1".to_string(), "Edit".to_string(), serde_json::json!({}));

    assert!(app.get_tool_message_state("t1").is_some());
    assert!(app.get_tool_message_state("nonexistent").is_none());
}

#[test]
fn test_active_tool_messages_iterator() {
    let mut app = App::new(PermissionMode::default());

    app.begin_tool_message("t1".to_string(), "A".to_string(), serde_json::json!({}));
    app.begin_tool_message("t2".to_string(), "B".to_string(), serde_json::json!({}));

    let active: Vec<_> = app.active_tool_messages().collect();
    assert_eq!(active.len(), 2);

    // Finalize one
    let result = rustyclawd::tui::ToolResult {
        exit_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
        is_error: false,
        raw_content: String::new(),
        structured_content: None,
    };
    app.finalize_tool_message("t1", result);

    let active: Vec<_> = app.active_tool_messages().collect();
    assert_eq!(active.len(), 1);
}

#[test]
fn test_tool_error_result() {
    let mut app = App::new(PermissionMode::default());

    app.begin_tool_message(
        "t-err".to_string(),
        "Bash".to_string(),
        serde_json::json!({}),
    );

    let result = rustyclawd::tui::ToolResult {
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "command not found".to_string(),
        is_error: true,
        raw_content: "error".to_string(),
        structured_content: None,
    };
    app.finalize_tool_message("t-err", result);

    assert!(!app.has_active_tools());
    // The message should be marked as error
    let state = app.get_tool_message_state("t-err").unwrap();
    assert!(state.completed);
    assert!(state.result.as_ref().unwrap().is_error);
}

// ============================================================
// 6. Scrolling
// ============================================================

#[test]
fn test_scroll_up_from_bottom() {
    let mut app = App::new(PermissionMode::default());
    assert!(app.follow_bottom());

    app.scroll_up(5);
    assert!(!app.follow_bottom(), "scrolling up disables follow-bottom");
}

#[test]
fn test_scroll_down_returns_to_follow() {
    let mut app = App::new(PermissionMode::default());

    // Set max scroll so there is range to work with
    app.update_max_scroll(100);
    app.scroll_up(5);
    assert!(!app.follow_bottom());

    // Scroll down past the max should re-engage follow
    app.scroll_down(200);
    assert!(
        app.follow_bottom(),
        "scrolling past end re-engages follow-bottom"
    );
}

#[test]
fn test_scroll_to_bottom() {
    let mut app = App::new(PermissionMode::default());
    app.update_max_scroll(50);
    app.scroll_up(10);
    assert!(!app.follow_bottom());

    app.scroll_to_bottom();
    assert!(app.follow_bottom());
}

#[test]
fn test_scroll_down_noop_when_following() {
    let mut app = App::new(PermissionMode::default());
    assert!(app.follow_bottom());

    // Scroll down while already following should be a no-op
    app.scroll_down(10);
    assert!(app.follow_bottom());
    assert_eq!(app.scroll_offset(), 0);
}

#[test]
fn test_update_max_scroll() {
    let mut app = App::new(PermissionMode::default());
    app.update_max_scroll(200);

    // Scroll to a known position
    app.scroll_up(10);
    // Clamp: offset should be clamped when max_scroll changes to smaller value
    app.update_max_scroll(5);
    assert!(
        app.scroll_offset() <= 5,
        "offset should clamp to max_scroll"
    );
}

// ============================================================
// 7. Debug Panel
// ============================================================

#[test]
fn test_toggle_debug() {
    let mut app = App::new(PermissionMode::default());
    assert!(!app.debug_visible());

    app.toggle_debug();
    assert!(app.debug_visible());

    app.toggle_debug();
    assert!(!app.debug_visible());
}

#[test]
fn test_push_debug_message() {
    let mut app = App::new(PermissionMode::default());
    app.push_debug_message("test message".to_string());

    let msgs = app.debug_messages();
    assert!(
        msgs.iter().any(|m| m.contains("test message")),
        "debug messages should contain our message"
    );
}

#[test]
fn test_clear_debug_messages() {
    let mut app = App::new(PermissionMode::default());
    app.push_debug_message("first".to_string());
    app.push_debug_message("second".to_string());

    app.clear_debug_messages();
    assert!(app.debug_messages().is_empty());
}

#[test]
fn test_debug_scroll() {
    let mut app = App::new(PermissionMode::default());
    assert!(app.debug_follow_bottom());

    app.update_debug_max_scroll(100);
    app.scroll_debug_up(5);
    assert!(!app.debug_follow_bottom());

    app.scroll_debug_down(200);
    assert!(app.debug_follow_bottom());
}

// ============================================================
// 8. Modals
// ============================================================

#[test]
fn test_memory_modal_activate_and_clear() {
    let mut app = App::new(PermissionMode::default());
    assert!(!app.memory_modal_active());

    let destinations = vec![MemoryDestination {
        name: "User memory".to_string(),
        file_path: "/tmp/test.md".to_string(),
        description: Some("Test desc".to_string()),
        is_imported: false,
    }];

    app.activate_memory_modal("Remember this".to_string(), destinations);
    assert!(app.memory_modal_active());

    let modal = app.memory_modal().unwrap();
    assert_eq!(modal.memory_text, "Remember this");
    assert_eq!(modal.destinations.len(), 1);

    app.clear_memory_modal();
    assert!(!app.memory_modal_active());
}

#[test]
fn test_memory_modal_navigation() {
    let mut app = App::new(PermissionMode::default());

    let destinations = vec![
        MemoryDestination {
            name: "Dest A".to_string(),
            file_path: "/a".to_string(),
            description: None,
            is_imported: false,
        },
        MemoryDestination {
            name: "Dest B".to_string(),
            file_path: "/b".to_string(),
            description: None,
            is_imported: false,
        },
        MemoryDestination {
            name: "Dest C".to_string(),
            file_path: "/c".to_string(),
            description: None,
            is_imported: false,
        },
    ];

    app.activate_memory_modal("mem".to_string(), destinations);

    // Starts at index 0
    assert_eq!(app.memory_modal_selected().unwrap().name, "Dest A");

    app.memory_modal_next();
    assert_eq!(app.memory_modal_selected().unwrap().name, "Dest B");

    app.memory_modal_next();
    assert_eq!(app.memory_modal_selected().unwrap().name, "Dest C");

    // Wrap around to top
    app.memory_modal_next();
    assert_eq!(app.memory_modal_selected().unwrap().name, "Dest A");

    // Wrap around from top going up
    app.memory_modal_prev();
    assert_eq!(app.memory_modal_selected().unwrap().name, "Dest C");
}

#[test]
fn test_memory_modal_empty_destinations_does_not_activate() {
    let mut app = App::new(PermissionMode::default());
    app.activate_memory_modal("text".to_string(), vec![]);
    assert!(
        !app.memory_modal_active(),
        "empty destinations should not activate"
    );
}

#[test]
fn test_update_memory_text() {
    let mut app = App::new(PermissionMode::default());
    let destinations = vec![MemoryDestination {
        name: "D".to_string(),
        file_path: "/d".to_string(),
        description: None,
        is_imported: false,
    }];

    app.activate_memory_modal("original".to_string(), destinations);
    app.update_memory_text("updated".to_string());
    assert_eq!(app.memory_modal().unwrap().memory_text, "updated");
}

#[test]
fn test_permissions_modal_activate_and_clear() {
    let mut app = App::new(PermissionMode::default());
    assert!(!app.permissions_modal_active());

    app.activate_permissions_modal();
    assert!(app.permissions_modal_active());

    app.clear_permissions_modal();
    assert!(!app.permissions_modal_active());
}

// ============================================================
// 9. Autocomplete
// ============================================================

#[test]
fn test_autocomplete_activate() {
    let mut app = App::new(PermissionMode::default());
    assert!(!app.autocomplete_active());

    let items = vec![
        CompletionItem {
            command: "help".to_string(),
            description: Some("Show help".to_string()),
            argument_hint: None,
        },
        CompletionItem {
            command: "exit".to_string(),
            description: None,
            argument_hint: None,
        },
    ];

    app.activate_autocomplete(items);
    assert!(app.autocomplete_active());

    let ac = app.autocomplete().unwrap();
    assert_eq!(ac.items.len(), 2);
    assert_eq!(ac.selected, 0);
}

#[test]
fn test_autocomplete_navigation() {
    let mut app = App::new(PermissionMode::default());

    let items = vec![
        CompletionItem {
            command: "a".to_string(),
            description: None,
            argument_hint: None,
        },
        CompletionItem {
            command: "b".to_string(),
            description: None,
            argument_hint: None,
        },
        CompletionItem {
            command: "c".to_string(),
            description: None,
            argument_hint: None,
        },
    ];

    app.activate_autocomplete(items);

    // Starts at 0
    assert_eq!(app.autocomplete_selected().unwrap().command, "a");

    app.autocomplete_next();
    assert_eq!(app.autocomplete_selected().unwrap().command, "b");

    app.autocomplete_next();
    assert_eq!(app.autocomplete_selected().unwrap().command, "c");

    // Wrap to top
    app.autocomplete_next();
    assert_eq!(app.autocomplete_selected().unwrap().command, "a");

    // Wrap to bottom
    app.autocomplete_prev();
    assert_eq!(app.autocomplete_selected().unwrap().command, "c");
}

#[test]
fn test_autocomplete_clear() {
    let mut app = App::new(PermissionMode::default());

    let items = vec![CompletionItem {
        command: "test".to_string(),
        description: None,
        argument_hint: None,
    }];

    app.activate_autocomplete(items);
    assert!(app.autocomplete_active());

    app.clear_autocomplete();
    assert!(!app.autocomplete_active());
    assert!(app.autocomplete().is_none());
    assert!(app.autocomplete_selected().is_none());
}

#[test]
fn test_autocomplete_empty_items_does_not_activate() {
    let mut app = App::new(PermissionMode::default());
    app.activate_autocomplete(vec![]);
    assert!(!app.autocomplete_active());
}

#[test]
fn test_autocomplete_prev_on_inactive_is_noop() {
    let mut app = App::new(PermissionMode::default());
    app.autocomplete_prev(); // Should not panic
    app.autocomplete_next(); // Should not panic
}

// ============================================================
// 10. Dirty Flag
// ============================================================

#[test]
fn test_dirty_flag_on_init() {
    let app = App::new(PermissionMode::default());
    assert!(app.is_dirty(), "app starts dirty for initial render");
}

#[test]
fn test_clear_dirty() {
    let mut app = App::new(PermissionMode::default());
    app.clear_dirty();
    assert!(!app.is_dirty());
}

#[test]
fn test_insert_char_sets_dirty() {
    let mut app = App::new(PermissionMode::default());
    app.clear_dirty();

    app.insert_char('x');
    assert!(app.is_dirty());
}

#[test]
fn test_add_message_sets_dirty() {
    let mut app = App::new(PermissionMode::default());
    app.clear_dirty();

    app.add_message(Message::user("test".to_string()));
    assert!(app.is_dirty());
}

#[test]
fn test_scroll_sets_dirty() {
    let mut app = App::new(PermissionMode::default());
    app.clear_dirty();

    app.scroll_up(1);
    assert!(app.is_dirty());
}

#[test]
fn test_toggle_debug_sets_dirty() {
    let mut app = App::new(PermissionMode::default());
    app.clear_dirty();

    app.toggle_debug();
    assert!(app.is_dirty());
}

#[test]
fn test_set_error_sets_dirty() {
    let mut app = App::new(PermissionMode::default());
    app.clear_dirty();

    app.set_error("boom".to_string());
    assert!(app.is_dirty());
}

#[test]
fn test_exit_sets_dirty() {
    let mut app = App::new(PermissionMode::default());
    app.clear_dirty();

    app.exit();
    assert!(app.is_dirty());
}

#[test]
fn test_mark_dirty_explicit() {
    let mut app = App::new(PermissionMode::default());
    app.clear_dirty();
    assert!(!app.is_dirty());

    app.mark_dirty();
    assert!(app.is_dirty());
}

// ============================================================
// 11. Message Operations
// ============================================================

#[test]
fn test_add_message_user() {
    let mut app = App::new(PermissionMode::default());
    app.add_message(Message::user("hello".to_string()));

    assert_eq!(app.messages().len(), 1);
    assert_eq!(app.messages()[0].content, "hello");
    assert!(!app.messages()[0].streaming);
}

#[test]
fn test_add_message_assistant() {
    let mut app = App::new(PermissionMode::default());
    app.add_message(Message::assistant("response".to_string()));

    assert_eq!(app.messages().len(), 1);
    assert_eq!(app.messages()[0].content, "response");
}

#[test]
fn test_add_multiple_messages() {
    let mut app = App::new(PermissionMode::default());
    app.add_message(Message::user("q1".to_string()));
    app.add_message(Message::assistant("a1".to_string()));
    app.add_message(Message::user("q2".to_string()));

    assert_eq!(app.messages().len(), 3);
    assert_eq!(app.messages()[0].content, "q1");
    assert_eq!(app.messages()[1].content, "a1");
    assert_eq!(app.messages()[2].content, "q2");
}

#[test]
fn test_messages_slice_is_ordered() {
    let mut app = App::new(PermissionMode::default());
    for i in 0..10 {
        app.add_message(Message::user(format!("msg-{}", i)));
    }
    assert_eq!(app.messages().len(), 10);
    assert_eq!(app.messages()[0].content, "msg-0");
    assert_eq!(app.messages()[9].content, "msg-9");
}

#[test]
fn test_messages_mut() {
    let mut app = App::new(PermissionMode::default());
    app.add_message(Message::user("original".to_string()));

    // Mutate through messages_mut
    app.messages_mut()[0].content = "modified".to_string();
    assert_eq!(app.messages()[0].content, "modified");
}

// ============================================================
// 12. Event Handling (via handle_event)
// ============================================================

#[test]
fn test_ctrl_c_returns_exit() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let mut app = App::new(PermissionMode::default());
    let event = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));

    let result = rustyclawd::tui::handle_event(&mut app, event).unwrap();
    assert_eq!(result, EventResult::Exit);
    assert!(app.should_exit());
}

#[test]
fn test_enter_submits_non_empty_input() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let mut app = App::new(PermissionMode::default());
    app.insert_char('g');
    app.insert_char('o');

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let result = rustyclawd::tui::handle_event(&mut app, event).unwrap();
    assert_eq!(result, EventResult::Submit("go".to_string()));
    assert_eq!(app.input(), "");
}

#[test]
fn test_enter_on_empty_does_not_submit() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let mut app = App::new(PermissionMode::default());
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let result = rustyclawd::tui::handle_event(&mut app, event).unwrap();
    assert_eq!(result, EventResult::Continue);
}

#[test]
fn test_typing_during_streaming_blocked() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let mut app = App::new(PermissionMode::default());
    app.start_streaming_response();

    let event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    let _ = rustyclawd::tui::handle_event(&mut app, event).unwrap();
    assert_eq!(app.input(), "", "input blocked during streaming");
}

#[test]
fn test_enter_during_streaming_does_not_submit() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let mut app = App::new(PermissionMode::default());
    app.insert_char('a');
    app.start_streaming_response();

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let result = rustyclawd::tui::handle_event(&mut app, event).unwrap();
    assert_eq!(
        result,
        EventResult::Continue,
        "submit blocked during streaming"
    );
}

// ============================================================
// Additional state operations
// ============================================================

#[test]
fn test_set_and_clear_error() {
    let mut app = App::new(PermissionMode::default());
    assert!(app.error().is_none());

    app.set_error("something broke".to_string());
    assert_eq!(app.error(), Some("something broke"));

    app.clear_error();
    assert!(app.error().is_none());
}

#[test]
fn test_exit_flag() {
    let mut app = App::new(PermissionMode::default());
    assert!(!app.should_exit());

    app.exit();
    assert!(app.should_exit());
}

#[test]
fn test_cycle_permission_mode() {
    let mut app = App::new(PermissionMode::Ask);
    let mode = app.cycle_permission_mode();
    assert_eq!(mode, PermissionMode::AutoAccept);

    let mode = app.cycle_permission_mode();
    assert_eq!(mode, PermissionMode::Plan);

    let mode = app.cycle_permission_mode();
    assert_eq!(mode, PermissionMode::Ask);
}

#[test]
fn test_toggle_menu() {
    let mut app = App::new(PermissionMode::default());
    assert!(!app.menu_open());

    app.toggle_menu();
    assert!(app.menu_open());

    app.toggle_menu();
    assert!(!app.menu_open());
}

#[test]
fn test_mouse_mode() {
    let mut app = App::new(PermissionMode::default());
    assert!(app.mouse_mode_enabled());

    app.set_mouse_mode(false);
    assert!(!app.mouse_mode_enabled());

    app.set_mouse_mode(true);
    assert!(app.mouse_mode_enabled());
}

#[test]
fn test_cursor_movement() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char('a');
    app.insert_char('b');
    app.insert_char('c');

    assert_eq!(app.cursor_pos().1, 3);

    app.move_cursor_left();
    assert_eq!(app.cursor_pos().1, 2);

    app.move_cursor_right();
    assert_eq!(app.cursor_pos().1, 3);

    app.move_cursor_to_start();
    assert_eq!(app.cursor_pos().1, 0);

    app.move_cursor_to_end();
    assert_eq!(app.cursor_pos().1, 3);
}

#[test]
fn test_delete_char() {
    let mut app = App::new(PermissionMode::default());
    app.insert_char('a');
    app.insert_char('b');
    app.insert_char('c');

    // Move cursor to position 1 (after 'a')
    app.move_cursor_to_start();
    app.move_cursor_right();

    // Delete at cursor should remove 'b'
    app.delete_char();
    assert_eq!(app.input(), "ac");
}

#[test]
fn test_extended_thinking_lifecycle() {
    let mut app = App::new(PermissionMode::default());
    app.start_streaming_response();

    assert!(!app.is_extended_thinking());

    app.start_extended_thinking();
    assert!(app.is_extended_thinking());

    app.append_thinking_content();
    assert!(app.is_extended_thinking());

    app.stop_extended_thinking();
    assert!(!app.is_extended_thinking());
}

#[test]
fn test_thinking_duration() {
    let mut app = App::new(PermissionMode::default());
    app.start_streaming_response();
    app.start_extended_thinking();

    let duration = app.thinking_duration();
    assert!(duration.is_some());
}

#[test]
fn test_blocked_input_message_flag() {
    let mut app = App::new(PermissionMode::default());
    app.start_streaming_response();

    assert!(!app.has_shown_blocked_input_message());
    app.set_shown_blocked_input_message(true);
    assert!(app.has_shown_blocked_input_message());

    // Start thinking resets the flag
    app.start_extended_thinking();
    assert!(!app.has_shown_blocked_input_message());
}

#[test]
fn test_render_debug_info() {
    let app = App::new(PermissionMode::default());
    let info = app.get_render_debug();
    assert!(info.contains("messages=0"));
    assert!(info.contains("streaming=false"));
}

#[test]
fn test_shift_enter_inserts_newline_via_event() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let mut app = App::new(PermissionMode::default());
    app.insert_char('a');

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
    let result = rustyclawd::tui::handle_event(&mut app, event).unwrap();
    assert_eq!(result, EventResult::Continue);

    app.insert_char('b');
    assert_eq!(app.input(), "a\nb");
}

#[test]
fn test_backslash_enter_inserts_newline() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let mut app = App::new(PermissionMode::default());
    for c in "line1\\".chars() {
        app.insert_char(c);
    }
    assert_eq!(app.input(), "line1\\");

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let result = rustyclawd::tui::handle_event(&mut app, event).unwrap();
    assert_eq!(result, EventResult::Continue);

    // Backslash removed, newline inserted
    assert!(app.input().starts_with("line1\n"));
}

#[test]
fn test_escape_clears_autocomplete() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let mut app = App::new(PermissionMode::default());
    let items = vec![CompletionItem {
        command: "test".to_string(),
        description: None,
        argument_hint: None,
    }];
    app.activate_autocomplete(items);
    assert!(app.autocomplete_active());

    let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    let _ = rustyclawd::tui::handle_event(&mut app, event).unwrap();
    assert!(!app.autocomplete_active());
}

#[test]
fn test_escape_clears_memory_modal() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let mut app = App::new(PermissionMode::default());
    let destinations = vec![MemoryDestination {
        name: "D".to_string(),
        file_path: "/d".to_string(),
        description: None,
        is_imported: false,
    }];
    app.activate_memory_modal("text".to_string(), destinations);
    assert!(app.memory_modal_active());

    let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    let _ = rustyclawd::tui::handle_event(&mut app, event).unwrap();
    assert!(!app.memory_modal_active());
}

#[test]
fn test_escape_clears_permissions_modal() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let mut app = App::new(PermissionMode::default());
    app.activate_permissions_modal();
    assert!(app.permissions_modal_active());

    let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    let _ = rustyclawd::tui::handle_event(&mut app, event).unwrap();
    assert!(!app.permissions_modal_active());
}
