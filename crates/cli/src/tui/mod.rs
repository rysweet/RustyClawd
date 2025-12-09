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
mod compat;
mod event;
mod keybindings;
mod layout;
mod message;
mod token_counter;
mod ui;

// Re-export for backward compatibility with existing code
pub use app::App;
pub use event::{handle_event, poll_event, EventResult};
pub use message::{Message, Role};
pub use ui::render;

// Legacy exports (kept for compatibility with interactive.rs)
pub use compat::TuiState;
pub use message::Message as ChatMessage;
pub use message::Role as MessageRole;

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
    loop {
        // Render current state
        terminal.draw(|f| render(f, app))?;

        // Poll for events (16ms for 60 FPS)
        if let Some(event) = poll_event(Duration::from_millis(16))? {
            match handle_event(app, event)? {
                EventResult::Continue => {}
                EventResult::Submit(input) => {
                    app.add_message(Message::user(input.clone()));
                    on_submit(&input)?;
                }
                EventResult::Exit => {
                    break;
                }
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
