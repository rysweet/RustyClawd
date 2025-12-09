//! Rendering layer for TUI - pure functions, no state mutation

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::permission_mode::PermissionMode;
use crate::tui::app::App;
use crate::tui::layout::{LayoutConfig, LayoutOrganizer};
use crate::tui::message::{Message, Role};

/// Rust-themed colors for structural elements (borders, titles)
const RUST_ORANGE: Color = Color::Rgb(222, 165, 132);

/// Main render function - pure function, no state mutation
pub fn render(frame: &mut Frame, app: &App) {
    // Build layout configuration from app state
    let config = LayoutConfig {
        debug_visible: app.debug_visible(),
        debug_width: 60,
    };

    // Organize layout dynamically
    let layout = LayoutOrganizer::organize(frame.area(), &config);

    // Render status bar
    render_status_bar(frame, layout.status, app);

    // Split main area into messages + input
    let (messages_area, input_area) = LayoutOrganizer::split_main(layout.main);

    // Render main content
    render_messages(frame, messages_area, app);
    render_input(frame, input_area, app);

    // Render debug panel if visible
    if let Some(debug_area) = layout.debug {
        render_debug_panel(frame, debug_area, app);
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let mode_text = format!(" {} ", app.permission_mode().status_indicator());

    let mode_style = Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let mut status_spans = if let Some(error) = app.error() {
        vec![
            Span::styled(mode_text, mode_style),
            Span::raw(" "),
            Span::styled(error, Style::default().fg(Color::Red)),
        ]
    } else if app.is_streaming() {
        vec![
            Span::styled(mode_text, mode_style),
            Span::raw(" "),
            Span::styled("⚡ Streaming...", Style::default().fg(Color::Yellow)),
        ]
    } else {
        vec![
            Span::styled(mode_text, mode_style),
            Span::raw(" "),
            Span::styled(
                "🦀 RustyClawd",
                Style::default().fg(RUST_ORANGE).add_modifier(Modifier::BOLD),
            ),
        ]
    };

    // Add debug indicator and menu hint on the right
    let debug_status = if app.debug_visible() { "Debug:ON" } else { "Debug:OFF" };
    let debug_style = if app.debug_visible() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };

    // Calculate padding to right-align
    let left_width: usize = status_spans.iter().map(|s| s.content.len()).sum();
    let right_text = format!(" | {} | F1:Menu ", debug_status);
    let padding_width = area.width.saturating_sub((left_width + right_text.len()) as u16);

    status_spans.push(Span::raw(" ".repeat(padding_width as usize)));
    status_spans.push(Span::raw(" | "));
    status_spans.push(Span::styled(debug_status, debug_style));
    status_spans.push(Span::styled(" | F1:Menu ", Style::default().fg(Color::Gray)));

    let status = Paragraph::new(Line::from(status_spans)).style(Style::default().bg(Color::Black));

    frame.render_widget(status, area);
}

fn render_messages(frame: &mut Frame, area: Rect, app: &App) {
    let messages = app.messages();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(RUST_ORANGE))
        .title(vec![
            Span::styled("💬 ", Style::default().fg(RUST_ORANGE)),
            Span::styled(
                "Messages",
                Style::default()
                    .fg(RUST_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

    if messages.is_empty() {
        // Welcome screen
        let welcome_text = Text::from(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Welcome to RustyClawd! ",
                    Style::default()
                        .fg(RUST_ORANGE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("🦀", Style::default().fg(RUST_ORANGE)),
            ]),
            Line::from(""),
            Line::from("Type your message and press Enter to chat with Claude."),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Controls: ",
                    Style::default()
                        .fg(RUST_ORANGE)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Enter=Send | Ctrl+C/Ctrl+D=Exit | ↑↓=Scroll | Shift+Tab=Mode | F1=Debug",
                    Style::default().fg(Color::Gray),
                ),
            ]),
        ]);

        let paragraph = Paragraph::new(welcome_text)
            .block(block);

        frame.render_widget(paragraph, area);
    } else {
        // Build complete text content as styled text
        let mut text_lines = Vec::new();

        for message in messages {
            // Add message header
            let header_style = match message.role {
                Role::User => Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
                Role::Assistant => Style::default()
                    .fg(RUST_ORANGE)
                    .add_modifier(Modifier::BOLD),
                Role::System => Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            };

            let mut header_spans = vec![Span::styled(message.format_header(), header_style)];
            if message.streaming {
                header_spans.push(Span::styled(
                    " [streaming]",
                    Style::default().fg(Color::Yellow),
                ));
            }
            text_lines.push(Line::from(header_spans));

            // Add message content - preserve exact text structure
            if !message.content.is_empty() {
                // System messages: compact format
                if message.role == Role::System {
                    let content = message.content.replace('\n', " ");
                    text_lines.push(Line::from(Span::styled(
                        format!("    → {}", content),
                        Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
                    )));
                } else {
                    // Regular messages: preserve exact structure, plain text
                    for line in message.content.lines() {
                        text_lines.push(Line::from(line.to_string()));
                    }
                }
            }

            // Blank separator
            text_lines.push(Line::from(""));
        }

        let text = Text::from(text_lines);

        // IMPORTANT: When using Wrap, we cannot accurately calculate scroll offset
        // based on logical lines because wrapping creates MORE rendered lines.
        // Solution: Only apply scroll when user explicitly scrolls UP (not at bottom).
        // Otherwise, let Paragraph naturally show the end of content.

        let scroll_offset = if app.scroll_offset() == usize::MAX {
            // User is at "bottom" - show content from top (no scroll)
            // Content will naturally fill from top, showing most recent at bottom
            0
        } else {
            // User scrolled up - use their offset directly
            app.scroll_offset().min(u16::MAX as usize) as u16
        };

        let paragraph = Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: false })  // Wraps at widget's inner width automatically
            .scroll((scroll_offset, 0));

        frame.render_widget(paragraph, area);
    }
}

fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    let input = app.input();
    let cursor_pos = app.cursor_pos();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(RUST_ORANGE))
        .title(vec![
            Span::styled("✏️  ", Style::default().fg(RUST_ORANGE)),
            Span::styled(
                "Input",
                Style::default()
                    .fg(RUST_ORANGE)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

    let inner = block.inner(area);

    // Render block first
    frame.render_widget(block, area);

    // Simple input rendering - use terminal default colors
    let paragraph = Paragraph::new(input);

    frame.render_widget(paragraph, inner);

    // Set cursor position
    let cursor_char_pos = input[..cursor_pos].chars().count();
    frame.set_cursor_position((inner.x + cursor_char_pos as u16, inner.y));
}

fn render_debug_panel(frame: &mut Frame, area: Rect, app: &App) {
    let messages = app.debug_messages();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(vec![
            Span::styled("🔍 ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Debug Panel",
                Style::default()
                    .fg(Color::Yellow)
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

    let text = Text::from(text_lines);

    // Auto-scroll to bottom to show latest debug messages
    // Show from top (scroll = 0) to let content fill naturally
    let scroll_offset = 0;

    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false })  // Wrap at widget's inner width
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, area);
}