//! Message formatting functions for TUI rendering
//!
//! Handles formatting of user, assistant, and system messages with proper
//! text wrapping, indentation, and styling. Also provides shared utilities
//! for text wrapping and height calculation used across the TUI.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::tui::message::{Message, Role};

use super::ui::RUST_ORANGE;

/// Normalize line endings to Unix style (\n only)
/// Converts Windows (\r\n) and old Mac (\r) line endings to Unix (\n)
pub fn normalize_line_endings(text: &str) -> String {
    // Replace \r\n with \n first (Windows style)
    // Then replace any remaining \r with \n (old Mac style)
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Wrap text content with proper indentation for continuation lines
/// Returns a vector of strings, each representing a wrapped line
pub fn wrap_with_indent(
    content: &str,
    icon_width: usize,
    content_width: usize,
    subsequent_indent: &str,
) -> Vec<String> {
    use textwrap::{wrap, Options};

    if content.is_empty() {
        return vec![];
    }

    // Content width available (accounting for icon on first line)
    let effective_width = content_width.saturating_sub(icon_width).max(10);

    let options = Options::new(effective_width)
        .initial_indent("")
        .subsequent_indent(subsequent_indent)
        .break_words(true);

    wrap(content, options)
        .into_iter()
        .map(|cow| cow.into_owned())
        .collect()
}

/// Wrap text with proper indentation for continuation lines
/// Used for long parameter/response values that need to wrap
pub fn wrap_value_with_indent(
    text: &str,
    first_line_prefix: &str,
    continuation_prefix: &str,
    max_width: usize,
) -> Vec<String> {
    if text.is_empty() {
        return vec![first_line_prefix.to_string()];
    }

    // Calculate available width for first line
    let first_width = max_width.saturating_sub(first_line_prefix.len()).max(20);
    // Continuation lines get full continuation_prefix width
    let cont_width = max_width.saturating_sub(continuation_prefix.len()).max(20);

    let mut result = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        result.push(first_line_prefix.to_string());
        return result;
    }

    let mut current_line = String::new();
    let mut is_first = true;

    for word in words {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };

        let width_limit = if is_first { first_width } else { cont_width };

        if test_line.len() <= width_limit {
            current_line = test_line;
        } else {
            // Current line is full, push it
            if !current_line.is_empty() {
                let prefix = if is_first {
                    first_line_prefix
                } else {
                    continuation_prefix
                };
                result.push(format!("{}{}", prefix, current_line));
                is_first = false;
            }
            current_line = word.to_string();
        }
    }

    // Push final line
    if !current_line.is_empty() {
        let prefix = if is_first {
            first_line_prefix
        } else {
            continuation_prefix
        };
        result.push(format!("{}{}", prefix, current_line));
    }

    result
}

/// Calculate visual height of lines accounting for wrapping
/// This matches how ratatui's Paragraph widget wraps text
pub fn calculate_wrapped_height(lines: &[Line], content_width: usize) -> usize {
    let mut height = 0;
    for line in lines {
        let line_width: usize = line.spans.iter().map(|span| span.content.width()).sum();
        let wrapped_lines = if line_width == 0 {
            1
        } else {
            line_width.div_ceil(content_width)
        };
        height += wrapped_lines;
    }
    height
}

