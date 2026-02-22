//! Tool and JSON rendering functions for TUI
//!
//! Handles formatting of tool calls, JSON parameters, and response content
//! with tree-structured display, proper indentation, and text wrapping.

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use super::message_formatter::wrap_value_with_indent;

/// Format a tool call for display (compact JSON parameters)
/// Max width parameter controls truncation to fit header
pub fn format_tool_params(params: &serde_json::Value, max_width: usize) -> String {
    let formatted = match params {
        serde_json::Value::Object(map) if map.is_empty() => "()".to_string(),
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
                format!("({}, ...)", items.join(", "))
            } else {
                format!("({})", items.join(", "))
            }
        }
        _ => format!("({})", params),
    };

    // Truncate to max_width if needed
    if formatted.len() > max_width {
        format!("{}...)", &formatted[..max_width.saturating_sub(4)])
    } else {
        formatted
    }
}

/// Recursively format a JSON value with tree structure
/// Returns formatted lines with proper indentation
pub fn format_json_value(
    value: &serde_json::Value,
    parent_bar: &str,
    tree_char: &str,
    continuation_char: &str,
    key: Option<&str>,
    content_width: usize,
    depth: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Safety limit for recursion depth
    if depth > 10 {
        let label = key.map(|k| format!("{}: ", k)).unwrap_or_default();
        lines.push(Line::from(Span::styled(
            format!("{} {} {}[max depth exceeded]", parent_bar, tree_char, label),
            Style::default().fg(Color::DarkGray),
        )));
        return lines;
    }

    match value {
        serde_json::Value::Object(map) => {
            // Object - show as nested structure
            let label = key.map(|k| format!("{}: ", k)).unwrap_or_default();
            lines.push(Line::from(Span::styled(
                format!("{} {} {}", parent_bar, tree_char, label),
                Style::default().fg(Color::DarkGray),
            )));

            // Nested items
            let nested_parent_bar = format!("{} {} ", parent_bar, continuation_char);
            for (idx, (nested_key, nested_value)) in map.iter().enumerate() {
                let is_last = idx == map.len() - 1;
                let nested_tree_char = if is_last { "└─" } else { "├─" };
                let nested_continuation_char = if is_last { "  " } else { "│ " };

                let nested_lines = format_json_value(
                    nested_value,
                    nested_parent_bar.trim_end(),
                    nested_tree_char,
                    nested_continuation_char,
                    Some(nested_key),
                    content_width,
                    depth + 1,
                );
                lines.extend(nested_lines);
            }
        }
        serde_json::Value::Array(arr) => {
            // Array - show as indexed items
            let label = key.map(|k| format!("{}: ", k)).unwrap_or_default();
            lines.push(Line::from(Span::styled(
                format!(
                    "{} {} {}[{} items]",
                    parent_bar,
                    tree_char,
                    label,
                    arr.len()
                ),
                Style::default().fg(Color::DarkGray),
            )));

            // Array items
            let nested_parent_bar = format!("{} {} ", parent_bar, continuation_char);
            for (idx, item) in arr.iter().enumerate() {
                let is_last = idx == arr.len() - 1;
                let nested_tree_char = if is_last { "└─" } else { "├─" };
                let nested_continuation_char = if is_last { "  " } else { "│ " };
                let item_key = format!("[{}]", idx);

                let nested_lines = format_json_value(
                    item,
                    nested_parent_bar.trim_end(),
                    nested_tree_char,
                    nested_continuation_char,
                    Some(&item_key),
                    content_width,
                    depth + 1,
                );
                lines.extend(nested_lines);
            }
        }
        _ => {
            // Primitive value - format with wrapping
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => "null".to_string(),
                _ => unreachable!(),
            };

            let label = key.map(|k| format!("{}: ", k)).unwrap_or_default();
            let first_line_prefix = format!("{} {} {}", parent_bar, tree_char, label);
            let continuation_prefix = format!("{} {}  ", parent_bar, continuation_char);
            let wrapped_lines = wrap_value_with_indent(
                &value_str,
                &first_line_prefix,
                &continuation_prefix,
                content_width,
            );

            for wrapped_line in wrapped_lines {
                lines.push(Line::from(Span::styled(
                    wrapped_line,
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    lines
}

/// Format tool parameters in a clean, readable way (not raw JSON)
/// Returns tree-structured lines with proper indentation
/// `parent_has_more` indicates if there are more items after parameters (response, exit_code)
pub fn format_tool_parameters(
    params: &serde_json::Value,
    prefix: &str,
    parent_has_more: bool,
    content_width: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    match params {
        serde_json::Value::Object(map) if map.is_empty() => {
            lines.push(Line::from(Span::styled(
                format!("{} parameters: (none)", prefix),
                Style::default().fg(Color::DarkGray),
            )));
        }
        serde_json::Value::Object(map) => {
            // First line: parameters label
            lines.push(Line::from(Span::styled(
                format!("{} parameters:", prefix),
                Style::default().fg(Color::DarkGray),
            )));

            // Parent vertical bar if there are more items after parameters
            let parent_bar = if parent_has_more { "│" } else { " " };

            // Each parameter on its own line with proper indentation
            for (idx, (key, value)) in map.iter().enumerate() {
                let is_last_param = idx == map.len() - 1;
                let tree_char = if is_last_param { "└─" } else { "├─" };
                let continuation_char = if is_last_param { "  " } else { "│ " };

                // Use recursive formatter for all value types
                let value_lines = format_json_value(
                    value,
                    parent_bar,
                    tree_char,
                    continuation_char,
                    Some(key),
                    content_width,
                    0, // Start at depth 0
                );
                lines.extend(value_lines);
            }
        }
        _ => {
            lines.push(Line::from(Span::styled(
                format!("{} parameters: {}", prefix, params),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    lines
}

/// Format response content as JSON key-values (just like parameters)
/// Returns tree-structured lines with proper indentation
pub fn format_response_content(
    content: &str,
    prefix: &str,
    parent_has_more: bool,
    content_width: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if content.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("{} response: (empty)", prefix),
            Style::default().fg(Color::DarkGray),
        )));
        return lines;
    }

    // Try to parse as JSON
    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(content) {
        // Response is JSON - format like parameters
        lines.push(Line::from(Span::styled(
            format!("{} response:", prefix),
            Style::default().fg(Color::DarkGray),
        )));

        // Parent vertical bar if there are more items after response
        let parent_bar = if parent_has_more { "│" } else { " " };

        // Format each JSON field using recursive formatter
        if let serde_json::Value::Object(map) = json_value {
            for (idx, (key, value)) in map.iter().enumerate() {
                let is_last_field = idx == map.len() - 1;
                let tree_char = if is_last_field { "└─" } else { "├─" };
                let continuation_char = if is_last_field { "  " } else { "│ " };

                // Use recursive formatter for all value types (handles nested objects/arrays)
                let value_lines = format_json_value(
                    value,
                    parent_bar,
                    tree_char,
                    continuation_char,
                    Some(key),
                    content_width,
                    0, // Start at depth 0
                );
                lines.extend(value_lines);
            }
        }
    } else {
        // Plain text response - show as-is with wrapping
        let parent_bar = if parent_has_more { "│" } else { " " };
        let first_line_prefix = format!("{} ", prefix);
        let continuation_prefix = format!("{}   ", parent_bar);
        let wrapped_lines = wrap_value_with_indent(
            content,
            &first_line_prefix,
            &continuation_prefix,
            content_width,
        );

        for wrapped_line in wrapped_lines {
            lines.push(Line::from(Span::styled(
                wrapped_line,
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    lines
}
