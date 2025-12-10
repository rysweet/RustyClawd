//! Rendering layer for TUI - pure functions, no state mutation

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use crate::permission_mode::PermissionMode;
use crate::tui::app::App;
use crate::tui::layout::{LayoutConfig, LayoutOrganizer};
use crate::tui::message::{Message, Role};

/// Rust-themed colors for structural elements (borders, titles)
const RUST_ORANGE: Color = Color::Rgb(222, 165, 132);

/// Main render function - pure function, no state mutation
/// Returns max_scroll value for app state update
pub fn render(frame: &mut Frame, app: &App) -> usize {
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
    let max_scroll = render_messages(frame, messages_area, app);
    render_input(frame, input_area, app);

    // Render autocomplete popup if active (after input so it overlays)
    if app.autocomplete_active() {
        render_autocomplete(frame, input_area, app);
    }

    // Render debug panel if visible
    if let Some(debug_area) = layout.debug {
        render_debug_panel(frame, debug_area, app);
    }

    // Return max_scroll for app state update
    max_scroll
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    // BRAILLE_SIX throbber pattern (Unicode Braille)
    const BRAILLE_FRAMES: &[&str] = &["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"];

    // Use system time to rotate throbber (simple, stateless)
    let frame_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() / 100) as usize % BRAILLE_FRAMES.len();
    let throbber = BRAILLE_FRAMES[frame_idx];

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
    } else if app.has_active_tools() {
        // Show throbber + tool name when tool is executing
        let tool_name = app.active_tool_name().unwrap_or_else(|| "tool".to_string());
        let status_text = format!("{} Executing: {}", throbber, tool_name);

        vec![
            Span::styled(mode_text, mode_style),
            Span::raw(" "),
            Span::styled(status_text, Style::default().fg(Color::Magenta)),
        ]
    } else if app.is_streaming() {
        // Show throbber + token count when streaming
        let status_text = if app.is_thinking() {
            // Thinking mode - show throbber without token count
            format!("{} Thinking...", throbber)
        } else if let Some(token_count) = app.token_count() {
            // Streaming mode - show throbber with live token count
            format!("{} Streaming  {}", throbber, token_count.format_compact())
        } else {
            // Fallback
            format!("{} Streaming...", throbber)
        };

        vec![
            Span::styled(mode_text, mode_style),
            Span::raw(" "),
            Span::styled(status_text, Style::default().fg(Color::Yellow)),
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

/// Generate animated throbber character (Braille patterns)
fn generate_throbber() -> char {
    const BRAILLE_FRAMES: [char; 8] = ['⠁', '⠂', '⠄', '⡀', '⢀', '⠠', '⠐', '⠈'];

    let frame_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() / 100) as usize % BRAILLE_FRAMES.len();

    BRAILLE_FRAMES[frame_idx]
}

/// Format a tool call for display (compact JSON parameters)
fn format_tool_params(params: &serde_json::Value) -> String {
    match params {
        serde_json::Value::Object(map) if map.is_empty() => "{}".to_string(),
        serde_json::Value::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .take(3) // Show max 3 params
                .map(|(k, v)| {
                    let value_str = match v {
                        serde_json::Value::String(s) if s.len() > 30 => {
                            format!("\"{}...\"", &s[..27])
                        }
                        serde_json::Value::String(s) => format!("\"{}\"", s),
                        _ => v.to_string(),
                    };
                    format!("{}: {}", k, value_str)
                })
                .collect();

            if map.len() > 3 {
                format!("{{ {}, ... }}", items.join(", "))
            } else {
                format!("{{ {} }}", items.join(", "))
            }
        }
        _ => params.to_string(),
    }
}

/// Truncate output for display (keep last N chars)
fn truncate_output(output: &str, max_chars: usize) -> String {
    if output.len() <= max_chars {
        output.to_string()
    } else {
        format!("...{}", &output[output.len() - max_chars..])
    }
}

