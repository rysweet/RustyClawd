//! Rendering layer for TUI - pure functions, no state mutation
//!
//! This module coordinates rendering of the TUI by delegating to:
//! - `message_formatter`: User/assistant/system message formatting
//! - `tool_renderer`: Tool call and JSON parameter rendering

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::commands::permissions_ui;
use crate::tui::app::{App, LayoutCache};
use crate::tui::layout::{LayoutConfig, LayoutOrganizer};
use crate::tui::message::Role;

use super::message_formatter::{calculate_wrapped_height, format_message_by_role};
use super::tool_renderer::{format_response_content, format_tool_parameters, format_tool_params};

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

fn render_status_bar(frame: &mut Frame, area: Rect, app: &mut App, throbber: char) {
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
    let mouse_mode_enabled = app.mouse_mode_enabled();

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
    let mouse_status = if mouse_mode_enabled {
        "Mouse:ON"
    } else {
        "Mouse:OFF"
    };
    let left_width: usize = status_spans.iter().map(|s| s.content.width()).sum();
    let right_text = format!(
        " | {} | {}{} | F1:Menu ",
        mouse_status, debug_status, follow_indicator
    );
    let padding_width = area
        .width
        .saturating_sub((left_width + right_text.width()) as u16);

    status_spans.push(Span::raw(" ".repeat(padding_width as usize)));
    status_spans.push(Span::raw(" | "));

    // Mouse mode indicator
    let mouse_style = if mouse_mode_enabled {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Red)
    };
    status_spans.push(Span::styled(mouse_status, mouse_style));
    status_spans.push(Span::raw(" | "));

    // Track debug button click region
    let debug_x = left_width + padding_width as usize + 3 + mouse_status.width() + 3; // " | " is 3 chars each
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

fn render_messages(frame: &mut Frame, area: Rect, app: &mut App, throbber: char) -> usize {
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

fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    // Focus-aware styling (scrollbar only for now - TextArea block styling requires mutable access)
    let is_focused = app.focus_input().get();
    let scrollbar_color = if is_focused {
        Color::White
    } else {
        RUST_ORANGE
    };

    // CORRECT: Render TextArea directly with immutable borrow
    // TextArea implements Widget trait
    // Styling was configured once during initialization (App::new)
    // TODO: Update TextArea's block style dynamically based on focus
    frame.render_widget(&app.input, area);

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
        // Goal: Keep selected item in visible window [scroll_offset, scroll_offset + max_visible_items)
        let scroll_offset = if total_items <= max_visible_items {
            // All items fit, no scrolling needed
            0
        } else {
            // Center selection, clamped to valid range [0, total_items - max_visible_items]
            selected
                .saturating_sub(max_visible_items / 2)
                .min(total_items.saturating_sub(max_visible_items))
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
                let mut line_spans = vec![Span::styled(
                    format!("/{}", item.command),
                    if is_selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(RUST_ORANGE)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(RUST_ORANGE)
                    },
                )];

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

        // Clear the area behind the popup first to prevent text bleed-through
        frame.render_widget(Clear, popup_area);

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

        // Render scrollbar if there are more items than visible
        if total_items > max_visible_items {
            let max_scroll = total_items.saturating_sub(max_visible_items);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .style(Style::default().fg(RUST_ORANGE));

            let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll_offset);

            frame.render_stateful_widget(scrollbar, popup_area, &mut scrollbar_state);
        }
    }
}

