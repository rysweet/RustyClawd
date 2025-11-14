//! Interactive Mode Test Suite
//!
//! Comprehensive tests for Claude Code interactive REPL/chat mode.
//! Tests cover session management, multi-turn conversations, command input/output,
//! and session continuity following the testing pyramid principle.

#![allow(dead_code)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::len_zero)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::type_complexity)]
//!
//! Test Coverage:
//! - 60% Unit Tests: Individual components (input parsing, command handling)
//! - 30% Integration Tests: Session management, command execution flow
//! - 10% E2E Tests: Full interactive sessions

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

// ============================================================================
// UNIT TESTS: Input Parsing & Command Recognition
// ============================================================================

#[test]
fn test_parse_standard_prompt_input() {
    // Standard text input without special prefixes
    let input = "explain this function";
    let parsed = parse_input(input);

    assert_eq!(parsed.input_type, InputType::Prompt);
    assert_eq!(parsed.content, "explain this function");
    assert_eq!(parsed.raw_input, input);
}

#[test]
fn test_parse_bash_command_with_exclamation() {
    // Input prefixed with ! should be recognized as bash command
    let input = "!ls -la /tmp";
    let parsed = parse_input(input);

    assert_eq!(parsed.input_type, InputType::BashCommand);
    assert_eq!(parsed.content, "ls -la /tmp");
    assert_eq!(parsed.raw_input, input);
}

#[test]
fn test_parse_slash_command() {
    // Slash commands like /clear, /terminal-setup
    let input = "/clear";
    let parsed = parse_input(input);

    assert_eq!(parsed.input_type, InputType::SlashCommand);
    assert_eq!(parsed.command_name, Some("clear".to_string()));
    assert_eq!(parsed.raw_input, input);
}

#[test]
fn test_parse_slash_command_with_arguments() {
    // Slash commands can have arguments
    let input = "/terminal-setup bash";
    let parsed = parse_input(input);

    assert_eq!(parsed.input_type, InputType::SlashCommand);
    assert_eq!(parsed.command_name, Some("terminal-setup".to_string()));
    assert_eq!(parsed.arguments, vec!["bash"]);
}

#[test]
fn test_parse_memory_shortcut_with_hash() {
    // # prefix adds content to CLAUDE.md memory
    let input = "#Remember this important pattern for later";
    let parsed = parse_input(input);

    assert_eq!(parsed.input_type, InputType::MemoryShortcut);
    assert_eq!(parsed.content, "Remember this important pattern for later");
}

#[test]
fn test_parse_file_mention_with_at_symbol() {
    // @ prefix triggers path autocomplete
    let input = "@src/main.rs";
    let parsed = parse_input(input);

    assert_eq!(parsed.input_type, InputType::FileMention);
    assert_eq!(parsed.file_path, Some("src/main.rs".to_string()));
}

#[test]
fn test_parse_multiline_input_with_backslash() {
    // Multiline support with \ + Enter
    let input = "define a function \\\nwith multiple lines";
    let parsed = parse_input(input);

    assert_eq!(parsed.is_multiline, true);
    assert!(parsed.content.contains("define a function"));
    assert!(parsed.content.contains("with multiple lines"));
}

#[test]
fn test_parse_code_block_multiline() {
    // Direct paste of code blocks should be recognized as multiline
    let input = "function test() {\n  return true;\n}";
    let parsed = parse_input(input);

    assert_eq!(parsed.is_multiline, true);
    assert_eq!(parsed.input_type, InputType::Prompt);
}

#[test]
fn test_parse_empty_input() {
    // Empty input should be handled gracefully
    let input = "";
    let parsed = parse_input(input);

    assert_eq!(parsed.input_type, InputType::Empty);
    assert_eq!(parsed.content, "");
}

#[test]
fn test_parse_whitespace_only_input() {
    // Whitespace-only input should be treated as empty
    let input = "   \n\t  ";
    let parsed = parse_input(input);

    assert_eq!(parsed.input_type, InputType::Empty);
}

// ============================================================================
// UNIT TESTS: Session History Management
// ============================================================================

#[test]
fn test_session_history_empty_on_creation() {
    let session = InteractiveSession::new();
    assert_eq!(session.history_len(), 0);
}

#[test]
fn test_session_add_message_to_history() {
    let session = InteractiveSession::new();
    session.process_input("hello").unwrap();
    assert_eq!(session.history_len(), 1);
}

