//! Permissions Search UI Rendering
//!
//! Renders the interactive permissions search modal using ratatui.
//! Displays permission rules in a table format with search input,
//! match count, and keyboard instructions.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Row, Table, Widget, Wrap},
};

use super::permissions_search_state::PermissionsSearchState;

/// Rust orange color for TUI styling (consistent with rest of app)
const RUST_ORANGE: Color = Color::Rgb(222, 165, 132);

/// Render the permissions search modal
///
/// Displays the full permissions search interface including:
/// - Header with title
/// - Search input field
/// - Match count
/// - Permission rules table
/// - Footer with keyboard instructions
///
/// # Arguments
///
/// * `state` - Current permissions search state
/// * `area` - Full terminal area to render into
/// * `buf` - Buffer to render into
///
/// # Example
///
/// ```ignore
/// use rustyclawd::commands::permissions_search_state::PermissionsSearchState;
/// use rustyclawd::commands::permissions_ui::render_permissions_search;
///
/// let state = PermissionsSearchState::new();
/// // In your TUI render loop:
/// // render_permissions_search(&state, terminal_area, &mut buffer);
/// ```
pub fn render_permissions_search(state: &PermissionsSearchState, area: Rect, buf: &mut Buffer) {
    // Calculate centered modal area (80% width, 80% height)
    let modal_area = centered_rect(80, 80, area);

    // Clear background behind modal
    Clear.render(modal_area, buf);

    // Create main layout: header | search | table | footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Search input + match count
            Constraint::Min(10),   // Table (flexible)
            Constraint::Length(4), // Footer instructions
        ])
        .split(modal_area);

    // Render header
    render_header(chunks[0], buf);

    // Render search input and match count
    render_search_input(state, chunks[1], buf);

    // Render table
    render_table(state, chunks[2], buf);

    // Render footer instructions
    render_footer(state, chunks[3], buf);
}

/// Render the header with title
fn render_header(area: Rect, buf: &mut Buffer) {
    let title = Paragraph::new("PERMISSIONS")
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(RUST_ORANGE)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT));

    title.render(area, buf);
}

/// Render search input field and match count
fn render_search_input(state: &PermissionsSearchState, area: Rect, buf: &mut Buffer) {
    let (matches, total) = state.match_count();

    // Build search display text
    let search_text = if state.is_searching() {
        format!("Search: {} ", state.search_query())
    } else {
        "Search: (press '/' to search)".to_string()
    };

    let match_text = if state.is_searching() && !state.search_query().is_empty() {
        format!("({} of {} matches)", matches, total)
    } else {
        format!("({} tools)", total)
    };

    let mut spans = vec![
        Span::styled(
            search_text,
            Style::default().fg(if state.is_searching() {
                Color::Yellow
            } else {
                Color::Gray
            }),
        ),
        Span::styled(match_text, Style::default().fg(Color::DarkGray)),
    ];

    // Add cursor if searching
    if state.is_searching() {
        spans.insert(
            1,
            Span::styled(
                "█",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        );
    }

    let search_line = Line::from(spans);

    let search_widget = Paragraph::new(search_line)
        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
        .alignment(Alignment::Left);

    search_widget.render(area, buf);
}

/// Render the permissions table
fn render_table(state: &PermissionsSearchState, area: Rect, buf: &mut Buffer) {
    let rules = state.filtered_rules();

    // Build table rows
    let rows: Vec<Row> = rules
        .iter()
        .enumerate()
        .map(|(idx, rule)| {
            let is_selected = idx == state.selected_index();

            // Tool name column
            let tool_name = if is_selected {
                Span::styled(
                    format!("▶ {}", rule.tool_name),
                    Style::default()
                        .fg(RUST_ORANGE)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw(format!("  {}", rule.tool_name))
            };

            // Permission columns (checkmark or X)
            let ask_icon = if rule.allow_in_ask { "✓" } else { "✗" };
            let auto_icon = if rule.allow_in_auto_accept {
                "✓"
            } else {
                "✗"
            };
            let plan_icon = if rule.allow_in_plan { "✓" } else { "✗" };

            // Style based on selection
            let row_style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let ask_style = if rule.allow_in_ask {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };

            let auto_style = if rule.allow_in_auto_accept {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };

            let plan_style = if rule.allow_in_plan {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };

            // Add blocked indicator for plan mode
            let blocked_text = if !rule.allow_in_plan {
                Span::styled("  (blocked)", Style::default().fg(Color::Red))
            } else {
                Span::raw("")
            };

            Row::new(vec![
                Line::from(tool_name),
                Line::from(Span::styled(ask_icon, ask_style)),
                Line::from(Span::styled(auto_icon, auto_style)),
                Line::from(vec![Span::styled(plan_icon, plan_style), blocked_text]),
            ])
            .style(row_style)
        })
        .collect();

    // Empty state
    let rows = if rows.is_empty() {
        vec![Row::new(vec![
            Line::from(Span::styled(
                "  No matches found",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(""),
            Line::from(""),
        ])]
    } else {
        rows
    };

    // Create header
    let header = Row::new(vec!["Tool Name", "Ask", "Auto-Accept", "Plan"])
        .style(
            Style::default()
                .fg(RUST_ORANGE)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    // Create table
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40), // Tool name
            Constraint::Percentage(15), // Ask
            Constraint::Percentage(20), // Auto-Accept
            Constraint::Percentage(25), // Plan
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::LEFT | Borders::RIGHT));

    table.render(area, buf);
}

/// Render footer with keyboard instructions
fn render_footer(state: &PermissionsSearchState, area: Rect, buf: &mut Buffer) {
    let instructions = if state.is_searching() {
        vec![
            "Type to search",
            "↑↓ Navigate",
            "Backspace to delete",
            "Esc to clear search",
        ]
    } else {
        vec![
            "/ Search by tool name",
            "↑↓ Navigate results",
            "Esc Close modal",
        ]
    };

    let instruction_text: Vec<Line> = instructions
        .iter()
        .map(|&text| {
            Line::from(vec![
                Span::styled("  • ", Style::default().fg(RUST_ORANGE)),
                Span::raw(text),
            ])
        })
        .collect();

    let footer = Paragraph::new(instruction_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);

    footer.render(area, buf);
}

/// Create a centered rectangle within the given area
///
/// # Arguments
///
/// * `percent_x` - Percentage of width (0-100)
/// * `percent_y` - Percentage of height (0-100)
/// * `r` - Outer area to center within
///
/// # Returns
///
/// Centered Rect with specified percentage dimensions
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_centered_rect_dimensions() {
        let outer = Rect::new(0, 0, 100, 100);
        let centered = centered_rect(80, 80, outer);

        // Should be 80% of width and height
        assert!(centered.width >= 78 && centered.width <= 82);
        assert!(centered.height >= 78 && centered.height <= 82);

        // Should be centered
        let margin_x = (outer.width - centered.width) / 2;
        let margin_y = (outer.height - centered.height) / 2;
        assert!((8..=12).contains(&margin_x));
        assert!((8..=12).contains(&margin_y));
    }

    #[test]
    fn test_centered_rect_zero_area() {
        let outer = Rect::new(0, 0, 0, 0);
        let centered = centered_rect(80, 80, outer);

        assert_eq!(centered.width, 0);
        assert_eq!(centered.height, 0);
    }

    #[test]
    fn test_centered_rect_100_percent() {
        let outer = Rect::new(0, 0, 100, 100);
        let centered = centered_rect(100, 100, outer);

        // Should take full area
        assert_eq!(centered.width, outer.width);
        assert_eq!(centered.height, outer.height);
    }
}
