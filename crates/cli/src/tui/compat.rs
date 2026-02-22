//! Compatibility wrapper for legacy TuiState interface
//!
//! Provides the old TuiState API on top of the new App architecture
//! for backward compatibility with interactive.rs

use super::{App, Message, PermissionMode};
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use std::time::Duration;

/// Completion callback type
pub type CompletionCallback = Box<dyn Fn(&str) -> Vec<(String, Option<String>)> + Send>;

/// Legacy TuiState compatibility wrapper
pub struct TuiState {
    /// Core app state
    app: App,

    /// Terminal instance
    terminal: Terminal<CrosstermBackend<Stdout>>,

    /// Completion callback (not implemented yet)
    #[allow(dead_code)]
    completion_callback: Option<CompletionCallback>,

    /// Track if cleanup has been done (for idempotency)
    cleaned_up: bool,

    /// Last input value (to detect changes for autocomplete updates)
    last_input: String,
}

impl TuiState {
    /// Create a new TUI state
    pub fn new() -> Result<Self> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();

        // CRITICAL: Enter alternate screen to isolate TUI from terminal history
        execute!(stdout, EnterAlternateScreen)?;

        // Note: mouse capture is intentionally NOT enabled here.
        // Enabling EnableMouseCapture intercepts all mouse events and prevents
        // the terminal emulator from handling native text selection.
        // Users can select and copy text freely without any toggle needed.

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            app: App::new(PermissionMode::default()),
            terminal,
            completion_callback: None,
            cleaned_up: false,
            last_input: String::new(),
        })
    }

    /// Set the completion callback function
    pub fn set_completion_callback(&mut self, callback: CompletionCallback) {
        self.completion_callback = Some(callback);
    }

    /// Get the current permission mode
    pub fn permission_mode(&self) -> PermissionMode {
        self.app.permission_mode()
    }

    /// Set the permission mode
    pub fn set_permission_mode(&mut self, mode: PermissionMode) {
        // Update internal app state
        while self.app.permission_mode() != mode {
            self.app.cycle_permission_mode();
        }
    }

    /// Get all chat messages
    pub fn messages(&self) -> &[Message] {
        self.app.messages()
    }

    /// Check if UI needs re-rendering
    pub fn is_dirty(&self) -> bool {
        self.app.is_dirty()
    }

    /// Clear dirty flag after rendering
    pub fn clear_dirty(&mut self) {
        self.app.clear_dirty();
    }

    /// Mark UI as needing redraw
    pub fn mark_dirty(&mut self) {
        self.app.mark_dirty();
    }

    /// Check if application should exit
    pub fn should_exit(&self) -> bool {
        self.app.should_exit()
    }

    /// Cycle to the next permission mode
    pub fn cycle_permission_mode(&mut self) -> PermissionMode {
        self.app.cycle_permission_mode()
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: Message) {
        self.app.add_message(message);
    }

    /// Start a new streaming message from assistant
    pub fn begin_streaming_message(&mut self) -> usize {
        self.app.start_streaming_response()
    }

    /// Append text to message at index
    pub fn append_to_message(&mut self, _index: usize, text: &str) {
        self.app.append_streaming_content(text);
    }

    /// Finalize streaming message
    pub fn finalize_streaming_message(&mut self, _index: usize) {
        self.app.finish_streaming();
    }

    /// Set status message
    pub fn set_status(&mut self, status: String) {
        if status.contains("error") || status.contains("Error") {
            self.app.set_error(status);
        } else {
            self.app.clear_error();
        }
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self) -> Result<Option<String>> {
        use super::event::{handle_event, poll_event, EventResult};

        if let Some(event) = poll_event(Duration::from_millis(100))? {
            match handle_event(&mut self.app, event)? {
                EventResult::Continue => Ok(None),
                EventResult::Submit(input) => {
                    // Check if this is a /permissions command
                    if input.trim() == "/permissions" {
                        // Open permissions modal instead of returning input
                        self.app.activate_permissions_modal();
                        Ok(None)
                    } else {
                        Ok(Some(input))
                    }
                }
                EventResult::SaveMemory(memory_text, file_path) => {
                    // Save memory to file
                    self.save_memory_to_file(&memory_text, &file_path)?;
                    Ok(None)
                }
                EventResult::ToggleDebugPane => {
                    // Debug pane already toggled in event handler
                    Ok(None)
                }
                EventResult::ToggleMessage { index: _ } => {
                    // Message collapse state already toggled in event handler
                    Ok(None)
                }
                EventResult::OpenMenu => {
                    // Menu functionality not yet implemented
                    Ok(None)
                }
                EventResult::Resize => {
                    // Terminal resized - update internal buffers and force full redraw
                    self.terminal.autoresize()?;
                    self.terminal.clear()?;
                    self.app.mark_dirty(); // Trigger immediate repaint
                    Ok(None)
                }
                EventResult::Exit => Ok(Some("/exit".to_string())),
            }
        } else {
            Ok(None)
        }
    }

    /// Handle a terminal event (without polling)
    /// Returns Some(input) if user submitted, None otherwise
    pub fn handle_event(&mut self, event: crossterm::event::Event) -> Result<Option<String>> {
        use super::event::{handle_event, EventResult};
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

        // Check if this is a navigation key event before processing
        let is_navigation_key = matches!(
            event,
            crossterm::event::Event::Key(KeyEvent {
                code: KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::PageUp
                    | KeyCode::PageDown
                    | KeyCode::Home
                    | KeyCode::End,
                kind: KeyEventKind::Press,
                ..
            })
        );

        let result = handle_event(&mut self.app, event)?;

        // Only update autocomplete/memory modal if:
        // 1. This was NOT a navigation key (prevents resetting selection during navigation)
        // 2. AND input actually changed (prevents unnecessary updates)
        let current_input = self.app.input().to_string();
        if !is_navigation_key && current_input != self.last_input {
            self.last_input = current_input;
            self.update_autocomplete_if_needed();
            self.update_memory_modal_if_needed();
        }

        match result {
            EventResult::Continue => Ok(None),
            EventResult::Submit(input) => Ok(Some(input)),
            EventResult::SaveMemory(memory_text, file_path) => {
                // Save memory to file
                self.save_memory_to_file(&memory_text, &file_path)?;

                // Add confirmation message
                let truncated = if memory_text.len() > 60 {
                    format!("{}...", &memory_text[..60])
                } else {
                    memory_text.clone()
                };
                let confirmation = format!("💾 Memory saved: \"{}\" → {}", truncated, file_path);
                self.app.add_message(super::Message::system(confirmation));

                Ok(None)
            }
            EventResult::ToggleDebugPane => {
                // Debug pane already toggled in event handler
                Ok(None)
            }
            EventResult::ToggleMessage { index: _ } => {
                // Message collapse state already toggled in event handler
                Ok(None)
            }
            EventResult::OpenMenu => {
                // Menu functionality not yet implemented
                Ok(None)
            }
            EventResult::Resize => {
                // Terminal resized - update internal buffers and force full redraw
                self.terminal.autoresize()?;
                self.terminal.clear()?;
                self.app.mark_dirty(); // Trigger immediate repaint
                Ok(None)
            }
            EventResult::Exit => {
                self.app.exit();
                Ok(None)
            }
        }
    }

    /// Update autocomplete based on current input
    /// Called after any input change to refresh slash command completions
    fn update_autocomplete_if_needed(&mut self) {
        let input = self.app.input();

        // Only show autocomplete for slash commands
        if !input.starts_with('/') {
            if self.app.autocomplete_active() {
                self.app.clear_autocomplete();
            }
            return;
        }

        // Get completions from callback if available
        if let Some(ref callback) = self.completion_callback {
            // Strip the leading '/' before calling completion callback
            // The registry expects command names without the slash prefix
            let prefix = &input[1..]; // Skip the '/'
            let completions = callback(prefix);

            // Convert to CompletionItem format
            let items: Vec<super::CompletionItem> = completions
                .into_iter()
                .map(|(command, desc_or_hint)| super::CompletionItem {
                    command,
                    description: desc_or_hint,
                    argument_hint: None,
                })
                .collect();

            // If we have exactly one match and it equals our input (minus the /),
            // don't show autocomplete - user has completed their selection
            if items.len() == 1 && items[0].command == prefix {
                if self.app.autocomplete_active() {
                    self.app.clear_autocomplete();
                }
                return;
            }

            // Activate autocomplete with filtered items
            self.app.activate_autocomplete(items);
        }
    }

    /// Update memory modal based on current input
    /// Called after any input change to check for memory trigger (#)
    fn update_memory_modal_if_needed(&mut self) {
        let input = self.app.input();

        // Clear modal if input no longer starts with '#'
        if !input.starts_with('#') {
            if self.app.memory_modal_active() {
                self.app.clear_memory_modal();
            }
            return;
        }

        // Extract memory text (everything after #, trimmed)
        let memory_text = input[1..].trim().to_string();

        // Modal should appear immediately when user types '#'
        if !self.app.memory_modal_active() {
            // First time - activate modal (sets selection to 0)
            let destinations = self.discover_memory_destinations();
            self.app.activate_memory_modal(memory_text, destinations);
        } else {
            // Modal already active - just update the text (preserves selection)
            self.app.update_memory_text(memory_text);
        }
    }

    /// Discover available memory destinations
    fn discover_memory_destinations(&self) -> Vec<super::MemoryDestination> {
        let mut destinations = Vec::new();

        // 1. User memory (~/.claude/CLAUDE.md)
        if let Some(home) = std::env::var("HOME")
            .ok()
            .or_else(|| std::env::var("USERPROFILE").ok())
        {
            let user_memory_path = format!("{}/.claude/CLAUDE.md", home);
            destinations.push(super::MemoryDestination {
                name: "User memory".to_string(),
                file_path: user_memory_path.clone(),
                description: Some(format!("Saved in {}", user_memory_path)),
                is_imported: false,
            });
        }

        // 2. Project memory (./CLAUDE.md)
        let project_memory_path = "./CLAUDE.md".to_string();
        destinations.push(super::MemoryDestination {
            name: "Project memory".to_string(),
            file_path: project_memory_path.clone(),
            description: Some("Checked in at ./CLAUDE.md".to_string()),
            is_imported: false,
        });

        // 3. Imported context files (.claude/context/*.md)
        if let Ok(entries) = std::fs::read_dir(".claude/context") {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Some(path_str) = entry.path().to_str() {
                            if path_str.ends_with(".md") {
                                let file_name = entry.file_name();
                                let file_name_str = file_name.to_string_lossy();
                                destinations.push(super::MemoryDestination {
                                    name: file_name_str.to_string(),
                                    file_path: path_str.to_string(),
                                    description: Some("@-imported".to_string()),
                                    is_imported: true,
                                });
                            }
                        }
                    }
                }
            }
        }

        destinations
    }

    /// Save memory to file
    fn save_memory_to_file(&self, memory_text: &str, file_path: &str) -> Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        // Get current timestamp
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

        // Format memory entry (inline format: ## Memory - TIMESTAMP - TEXT)
        let memory_entry = format!("\n## Memory - {} - {}\n", timestamp, memory_text);

        // Append to file (create if doesn't exist)
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;

        file.write_all(memory_entry.as_bytes())?;

        Ok(())
    }

    /// Draw the TUI
    pub fn draw(&mut self) -> Result<()> {
        // Render and capture max_scroll and layout_cache for app state update
        let mut max_scroll = 0;
        let mut debug_max_scroll = 0;
        let mut layout_cache = super::focus_manager::LayoutCache::default();
        self.terminal.draw(|f| {
            let (scroll, debug_scroll, cache) = super::ui::render(f, &mut self.app);
            max_scroll = scroll;
            debug_max_scroll = debug_scroll;
            layout_cache = cache;
        })?;

        // Update app's max_scroll, debug_max_scroll, and layout_cache so scroll operations can clamp properly
        // and focus system can perform hit testing
        self.app.update_max_scroll(max_scroll);
        self.app.update_debug_max_scroll(debug_max_scroll);
        self.app.update_layout_cache(layout_cache);

        Ok(())
    }

    /// Push a debug message to the debug panel
    pub fn push_debug(&mut self, message: String) {
        self.app.push_debug_message(message);
    }

    /// Update token count during streaming
    pub fn update_token_count(&mut self, input: u32, output: u32) {
        self.app.update_token_count(input, output);
    }

    /// Start extended thinking phase
    pub fn start_extended_thinking(&mut self) {
        self.app.start_extended_thinking();
    }

    /// Note transition to receiving thinking content
    pub fn append_thinking_content(&mut self) {
        self.app.append_thinking_content();
    }

    /// Stop extended thinking phase
    pub fn stop_extended_thinking(&mut self) {
        self.app.stop_extended_thinking();
    }

    /// Check if currently streaming
    pub fn is_streaming(&self) -> bool {
        self.app.is_streaming()
    }

    /// Begin a new tool execution message
    pub fn begin_tool_message(
        &mut self,
        tool_id: String,
        tool_name: String,
        params: serde_json::Value,
    ) -> usize {
        self.app.begin_tool_message(tool_id, tool_name, params)
    }

    /// Finalize a tool execution message with result
    pub fn finalize_tool_message(&mut self, tool_id: &str, result: crate::tui::ToolResult) {
        self.app.finalize_tool_message(tool_id, result);
    }

    /// Check if any tools are currently executing
    pub fn has_active_tools(&self) -> bool {
        self.app.has_active_tools()
    }

    /// Get name of active tool (for status bar compatibility)
    pub fn active_tool_name(&self) -> Option<String> {
        self.app.active_tool_name()
    }

    /// Activate autocomplete with given items
    pub fn activate_autocomplete(&mut self, items: Vec<super::CompletionItem>) {
        self.app.activate_autocomplete(items);
    }

    /// Clear autocomplete
    pub fn clear_autocomplete(&mut self) {
        self.app.clear_autocomplete();
    }

    /// Check if autocomplete is active
    pub fn autocomplete_active(&self) -> bool {
        self.app.autocomplete_active()
    }

    /// Cleanup terminal (idempotent - safe to call multiple times)
    pub fn cleanup(&mut self) -> Result<()> {
        use crossterm::terminal::Clear;
        use crossterm::terminal::ClearType;

        // Idempotent - only cleanup once
        if self.cleaned_up {
            return Ok(());
        }

        // Show cursor first (may have been hidden)
        let _ = self.terminal.show_cursor();

        // Clear the current screen before leaving
        let _ = execute!(self.terminal.backend_mut(), Clear(ClearType::All));

        // Disable raw mode (restore terminal input processing)
        let _ = disable_raw_mode();

        // CRITICAL: Leave alternate screen to restore terminal history
        // This MUST succeed for proper cleanup
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;

        // Mark as cleaned up
        self.cleaned_up = true;

        Ok(())
    }
}

impl Drop for TuiState {
    fn drop(&mut self) {
        // Best effort cleanup
        let _ = self.cleanup();
    }
}