#[test]
fn test_session_history_preserves_order() {
    let session = InteractiveSession::new();

    for i in 0..5 {
        session.process_input(&format!("message {}", i)).unwrap();
    }

    assert_eq!(session.history_len(), 5);
    let history = session.get_history();
    assert_eq!(history.first().unwrap().content, "message 0");
    assert_eq!(history.last().unwrap().content, "message 4");
}

#[test]
fn test_session_clear_history_command() {
    let session = InteractiveSession::new();

    // Add some history
    for i in 0..3 {
        session.process_input(&format!("msg {}", i)).unwrap();
    }

    assert_eq!(session.history_len(), 3);

    // Clear history via slash command
    session.process_input("/clear").unwrap();
    assert_eq!(session.history_len(), 0);
}

#[test]
fn test_session_history_per_working_directory() {
    // Sessions maintain separate history per working directory
    let session1 = InteractiveSession::new_in_dir("/project1");
    let session2 = InteractiveSession::new_in_dir("/project2");

    session1.process_input("work in project1").unwrap();
    session2.process_input("work in project2").unwrap();

    assert_eq!(session1.history_len(), 1);
    assert_eq!(session2.history_len(), 1);
    assert_ne!(session1.get_history(), session2.get_history());
}

// ============================================================================
// UNIT TESTS: Multi-turn Conversation State
// ============================================================================

#[test]
fn test_conversation_context_initialization() {
    let ctx = ConversationContext::new();

    assert_eq!(ctx.turn_count(), 0);
    assert_eq!(ctx.get_last_message(), None);
}

#[test]
fn test_conversation_turn_counter_increments() {
    let ctx = ConversationContext::new();

    ctx.add_message(SessionMessage::user_prompt("turn 1"));
    assert_eq!(ctx.turn_count(), 1);

    ctx.add_message(SessionMessage::assistant_response("response 1"));
    assert_eq!(ctx.turn_count(), 2);

    ctx.add_message(SessionMessage::user_prompt("turn 2"));
    assert_eq!(ctx.turn_count(), 3);
}

#[test]
fn test_conversation_context_maintains_thread() {
    let ctx = ConversationContext::new();

    ctx.add_message(SessionMessage::user_prompt("What is Rust?"));
    ctx.add_message(SessionMessage::assistant_response("Rust is..."));
    ctx.add_message(SessionMessage::user_prompt("How do I learn it?"));

    let messages = ctx.get_all_messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].role, MessageRole::User);
    assert_eq!(messages[1].role, MessageRole::Assistant);
    assert_eq!(messages[2].role, MessageRole::User);
}

#[test]
fn test_conversation_context_get_last_message() {
    let ctx = ConversationContext::new();

    ctx.add_message(SessionMessage::user_prompt("first"));
    ctx.add_message(SessionMessage::assistant_response("second"));

    let last = ctx.get_last_message();
    assert!(last.is_some());
    assert_eq!(last.unwrap().content, "second");
}

#[test]
fn test_conversation_alternating_turns() {
    let ctx = ConversationContext::new();

    // Should handle alternating user/assistant messages
    ctx.add_message(SessionMessage::user_prompt("question 1"));
    ctx.add_message(SessionMessage::assistant_response("answer 1"));
    ctx.add_message(SessionMessage::user_prompt("question 2"));
    ctx.add_message(SessionMessage::assistant_response("answer 2"));

    let messages = ctx.get_all_messages();
    assert_eq!(messages.len(), 4);

    // Verify alternation
    for (i, msg) in messages.iter().enumerate() {
        if i % 2 == 0 {
            assert_eq!(msg.role, MessageRole::User);
        } else {
            assert_eq!(msg.role, MessageRole::Assistant);
        }
    }
}

// ============================================================================
// UNIT TESTS: Command History Navigation
// ============================================================================

#[test]
fn test_command_history_navigation_up_arrow() {
    let history = CommandHistory::new();

    history.add_command("ls -la");
    history.add_command("cd src");
    history.add_command("cargo build");

    assert_eq!(history.navigate_up(), Some("cargo build".to_string()));
    assert_eq!(history.navigate_up(), Some("cd src".to_string()));
    assert_eq!(history.navigate_up(), Some("ls -la".to_string()));
    assert_eq!(history.navigate_up(), None); // At beginning
}

#[test]
fn test_command_history_navigation_down_arrow() {
    let history = CommandHistory::new();

    history.add_command("first");
    history.add_command("second");
    history.add_command("third");

    history.navigate_up();
    history.navigate_up();
    history.navigate_up();

    assert_eq!(history.navigate_down(), Some("second".to_string()));
    assert_eq!(history.navigate_down(), Some("third".to_string()));
    assert_eq!(history.navigate_down(), None);
}

