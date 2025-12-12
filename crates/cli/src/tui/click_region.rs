//! Click region tracking for mouse interactions
//!
//! Handles coordinate translation between absolute screen space and panel-relative viewport space.
//! Each panel (messages, debug, etc.) registers regions relative to its inner area,
//! and hit testing automatically translates mouse coordinates to the appropriate space.

use ratatui::layout::{Position, Rect};
use ratatui::widgets::{Block, Borders};

use super::app::LayoutCache;

/// Identifies what was clicked
#[derive(Debug, Clone, PartialEq)]
pub enum ClickTarget {
    /// Message in conversation (index into App::messages)
    Message { index: usize },
    /// Debug message (index into App::debug_messages) - future feature
    DebugMessage { index: usize },
    /// Status bar button (e.g., "menu", "debug")
    StatusBarItem { id: String },
    /// Clicked nothing interactive
    Background,
}

/// Tracks clickable regions in the UI
/// Updated during render as widgets are drawn
///
/// Coordinate spaces:
/// - Status bar items: Absolute screen coordinates
/// - Panel regions (messages, debug): Relative to panel inner area (after borders)
#[derive(Debug, Default)]
pub struct ClickableRegions {
    /// Message areas (index -> viewport-relative rect)
    /// Rects are relative to messages panel inner area
    messages: Vec<(usize, Rect)>,

    /// Debug message areas (index -> viewport-relative rect)
    /// Rects are relative to debug panel inner area
    debug_messages: Vec<(usize, Rect)>,

    /// Status bar item areas (id -> absolute screen rect)
    status_items: Vec<(String, Rect)>,
}

