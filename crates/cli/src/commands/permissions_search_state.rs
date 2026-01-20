//! Permissions Search State Management
//!
//! Manages the state for the interactive permissions search modal, including:
//! - Search query input
//! - Filtered results
//! - Selected item tracking
//! - Search mode activation
//!
//! The state machine handles transitions between normal and search modes,
//! and manages the filtering of permission rules based on user input.

use super::permission_rules::{filter_rules, get_all_rules, PermissionRule};

/// State for the permissions search interface
#[derive(Debug, Clone)]
pub struct PermissionsSearchState {
    /// All available rules (never filtered)
    all_rules: Vec<PermissionRule>,

    /// Currently displayed rules (filtered by search)
    filtered_rules: Vec<PermissionRule>,

    /// Search query string
    search_query: String,

    /// Whether search mode is currently active
    is_searching: bool,

    /// Currently selected rule index (in filtered_rules)
    selected_index: usize,

    /// Scroll offset for display (top visible line)
    scroll_offset: usize,
}

impl PermissionsSearchState {
    /// Create a new permissions search state with all rules loaded
    ///
    /// # Example
    ///
    /// ```
    /// use rustyclawd::commands::permissions_search_state::PermissionsSearchState;
    ///
    /// let state = PermissionsSearchState::new();
    /// assert!(!state.is_searching());
    /// ```
    pub fn new() -> Self {
        let all_rules = get_all_rules();
        let filtered_rules = all_rules.clone();

        Self {
            all_rules,
            filtered_rules,
            search_query: String::new(),
            is_searching: false,
            selected_index: 0,
            scroll_offset: 0,
        }
    }

    /// Enter search mode (triggered by '/')
    ///
    /// Clears any existing search query and resets selection to top.
    ///
    /// # Example
    ///
    /// ```
    /// use rustyclawd::commands::permissions_search_state::PermissionsSearchState;
    ///
    /// let mut state = PermissionsSearchState::new();
    /// state.enter_search_mode();
    /// assert!(state.is_searching());
    /// ```
    pub fn enter_search_mode(&mut self) {
        self.is_searching = true;
        self.search_query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.update_filtered_rules();
    }

    /// Exit search mode (triggered by Esc when in search)
    ///
    /// Clears search query and restores all rules.
    ///
    /// # Example
    ///
    /// ```
    /// use rustyclawd::commands::permissions_search_state::PermissionsSearchState;
    ///
    /// let mut state = PermissionsSearchState::new();
    /// state.enter_search_mode();
    /// state.exit_search_mode();
    /// assert!(!state.is_searching());
    /// ```
    pub fn exit_search_mode(&mut self) {
        self.is_searching = false;
        self.search_query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.update_filtered_rules();
    }

    /// Handle character input in search mode
    ///
    /// Appends character to search query and re-filters results.
    ///
    /// # Arguments
    ///
    /// * `c` - Character to append to search query
    ///
    /// # Example
    ///
    /// ```
    /// use rustyclawd::commands::permissions_search_state::PermissionsSearchState;
    ///
    /// let mut state = PermissionsSearchState::new();
    /// state.enter_search_mode();
    /// state.handle_char_input('b');
    /// state.handle_char_input('a');
    /// state.handle_char_input('s');
    /// assert!(state.search_query().contains("bas"));
    /// ```
    pub fn handle_char_input(&mut self, c: char) {
        if !self.is_searching {
            return;
        }

        self.search_query.push(c);
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.update_filtered_rules();
    }

    /// Handle backspace in search mode
    ///
    /// Removes last character from search query and re-filters.
    ///
    /// # Example
    ///
    /// ```
    /// use rustyclawd::commands::permissions_search_state::PermissionsSearchState;
    ///
    /// let mut state = PermissionsSearchState::new();
    /// state.enter_search_mode();
    /// state.handle_char_input('a');
    /// state.handle_char_input('b');
    /// state.handle_backspace();
    /// assert_eq!(state.search_query(), "a");
    /// ```
    pub fn handle_backspace(&mut self) {
        if !self.is_searching {
            return;
        }

        self.search_query.pop();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.update_filtered_rules();
    }

    /// Navigate selection to previous item (up arrow)
    ///
    /// Wraps to bottom if at top.
    ///
    /// # Example
    ///
    /// ```
    /// use rustyclawd::commands::permissions_search_state::PermissionsSearchState;
    ///
    /// let mut state = PermissionsSearchState::new();
    /// state.select_previous();
    /// // Wraps to last item
    /// assert!(state.selected_index() > 0);
    /// ```
    pub fn select_previous(&mut self) {
        if self.filtered_rules.is_empty() {
            return;
        }

        if self.selected_index == 0 {
            // Wrap to bottom
            self.selected_index = self.filtered_rules.len() - 1;
        } else {
            self.selected_index -= 1;
        }

        self.ensure_selected_visible();
    }

