//! Message panel rendering - chat history with scroll, tool results, click regions
//!
//! Extracted from ui.rs to keep each rendering module under 300 LOC.
//! This is the largest renderer (~380 lines) but cannot be split further
//! without introducing artificial boundaries within a single scroll-aware render pass.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use super::app::App;
use super::message::Role;
use super::message_formatter::{calculate_wrapped_height, format_message_by_role};
use super::tool_renderer::{format_response_content, format_tool_parameters, format_tool_params};
use super::ui::RUST_ORANGE;

pub(super) fn render_messages(
    frame: &mut Frame,
    area: Rect,
    app: &mut App,
    throbber: char,
) -> usize {
    // Clear click regions at start of render
    app.click_regions.clear();

    // Clone messages to avoid borrow conflicts
    let messages = app.messages().to_vec();

    // Focus-aware border styling
    let is_focused = app.focus_messages().get();
    let border_color = if is_focused {
        Color::White
    } else {
        RUST_ORANGE
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(vec![
            Span::styled("💬 ", Style::default().fg(border_color)),
            Span::styled(
                "Messages",
                Style::default()
                    .fg(border_color)
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
            Line::from("Type your message and press Enter to start chatting."),
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

        let paragraph = Paragraph::new(welcome_text).block(block);

        frame.render_widget(paragraph, area);

        // No scrollable content on welcome screen
        0
    } else {
        // Build complete text content as styled text
        let mut text_lines = Vec::new();
        let inner_area = block.inner(area);

        // Calculate content width BEFORE message loop (needed for wrapping calculations)
        let content_width = area.width.saturating_sub(2) as usize;

        // Calculate viewport height for scroll offset calculation
        let viewport_height = area.height.saturating_sub(2) as usize;

        // DEFENSIVE: Ensure viewport has minimum height
        if viewport_height == 0 {
            // Terminal too small - render without scrolling
            let paragraph = Paragraph::new(Text::from(vec![]))
                .block(block)
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
            return 0;
        }

        // Track cumulative VISUAL position (accounting for wrapping)
        let mut cumulative_visual_line = 0;

        // Store message positions for click region creation AFTER we calculate final scroll_offset
        let mut message_positions: Vec<(usize, usize, usize)> = Vec::new(); // (msg_idx, viewport_start, clickable_end)

        for (msg_idx, message) in messages.iter().enumerate() {
            // Skip hidden messages (slash command prompts) from UI display
            if message.hidden {
                continue;
            }

            // Skip empty assistant messages (tool-only responses show green dot with no content)
            if message.role == Role::Assistant && message.content.trim().is_empty() {
                continue;
            }

            // Track starting positions in BOTH coordinate spaces
            let buffer_start = text_lines.len();
            let viewport_start = cumulative_visual_line;
            // Check if this is a tool message (dynamic rendering)
            if message.role == Role::System {
                if let Some((_tool_id, tool_state)) = app.tool_message_by_index(msg_idx) {
                    // This is a tool execution message - render dynamically
                    // Use stored elapsed_duration if completed, otherwise calculate real-time
                    let elapsed = if tool_state.completed {
                        tool_state.elapsed_duration.unwrap_or(0)
                    } else {
                        tool_state.start_time.elapsed().as_secs()
                    };

                    if tool_state.completed {
                        // Tool completed - show final result
                        if let Some(ref result) = tool_state.result {
                            let status_dot = "●";
                            let dot_color = if result.is_error {
                                Color::Red
                            } else {
                                Color::Green
                            };
                            let collapse_arrow = if message.collapsed { "▶" } else { "▼" };

                            // Calculate available width for header
                            // Format: "▼ ● Bash(params) (9999s)"
                            // Components: arrow(1) + space(1) + dot(1) + space(1) + tool_name + params + space(1) + timer(7)
                            let timer_text = format!("({}s)", elapsed.min(9999));
                            let fixed_width =
                                1 + 1 + 1 + 1 + tool_state.tool_name.len() + 1 + timer_text.len();
                            let available_for_params = content_width.saturating_sub(fixed_width);

                            let params_text =
                                format_tool_params(&tool_state.params, available_for_params);
                            let header_text =
                                format!("{}{} {}", tool_state.tool_name, params_text, timer_text);

                            // Header line: "▼ ● Bash(sleep 20) (  12s)"
                            text_lines.push(Line::from(vec![
                                Span::styled(collapse_arrow, Style::default().fg(Color::Gray)),
                                Span::raw(" "),
                                Span::styled(
                                    status_dot,
                                    Style::default().fg(dot_color).add_modifier(Modifier::BOLD),
                                ),
                                Span::raw(" "),
                                Span::styled(header_text, Style::default().fg(Color::DarkGray)),
                            ]));

                            if message.collapsed {
                                // COLLAPSED: Show only stdout truncated to one line
                                if !result.stdout.is_empty() {
                                    let first_line = result.stdout.lines().next().unwrap_or("");
                                    // Calculate max width for stdout: content_width - "└─ " (3 chars)
                                    let max_stdout_width = content_width.saturating_sub(3);
                                    let truncated = if first_line.len() > max_stdout_width {
                                        format!(
                                            "{}...",
                                            &first_line[..max_stdout_width.saturating_sub(3)]
                                        )
                                    } else {
                                        first_line.to_string()
                                    };
                                    text_lines.push(Line::from(Span::styled(
                                        format!("└─ {}", truncated),
                                        Style::default().fg(Color::DarkGray),
                                    )));
                                }
                            } else {
                                // EXPANDED: Show full details with clean formatting

                                // Show parameters (with clean formatting, not raw JSON)
                                let params_prefix = "├─"; // Parameters always has items after it (response)
                                let parent_has_more = true; // Always true - there's always response and possibly exit_code after
                                let params_lines = format_tool_parameters(
                                    &tool_state.params,
                                    params_prefix,
                                    parent_has_more,
                                    content_width,
                                );
                                text_lines.extend(params_lines);

                                // Show response content (parse as JSON and format like parameters)
                                let response_prefix = "└─"; // Response is always last
                                let response_has_more = false; // Response is always last item
                                let response_lines = format_response_content(
                                    &result.raw_content,
                                    response_prefix,
                                    response_has_more,
                                    content_width,
                                );
                                text_lines.extend(response_lines);
                            }
                        }
                    } else {
                        // Tool still running - show throbber + timer
                        let collapse_arrow = if message.collapsed { "▶" } else { "▼" };

                        // Calculate available width for header (same as completed)
                        let timer_text = format!("({}s)", elapsed.min(9999));
                        let fixed_width =
                            1 + 1 + 1 + 1 + tool_state.tool_name.len() + 1 + timer_text.len();
                        let available_for_params = content_width.saturating_sub(fixed_width);

                        let params_text =
                            format_tool_params(&tool_state.params, available_for_params);
                        let header_text =
                            format!("{}{} {}", tool_state.tool_name, params_text, timer_text);

                        // Header: "▼ ⣾ Bash(ls -la) (  12s)"
                        text_lines.push(Line::from(vec![
                            Span::styled(collapse_arrow, Style::default().fg(Color::Gray)),
                            Span::raw(" "),
                            Span::styled(
                                format!("{} ", throbber),
                                Style::default()
                                    .fg(Color::Magenta)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(header_text, Style::default().fg(Color::DarkGray)),
                        ]));
                    }

                    // Blank separator
                    text_lines.push(Line::from(""));

                    // Calculate visual height for this tool message (accounting for wrapping)
                    let buffer_end = text_lines.len();
                    let visual_height = calculate_wrapped_height(
                        &text_lines[buffer_start..buffer_end],
                        content_width,
                    );
                    let viewport_end = cumulative_visual_line + visual_height;
                    cumulative_visual_line = viewport_end;

                    // Store position for click region creation later (after we know final scroll_offset)
                    // Exclude the trailing blank separator line from click region (subtract 1 from viewport_end)
                    let clickable_end = viewport_end.saturating_sub(1);
                    message_positions.push((msg_idx, viewport_start, clickable_end));

                    continue; // Skip normal message rendering
                }
            }

            // Normal message rendering (non-tool messages)
            // Use formatting functions from message_formatter module
            let message_lines = format_message_by_role(message, throbber, content_width);

            // Add formatted lines to buffer
            // Trailing blank separator will be removed globally after loop
            text_lines.extend(message_lines);

            // Calculate visual height for this message (accounting for wrapping)
            let buffer_end = text_lines.len();
            let visual_height =
                calculate_wrapped_height(&text_lines[buffer_start..buffer_end], content_width);
            let viewport_end = cumulative_visual_line + visual_height;
            cumulative_visual_line = viewport_end;

            // Store position for click region creation if message is collapsible
            if message.collapsible {
                // Exclude the trailing blank separator line from click region (subtract 1 from viewport_end)
                let clickable_end = viewport_end.saturating_sub(1);
                message_positions.push((msg_idx, viewport_start, clickable_end));
            }
        }

        // DEFENSIVE: Remove any trailing blank separator to prevent overflow
        // This handles both regular messages and tool messages
        if !text_lines.is_empty() {
            if let Some(last_line) = text_lines.last() {
                if last_line.spans.is_empty()
                    || (last_line.spans.len() == 1 && last_line.spans[0].content.is_empty())
                {
                    text_lines.pop();
                    // Adjust content height since we removed a line
                    cumulative_visual_line = cumulative_visual_line.saturating_sub(1);
                }
            }
        }

        // Content height already calculated as cumulative_visual_line
        let content_height = cumulative_visual_line;

        let text = Text::from(text_lines);

        // Scroll handling: Calculate actual scroll position
        // Subtract 2 for block borders
        let viewport_height = area.height.saturating_sub(2) as usize;

        // DEFENSIVE: Ensure viewport has minimum height to prevent calculation issues
        // If terminal is too small, viewport_height could be 0, causing max_scroll overflow
        if viewport_height == 0 {
            // Terminal too small to show content - render without scrolling
            let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
            return 0;
        }

        // Calculate max scroll with proper boundary check
        // If content fits in viewport, max_scroll is 0 (no scrolling needed)
        let max_scroll = content_height.saturating_sub(viewport_height);

        // Determine scroll offset
        // DEFENSIVE: Handle very large content (>65535 lines) by capping scroll_offset type
        // but warning if we're hitting the limit
        let scroll_offset = if app.follow_bottom() {
            // Auto-follow bottom - show last viewport worth of content
            // CRITICAL: When following bottom, ALWAYS use max_scroll to show latest messages
            if max_scroll > u16::MAX as usize {
                // Content exceeds u16 limit - scroll as far as possible and log warning
                tracing::warn!(
                    "Content height {} exceeds scroll limit, bottom may be cut off",
                    content_height
                );
                u16::MAX
            } else {
                max_scroll as u16
            }
        } else {
            // Manual scroll - clamp to valid range [0, max_scroll]
            let clamped = app.scroll_offset().min(max_scroll);
            if clamped > u16::MAX as usize {
                tracing::warn!(
                    "Scroll offset {} exceeds u16 limit, clamping to {}",
                    clamped,
                    u16::MAX
                );
                u16::MAX
            } else {
                clamped as u16
            }
        };

        // NOW create click regions using the FINAL scroll_offset (not the initial app.scroll_offset())
        // This ensures click regions match the actual rendered positions
        let scroll_offset_usize = scroll_offset as usize;
        for (msg_idx, viewport_start, clickable_end) in message_positions {
            // Check if any part of message is visible in viewport
            if clickable_end > scroll_offset_usize {
                let visible_y = viewport_start.saturating_sub(scroll_offset_usize);

                let visible_height =
                    clickable_end.saturating_sub(scroll_offset_usize.max(viewport_start));

                app.click_regions.add_message(
                    msg_idx,
                    Rect {
                        x: 0, // Relative to inner_area
                        y: visible_y as u16,
                        width: inner_area.width,
                        height: visible_height as u16,
                    },
                );
            }
        }

        let paragraph = Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: false }) // Wraps at widget's inner width automatically
            .scroll((scroll_offset, 0));

        frame.render_widget(paragraph, area);

        // Render scrollbar on the right edge
        if content_height > viewport_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .style(Style::default().fg(RUST_ORANGE));

            let mut scrollbar_state =
                ScrollbarState::new(max_scroll).position(scroll_offset as usize);

            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }

        // Return max_scroll for app state update
        max_scroll
    }
}