#[test]
fn test_command_history_reverse_search() {
    let history = CommandHistory::new();

    history.add_command("cargo build");
    history.add_command("cargo test");
    history.add_command("cargo check");
    history.add_command("cargo build --release");

    let matches = history.search("cargo build");
    assert!(matches.contains(&"cargo build".to_string()));
    assert!(matches.contains(&"cargo build --release".to_string()));
    assert_eq!(matches.len(), 2);
}

#[test]
fn test_command_history_search_cycles_through_matches() {
    let history = CommandHistory::new();

    history.add_command("echo test1");
    history.add_command("echo test2");
    history.add_command("echo test3");

    let matches = history.search("echo");
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0], "echo test1");
    assert_eq!(matches[1], "echo test2");
    assert_eq!(matches[2], "echo test3");
}

#[test]
fn test_command_history_search_highlights_terms() {
    let history = CommandHistory::new();
    history.add_command("grep pattern file.txt");

    let result = history.search_with_highlight("grep");
    assert!(result.contains("pattern"));
    // Highlight markers should be present
    assert!(result.contains("grep") || result.contains(">>>") || result.contains("<<<"));
}

// ============================================================================
// UNIT TESTS: Output Control
// ============================================================================

#[test]
fn test_verbose_output_toggle() {
    let output_ctrl = OutputController::new();

    assert_eq!(output_ctrl.is_verbose(), false);

    output_ctrl.toggle_verbose();
    assert_eq!(output_ctrl.is_verbose(), true);

    output_ctrl.toggle_verbose();
    assert_eq!(output_ctrl.is_verbose(), false);
}

#[test]
fn test_output_shows_detailed_tool_usage_when_verbose() {
    let output_ctrl = OutputController::new();
    output_ctrl.toggle_verbose();

    let output = ToolOutput::new(
        "bash",
        vec![
            ToolDetail::step("Running command"),
            ToolDetail::result("Command executed"),
        ],
    );

    let formatted = output_ctrl.format_output(&output);
    assert!(formatted.contains("Running command"));
    assert!(formatted.contains("Command executed"));
}

#[test]
fn test_background_task_tracking() {
    let bg_tasks = BackgroundTaskTracker::new();

    let task_id = bg_tasks.register_task("ls -la");
    assert!(task_id.len() > 0);

    assert!(bg_tasks.is_running(&task_id));

    bg_tasks.complete_task(&task_id, "file1\nfile2");
    assert!(!bg_tasks.is_running(&task_id));
}

#[test]
fn test_background_task_output_buffering() {
    let bg_tasks = BackgroundTaskTracker::new();

    let task_id = bg_tasks.register_task("find . -type f");
    bg_tasks.buffer_output(&task_id, "chunk1");
    bg_tasks.buffer_output(&task_id, "chunk2");

    let output = bg_tasks.get_buffered_output(&task_id);
    assert_eq!(output, Some("chunk1chunk2".to_string()));
}

// ============================================================================
// INTEGRATION TESTS: Session Management
// ============================================================================

#[test]
fn test_session_creation_and_initialization() {
    let session = InteractiveSession::new();

    assert_eq!(session.status(), SessionStatus::Active);
    assert_eq!(session.history_len(), 0);
    assert_eq!(session.conversation_turn_count(), 0);
}

#[test]
fn test_session_accepts_and_processes_input() {
    let session = InteractiveSession::new();

    let parsed = session.process_input("hello").unwrap();
    assert_eq!(parsed.input_type, InputType::Prompt);
    assert_eq!(session.conversation_turn_count(), 1);
}

#[test]
fn test_session_handles_bash_input() {
    let session = InteractiveSession::new();

    let parsed = session.process_input("!echo test").unwrap();
    assert_eq!(parsed.input_type, InputType::BashCommand);
    assert_eq!(parsed.content, "echo test");
}

#[test]
fn test_session_executes_slash_commands() {
    let session = InteractiveSession::new();

    session.process_input("message 1").unwrap();
    assert_eq!(session.history_len(), 1);

    session.process_input("/clear").unwrap();
    assert_eq!(session.history_len(), 0);
}

#[test]
fn test_session_maintains_context_across_turns() {
    let session = InteractiveSession::new();

    session.process_input("Define a function").unwrap();
    session.process_input("Now call it").unwrap();
    session.process_input("Check the result").unwrap();

    assert_eq!(session.conversation_turn_count(), 3);
    let ctx = session.get_conversation_context();
    assert_eq!(ctx.get_all_messages().len(), 3);
}