impl ClickableRegions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all regions (called at start of render)
    pub fn clear(&mut self) {
        self.messages.clear();
        self.debug_messages.clear();
        self.status_items.clear();
    }

    /// Register a message's position (viewport-relative coordinates)
    pub fn add_message(&mut self, index: usize, rect: Rect) {
        self.messages.push((index, rect));
    }

    /// Register a debug message's position (viewport-relative coordinates)
    pub fn add_debug_message(&mut self, index: usize, rect: Rect) {
        self.debug_messages.push((index, rect));
    }

    /// Register a status bar item's position (absolute screen coordinates)
    pub fn add_status_item(&mut self, id: impl Into<String>, rect: Rect) {
        self.status_items.push((id.into(), rect));
    }

    /// Hit-test a click position with automatic coordinate translation
    ///
    /// Takes mouse coordinates in absolute screen space and translates them to the appropriate
    /// panel-relative coordinate space before checking regions.
    ///
    /// # Arguments
    /// * `mouse_x`, `mouse_y` - Absolute screen coordinates from mouse event
    /// * `layout` - Current layout cache with panel areas
    ///
    /// # Coordinate Translation
    /// - Status bar: Uses absolute coords directly (no translation)
    /// - Messages panel: Translates to inner area relative coords using Block::inner()
    /// - Debug panel: Translates to inner area relative coords using Block::inner()
    pub fn hit_test(&self, mouse_x: u16, mouse_y: u16, layout: &LayoutCache) -> ClickTarget {
        let mouse_pos = Position::new(mouse_x, mouse_y);

        // Check status bar items first (absolute screen coordinates, no translation)
        for (id, rect) in &self.status_items {
            if rect.contains(mouse_pos) {
                return ClickTarget::StatusBarItem { id: id.clone() };
            }
        }

        // Check messages panel (translate to panel-relative coordinates)
        if layout.messages_area.contains(mouse_pos) {
            // Use Block::inner() to properly account for borders
            let inner = Block::default()
                .borders(Borders::ALL)
                .inner(layout.messages_area);

            let inner_x = mouse_x.saturating_sub(inner.x);
            let inner_y = mouse_y.saturating_sub(inner.y);
            let inner_pos = Position::new(inner_x, inner_y);

            for (index, rect) in &self.messages {
                if rect.contains(inner_pos) {
                    return ClickTarget::Message { index: *index };
                }
            }
        }

        // Check debug panel (translate to panel-relative coordinates)
        if let Some(debug_area) = layout.debug_area {
            if debug_area.contains(mouse_pos) {
                // Use Block::inner() to properly account for borders
                let inner = Block::default()
                    .borders(Borders::ALL)
                    .inner(debug_area);

                let inner_x = mouse_x.saturating_sub(inner.x);
                let inner_y = mouse_y.saturating_sub(inner.y);
                let inner_pos = Position::new(inner_x, inner_y);

                for (index, rect) in &self.debug_messages {
                    if rect.contains(inner_pos) {
                        return ClickTarget::DebugMessage { index: *index };
                    }
                }
            }
        }

        ClickTarget::Background
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_hit_test() {
        let mut regions = ClickableRegions::new();
        // Register message in viewport-relative coords (relative to inner area)
        regions.add_message(0, Rect::new(0, 0, 78, 3));

        // Create layout with messages panel at screen position (0, 1) with borders
        // Inner area will be at (1, 2) after accounting for borders
        let layout = LayoutCache {
            messages_area: Rect::new(0, 1, 80, 20),
            input_area: Rect::new(0, 21, 80, 3),
            debug_area: None,
        };

        // Click at absolute screen position (11, 3)
        // Should translate to inner (10, 1) which hits the message
        assert_eq!(
            regions.hit_test(11, 3, &layout),
            ClickTarget::Message { index: 0 }
        );

        // Click outside message area
        assert_eq!(
            regions.hit_test(10, 50, &layout),
            ClickTarget::Background
        );
    }

    #[test]
    fn test_status_bar_hit_test() {
        let mut regions = ClickableRegions::new();
        // Status bar items use absolute screen coordinates
        regions.add_status_item("menu", Rect::new(0, 0, 10, 1));
        regions.add_status_item("debug", Rect::new(70, 0, 10, 1));

        let layout = LayoutCache {
            messages_area: Rect::new(0, 1, 80, 20),
            input_area: Rect::new(0, 21, 80, 3),
            debug_area: None,
        };

        assert_eq!(
            regions.hit_test(5, 0, &layout),
            ClickTarget::StatusBarItem { id: "menu".to_string() }
        );

        assert_eq!(
            regions.hit_test(75, 0, &layout),
            ClickTarget::StatusBarItem { id: "debug".to_string() }
        );
    }

    #[test]
    fn test_debug_panel_hit_test() {
        let mut regions = ClickableRegions::new();
        // Register debug message in viewport-relative coords
        regions.add_debug_message(0, Rect::new(0, 0, 58, 2));

        // Layout with debug panel on right side
        let layout = LayoutCache {
            messages_area: Rect::new(0, 1, 120, 40),
            input_area: Rect::new(0, 41, 120, 3),
            debug_area: Some(Rect::new(120, 1, 60, 40)),
        };

        // Click at absolute screen position (121, 2)
        // Should translate to debug panel inner (0, 0) which hits the debug message
        assert_eq!(
            regions.hit_test(121, 2, &layout),
            ClickTarget::DebugMessage { index: 0 }
        );
    }

    #[test]
    fn test_background_click() {
        let regions = ClickableRegions::new();

        let layout = LayoutCache {
            messages_area: Rect::new(0, 1, 80, 20),
            input_area: Rect::new(0, 21, 80, 3),
            debug_area: None,
        };

        assert_eq!(
            regions.hit_test(50, 50, &layout),
            ClickTarget::Background
        );
    }

    #[test]
    fn test_coordinate_translation() {
        let mut regions = ClickableRegions::new();
        // Message at viewport (5, 10) with size (70, 2)
        regions.add_message(0, Rect::new(5, 10, 70, 2));

        // Messages panel at screen (0, 1) with borders
        // Inner area will be (1, 2)
        let layout = LayoutCache {
            messages_area: Rect::new(0, 1, 80, 40),
            input_area: Rect::new(0, 41, 80, 3),
            debug_area: None,
        };

        // Click at absolute screen (11, 13)
        // Translates to inner (10, 11) which is (5+5, 10+1) = within the message rect
        assert_eq!(
            regions.hit_test(11, 13, &layout),
            ClickTarget::Message { index: 0 }
        );

        // Click at absolute screen (11, 14)
        // Translates to inner (10, 12) which is outside the message rect (y=12 > 10+2)
        assert_eq!(
            regions.hit_test(11, 14, &layout),
            ClickTarget::Background
        );
    }
}
