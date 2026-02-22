//! Reusable scroll controller for any scrollable panel.
//! Manages offset, follow-bottom mode, and max-scroll clamping.

/// Reusable scroll controller for any scrollable panel.
/// Manages offset, follow-bottom mode, and max-scroll clamping.
pub struct ScrollController {
    /// Current scroll offset (lines from top)
    offset: usize,
    /// Auto-follow bottom (true = stick to bottom, false = manual scroll)
    follow_bottom: bool,
    /// Maximum valid scroll offset (updated by renderer each frame)
    max_scroll: usize,
}

impl ScrollController {
    pub fn new() -> Self {
        Self {
            offset: 0,
            follow_bottom: true, // Start in auto-follow mode
            max_scroll: 0,       // Will be updated by renderer
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn follow_bottom(&self) -> bool {
        self.follow_bottom
    }

    pub fn scroll_up(&mut self, lines: usize) {
        // If we're in follow mode, transition to manual scroll mode
        if self.follow_bottom {
            self.follow_bottom = false;
            // Initialize offset to max_scroll so we're actually at the bottom
            self.offset = self.max_scroll;
        }
        // Now scroll up from current position
        self.offset = self.offset.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        // If already following bottom, stay there
        if self.follow_bottom {
            return;
        }
        // Increment scroll offset and clamp to valid range
        self.offset = self.offset.saturating_add(lines);
        // Clamp to max_scroll to prevent phantom accumulation
        if self.offset >= self.max_scroll {
            // At or past the bottom - switch to follow mode
            self.follow_bottom = true;
            self.offset = 0; // Renderer will set to max_scroll
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.follow_bottom = true;
        self.offset = 0; // Will be set to max_scroll during render
    }

    /// Update max_scroll from renderer (called after content height calculation).
    /// Also clamps offset to prevent invalid state after terminal resize.
    pub fn update_max_scroll(&mut self, max_scroll: usize) {
        self.max_scroll = max_scroll;
        // Clamp offset when max_scroll changes (e.g., after terminal resize)
        // Only clamp if NOT in follow_bottom mode (which uses max_scroll directly in render)
        if !self.follow_bottom && self.offset > max_scroll {
            self.offset = max_scroll;
        }
    }
}