    /// Navigate selection to next item (down arrow)
    ///
    /// Wraps to top if at bottom.
    ///
    /// # Example
    ///
    /// ```
    /// use rustyclawd::commands::permissions_search_state::PermissionsSearchState;
    ///
    /// let mut state = PermissionsSearchState::new();
    /// let initial = state.selected_index();
    /// state.select_next();
    /// assert_eq!(state.selected_index(), initial + 1);
    /// ```
    pub fn select_next(&mut self) {
        if self.filtered_rules.is_empty() {
            return;
        }

        self.selected_index = (self.selected_index + 1) % self.filtered_rules.len();
        self.ensure_selected_visible();
    }

    /// Get currently selected rule (if any)
    ///
    /// Returns None if no rules are displayed.
    ///
    /// # Returns
    ///
    /// Option<&PermissionRule> - Reference to selected rule
    ///
    /// # Example
    ///
    /// ```
    /// use rustyclawd::commands::permissions_search_state::PermissionsSearchState;
    ///
    /// let state = PermissionsSearchState::new();
    /// let selected = state.selected_rule();
    /// assert!(selected.is_some());
    /// ```
    pub fn selected_rule(&self) -> Option<&PermissionRule> {
        self.filtered_rules.get(self.selected_index)
    }

    /// Get search query string
    ///
    /// # Returns
    ///
    /// &str - Current search query
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Check if search mode is active
    ///
    /// # Returns
    ///
    /// bool - True if in search mode
    pub fn is_searching(&self) -> bool {
        self.is_searching
    }

    /// Get filtered rules (currently displayed)
    ///
    /// # Returns
    ///
    /// &[PermissionRule] - Slice of filtered rules
    pub fn filtered_rules(&self) -> &[PermissionRule] {
        &self.filtered_rules
    }

    /// Get all rules (unfiltered)
    ///
    /// # Returns
    ///
    /// &[PermissionRule] - Slice of all rules
    pub fn all_rules(&self) -> &[PermissionRule] {
        &self.all_rules
    }

    /// Get currently selected index
    ///
    /// # Returns
    ///
    /// usize - Index of selected item in filtered_rules
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Get current scroll offset
    ///
    /// # Returns
    ///
    /// usize - Scroll offset (lines from top)
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset (used by UI renderer)
    ///
    /// # Arguments
    ///
    /// * `offset` - New scroll offset
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    /// Get match count as tuple (matches, total)
    ///
    /// # Returns
    ///
    /// (usize, usize) - (filtered count, total count)
    ///
    /// # Example
    ///
    /// ```
    /// use rustyclawd::commands::permissions_search_state::PermissionsSearchState;
    ///
    /// let mut state = PermissionsSearchState::new();
    /// let (matches, total) = state.match_count();
    /// assert_eq!(matches, total); // No filter applied initially
    ///
    /// state.enter_search_mode();
    /// state.handle_char_input('b');
    /// state.handle_char_input('a');
    /// state.handle_char_input('s');
    /// let (matches, total) = state.match_count();
    /// assert!(matches <= total);
    /// ```
    pub fn match_count(&self) -> (usize, usize) {
        (self.filtered_rules.len(), self.all_rules.len())
    }

    /// Update filtered rules based on current search query
    ///
    /// Internal helper called after search query changes.
    fn update_filtered_rules(&mut self) {
        self.filtered_rules = filter_rules(&self.all_rules, &self.search_query);

        // Clamp selection to valid range
        if !self.filtered_rules.is_empty() && self.selected_index >= self.filtered_rules.len() {
            self.selected_index = self.filtered_rules.len() - 1;
        } else if self.filtered_rules.is_empty() {
            self.selected_index = 0;
        }
    }

    /// Ensure selected item is visible in viewport
    ///
    /// Adjusts scroll_offset to keep selected item on screen.
    /// Called internally after selection changes.
    fn ensure_selected_visible(&mut self) {
        // This will be updated by UI renderer based on visible height
        // For now, just ensure scroll offset doesn't go past selection
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
        // Upper bound check will be done by UI renderer with actual viewport height
    }
}

impl Default for PermissionsSearchState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_state_initial_state() {
        let state = PermissionsSearchState::new();