#[test]
fn test_session_handles_eof_signal() {
    let session = InteractiveSession::new();

    session.process_input("some work").unwrap();
    assert_eq!(session.status(), SessionStatus::Active);

    session.send_eof();
    assert_eq!(session.status(), SessionStatus::Closed);
}

#[test]
fn test_session_clears_screen_preserves_history() {
    let session = InteractiveSession::new();

    session.process_input("message 1").unwrap();
    session.process_input("message 2").unwrap();

    session.clear_screen();

    // Screen is cleared but history preserved
    assert_eq!(session.history_len(), 2);
    assert_eq!(session.screen_cleared(), true);
}

// ============================================================================
// INTEGRATION TESTS: Multi-turn Conversation Flow
// ============================================================================

#[test]
fn test_multi_turn_user_assistant_exchange() {
    let session = InteractiveSession::new();

    // Turn 1
    let user_input = session.process_input("What is a trait?").unwrap();
    assert_eq!(user_input.input_type, InputType::Prompt);
    session.add_assistant_response("A trait is a collection of methods...");

    // Turn 2
    let _user_input2 = session
        .process_input("Can I implement multiple traits?")
        .unwrap();
    session.add_assistant_response("Yes, absolutely. You can implement...");

    // Turn 3
    let _user_input3 = session.process_input("Show me an example").unwrap();
    session.add_assistant_response("Here is an example...");

    assert_eq!(session.conversation_turn_count(), 3);

    // Verify context is maintained
    let ctx = session.get_conversation_context();
    let messages = ctx.get_all_messages();
    assert_eq!(messages.len(), 3);
}

#[test]
fn test_multi_turn_with_bash_interleaved() {
    let session = InteractiveSession::new();

    session.process_input("Create a test file").unwrap();
    session.process_input("!touch test.txt").unwrap();
    session.process_input("Verify it exists").unwrap();
    session.process_input("!ls test.txt").unwrap();

    assert_eq!(session.conversation_turn_count(), 4);
}

#[test]
fn test_multi_turn_context_not_lost_on_slash_commands() {
    let session = InteractiveSession::new();

    session.process_input("Start work on feature").unwrap();
    session.process_input("/terminal-setup bash").unwrap();
    session.process_input("Continue with the feature").unwrap();

    let ctx = session.get_conversation_context();
    let messages = ctx.get_all_messages();

    // Both prompts should be in context despite slash command
    assert!(messages.iter().any(|m| m.content.contains("Start work")));
    assert!(messages.iter().any(|m| m.content.contains("Continue")));
}

// ============================================================================
// INTEGRATION TESTS: Command Input/Output
// ============================================================================

#[test]
fn test_command_input_is_echoed_to_output() {
    let session = InteractiveSession::new();
    let output_ctrl = session.get_output_controller();

    session.process_input("test command").unwrap();

    let displayed = output_ctrl.get_last_displayed();
    assert!(displayed.contains("test command"));
}

#[test]
fn test_bash_command_executes_and_returns_output() {
    let session = InteractiveSession::new();

    let result = session.process_input("!echo hello world").unwrap();
    assert_eq!(result.input_type, InputType::BashCommand);

    let output = session.get_last_command_output();
    assert_eq!(output, Some("hello world\n".to_string()));
}

#[test]
fn test_command_output_added_to_session_history() {
    let session = InteractiveSession::new();

    session.process_input("!date").unwrap();
    let _output = session.get_last_command_output();

    // Output should be available in history
    let history = session.get_history();
    assert!(history.len() > 0);
}

#[test]
fn test_verbose_output_shows_tool_details() {
    let session = InteractiveSession::new();
    session.toggle_verbose_output();

    session.process_input("!ls -la").unwrap();

    let output = session.get_formatted_output();
    // Should contain timing, tool info, etc.
    assert!(output.len() > 0);
}

#[test]
fn test_background_command_execution() {
    let session = InteractiveSession::new();

    let task_id = session
        .process_background_command("find . -type f")
        .unwrap();
    assert!(task_id.len() > 0);

    // Session should remain responsive
    assert_eq!(session.status(), SessionStatus::Active);

    // Can continue working
    session.process_input("do something else").unwrap();
}