/// Format user message (icon column + continuation indent)
pub fn format_user_message(message: &Message, content_width: usize) -> Vec<Line<'static>> {
    const ICON: &str = "$ ";
    const INDENT: &str = "  ";

    let mut result = Vec::new();

    if message.content.is_empty() {
        // Empty message - just show icon
        result.push(Line::from(Span::styled(
            ICON,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        result.push(Line::from(""));
        return result;
    }

    // Normalize line endings first to handle \r\n and bare \r
    let normalized_content = normalize_line_endings(&message.content);

    // Handle each paragraph (newline-separated) separately
    let mut is_first_line_overall = true;
    let paragraphs: Vec<&str> = normalized_content.lines().collect();

    for paragraph in paragraphs.iter() {
        // Empty paragraphs represent blank lines (from \n\n in content)
        // Push an empty Line to create visual blank line
        if paragraph.is_empty() {
            result.push(Line::from(""));
            continue;
        }

        // Wrap this paragraph with proper indentation
        let wrapped = wrap_with_indent(paragraph, ICON.len(), content_width, INDENT);

        for (line_idx, line_content) in wrapped.iter().enumerate() {
            if is_first_line_overall && line_idx == 0 {
                // Very first line: icon + content
                result.push(Line::from(vec![
                    Span::styled(
                        ICON,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(line_content.clone()),
                ]));
                is_first_line_overall = false;
            } else if line_idx == 0 {
                // First line of non-first paragraph: needs indent
                result.push(Line::from(format!("{}{}", INDENT, line_content)));
            } else {
                // Continuation: already indented by wrap_with_indent
                result.push(Line::from(line_content.clone()));
            }
        }
    }

    // Add blank line separator between messages
    result.push(Line::from(""));
    result
}

/// Format LLM/assistant message (icon column + continuation indent)
pub fn format_assistant_message(
    message: &Message,
    throbber: char,
    content_width: usize,
) -> Vec<Line<'static>> {
    const INDENT: &str = "  ";

    let (icon, icon_style) = if message.streaming {
        (
            format!("{} ", throbber),
            Style::default()
                .fg(RUST_ORANGE)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            "● ".to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    };

    let mut result = Vec::new();

    if message.content.is_empty() {
        // Empty message - just show icon
        result.push(Line::from(Span::styled(icon, icon_style)));
        result.push(Line::from(""));
        return result;
    }

    // Normalize line endings first to handle \r\n and bare \r
    let normalized_content = normalize_line_endings(&message.content);

    // Handle each paragraph (newline-separated) separately
    let mut is_first_line_overall = true;
    let paragraphs: Vec<&str> = normalized_content.lines().collect();

    for paragraph in paragraphs.iter() {
        // Empty paragraphs represent blank lines (from \n\n in content)
        // Push an empty Line to create visual blank line
        if paragraph.is_empty() {
            result.push(Line::from(""));
            continue;
        }

        // Wrap this paragraph with proper indentation
        let wrapped = wrap_with_indent(paragraph, icon.len(), content_width, INDENT);

        for (line_idx, line_content) in wrapped.iter().enumerate() {
            if is_first_line_overall && line_idx == 0 {
                // Very first line: icon + content
                result.push(Line::from(vec![
                    Span::styled(icon.clone(), icon_style),
                    Span::raw(line_content.clone()),
                ]));
                is_first_line_overall = false;
            } else if line_idx == 0 {
                // First line of non-first paragraph: needs indent
                result.push(Line::from(format!("{}{}", INDENT, line_content)));
            } else {
                // Continuation: already indented by wrap_with_indent
                result.push(Line::from(line_content.clone()));
            }
        }
    }

    // Add blank line separator between messages
    result.push(Line::from(""));
    result
}

/// Format system message (icon column + continuation indent)
pub fn format_system_message(message: &Message, content_width: usize) -> Vec<Line<'static>> {
    const INDENT: &str = "  ";

    let icon = if message.collapsible {
        if message.collapsed {
            "▶ "
        } else {
            "▼ "
        }
    } else {
        "→ "
    };

    let content = message.display_content();
    let style = Style::default()
        .fg(Color::Gray)
        .add_modifier(Modifier::ITALIC);

    let mut result = Vec::new();

    if content.is_empty() {
        // Empty message - just show icon
        result.push(Line::from(Span::styled(
            icon,
            Style::default().fg(Color::Gray),
        )));
        result.push(Line::from(""));
        return result;
    }

    // Normalize line endings first to handle \r\n and bare \r
    let normalized_content = normalize_line_endings(content);

    // Handle each paragraph (newline-separated) separately
    let mut is_first_line_overall = true;
    let paragraphs: Vec<&str> = normalized_content.lines().collect();

    for paragraph in paragraphs.iter() {
        // Empty paragraphs represent blank lines (from \n\n in content)
        // Push an empty Line to create visual blank line
        if paragraph.is_empty() {
            result.push(Line::from(""));
            continue;
        }

        // Wrap this paragraph with proper indentation
        let wrapped = wrap_with_indent(paragraph, icon.len(), content_width, INDENT);

        for (line_idx, line_content) in wrapped.iter().enumerate() {
            if is_first_line_overall && line_idx == 0 {
                // Very first line: icon + content
                result.push(Line::from(vec![
                    Span::styled(icon, Style::default().fg(Color::Gray)),
                    Span::styled(line_content.clone(), style),
                ]));
                is_first_line_overall = false;
            } else if line_idx == 0 {
                // First line of non-first paragraph: needs indent
                result.push(Line::from(Span::styled(
                    format!("{}{}", INDENT, line_content),
                    style,
                )));
            } else {
                // Continuation: already indented by wrap_with_indent
                result.push(Line::from(Span::styled(line_content.clone(), style)));
            }
        }
    }

    // Add blank line separator between messages
    result.push(Line::from(""));
    result
}

/// Format lines for a specific message role (dispatch helper)
pub fn format_message_by_role(
    message: &Message,
    throbber: char,
    content_width: usize,
) -> Vec<Line<'static>> {
    match message.role {
        Role::User => format_user_message(message, content_width),
        Role::Assistant => format_assistant_message(message, throbber, content_width),
        Role::System => format_system_message(message, content_width),
    }
}
