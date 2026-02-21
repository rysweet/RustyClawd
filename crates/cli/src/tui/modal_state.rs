//! Modal state management
//!
//! Manages memory modal and permissions modal state.
//! Centralizes modal activation, navigation, and cleanup.

use crate::commands::permissions_search_state::PermissionsSearchState;

/// Memory destination for saving user memories
#[derive(Clone, Debug)]
pub struct MemoryDestination {
    /// Display name (e.g., "User memory", "Project memory")
    pub name: String,
    /// File path where memory will be saved
    pub file_path: String,
    /// Optional description/hint (e.g., "Saved in ~/.claude/CLAUDE.md")
    pub description: Option<String>,
    /// Whether this is an imported context file
    pub is_imported: bool,
}

/// Memory modal state
#[derive(Clone, Debug)]
pub struct MemoryModalState {
    /// Memory text to be saved
    pub memory_text: String,
    /// Available destinations
    pub destinations: Vec<MemoryDestination>,
    /// Currently selected index
    pub selected: usize,
}

/// Manages memory and permissions modals
pub struct ModalManager {
    /// Memory modal state
    memory_modal: Option<MemoryModalState>,

    /// Permissions search modal state
    permissions_modal: Option<PermissionsSearchState>,
}

impl ModalManager {
    pub fn new() -> Self {
        Self {
            memory_modal: None,
            permissions_modal: None,
        }
    }

    // === Memory modal ===

    /// Activate memory modal with destinations
    pub fn activate_memory_modal(
        &mut self,
        memory_text: String,
        destinations: Vec<MemoryDestination>,
    ) {
        if destinations.is_empty() {
            self.memory_modal = None;
        } else {
            self.memory_modal = Some(MemoryModalState {
                memory_text,
                destinations,
                selected: 0,
            });
        }
    }

    /// Update memory text without resetting selection
    pub fn update_memory_text(&mut self, memory_text: String) {
        if let Some(ref mut modal) = self.memory_modal {
            modal.memory_text = memory_text;
        }
    }

    /// Clear memory modal
    pub fn clear_memory_modal(&mut self) {
        self.memory_modal = None;
    }

    /// Navigate memory modal selection up
    pub fn memory_modal_prev(&mut self) {
        if let Some(ref mut modal) = self.memory_modal {
            if modal.selected > 0 {
                modal.selected -= 1;
            } else {
                // Wrap to bottom
                modal.selected = modal.destinations.len().saturating_sub(1);
            }
        }
    }

    /// Navigate memory modal selection down
    pub fn memory_modal_next(&mut self) {
        if let Some(ref mut modal) = self.memory_modal {
            if modal.selected < modal.destinations.len().saturating_sub(1) {
                modal.selected += 1;
            } else {
                // Wrap to top
                modal.selected = 0;
            }
        }
    }

    /// Get selected memory destination
    pub fn memory_modal_selected(&self) -> Option<&MemoryDestination> {
        self.memory_modal
            .as_ref()
            .and_then(|modal| modal.destinations.get(modal.selected))
    }

    /// Check if memory modal is active
    pub fn memory_modal_active(&self) -> bool {
        self.memory_modal.is_some()
    }

    /// Get memory modal state (for rendering)
    pub fn memory_modal(&self) -> Option<&MemoryModalState> {
        self.memory_modal.as_ref()
    }

    // === Permissions modal ===

    /// Activate permissions search modal
    pub fn activate_permissions_modal(&mut self) {
        self.permissions_modal = Some(PermissionsSearchState::new());
    }

    /// Clear permissions modal
    pub fn clear_permissions_modal(&mut self) {
        self.permissions_modal = None;
    }

    /// Check if permissions modal is active
    pub fn permissions_modal_active(&self) -> bool {
        self.permissions_modal.is_some()
    }

    /// Get mutable reference to permissions modal state
    pub fn permissions_modal_mut(&mut self) -> Option<&mut PermissionsSearchState> {
        self.permissions_modal.as_mut()
    }

    /// Get permissions modal state (for rendering)
    pub fn permissions_modal(&self) -> Option<&PermissionsSearchState> {
        self.permissions_modal.as_ref()
    }
}
