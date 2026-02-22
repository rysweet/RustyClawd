//! Rendering layer for TUI - orchestrates rendering by delegating to sub-modules
//!
//! This module coordinates rendering of the TUI by delegating to:
//! - `render_status_bar`: Status bar with mode indicator, streaming status
//! - `render_messages`: Message panel with scroll, tool results, click regions
//! - `render_popups`: Autocomplete, memory modal, permissions modal overlays
//! - `render_debug`: Debug panel with scrollable log viewer
//! - `message_formatter`: User/assistant/system message formatting
//! - `tool_renderer`: Tool call and JSON parameter rendering

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::tui::app::{App, LayoutCache};
use crate::tui::layout::{LayoutConfig, LayoutOrganizer};

use super::render_debug::render_debug_panel;
use super::render_messages::render_messages;
use super::render_popups::{render_autocomplete, render_memory_modal, render_permissions_modal};
use super::render_status_bar::render_status_bar;

/// Rust-themed colors for structural elements (borders, titles)
pub(super) const RUST_ORANGE: Color = Color::Rgb(222, 165, 132);

/// Main render function - updates TextArea block style for focus-aware rendering
/// Returns (max_scroll, debug_max_scroll, layout_cache) tuple for app state update
pub fn render(frame: &mut Frame, app: &mut App) -> (usize, usize, LayoutCache) {
    // Calculate throbber frame ONCE per render to ensure synchronization
    // across all UI components (status bar, streaming indicators, etc.)
    // Inverted Braille pattern: all dots filled except one moving gap
    const BRAILLE_FRAMES: [char; 8] = ['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];
    let frame_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default() // OK to use default: system time always available on supported platforms
        .as_millis()
        / 100) as usize
        % BRAILLE_FRAMES.len();
    let throbber = BRAILLE_FRAMES[frame_idx];

    // Build layout configuration from app state
    let config = LayoutConfig {
        debug_visible: app.debug_visible(),
        debug_width: 60,
    };

    // Organize layout dynamically
    let layout = LayoutOrganizer::organize(frame.area(), &config);

    // Render status bar (pass throbber for synchronization)
    render_status_bar(frame, layout.status, app, throbber);

    // Split main area into messages + input (dynamic input height based on line count)
    let (messages_area, input_area) = LayoutOrganizer::split_main(layout.main, app);

    // Render main content (pass throbber for synchronization)
    let max_scroll = render_messages(frame, messages_area, app, throbber);

    // Update TextArea block style for focus-aware border color
    app.update_input_focus_style();

    // Update soft wrap width based on input area (accounting for borders: -2)
    let inner_width = input_area.width.saturating_sub(2);
    app.update_soft_wrap_width(inner_width);

    render_input(frame, input_area, app);

    // Render autocomplete popup if active (after input so it overlays)
    if app.autocomplete_active() {
        render_autocomplete(frame, input_area, app);
    }

    // Render memory modal if active (after autocomplete)
    if app.memory_modal_active() {
        render_memory_modal(frame, input_area, app);
    }

    // Render permissions modal if active (highest priority overlay)
    if app.permissions_modal_active() {
        render_permissions_modal(frame, frame.area(), app);
    }

    // Render debug panel if visible
    let debug_max_scroll = if let Some(debug_area) = layout.debug {
        render_debug_panel(frame, debug_area, app)
    } else {
        0
    };

    // Build layout cache for focus hit testing
    let cache = LayoutCache {
        messages_area,
        input_area,
        debug_area: layout.debug,
    };

    // Return max_scroll, debug_max_scroll, and layout cache for app state update
    (max_scroll, debug_max_scroll, cache)
}

fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    // Focus-aware styling (scrollbar only for now - TextArea block styling requires mutable access)
    let is_focused = app.focus_input().get();
    let scrollbar_color = if is_focused {
        Color::White
    } else {
        RUST_ORANGE
    };

    frame.render_widget(&app.input_state.input, area);

    // Render scrollbar only when content actually exceeds viewport
    let content_lines = app.input_line_count();
    // Calculate actual viewport height (area height - 2 for borders)
    let viewport_lines = area.height.saturating_sub(2) as usize;

    // Only show scrollbar if content overflows viewport
    if content_lines > viewport_lines {
        let cursor_pos = app.cursor_pos().0; // Row position
        let max_scroll = content_lines.saturating_sub(viewport_lines);

        let mut scrollbar_state =
            ScrollbarState::new(max_scroll).position(cursor_pos.saturating_sub(viewport_lines / 2));

        // Render scrollbar on right edge of input area (inside the border)
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(scrollbar_color))
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        frame.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut scrollbar_state,
        );
    }

    // Cursor is handled automatically by TextArea
    // No manual cursor positioning needed
}
