//! Focus management, layout cache, and pane wrappers for App.
//!
//! Extracted from app.rs to keep the main App module focused.
//! Contains LayoutCache, PaneWrapper (rat-focus integration), and focus caching logic.

use super::App;
use rat_focus::{Focus, FocusBuilder, FocusFlag, HasFocus, Navigation};
use ratatui::layout::Rect;

/// Layout cache - stores pane areas from last render for hit testing
#[derive(Clone, Debug, Default)]
pub struct LayoutCache {
    /// Messages pane area
    pub messages_area: Rect,
    /// Input pane area
    pub input_area: Rect,
    /// Debug panel area (when visible)
    pub debug_area: Option<Rect>,
}

/// Generic pane wrapper for rat-focus integration.
/// Z_ORDER=0 uses Regular navigation (keyboard Tab); Z_ORDER>0 uses Mouse-only navigation.
pub struct PaneWrapper<const Z_ORDER: u16> {
    pub focus: FocusFlag,
    pub area: Rect,
}

impl<const Z_ORDER: u16> HasFocus for PaneWrapper<Z_ORDER> {
    fn build(&self, builder: &mut FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn area_z(&self) -> u16 {
        Z_ORDER
    }

    fn navigable(&self) -> Navigation {
        if Z_ORDER == 0 {
            Navigation::Regular
        } else {
            Navigation::Mouse
        }
    }
}

/// Type aliases preserving the original names for backward compatibility
pub type MessagesPaneWrapper = PaneWrapper<0>;
pub type InputPaneWrapper = PaneWrapper<0>;
pub type DebugPaneWrapper = PaneWrapper<0>;
pub type AutocompletePopupWrapper = PaneWrapper<1>;
pub type MemoryModalWrapper = PaneWrapper<2>;
pub type PermissionsModalWrapper = PaneWrapper<3>;

impl App {
    // === Focus management ===

    /// Get focus flag for messages pane
    pub fn focus_messages(&self) -> FocusFlag {
        self.focus_messages.clone()
    }

    /// Get focus flag for input pane
    pub fn focus_input(&self) -> FocusFlag {
        self.focus_input.clone()
    }

    /// Get focus flag for debug panel
    pub fn focus_debug(&self) -> FocusFlag {
        self.focus_debug.clone()
    }

    /// Get focus flag for autocomplete popup
    pub fn focus_autocomplete(&self) -> FocusFlag {
        self.focus_autocomplete.clone()
    }

    /// Get focus flag for memory modal
    pub fn focus_memory_modal(&self) -> FocusFlag {
        self.focus_memory_modal.clone()
    }

    /// Get focus flag for permissions modal
    pub fn focus_permissions_modal(&self) -> FocusFlag {
        self.focus_permissions_modal.clone()
    }

    /// Update layout cache from render
    pub fn update_layout_cache(&mut self, cache: LayoutCache) {
        // Invalidate focus when layout areas change (e.g., terminal resize)
        if self.layout_cache.messages_area != cache.messages_area
            || self.layout_cache.input_area != cache.input_area
            || self.layout_cache.debug_area != cache.debug_area
        {
            self.focus_dirty = true;
        }
        self.layout_cache = cache;
    }

    /// Get layout cache
    pub fn layout_cache(&self) -> &LayoutCache {
        &self.layout_cache
    }

    // === Focus caching ===

    /// Mark the focus structure as needing a rebuild
    pub fn invalidate_focus(&mut self) {
        self.focus_dirty = true;
    }

    /// Check if focus needs rebuilding
    pub fn is_focus_dirty(&self) -> bool {
        self.focus_dirty
    }

    /// Get the cached focus, rebuilding if necessary.
    /// Returns None if layout cache is not initialized (zero-size area).
    pub fn get_or_rebuild_focus(&mut self) -> Option<&mut Focus> {
        let cache = self.layout_cache.clone();

        // Skip if layout cache is not initialized
        if cache.messages_area.width == 0 && cache.messages_area.height == 0 {
            return None;
        }

        if self.focus_dirty || self.cached_focus.is_none() {
            // Rebuild focus structure
            let mut builder = FocusBuilder::default();

            let messages_wrapper = MessagesPaneWrapper {
                focus: self.focus_messages.clone(),
                area: cache.messages_area,
            };
            HasFocus::build(&messages_wrapper, &mut builder);

            let input_wrapper = InputPaneWrapper {
                focus: self.focus_input.clone(),
                area: cache.input_area,
            };
            HasFocus::build(&input_wrapper, &mut builder);

            if let Some(debug_area) = cache.debug_area {
                let debug_wrapper = DebugPaneWrapper {
                    focus: self.focus_debug.clone(),
                    area: debug_area,
                };
                HasFocus::build(&debug_wrapper, &mut builder);
            }

            if self.autocomplete.is_active() {
                let autocomplete_wrapper = AutocompletePopupWrapper {
                    focus: self.focus_autocomplete.clone(),
                    area: cache.input_area,
                };
                HasFocus::build(&autocomplete_wrapper, &mut builder);
            }

            if self.modals.memory_modal_active() {
                let memory_modal_wrapper = MemoryModalWrapper {
                    focus: self.focus_memory_modal.clone(),
                    area: cache.input_area,
                };
                HasFocus::build(&memory_modal_wrapper, &mut builder);
            }

            if self.modals.permissions_modal_active() {
                let permissions_modal_wrapper = PermissionsModalWrapper {
                    focus: self.focus_permissions_modal.clone(),
                    area: cache.input_area,
                };
                HasFocus::build(&permissions_modal_wrapper, &mut builder);
            }

            self.cached_focus = Some(builder.build());
            self.focus_dirty = false;
        }

        self.cached_focus.as_mut()
    }
}
