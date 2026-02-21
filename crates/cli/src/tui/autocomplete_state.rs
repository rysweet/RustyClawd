//! Autocomplete state management for slash command completion.
//!
//! Self-contained "brick" module owning all autocomplete data and navigation logic.
//! The public API (`AutocompleteManager`) wraps an `Option<AutocompleteState>` and
//! exposes activate / clear / navigate / query methods.

/// Completion item for slash command autocomplete
#[derive(Clone, Debug)]
pub struct CompletionItem {
    /// Command name (without leading /)
    pub command: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional argument hint
    pub argument_hint: Option<String>,
}

/// Autocomplete state for slash commands
#[derive(Clone, Debug)]
pub struct AutocompleteState {
    /// All available completions
    pub items: Vec<CompletionItem>,
    /// Currently selected index
    pub selected: usize,
}

/// Manager wrapping `Option<AutocompleteState>` with clean navigation API.
pub struct AutocompleteManager {
    state: Option<AutocompleteState>,
}

impl AutocompleteManager {
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Activate autocomplete with the given completions.
    /// Returns `true` if focus changed (i.e. state transitioned from None to Some or vice versa).
    pub fn activate(&mut self, items: Vec<CompletionItem>) -> bool {
        if items.is_empty() {
            let had_state = self.state.is_some();
            self.state = None;
            had_state // focus changed only if we cleared existing state
        } else {
            let was_none = self.state.is_none();
            self.state = Some(AutocompleteState { items, selected: 0 });
            was_none // focus changed if we went from None -> Some
        }
    }

    /// Clear autocomplete. Returns `true` if focus changed (had active state).
    pub fn clear(&mut self) -> bool {
        let had_state = self.state.is_some();
        self.state = None;
        had_state
    }

    /// Navigate to next item (wraps around).
    pub fn next(&mut self) {
        if let Some(ref mut ac) = self.state {
            if ac.selected < ac.items.len().saturating_sub(1) {
                ac.selected += 1;
            } else {
                ac.selected = 0;
            }
        }
    }

    /// Navigate to previous item (wraps around).
    pub fn prev(&mut self) {
        if let Some(ref mut ac) = self.state {
            if ac.selected > 0 {
                ac.selected -= 1;
            } else {
                ac.selected = ac.items.len().saturating_sub(1);
            }
        }
    }

    /// Get the currently selected completion item.
    pub fn selected(&self) -> Option<&CompletionItem> {
        self.state.as_ref().and_then(|ac| ac.items.get(ac.selected))
    }

    /// Check if autocomplete is active.
    pub fn is_active(&self) -> bool {
        self.state.is_some()
    }

    /// Get the autocomplete state (for rendering).
    pub fn state(&self) -> Option<&AutocompleteState> {
        self.state.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_inactive() {
        let mgr = AutocompleteManager::new();
        assert!(!mgr.is_active());
        assert!(mgr.selected().is_none());
        assert!(mgr.state().is_none());
    }

    #[test]
    fn test_activate_with_items() {
        let mut mgr = AutocompleteManager::new();
        let changed = mgr.activate(vec![
            CompletionItem {
                command: "help".to_string(),
                description: Some("Show help".to_string()),
                argument_hint: None,
            },
            CompletionItem {
                command: "exit".to_string(),
                description: None,
                argument_hint: None,
            },
        ]);

        assert!(changed);
        assert!(mgr.is_active());
        assert_eq!(mgr.selected().unwrap().command, "help");
        assert_eq!(mgr.state().unwrap().items.len(), 2);
        assert_eq!(mgr.state().unwrap().selected, 0);
    }

    #[test]
    fn test_activate_empty_does_not_activate() {
        let mut mgr = AutocompleteManager::new();
        let changed = mgr.activate(vec![]);
        assert!(!changed); // was None, still None
        assert!(!mgr.is_active());
    }

    #[test]
    fn test_activate_empty_clears_existing() {
        let mut mgr = AutocompleteManager::new();
        mgr.activate(vec![CompletionItem {
            command: "a".to_string(),
            description: None,
            argument_hint: None,
        }]);
        assert!(mgr.is_active());

        let changed = mgr.activate(vec![]);
        assert!(changed); // had state, now cleared
        assert!(!mgr.is_active());
    }

    #[test]
    fn test_clear() {
        let mut mgr = AutocompleteManager::new();
        mgr.activate(vec![CompletionItem {
            command: "a".to_string(),
            description: None,
            argument_hint: None,
        }]);

        let changed = mgr.clear();
        assert!(changed);
        assert!(!mgr.is_active());
        assert!(mgr.selected().is_none());
    }

    #[test]
    fn test_clear_when_inactive() {
        let mut mgr = AutocompleteManager::new();
        let changed = mgr.clear();
        assert!(!changed);
    }

    #[test]
    fn test_navigation_wraps() {
        let mut mgr = AutocompleteManager::new();
        mgr.activate(vec![
            CompletionItem {
                command: "a".to_string(),
                description: None,
                argument_hint: None,
            },
            CompletionItem {
                command: "b".to_string(),
                description: None,
                argument_hint: None,
            },
            CompletionItem {
                command: "c".to_string(),
                description: None,
                argument_hint: None,
            },
        ]);

        assert_eq!(mgr.selected().unwrap().command, "a");

        mgr.next();
        assert_eq!(mgr.selected().unwrap().command, "b");

        mgr.next();
        assert_eq!(mgr.selected().unwrap().command, "c");

        // Wrap to top
        mgr.next();
        assert_eq!(mgr.selected().unwrap().command, "a");

        // Wrap to bottom
        mgr.prev();
        assert_eq!(mgr.selected().unwrap().command, "c");
    }

    #[test]
    fn test_nav_on_inactive_is_noop() {
        let mut mgr = AutocompleteManager::new();
        mgr.next(); // should not panic
        mgr.prev(); // should not panic
    }
}