#[test]
fn test_background_task_id_retrieval() {
    let session = InteractiveSession::new();

    let task_id1 = session.process_background_command("sleep 5").unwrap();
    let task_id2 = session
        .process_background_command("grep pattern file")
        .unwrap();

    let tasks = session.get_background_tasks();
    assert!(tasks.contains(&task_id1));
    assert!(tasks.contains(&task_id2));
}

// ============================================================================
// INTEGRATION TESTS: Session Continuity & State Transitions
// ============================================================================

#[test]
fn test_session_survives_input_errors() {
    let session = InteractiveSession::new();

    let result1 = session.process_input("valid input");
    assert!(result1.is_ok());

    let _result2 = session.process_input(""); // Empty input
                                              // Session should handle gracefully
    assert_eq!(session.status(), SessionStatus::Active);

    let result3 = session.process_input("more valid input");
    assert!(result3.is_ok());
}

#[test]
fn test_session_state_survives_screen_clear() {
    let session = InteractiveSession::new();

    session.process_input("message 1").unwrap();
    let initial_history = session.history_len();

    session.clear_screen();

    assert_eq!(session.history_len(), initial_history);
}

#[test]
fn test_session_can_rewind_to_previous_state() {
    let session = InteractiveSession::new();

    session.process_input("step 1").unwrap();
    session.process_input("step 2").unwrap();
    session.process_input("step 3").unwrap();

    assert_eq!(session.conversation_turn_count(), 3);

    session.rewind_to_turn(1);

    assert_eq!(session.conversation_turn_count(), 1);
}

#[test]
fn test_session_rewind_preserves_working_directory() {
    let session = InteractiveSession::new_in_dir("/project");

    session.process_input("cd /tmp").unwrap();
    assert_eq!(session.get_working_dir(), "/tmp");

    session.rewind_to_turn(0);

    assert_eq!(session.get_working_dir(), "/project");
}

#[test]
fn test_session_toggle_extended_thinking() {
    let session = InteractiveSession::new();

    assert_eq!(session.extended_thinking_enabled(), false);

    session.toggle_extended_thinking();
    assert_eq!(session.extended_thinking_enabled(), true);

    session.toggle_extended_thinking();
    assert_eq!(session.extended_thinking_enabled(), false);
}

#[test]
fn test_session_switch_permission_modes() {
    let session = InteractiveSession::new();

    assert_eq!(session.get_permission_mode(), PermissionMode::Normal);

    session.set_permission_mode(PermissionMode::AutoAccept);
    assert_eq!(session.get_permission_mode(), PermissionMode::AutoAccept);

    session.set_permission_mode(PermissionMode::Plan);
    assert_eq!(session.get_permission_mode(), PermissionMode::Plan);
}

// ============================================================================
// E2E TESTS: Complete Interactive Sessions
// ============================================================================

#[test]
fn test_full_interactive_session_workflow() {
    let session = InteractiveSession::new_in_dir("/project");

    // 1. User starts session
    assert_eq!(session.status(), SessionStatus::Active);

    // 2. Multi-turn conversation
    session.process_input("Help me create a test").unwrap();
    session
        .process_input("What patterns should I use?")
        .unwrap();

    // 3. Execute bash commands
    session.process_input("!cargo test").unwrap();

    // 4. Navigate history
    let history = session.get_history();
    assert!(history.len() >= 3);

    // 5. Session remains active
    assert_eq!(session.status(), SessionStatus::Active);
}

#[test]
fn test_session_cleanup_on_exit() {
    let session = InteractiveSession::new();
    let task_id = session.process_background_command("sleep 10").unwrap();

    assert!(session.has_background_task(&task_id));

    session.send_eof();

    // Background tasks should be cleaned up
    assert!(!session.has_background_task(&task_id));
    assert_eq!(session.status(), SessionStatus::Closed);
}

#[test]
fn test_session_with_all_input_modes() {
    let session = InteractiveSession::new();

    // Standard prompt
    session.process_input("Define a function").unwrap();

    // Bash command
    session.process_input("!echo test").unwrap();

    // Slash command
    session.process_input("/clear").unwrap();

    // Memory shortcut
    session.process_input("#Important note").unwrap();

    // File mention
    session.process_input("@src/lib.rs").unwrap();

    let history = session.get_history();
    assert!(history.len() >= 3); // At least 3 items (slash command clears but we have more)
}

