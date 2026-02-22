//! Popup/overlay rendering - autocomplete, memory modal, permissions modal
//!
//! Extracted from ui.rs to keep each rendering module under 300 LOC.
//! All overlays follow the Clear-then-render pattern for opaque popups.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};
use unicode_width::UnicodeWidthStr;

use super::app::App;
use super::ui::RUST_ORANGE;
use crate::commands::permissions_ui;

pub(super) fn render_autocomplete(frame: &mut Frame, input_area: Rect, app: &App) {
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

pub(super) fn render_memory_modal(frame: &mut Frame, input_area: Rect, app: &App) {
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

pub(super) fn render_permissions_modal(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(state) = app.permissions_modal() {
        // Clear the full area behind the modal to prevent text bleed-through
        // (permissions_ui also clears internally, but clearing here ensures
        // consistency with render_autocomplete and render_memory_modal patterns)
        frame.render_widget(Clear, area);
        // Use the permissions_ui module to render the modal
        permissions_ui::render_permissions_search(state, area, frame.buffer_mut());
    }
}
