//! Dynamic layout management for TUI components

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Layout configuration based on visible panels
pub struct LayoutConfig {
    /// Whether debug panel is visible
    pub debug_visible: bool,
    /// Debug panel width (when visible)
    pub debug_width: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            debug_visible: false,
            debug_width: 60, // Default debug panel width
        }
    }
}

/// Organized layout areas for TUI components
pub struct LayoutAreas {
    /// Status bar at top
    pub status: Rect,
    /// Main content area (messages + input)
    pub main: Rect,
    /// Debug panel on right (if visible)
    pub debug: Option<Rect>,
}

/// Dynamic layout organizer - adapts to visible panels
pub struct LayoutOrganizer;

impl LayoutOrganizer {
    /// Calculate layout based on configuration
    pub fn organize(area: Rect, config: &LayoutConfig) -> LayoutAreas {
        // First split: status bar + content
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Status bar
                Constraint::Min(0),     // Content area
            ])
            .split(area);

        let status = vertical[0];
        let content_area = vertical[1];

        // If debug is visible, split content horizontally
        if config.debug_visible {
            let horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(0),                     // Main content
                    Constraint::Length(config.debug_width), // Debug panel
                ])
                .split(content_area);

            LayoutAreas {
                status,
                main: horizontal[0],
                debug: Some(horizontal[1]),
            }
        } else {
            // Full width for main content
            LayoutAreas {
                status,
                main: content_area,
                debug: None,
            }
        }
    }

    /// Split main area into messages + input
    pub fn split_main(main_area: Rect) -> (Rect, Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),     // Messages
                Constraint::Length(3),  // Input
            ])
            .split(main_area);

        (chunks[0], chunks[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_without_debug() {
        let area = Rect::new(0, 0, 100, 30);
        let config = LayoutConfig::default();

        let layout = LayoutOrganizer::organize(area, &config);

        assert_eq!(layout.status.height, 1);
        assert_eq!(layout.main.height, 29);
        assert!(layout.debug.is_none());
        assert_eq!(layout.main.width, 100);
    }

    #[test]
    fn test_layout_with_debug() {
        let area = Rect::new(0, 0, 100, 30);
        let mut config = LayoutConfig::default();
        config.debug_visible = true;
        config.debug_width = 40;

        let layout = LayoutOrganizer::organize(area, &config);

        assert_eq!(layout.status.height, 1);
        assert!(layout.debug.is_some());
        assert_eq!(layout.debug.unwrap().width, 40);
        assert_eq!(layout.main.width, 60); // 100 - 40
    }

    #[test]
    fn test_split_main() {
        let main_area = Rect::new(0, 0, 80, 20);
        let (messages, input) = LayoutOrganizer::split_main(main_area);

        assert_eq!(input.height, 3);
        assert_eq!(messages.height, 17); // 20 - 3
    }
}
