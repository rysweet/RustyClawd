//! Terminal User Interface (TUI) for RustyClawd
//!
//! Clean ratatui architecture with separation of concerns:
//! - `app`: Application state management
//! - `ui`: Pure rendering functions
//! - `event`: Event handling and input processing
//! - `message`: Message types and formatting
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │          Event Loop                 │
//! │  Poll → Handle → Render → Repeat    │
//! └─────────────────────────────────────┘
//!        │         │         │
//!        ▼         ▼         ▼
//!   Crossterm    App      Ratatui
//!    Events     State     Widgets
//! ```

mod app;
mod autocomplete_state;
mod click_region;
mod compat;
mod debug_panel;
mod event;
mod keybindings;
mod layout;
mod message;
mod message_formatter;
mod modal_state;
mod soft_wrap;
mod thinking_indicator;
mod thinking_state;
mod token_counter;
mod tool_messages;
mod tool_renderer;
mod ui;

// Re-export for backward compatibility with existing code
pub use app::{App, CompletionItem, LayoutCache, ToolResult};
pub use event::{handle_event, poll_event, EventResult};
pub use message::Message;
#[allow(unused_imports)] // Re-export for library consumers and tests
pub use message::Role as MessageRole;
pub use modal_state::MemoryDestination;
pub use ui::render;

// Legacy exports (kept for compatibility with interactive.rs)
pub use compat::TuiState;
pub use message::Message as ChatMessage;

// Extended thinking exports (for testing)
#[allow(unused_imports)] // Re-export for library consumers and tests
pub use thinking_state::{ThinkingPhase, ThinkingState};

// New clean API
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

// Re-export PermissionMode from parent
pub use crate::permission_mode::PermissionMode;

/// Initialize terminal for TUI mode
pub fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore terminal to normal mode
pub fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Main TUI event loop
pub fn run_event_loop<F>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    mut on_submit: F,
) -> Result<()>
where
    F: FnMut(&str) -> Result<()>,
{
    // Helper function to render and update app state
    let do_render =
        |terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, app: &mut App| -> Result<()> {
            let mut max_scroll = 0;
            let mut debug_max_scroll = 0;
            let mut layout_cache = LayoutCache::default();
            terminal.draw(|f| {
                let (scroll, debug_scroll, cache) = render(f, app);
                max_scroll = scroll;
                debug_max_scroll = debug_scroll;
                layout_cache = cache;
            })?;
            app.update_max_scroll(max_scroll);
            app.update_debug_max_scroll(debug_max_scroll);
            app.update_layout_cache(layout_cache);
            Ok(())
        };

    loop {
        // Render current state
        do_render(terminal, app)?;

        // Poll for events (16ms for 60 FPS)
        if let Some(event) = poll_event(Duration::from_millis(16))? {
            let state_changed = match handle_event(app, event)? {
                EventResult::Continue => true, // Assume state changed (typing, navigation, etc.)
                EventResult::Submit(input) => {
                    app.add_message(Message::user(input.clone()));
                    on_submit(&input)?;
                    true
                }
                EventResult::SaveMemory(memory_text, file_path) => {
                    // Save memory to file
                    use std::fs::OpenOptions;
                    use std::io::Write;
                    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                    let memory_entry = format!("\n## Memory - {} - {}\n", timestamp, memory_text);
                    let mut file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&file_path)?;
                    file.write_all(memory_entry.as_bytes())?;
                    true
                }
                EventResult::ToggleDebugPane => {
                    // Toggle debug pane visibility
                    app.toggle_debug();
                    true
                }
                EventResult::ToggleMessage { index: _ } => {
                    // Message collapse state already updated in handle_mouse_event
                    // Just need to trigger repaint
                    true
                }
                EventResult::OpenMenu => {
                    // Menu functionality not yet implemented
                    // For now, just continue (placeholder for future)
                    true
                }
                EventResult::Resize => {
                    // Terminal resized - update internal buffers and force full redraw
                    terminal.autoresize()?;
                    terminal.clear()?;
                    true // Force immediate repaint
                }
                EventResult::Exit => {
                    break;
                }
            };

            // CRITICAL: Render immediately after state change for instant feedback
            if state_changed {
                do_render(terminal, app)?;
            }
        }

        // Check exit flag
        if app.should_exit() {
            break;
        }
    }

    Ok(())
}

/// Setup panic hook for clean terminal restore
pub fn setup_panic_hook() {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best effort terminal restore
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_panic(info);
    }));
}