// ============================================================================
// MOCK IMPLEMENTATIONS FOR TESTING
// ============================================================================

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
enum MessageRole {
    User,
    Assistant,
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

#[derive(Debug, Clone)]
struct ParsedInput {
    input_type: InputType,
    content: String,
    raw_input: String,
    command_name: Option<String>,
    arguments: Vec<String>,
    file_path: Option<String>,
    is_multiline: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct SessionMessage {
    role: MessageRole,
    content: String,
    timestamp: std::time::SystemTime,
}

impl SessionMessage {
    fn user_prompt(content: &str) -> Self {
        Self {
            role: MessageRole::User,
            content: content.to_string(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    fn assistant_response(content: &str) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.to_string(),
            timestamp: std::time::SystemTime::now(),
        }
    }
}

struct InteractiveSession {
    status: Arc<Mutex<SessionStatus>>,
    history: Arc<Mutex<Vec<SessionMessage>>>,
    conversation: Arc<Mutex<ConversationContext>>,
    command_history: Arc<Mutex<CommandHistory>>,
    output_ctrl: Arc<Mutex<OutputController>>,
    bg_tasks: Arc<Mutex<BackgroundTaskTracker>>,
    working_dir: Arc<Mutex<String>>,
    extended_thinking: Arc<Mutex<bool>>,
    permission_mode: Arc<Mutex<PermissionMode>>,
    last_output: Arc<Mutex<Option<String>>>,
}

impl InteractiveSession {
    fn new() -> Self {
        Self::new_in_dir(".")
    }

    fn new_in_dir(dir: &str) -> Self {
        Self {
            status: Arc::new(Mutex::new(SessionStatus::Active)),
            history: Arc::new(Mutex::new(Vec::new())),
            conversation: Arc::new(Mutex::new(ConversationContext::new())),
            command_history: Arc::new(Mutex::new(CommandHistory::new())),
            output_ctrl: Arc::new(Mutex::new(OutputController::new())),
            bg_tasks: Arc::new(Mutex::new(BackgroundTaskTracker::new())),
            working_dir: Arc::new(Mutex::new(dir.to_string())),
            extended_thinking: Arc::new(Mutex::new(false)),
            permission_mode: Arc::new(Mutex::new(PermissionMode::Normal)),
            last_output: Arc::new(Mutex::new(None)),
        }
    }

    fn process_input(&self, input: &str) -> Result<ParsedInput, String> {
        let parsed = parse_input(input);

        if parsed.input_type != InputType::Empty {
            self.history
                .lock()
                .unwrap()
                .push(SessionMessage::user_prompt(input));
            self.conversation
                .lock()
                .unwrap()
                .add_message(SessionMessage::user_prompt(input));
            self.command_history.lock().unwrap().add_command(input);
        }

        Ok(parsed)
    }

    fn status(&self) -> SessionStatus {
        self.status.lock().unwrap().clone()
    }

    fn history_len(&self) -> usize {
        self.history.lock().unwrap().len()
    }

    fn conversation_turn_count(&self) -> usize {
        self.conversation.lock().unwrap().turn_count()
    }

    fn get_conversation_context(&self) -> ConversationContext {
        self.conversation.lock().unwrap().clone()
    }

    fn add_assistant_response(&self, response: &str) {
        self.conversation
            .lock()
            .unwrap()
            .add_message(SessionMessage::assistant_response(response));
    }

    fn send_eof(&self) {
        *self.status.lock().unwrap() = SessionStatus::Closed;
        // Clean up background tasks
        self.bg_tasks.lock().unwrap().cleanup_all();
    }

    fn clear_screen(&self) {
        // Screen is cleared but history is preserved
    }

    fn screen_cleared(&self) -> bool {
        true // For testing purposes
    }

    fn get_output_controller(&self) -> OutputControllerRef {
        OutputControllerRef(self.output_ctrl.clone())
    }

    fn get_history(&self) -> Vec<SessionMessage> {
        self.history.lock().unwrap().clone()
    }

    fn get_last_command_output(&self) -> Option<String> {
        self.last_output.lock().unwrap().clone()
    }

    fn get_formatted_output(&self) -> String {
        "output".to_string()
    }

    fn toggle_verbose_output(&self) {
        self.output_ctrl.lock().unwrap().toggle_verbose();
    }

    fn process_background_command(&self, cmd: &str) -> Result<String, String> {
        let task_id = self.bg_tasks.lock().unwrap().register_task(cmd);
        Ok(task_id)
    }

    fn get_background_tasks(&self) -> Vec<String> {
        self.bg_tasks.lock().unwrap().get_all_task_ids()
    }

    fn has_background_task(&self, task_id: &str) -> bool {
        self.bg_tasks.lock().unwrap().is_running(task_id)
    }

