//! Dynamic layout management for TUI components

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::tui::app::App;

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

/// Minimum width for main content area (messages + input)
const MIN_MAIN_WIDTH: u16 = 40;

/// Dynamic layout organizer - adapts to visible panels
pub struct LayoutOrganizer;

impl LayoutOrganizer {
    /// Calculate layout based on configuration
    pub fn organize(area: Rect, config: &LayoutConfig) -> LayoutAreas {
        // First split: status bar + content
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Status bar
                Constraint::Min(0),    // Content area
            ])
            .split(area);

        let status = vertical[0];
        let content_area = vertical[1];

        // DEFENSIVE: Auto-hide debug panel if terminal too narrow
        // This prevents zero-width main area which causes rendering errors
        let effective_debug_visible =
            config.debug_visible && content_area.width >= (MIN_MAIN_WIDTH + config.debug_width);

        // If debug is visible AND there's enough space, split content horizontally
        if effective_debug_visible {
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
            // Full width for main content (debug hidden or insufficient space)
            LayoutAreas {
                status,
                main: content_area,
                debug: None,
            }
        }
    }

    /// Split main area into messages + input (with dynamic input height)
    pub fn split_main(main_area: Rect, app: &App) -> (Rect, Rect) {
        let input_height = calculate_input_height(app);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),               // Messages
                Constraint::Length(input_height), // Input (dynamic: 1-10 lines + 2 borders)
            ])
            .split(main_area);

        (chunks[0], chunks[1])
    }
}

/// Calculate dynamic input height based on line count (1-10 lines + 2 borders = 3-12 total)
fn calculate_input_height(app: &App) -> u16 {
    let line_count = app.input_line_count();
    // Min 3 (1 line + 2 borders), Max 12 (10 lines + 2 borders)
    (line_count as u16 + 2).clamp(3, 12)
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
        let config = LayoutConfig {
            debug_visible: true,
            debug_width: 40,
        };

        let layout = LayoutOrganizer::organize(area, &config);

        assert_eq!(layout.status.height, 1);
        assert!(layout.debug.is_some());
        assert_eq!(layout.debug.unwrap().width, 40);
        assert_eq!(layout.main.width, 60); // 100 - 40
    }

    #[test]
    fn test_split_main() {
        use crate::permission_mode::PermissionMode;

        let main_area = Rect::new(0, 0, 80, 20);
        let app = App::new(PermissionMode::default());

        let (messages, input) = LayoutOrganizer::split_main(main_area, &app);

        // With empty input (1 line), input height should be 3 (1 line + 2 borders)
        assert_eq!(input.height, 3);
        assert_eq!(messages.height, 17); // 20 - 3
    }
}