        assert!(!state.is_searching());
        assert_eq!(state.search_query(), "");
        assert!(!state.all_rules().is_empty());
        assert_eq!(state.filtered_rules().len(), state.all_rules().len());
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_enter_search_mode() {
        let mut state = PermissionsSearchState::new();

        state.enter_search_mode();

        assert!(state.is_searching());
        assert_eq!(state.search_query(), "");
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_exit_search_mode() {
        let mut state = PermissionsSearchState::new();

        state.enter_search_mode();
        state.handle_char_input('b');
        state.handle_char_input('a');
        state.exit_search_mode();

        assert!(!state.is_searching());
        assert_eq!(state.search_query(), "");
        assert_eq!(state.filtered_rules().len(), state.all_rules().len());
    }

    #[test]
    fn test_handle_char_input() {
        let mut state = PermissionsSearchState::new();
        state.enter_search_mode();

        state.handle_char_input('b');
        state.handle_char_input('a');
        state.handle_char_input('s');

        assert_eq!(state.search_query(), "bas");
        // Should have filtered results
        assert!(state.filtered_rules().len() <= state.all_rules().len());
    }

    #[test]
    fn test_handle_char_input_resets_selection() {
        let mut state = PermissionsSearchState::new();
        state.enter_search_mode();

        state.select_next();
        state.select_next();
        let index_before = state.selected_index();
        assert!(index_before > 0);

        state.handle_char_input('x');
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_handle_backspace() {
        let mut state = PermissionsSearchState::new();
        state.enter_search_mode();

        state.handle_char_input('a');
        state.handle_char_input('b');
        state.handle_char_input('c');
        assert_eq!(state.search_query(), "abc");

        state.handle_backspace();
        assert_eq!(state.search_query(), "ab");

        state.handle_backspace();
        assert_eq!(state.search_query(), "a");

        state.handle_backspace();
        assert_eq!(state.search_query(), "");
    }

    #[test]
    fn test_select_previous_wraps_to_bottom() {
        let mut state = PermissionsSearchState::new();

        let initial_index = state.selected_index();
        assert_eq!(initial_index, 0);

        state.select_previous();
        assert_eq!(state.selected_index(), state.filtered_rules().len() - 1);
    }

    #[test]
    fn test_select_next_wraps_to_top() {
        let mut state = PermissionsSearchState::new();

        // Navigate to last item
        let last_index = state.filtered_rules().len() - 1;
        while state.selected_index() < last_index {
            state.select_next();
        }

        assert_eq!(state.selected_index(), last_index);

        // Next should wrap to 0
        state.select_next();
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_navigation() {
        let mut state = PermissionsSearchState::new();

        assert_eq!(state.selected_index(), 0);

        state.select_next();
        assert_eq!(state.selected_index(), 1);

        state.select_next();
        assert_eq!(state.selected_index(), 2);

        state.select_previous();
        assert_eq!(state.selected_index(), 1);

        state.select_previous();
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_selected_rule() {
        let state = PermissionsSearchState::new();

        let selected = state.selected_rule();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().tool_name, state.filtered_rules()[0].tool_name);
    }

    #[test]
    fn test_selected_rule_none_when_empty() {
        let mut state = PermissionsSearchState::new();
        state.enter_search_mode();

        // Search for something that doesn't exist
        state.handle_char_input('x');
        state.handle_char_input('y');
        state.handle_char_input('z');
        state.handle_char_input('_');
        state.handle_char_input('n');
        state.handle_char_input('o');
        state.handle_char_input('_');
        state.handle_char_input('m');
        state.handle_char_input('a');
        state.handle_char_input('t');
        state.handle_char_input('c');
        state.handle_char_input('h');

        // No results
        assert!(state.filtered_rules().is_empty());
        assert!(state.selected_rule().is_none());
    }

    #[test]
    fn test_match_count() {
        let mut state = PermissionsSearchState::new();

        // Initially all rules visible
        let (matches, total) = state.match_count();
        assert_eq!(matches, total);

        // Filter to subset
        state.enter_search_mode();
        state.handle_char_input('b');
        state.handle_char_input('a');
        state.handle_char_input('s');

        let (matches, total) = state.match_count();
        assert!(matches <= total);
        assert!(matches > 0); // "bas" should match "Bash"
    }

    #[test]
    fn test_filtering_updates_selection_bounds() {
        let mut state = PermissionsSearchState::new();
        state.enter_search_mode();

        // Navigate to somewhere in the middle
        for _ in 0..5 {
            state.select_next();
        }
        let index_before = state.selected_index();
        assert!(index_before >= 5);

        // Filter to single result
        state.handle_char_input('b');
        state.handle_char_input('a');
        state.handle_char_input('s');
        state.handle_char_input('h');

        // Selection should be clamped to valid range (0 for "Bash" only)
        assert_eq!(state.selected_index(), 0);
        assert!(state.selected_index() < state.filtered_rules().len());
    }

    #[test]
    fn test_default() {
        let state = PermissionsSearchState::default();
        assert!(!state.is_searching());
        assert!(!state.all_rules().is_empty());
    }
}
