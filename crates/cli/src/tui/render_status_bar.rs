//! Status bar rendering - mode indicator, streaming status, debug toggle
//!
//! Extracted from ui.rs to keep each rendering module under 300 LOC.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use unicode_width::UnicodeWidthStr;

use super::app::App;
use super::ui::RUST_ORANGE;

pub(super) fn render_status_bar(frame: &mut Frame, area: Rect, app: &mut App, throbber: char) {
    // Throbber is now passed in from render() to ensure synchronization

    // Read all app state FIRST before any mutations
    let mode_text = format!(" {} ", app.permission_mode().status_indicator());
    let error = app.error().map(|s| s.to_string());
    let has_active_tools = app.has_active_tools();
    let active_tool_name = app.active_tool_name();
    let is_streaming = app.is_streaming();
    let is_thinking = app.is_thinking();
    let is_extended_thinking = app.is_extended_thinking();
    let thinking_duration = app.thinking_duration();
    let token_count = app.token_count();
    let debug_visible = app.debug_visible();
    let follow_bottom = app.follow_bottom();

    let mode_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let mut status_spans = if let Some(error) = error {
        vec![
            Span::styled(mode_text, mode_style),
            Span::raw(" "),
            Span::styled(error, Style::default().fg(Color::Red)),
        ]
    } else if has_active_tools {
        // Show throbber + tool name when tool is executing
        let tool_name = active_tool_name.unwrap_or_else(|| "tool".to_string());
        let status_text = format!("{} Executing: {}", throbber, tool_name);

        vec![
            Span::styled(mode_text.clone(), mode_style),
            Span::raw(" "),
            Span::styled(status_text, Style::default().fg(Color::Magenta)),
        ]
    } else if is_streaming {
        // Show throbber + token count when streaming
        let status_text = if is_extended_thinking {
            // Extended thinking mode - show shimmer indicator with duration
            crate::tui::thinking_indicator::render_thinking_indicator(thinking_duration)
        } else if is_thinking {
            // Basic thinking mode - show throbber without token count
            format!("{} Thinking...", throbber)
        } else if let Some(token_count) = token_count {
            // Streaming mode - show throbber with live token count
            format!("{} Streaming  {}", throbber, token_count.format_compact())
        } else {
            // Fallback
            format!("{} Streaming...", throbber)
        };

        let status_color = if is_extended_thinking {
            Color::Magenta // Use magenta for extended thinking to distinguish from regular streaming
        } else {
            Color::Yellow
        };

        vec![
            Span::styled(mode_text.clone(), mode_style),
            Span::raw(" "),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]
    } else {
        vec![
            Span::styled(mode_text, mode_style),
            Span::raw(" "),
            Span::styled(
                "🦀 RustyClawd",
                Style::default()
                    .fg(RUST_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]
    };

    // Add debug indicator, follow-bottom indicator, and menu hint on the right
    let debug_status = if debug_visible {
        "Debug:ON"
    } else {
        "Debug:OFF"
    };
    let debug_style = if debug_visible {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };

    // Follow-bottom indicator: show "📌 Pinned" when user has scrolled up (NOT following bottom)
    let follow_indicator = if !follow_bottom { " | 📌 Pinned" } else { "" };

    // Calculate padding to right-align - Unicode display width
    let left_width: usize = status_spans.iter().map(|s| s.content.width()).sum();
    let right_text = format!(" | {}{} | F1:Menu ", debug_status, follow_indicator);
    let padding_width = area
        .width
        .saturating_sub((left_width + right_text.width()) as u16);

    status_spans.push(Span::raw(" ".repeat(padding_width as usize)));
    status_spans.push(Span::raw(" | "));

    // Track debug button click region
    let debug_x = left_width + padding_width as usize + 3; // " | " is 3 chars
    let debug_rect = Rect {
        x: area.x + debug_x as u16,
        y: area.y,
        width: debug_status.width() as u16,
        height: 1,
    };
    app.click_regions.add_status_item("debug", debug_rect);

    status_spans.push(Span::styled(debug_status, debug_style));
    if !follow_bottom {
        status_spans.push(Span::styled(
            " | 📌 Pinned",
            Style::default().fg(Color::Yellow),
        ));
    }

    // Track menu button click region
    let menu_text = " | F1:Menu ";
    let menu_x = area.width.saturating_sub(menu_text.width() as u16);
    let menu_rect = Rect {
        x: area.x + menu_x,
        y: area.y,
        width: menu_text.width() as u16,
        height: 1,
    };
    app.click_regions.add_status_item("menu", menu_rect);

    status_spans.push(Span::styled(menu_text, Style::default().fg(Color::Gray)));

    let status = Paragraph::new(Line::from(status_spans)).style(Style::default().bg(Color::Black));

    frame.render_widget(status, area);
}