fn render_messages(frame: &mut Frame, area: Rect, app: &App) -> usize {
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

        // No scrollable content on welcome screen
        return 0;
    } else {
        // Build complete text content as styled text
        let mut text_lines = Vec::new();

        for (msg_idx, message) in messages.iter().enumerate() {
            // Check if this is a tool message (dynamic rendering)
            if message.role == Role::System {
                if let Some((_tool_id, tool_state)) = app.tool_message_by_index(msg_idx) {
                    // This is a tool execution message - render dynamically
                    let elapsed = tool_state.start_time.elapsed().as_secs();

                    if tool_state.completed {
                        // Tool completed - show final result
                        if let Some(ref result) = tool_state.result {
                            let icon = if result.is_error { "✗" } else { "✓" };
                            let icon_color = if result.is_error { Color::Red } else { Color::Green };

                            // Header: "✓ Bash { command: "ls -la" } (2s)"
                            // Icon is colored, rest is dark grey
                            let header_text = format!(
                                " {} {} ({}s)",
                                tool_state.tool_name,
                                format_tool_params(&tool_state.params),
                                elapsed
                            );
                            text_lines.push(Line::from(vec![
                                Span::styled(icon, Style::default().fg(icon_color).add_modifier(Modifier::BOLD)),
                                Span::styled(header_text, Style::default().fg(Color::DarkGray)),
                            ]));

                            // Result: exit code + stdout (truncated)
                            if let Some(exit_code) = result.exit_code {
                                text_lines.push(Line::from(Span::styled(
                                    format!("    exit_code: {}", exit_code),
                                    Style::default().fg(Color::DarkGray),
                                )));
                            }

                            if !result.stdout.is_empty() {
                                let truncated = truncate_output(&result.stdout, 200);
                                text_lines.push(Line::from(Span::styled(
                                    format!("    stdout: {}", truncated),
                                    Style::default().fg(Color::DarkGray),
                                )));
                            }

                            if !result.stderr.is_empty() {
                                let truncated = truncate_output(&result.stderr, 200);
                                text_lines.push(Line::from(Span::styled(
                                    format!("    stderr: {}", truncated),
                                    Style::default().fg(Color::DarkGray),
                                )));
                            }
                        }
                    } else {
                        // Tool still running - show throbber + timer
                        // Throbber is colored, rest is dark grey
                        let throbber = generate_throbber();
                        let header_text = format!(
                            " {} {} ({}s)",
                            tool_state.tool_name,
                            format_tool_params(&tool_state.params),
                            elapsed
                        );
                        text_lines.push(Line::from(vec![
                            Span::styled(
                                throbber.to_string(),
                                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
                            ),
                            Span::styled(header_text, Style::default().fg(Color::DarkGray)),
                        ]));
                    }

                    // Blank separator
                    text_lines.push(Line::from(""));
                    continue; // Skip normal message rendering
                }
            }

            // Normal message rendering (non-tool messages)
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

        // Calculate ACTUAL content height accounting for line wrapping
        // Ratatui's Paragraph with wrap() will wrap lines at widget width
        // Subtract 2 for block borders to get actual content width
        let content_width = area.width.saturating_sub(2) as usize;

        // Count actual rendered lines (accounting for wrapping)
        let mut content_height = 0;
        for line in &text_lines {
            // Calculate how many screen lines this Line will take when wrapped
            let line_width: usize = line.spans.iter().map(|span| span.content.len()).sum();
            if line_width == 0 {
                content_height += 1; // Empty line still takes 1 line
            } else {
                // Calculate wrapped lines: ceil(line_width / content_width)
                content_height += (line_width + content_width - 1) / content_width;
            }
        }

        let text = Text::from(text_lines);

        // Scroll handling: Calculate actual scroll position
        // Subtract 2 for block borders
        let viewport_height = area.height.saturating_sub(2) as usize;

        // Calculate max scroll with proper boundary check
        // If content fits in viewport, max_scroll is 0 (no scrolling needed)
        let max_scroll = if content_height > viewport_height {
            content_height - viewport_height
        } else {
            0
        };

        // Determine scroll offset
        let scroll_offset = if app.follow_bottom() {
            // Auto-follow bottom - show last viewport worth of content
            // CRITICAL: When following bottom, ALWAYS use max_scroll to show latest messages
            max_scroll.min(u16::MAX as usize) as u16
        } else {
            // Manual scroll - clamp to valid range [0, max_scroll]
            let clamped = app.scroll_offset().min(max_scroll);
            clamped.min(u16::MAX as usize) as u16
        };

        let paragraph = Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: false })  // Wraps at widget's inner width automatically
            .scroll((scroll_offset, 0));

        frame.render_widget(paragraph, area);

        // Render scrollbar on the right edge
        if content_height > viewport_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .style(Style::default().fg(RUST_ORANGE));

            let mut scrollbar_state = ScrollbarState::new(max_scroll)
                .position(scroll_offset as usize);

            frame.render_stateful_widget(
                scrollbar,
                area,
                &mut scrollbar_state,
            );
        }

        // Return max_scroll for app state update
        max_scroll
    }
}

