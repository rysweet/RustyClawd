//! Input viewport management for horizontal scrolling
//!
//! This module provides functionality to manage a scrollable viewport for text input,
//! ensuring the cursor remains visible when the input text exceeds the available width.

use unicode_segmentation::UnicodeSegmentation;

/// Viewport information for rendering scrollable input
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputViewport {
    /// The visible portion of the text
    pub visible_text: String,
    /// Cursor position within the visible viewport (0-indexed)
    pub viewport_cursor_pos: usize,
    /// Offset of the viewport into the full text (in grapheme clusters)
    pub viewport_offset: usize,
}

/// Calculate the viewport for a given input text and cursor position
///
/// # Arguments
/// * `text` - The full input text
/// * `cursor_pos` - The cursor position in the full text (in grapheme clusters)
/// * `available_width` - The width available for displaying text
///
/// # Returns
/// An `InputViewport` containing the visible text and adjusted cursor position
///
/// # Algorithm
/// 1. If text fits within available width, show all text
/// 2. Otherwise, calculate viewport offset to keep cursor visible
/// 3. Center cursor when possible, showing context on both sides
/// 4. Extract visible substring using grapheme-safe operations
pub fn calculate_viewport(text: &str, cursor_pos: usize, available_width: usize) -> InputViewport {
    let text_len = count_graphemes(text);

    // If text fits within available width, no scrolling needed
    if text_len <= available_width {
        return InputViewport {
            visible_text: text.to_string(),
            viewport_cursor_pos: cursor_pos,
            viewport_offset: 0,
        };
    }

    // Calculate viewport offset to keep cursor visible
    // Try to center the cursor with context on both sides
    let half_width = available_width / 2;

    let viewport_offset = if cursor_pos <= half_width {
        // Cursor is near the start, show from beginning
        0
    } else if cursor_pos >= text_len.saturating_sub(half_width) {
        // Cursor is near the end, show the end portion
        text_len.saturating_sub(available_width)
    } else {
        // Cursor is in the middle, center it
        cursor_pos.saturating_sub(half_width)
    };

    // Extract visible text
    let visible_text = extract_visible_text(text, viewport_offset, available_width);

    // Calculate cursor position within viewport
    let viewport_cursor_pos = cursor_pos.saturating_sub(viewport_offset);

    InputViewport {
        visible_text,
        viewport_cursor_pos,
        viewport_offset,
    }
}

/// Extract a substring of text using grapheme-safe operations
///
/// # Arguments
/// * `text` - The source text
/// * `start` - Start position in grapheme clusters
/// * `length` - Maximum length in grapheme clusters
///
/// # Returns
/// A substring containing up to `length` grapheme clusters starting at `start`
pub fn extract_visible_text(text: &str, start: usize, length: usize) -> String {
    grapheme_substring(text, start, start + length)
}

/// Calculate cursor coordinates for rendering
///
/// # Arguments
/// * `viewport` - The viewport information
/// * `prompt_width` - Width of the prompt (e.g., "You> ")
/// * `area_x` - X coordinate of the input area
/// * `area_y` - Y coordinate of the input area
///
/// # Returns
/// A tuple of (cursor_x, cursor_y) for positioning the terminal cursor
pub fn calculate_cursor_coords(
    viewport: &InputViewport,
    prompt_width: u16,
    area_x: u16,
    area_y: u16,
) -> (u16, u16) {
    let cursor_x = area_x + prompt_width + viewport.viewport_cursor_pos as u16;
    let cursor_y = area_y;
    (cursor_x, cursor_y)
}

/// Count grapheme clusters in a string
///
/// This is the correct way to count "characters" as perceived by users,
/// as it properly handles multi-byte Unicode characters, combining marks,
/// and emoji.
fn count_graphemes(text: &str) -> usize {
    text.graphemes(true).count()
}