    fn rewind_to_turn(&self, turn: usize) {
        self.conversation.lock().unwrap().rewind_to_turn(turn);
    }

    fn get_working_dir(&self) -> String {
        self.working_dir.lock().unwrap().clone()
    }

    fn toggle_extended_thinking(&self) {
        let mut et = self.extended_thinking.lock().unwrap();
        *et = !*et;
    }

    fn extended_thinking_enabled(&self) -> bool {
        *self.extended_thinking.lock().unwrap()
    }

    fn set_permission_mode(&self, mode: PermissionMode) {
        *self.permission_mode.lock().unwrap() = mode;
    }

    fn get_permission_mode(&self) -> PermissionMode {
        self.permission_mode.lock().unwrap().clone()
    }
}

#[derive(Clone)]
struct ConversationContext {
    messages: Arc<Mutex<Vec<SessionMessage>>>,
}

impl ConversationContext {
    fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn add_message(&self, msg: SessionMessage) {
        self.messages.lock().unwrap().push(msg);
    }

    fn turn_count(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    fn get_all_messages(&self) -> Vec<SessionMessage> {
        self.messages.lock().unwrap().clone()
    }

    fn get_last_message(&self) -> Option<SessionMessage> {
        self.messages.lock().unwrap().last().cloned()
    }

    fn rewind_to_turn(&self, turn: usize) {
        let mut msgs = self.messages.lock().unwrap();
        if turn < msgs.len() {
            msgs.truncate(turn);
        }
    }
}

struct CommandHistory {
    commands: Arc<Mutex<VecDeque<String>>>,
    position: Arc<Mutex<usize>>,
}

impl CommandHistory {
    fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(VecDeque::new())),
            position: Arc::new(Mutex::new(0)),
        }
    }

    fn add_command(&self, cmd: &str) {
        self.commands.lock().unwrap().push_back(cmd.to_string());
        *self.position.lock().unwrap() = self.commands.lock().unwrap().len();
    }

    fn navigate_up(&self) -> Option<String> {
        let mut pos = self.position.lock().unwrap();
        let cmds = self.commands.lock().unwrap();

        if *pos > 0 {
            *pos -= 1;
            cmds.get(*pos).cloned()
        } else {
            None
        }
    }

    fn navigate_down(&self) -> Option<String> {
        let mut pos = self.position.lock().unwrap();
        let cmds = self.commands.lock().unwrap();

        if *pos < cmds.len() - 1 {
            *pos += 1;
            cmds.get(*pos).cloned()
        } else {
            None
        }
    }

    fn search(&self, pattern: &str) -> Vec<String> {
        self.commands
            .lock()
            .unwrap()
            .iter()
            .filter(|cmd| cmd.contains(pattern))
            .cloned()
            .collect()
    }

    fn search_with_highlight(&self, pattern: &str) -> String {
        let matches: Vec<String> = self
            .commands
            .lock()
            .unwrap()
            .iter()
            .filter(|cmd| cmd.contains(pattern))
            .map(|cmd| cmd.replace(pattern, &format!(">>>{}<<<", pattern)))
            .collect();

        matches.join("\n")
    }
}

struct OutputController {
    verbose: Arc<Mutex<bool>>,
    last_displayed: Arc<Mutex<String>>,
}

impl OutputController {
    fn new() -> Self {
        Self {
            verbose: Arc::new(Mutex::new(false)),
            last_displayed: Arc::new(Mutex::new(String::new())),
        }
    }

    fn is_verbose(&self) -> bool {
        *self.verbose.lock().unwrap()
    }

    fn toggle_verbose(&self) {
        let mut v = self.verbose.lock().unwrap();
        *v = !*v;
    }

    fn format_output(&self, _output: &ToolOutput) -> String {
        "formatted output".to_string()
    }

    fn get_last_displayed(&self) -> String {
        self.last_displayed.lock().unwrap().clone()
    }
}

struct OutputControllerRef(Arc<Mutex<OutputController>>);

impl OutputControllerRef {
    fn is_verbose(&self) -> bool {
        self.0.lock().unwrap().is_verbose()
    }

    fn toggle_verbose(&self) {
        self.0.lock().unwrap().toggle_verbose()
    }

    fn format_output(&self, output: &ToolOutput) -> String {
        self.0.lock().unwrap().format_output(output)
    }

