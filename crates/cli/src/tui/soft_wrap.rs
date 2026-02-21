//! Soft wrap state management for input text wrapping
//!
//! Tracks which lines are auto-wrapped (soft breaks) versus user-inserted newlines (hard breaks).
//! Used by the reflow engine in App to maintain correct cursor positioning during text wrapping.

use std::collections::HashSet;
use unicode_width::UnicodeWidthChar;

/// Soft wrap state - tracks which lines are auto-wrapped (not user newlines)
#[derive(Clone, Debug)]
pub struct SoftWrapState {
    /// Set of line indices that END with a soft break
    /// (i.e., line was created by auto-wrap, not user Enter)
    pub(crate) soft_break_lines: HashSet<usize>,

    /// Cached inner width of input area (excluding borders)
    inner_width: u16,

    /// Tab width for width calculations (synced with TextArea's tab_length)
    pub(crate) tab_width: u8,
}

impl Default for SoftWrapState {
    fn default() -> Self {
        Self {
            soft_break_lines: HashSet::new(),
            inner_width: 80, // Default, will be updated from layout
            tab_width: 4,    // Match TextArea default
        }
    }
}

#[allow(dead_code)] // Soft wrap tracking API used by rendering layer
impl SoftWrapState {
    /// Check if a line ends with a soft break
    pub fn is_soft_break(&self, line_idx: usize) -> bool {
        self.soft_break_lines.contains(&line_idx)
    }

    /// Mark a line as ending with a soft break
    pub fn add_soft_break(&mut self, line_idx: usize) {
        self.soft_break_lines.insert(line_idx);
    }

    /// Remove soft break marker from a line
    pub fn remove_soft_break(&mut self, line_idx: usize) {
        self.soft_break_lines.remove(&line_idx);
    }

    /// Clear all soft break markers
    pub fn clear(&mut self) {
        self.soft_break_lines.clear();
    }

    /// Update inner width (called when layout changes)
    pub fn update_width(&mut self, width: u16) {
        self.inner_width = width;
    }

    /// Get current inner width
    pub fn inner_width(&self) -> u16 {
        self.inner_width
    }

    /// Calculate visual width of a line accounting for tabs and Unicode
    pub fn calculate_line_width(&self, line: &str) -> usize {
        let mut width = 0;
        for c in line.chars() {
            if c == '\t' {
                // Tab advances to next tab stop
                let tab_width = self.tab_width as usize;
                width += tab_width - (width % tab_width);
            } else {
                width += UnicodeWidthChar::width(c).unwrap_or(0);
            }
        }
        width
    }
}