/// Get max scroll for messages (used by compat layer to update app state)
pub fn get_max_scroll(app: &App, viewport_height: usize) -> usize {
    let message_count = app.messages().len();
    if message_count == 0 {
        return 0;
    }

    // Approximate line count (this is a rough estimate)
    // Each message has: 1 header + content lines + 1 separator
    let mut total_lines = 0;
    for msg in app.messages() {
        total_lines += 1; // header
        total_lines += msg.content.lines().count().max(1); // content
        total_lines += 1; // separator
    }

    total_lines.saturating_sub(viewport_height)
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

fn render_autocomplete(frame: &mut Frame, input_area: Rect, app: &App) {
    if let Some(autocomplete) = app.autocomplete() {
        // Calculate popup area - above input, max 10 items visible
        let max_visible_items = 10;
        let popup_height = (autocomplete.items.len().min(max_visible_items) + 2) as u16; // +2 for borders
        let popup_width = 60; // Fixed width for now

        // Position above input area
        if input_area.y < popup_height {
            // Not enough space above, skip rendering
            return;
        }

        let popup_area = Rect {
            x: input_area.x,
            y: input_area.y.saturating_sub(popup_height),
            width: popup_width.min(input_area.width),
            height: popup_height,
        };

        // Calculate scroll offset to keep selected item visible
        let selected = autocomplete.selected;
        let total_items = autocomplete.items.len();

        // Calculate which items to show (scrolling window)
        let scroll_offset = if total_items <= max_visible_items {
            // All items fit, no scrolling needed
            0
        } else if selected < max_visible_items / 2 {
            // Near top, show from beginning
            0
        } else if selected >= total_items - max_visible_items / 2 {
            // Near bottom, show last max_visible_items
            total_items.saturating_sub(max_visible_items)
        } else {
            // In middle, center the selection
            selected.saturating_sub(max_visible_items / 2)
        };

        let visible_end = (scroll_offset + max_visible_items).min(total_items);

        // Build list items (only for visible window)
        let items: Vec<ListItem> = autocomplete.items[scroll_offset..visible_end]
            .iter()
            .enumerate()
            .map(|(window_idx, item)| {
                let actual_idx = scroll_offset + window_idx;
                let is_selected = actual_idx == selected;

                // Format: /command - description
                let mut line_spans = vec![
                    Span::styled(
                        format!("/{}", item.command),
                        if is_selected {
                            Style::default()
                                .fg(Color::Black)
                                .bg(RUST_ORANGE)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(RUST_ORANGE)
                        },
                    ),
                ];

                if let Some(ref desc) = item.description {
                    if !desc.is_empty() {
                        line_spans.push(Span::styled(
                            format!(" - {}", desc),
                            if is_selected {
                                Style::default().fg(Color::Black).bg(RUST_ORANGE)
                            } else {
                                Style::default().fg(Color::Gray)
                            },
                        ));
                    }
                }

                ListItem::new(Line::from(line_spans))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(RUST_ORANGE))
                .title(vec![
                    Span::styled("🔍 ", Style::default().fg(RUST_ORANGE)),
                    Span::styled(
                        "Slash Commands",
                        Style::default()
                            .fg(RUST_ORANGE)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({}/{})", selected + 1, total_items),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
        );

        frame.render_widget(list, popup_area);
    }
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