fn render_memory_modal(frame: &mut Frame, input_area: Rect, app: &App) {
    if let Some(modal) = app.memory_modal() {
        // Calculate popup area - centered above input
        let max_visible_items = 10;
        let popup_height = (modal.destinations.len().min(max_visible_items) + 2) as u16; // +2 for borders
        let popup_width = 80; // Wider for paths

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

        // Clear the area behind the popup first
        frame.render_widget(Clear, popup_area);

        // Calculate available width for items (subtract 2 for borders)
        let item_width = popup_area.width.saturating_sub(2);

        // Calculate scroll offset to keep selected item visible
        let selected = modal.selected;
        let total_items = modal.destinations.len();

        // Calculate which items to show (scrolling window)
        // Goal: Keep selected item in visible window [scroll_offset, scroll_offset + max_visible_items)
        let scroll_offset = if total_items <= max_visible_items {
            // All items fit, no scrolling needed
            0
        } else {
            // Center selection, clamped to valid range [0, total_items - max_visible_items]
            selected
                .saturating_sub(max_visible_items / 2)
                .min(total_items.saturating_sub(max_visible_items))
        };

        let visible_end = (scroll_offset + max_visible_items).min(total_items);

        // Build list items with right-aligned paths (only for visible window)
        let items: Vec<ListItem> = modal.destinations[scroll_offset..visible_end]
            .iter()
            .enumerate()
            .map(|(window_idx, dest)| {
                let actual_idx = scroll_offset + window_idx;
                let is_selected = actual_idx == selected;
                build_memory_list_item(dest, is_selected, item_width)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(RUST_ORANGE))
                .title(vec![
                    Span::styled("📝 ", Style::default().fg(RUST_ORANGE)),
                    Span::styled(
                        "Select memory file to edit:",
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

/// Build a memory list item with right-aligned path
fn build_memory_list_item(
    dest: &crate::tui::MemoryDestination,
    is_selected: bool,
    available_width: u16,
) -> ListItem<'static> {
    let mut spans = Vec::new();

    // Selection indicator (always 2 chars: "> " or "  ")
    let selection_indicator = if is_selected { "> " } else { "  " };
    let selection_style = if is_selected {
        Style::default().fg(Color::Black).bg(RUST_ORANGE)
    } else {
        Style::default()
    };

    spans.push(Span::styled(
        selection_indicator.to_string(),
        selection_style,
    ));

    // Tree indicator for imported files ("└ " = 2 chars)
    let tree_indicator = if dest.is_imported { "└ " } else { "" };
    if !tree_indicator.is_empty() {
        spans.push(Span::styled(
            tree_indicator.to_string(),
            if is_selected {
                Style::default().fg(Color::Black).bg(RUST_ORANGE)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ));
    }

    // Item name
    let name_style = if is_selected {
        Style::default()
            .fg(Color::Black)
            .bg(RUST_ORANGE)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    spans.push(Span::styled(dest.name.clone(), name_style));

    // Calculate used width (sum of all span widths) - Unicode display width
    let left_width: usize = spans.iter().map(|s| s.content.width()).sum();

    // Get description/path for right side
    let right_text = dest.description.as_deref().unwrap_or("");
    let right_width = right_text.width();

    // Calculate padding (ensure at least 2 spaces between)
    let min_spacing = 2;
    let total_content = left_width + min_spacing + right_width;

    let padding_width = if total_content < available_width as usize {
        available_width as usize - left_width - right_width
    } else {
        min_spacing // Fallback to minimum spacing
    };

    // Add padding (CRITICAL: must have selection background if selected)
    let padding_style = if is_selected {
        Style::default().bg(RUST_ORANGE)
    } else {
        Style::default()
    };
    spans.push(Span::styled(" ".repeat(padding_width), padding_style));

    // Add right-aligned path/description
    let path_style = if is_selected {
        Style::default().fg(Color::Black).bg(RUST_ORANGE)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    spans.push(Span::styled(right_text.to_string(), path_style));

    ListItem::new(Line::from(spans))
}

fn render_permissions_modal(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(state) = app.permissions_modal() {
        // Clear the full area behind the modal to prevent text bleed-through
        // (permissions_ui also clears internally, but clearing here ensures
        // consistency with render_autocomplete and render_memory_modal patterns)
        frame.render_widget(Clear, area);
        // Use the permissions_ui module to render the modal
        permissions_ui::render_permissions_search(state, area, frame.buffer_mut());
    }
}

fn render_debug_panel(frame: &mut Frame, area: Rect, app: &App) -> usize {
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
