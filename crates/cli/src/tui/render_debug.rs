//! Debug panel rendering - scrollable debug log viewer
//!
//! Extracted from ui.rs to keep each rendering module under 300 LOC.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use super::app::App;
use super::message_formatter::calculate_wrapped_height;

pub(super) fn render_debug_panel(frame: &mut Frame, area: Rect, app: &App) -> usize {
    let messages = app.debug_messages();

    // Focus-aware border styling
    let is_focused = app.focus_debug().get();
    let border_color = if is_focused {
        Color::White
    } else {
        Color::Yellow
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(vec![
            Span::styled("🔍 ", Style::default().fg(border_color)),
            Span::styled(
                "Debug Panel",
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({} messages)", messages.len()),
                Style::default().fg(Color::Gray),
            ),
        ]);

    // Build text with automatic wrapping based on widget width
    let mut text_lines = Vec::new();
    for msg in messages {
        text_lines.push(Line::from(Span::styled(
            msg.as_str(),
            Style::default().fg(Color::Gray),
        )));
    }

    // Calculate content height accounting for wrapping
    let content_width = area.width.saturating_sub(2) as usize; // Subtract borders
    let content_height = calculate_wrapped_height(&text_lines, content_width);

    let text = Text::from(text_lines);

    // Calculate viewport height and max_scroll
    let viewport_height = area.height.saturating_sub(2) as usize; // Subtract borders
                                                                  // DEFENSIVE: Ensure viewport has minimum height
    if viewport_height == 0 {
        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
        return 0;
    }

    let max_scroll = content_height.saturating_sub(viewport_height);

    // Determine scroll offset based on follow_bottom mode (same as message panel)
    // DEFENSIVE: Handle very large content (>65535 lines)
    let scroll_offset = if app.debug_follow_bottom() {
        // Auto-follow bottom - show last viewport worth of content
        if max_scroll > u16::MAX as usize {
            tracing::warn!(
                "Debug content height {} exceeds scroll limit, bottom may be cut off",
                content_height
            );
            u16::MAX
        } else {
            max_scroll as u16
        }
    } else {
        // Manual scroll - clamp to valid range [0, max_scroll]
        let clamped = app.debug_scroll_offset().min(max_scroll);
        if clamped > u16::MAX as usize {
            tracing::warn!(
                "Debug scroll offset {} exceeds u16 limit, clamping to {}",
                clamped,
                u16::MAX
            );
            u16::MAX
        } else {
            clamped as u16
        }
    };

    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false }) // Wrap at widget's inner width
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, area);

    // Render scrollbar if content overflows
    if content_height > viewport_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .style(Style::default().fg(Color::Yellow));

        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll_offset as usize);

        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }

    max_scroll
}