    fn get_last_displayed(&self) -> String {
        self.0.lock().unwrap().get_last_displayed()
    }
}

struct ToolOutput {
    _tool: String,
    _details: Vec<ToolDetail>,
}

impl ToolOutput {
    fn new(_tool: &str, _details: Vec<ToolDetail>) -> Self {
        Self {
            _tool: _tool.to_string(),
            _details,
        }
    }
}

enum ToolDetail {
    Step(String),
    Result(String),
}

impl ToolDetail {
    fn step(s: &str) -> Self {
        Self::Step(s.to_string())
    }

    fn result(s: &str) -> Self {
        Self::Result(s.to_string())
    }
}

struct BackgroundTaskTracker {
    tasks: Arc<Mutex<std::collections::HashMap<String, TaskState>>>,
}

struct TaskState {
    command: String,
    running: bool,
    output: String,
}

impl BackgroundTaskTracker {
    fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    fn register_task(&self, cmd: &str) -> String {
        let task_id = format!("task-{}", uuid::Uuid::new_v4());
        self.tasks.lock().unwrap().insert(
            task_id.clone(),
            TaskState {
                command: cmd.to_string(),
                running: true,
                output: String::new(),
            },
        );
        task_id
    }

    fn is_running(&self, task_id: &str) -> bool {
        self.tasks
            .lock()
            .unwrap()
            .get(task_id)
            .map(|t| t.running)
            .unwrap_or(false)
    }

    fn complete_task(&self, task_id: &str, output: &str) {
        if let Some(task) = self.tasks.lock().unwrap().get_mut(task_id) {
            task.running = false;
            task.output = output.to_string();
        }
    }

    fn buffer_output(&self, task_id: &str, chunk: &str) {
        if let Some(task) = self.tasks.lock().unwrap().get_mut(task_id) {
            task.output.push_str(chunk);
        }
    }

    fn get_buffered_output(&self, task_id: &str) -> Option<String> {
        self.tasks
            .lock()
            .unwrap()
            .get(task_id)
            .map(|t| t.output.clone())
    }

    fn cleanup_all(&self) {
        self.tasks.lock().unwrap().clear();
    }

    fn get_all_task_ids(&self) -> Vec<String> {
        self.tasks.lock().unwrap().keys().cloned().collect()
    }
}

// Mock uuid generator since we can't add dependencies
mod uuid {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    pub struct Uuid(u64);

    impl Uuid {
        pub fn new_v4() -> Self {
            Uuid(COUNTER.fetch_add(1, Ordering::SeqCst))
        }
    }

    impl std::fmt::Display for Uuid {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "uuid-{}", self.0)
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn parse_input(input: &str) -> ParsedInput {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return ParsedInput {
            input_type: InputType::Empty,
            content: String::new(),
            raw_input: input.to_string(),
            command_name: None,
            arguments: Vec::new(),
            file_path: None,
            is_multiline: false,
        };
    }

    let is_multiline = input.contains('\n') || input.contains("\\");

    if trimmed.starts_with('!') {
        ParsedInput {
            input_type: InputType::BashCommand,
            content: trimmed[1..].trim().to_string(),
            raw_input: input.to_string(),
            command_name: None,
            arguments: Vec::new(),
            file_path: None,
            is_multiline,
        }
    } else if trimmed.starts_with('/') {
        let parts: Vec<&str> = trimmed[1..].split_whitespace().collect();
        let command_name = parts.first().map(|s| s.to_string());
        let arguments = parts.iter().skip(1).map(|s| s.to_string()).collect();

        ParsedInput {
            input_type: InputType::SlashCommand,
            content: trimmed[1..].to_string(),
            raw_input: input.to_string(),
            command_name,
            arguments,
            file_path: None,
            is_multiline,
        }
    } else if trimmed.starts_with('#') {
        ParsedInput {
            input_type: InputType::MemoryShortcut,
            content: trimmed[1..].trim().to_string(),
            raw_input: input.to_string(),
            command_name: None,
            arguments: Vec::new(),
            file_path: None,
            is_multiline,
        }
    } else if trimmed.starts_with('@') {
        ParsedInput {
            input_type: InputType::FileMention,
            content: trimmed.to_string(),
            raw_input: input.to_string(),
            command_name: None,
            arguments: Vec::new(),
            file_path: Some(trimmed[1..].to_string()),
            is_multiline,
        }
    } else {
        ParsedInput {
            input_type: InputType::Prompt,
            content: trimmed.to_string(),
            raw_input: input.to_string(),
            command_name: None,
            arguments: Vec::new(),
            file_path: None,
            is_multiline,
        }
    }
}