/// Extract a substring by grapheme cluster indices
///
/// # Arguments
/// * `text` - The source text
/// * `start` - Start index (inclusive) in grapheme clusters
/// * `end` - End index (exclusive) in grapheme clusters
///
/// # Returns
/// A substring containing grapheme clusters from `start` to `end`
fn grapheme_substring(text: &str, start: usize, end: usize) -> String {
    text.graphemes(true)
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_graphemes_ascii() {
        assert_eq!(count_graphemes("hello"), 5);
        assert_eq!(count_graphemes(""), 0);
        assert_eq!(count_graphemes("a"), 1);
    }

    #[test]
    fn test_count_graphemes_unicode() {
        // Emoji
        assert_eq!(count_graphemes("👋"), 1);
        assert_eq!(count_graphemes("hello👋world"), 11);

        // Multi-byte characters
        assert_eq!(count_graphemes("café"), 4);
        assert_eq!(count_graphemes("日本語"), 3);

        // Combining characters
        assert_eq!(count_graphemes("é"), 1); // e + combining acute
    }

    #[test]
    fn test_grapheme_substring_ascii() {
        assert_eq!(grapheme_substring("hello world", 0, 5), "hello");
        assert_eq!(grapheme_substring("hello world", 6, 11), "world");
        assert_eq!(grapheme_substring("hello world", 0, 11), "hello world");
    }

    #[test]
    fn test_grapheme_substring_unicode() {
        assert_eq!(grapheme_substring("hello👋world", 5, 6), "👋");
        assert_eq!(grapheme_substring("café", 0, 3), "caf");
        assert_eq!(grapheme_substring("日本語", 0, 2), "日本");
    }

    #[test]
    fn test_grapheme_substring_out_of_bounds() {
        assert_eq!(grapheme_substring("hello", 0, 100), "hello");
        assert_eq!(grapheme_substring("hello", 10, 20), "");
        assert_eq!(grapheme_substring("hello", 3, 3), "");
    }

    #[test]
    fn test_viewport_text_fits() {
        let viewport = calculate_viewport("hello", 3, 20);
        assert_eq!(viewport.visible_text, "hello");
        assert_eq!(viewport.viewport_cursor_pos, 3);
        assert_eq!(viewport.viewport_offset, 0);
    }

    #[test]
    fn test_viewport_cursor_at_start() {
        let text = "this is a very long text that needs scrolling";
        let viewport = calculate_viewport(text, 0, 20);

        assert_eq!(viewport.viewport_offset, 0);
        assert_eq!(viewport.viewport_cursor_pos, 0);
        assert_eq!(count_graphemes(&viewport.visible_text), 20);
    }

    #[test]
    fn test_viewport_cursor_at_end() {
        let text = "this is a very long text that needs scrolling";
        let text_len = count_graphemes(text);
        let available_width = 20;

        let viewport = calculate_viewport(text, text_len, available_width);

        assert_eq!(viewport.viewport_offset, text_len - available_width);
        assert_eq!(viewport.viewport_cursor_pos, available_width);
        assert_eq!(count_graphemes(&viewport.visible_text), available_width);
    }

    #[test]
    fn test_viewport_cursor_in_middle() {
        let text = "this is a very long text that needs scrolling";
        let cursor_pos = 25;
        let available_width = 20;

        let viewport = calculate_viewport(text, cursor_pos, available_width);

        // Cursor should be roughly centered
        assert!(viewport.viewport_offset > 0);
        assert!(viewport.viewport_offset < cursor_pos);
        assert_eq!(
            viewport.viewport_cursor_pos,
            cursor_pos - viewport.viewport_offset
        );
        assert_eq!(count_graphemes(&viewport.visible_text), available_width);
    }

    #[test]
    fn test_viewport_with_unicode() {
        let text = "hello 👋 world 🌍 test 🚀 end";
        let cursor_pos = 15; // Somewhere in the middle
        let available_width = 10;

        let viewport = calculate_viewport(text, cursor_pos, available_width);

        assert_eq!(count_graphemes(&viewport.visible_text), available_width);
        assert!(viewport.viewport_cursor_pos <= available_width);
    }

    #[test]
    fn test_viewport_near_start() {
        let text = "this is a very long text that needs scrolling";
        let cursor_pos = 5;
        let available_width = 20;

        let viewport = calculate_viewport(text, cursor_pos, available_width);

        // Should show from the start since cursor is near beginning
        assert_eq!(viewport.viewport_offset, 0);
        assert_eq!(viewport.viewport_cursor_pos, cursor_pos);
    }

    #[test]
    fn test_viewport_near_end() {
        let text = "this is a very long text that needs scrolling";
        let text_len = count_graphemes(text);
        let cursor_pos = text_len - 5;
        let available_width = 20;

        let viewport = calculate_viewport(text, cursor_pos, available_width);

        // Should show the end portion
        assert_eq!(viewport.viewport_offset, text_len - available_width);
        assert!(viewport.viewport_cursor_pos >= available_width - 10);
    }

    #[test]
    fn test_calculate_cursor_coords() {
        let viewport = InputViewport {
            visible_text: "visible text".to_string(),
            viewport_cursor_pos: 5,
            viewport_offset: 10,
        };

        let (cursor_x, cursor_y) = calculate_cursor_coords(&viewport, 5, 10, 20);

        assert_eq!(cursor_x, 10 + 5 + 5); // area_x + prompt_width + viewport_cursor_pos
        assert_eq!(cursor_y, 20); // area_y
    }

    #[test]
    fn test_extract_visible_text() {
        let text = "hello world test";
        assert_eq!(extract_visible_text(text, 0, 5), "hello");
        assert_eq!(extract_visible_text(text, 6, 5), "world");
        assert_eq!(extract_visible_text(text, 0, 100), text);
    }

    #[test]
    fn test_viewport_empty_text() {
        let viewport = calculate_viewport("", 0, 20);
        assert_eq!(viewport.visible_text, "");
        assert_eq!(viewport.viewport_cursor_pos, 0);
        assert_eq!(viewport.viewport_offset, 0);
    }

    #[test]
    fn test_viewport_single_character() {
        let viewport = calculate_viewport("a", 1, 20);
        assert_eq!(viewport.visible_text, "a");
        assert_eq!(viewport.viewport_cursor_pos, 1);
        assert_eq!(viewport.viewport_offset, 0);
    }

    #[test]
    fn test_viewport_exact_width() {
        let text = "12345678901234567890"; // Exactly 20 characters
        let viewport = calculate_viewport(text, 10, 20);

        assert_eq!(viewport.visible_text, text);
        assert_eq!(viewport.viewport_cursor_pos, 10);
        assert_eq!(viewport.viewport_offset, 0);
    }

    #[test]
    fn test_viewport_one_over_width() {
        let text = "123456789012345678901"; // 21 characters
        let viewport = calculate_viewport(text, 10, 20);

        assert_eq!(count_graphemes(&viewport.visible_text), 20);
        assert!(viewport.viewport_cursor_pos <= 20);
    }
}
