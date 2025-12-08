//! Permission Mode for tool execution control
//!
//! Provides three permission levels that control how tools are executed:
//! - Ask: Prompts user for permission before each tool execution
//! - AutoAccept: Automatically approves all tool executions
//! - Plan: Planning only mode that blocks modification tools
//!
//! Shift+Tab cycles through modes: Ask -> AutoAccept -> Plan -> Ask

/// Tools that are blocked in Plan mode (modification tools)
const PLAN_MODE_BLOCKED_TOOLS: &[&str] = &["Bash", "BashOutput", "KillShell", "Write", "Edit"];

/// Permission mode controlling tool execution behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    /// Default mode - prompts for permission before tool execution
    #[default]
    Ask,
    /// Auto-approves all tool executions without prompting
    AutoAccept,
    /// Planning only - disallows tool execution (read-only)
    Plan,
}

impl PermissionMode {
    /// Cycle to the next permission mode
    ///
    /// Order: Ask -> AutoAccept -> Plan -> Ask
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rusty_cli::permission_mode::PermissionMode;
    ///
    /// let mode = PermissionMode::Ask;
    /// assert_eq!(mode.cycle(), PermissionMode::AutoAccept);
    /// ```ignore
    pub fn cycle(&self) -> Self {
        match self {
            PermissionMode::Ask => PermissionMode::AutoAccept,
            PermissionMode::AutoAccept => PermissionMode::Plan,
            PermissionMode::Plan => PermissionMode::Ask,
        }
    }

    /// Get the display name for the permission mode
    ///
    /// # Returns
    ///
    /// - "Ask" for Ask mode
    /// - "Auto-Accept" for AutoAccept mode
    /// - "Plan" for Plan mode
    pub fn display_name(&self) -> &'static str {
        match self {
            PermissionMode::Ask => "Ask",
            PermissionMode::AutoAccept => "Auto-Accept",
            PermissionMode::Plan => "Plan",
        }
    }

    /// Get the status bar indicator for the permission mode
    ///
    /// Returns a short string with icon prefix for TUI status bar display.
    ///
    /// # Returns
    ///
    /// - "? Ask" for Ask mode
    /// - "* Auto" for AutoAccept mode
    /// - "! Plan" for Plan mode
    pub fn status_indicator(&self) -> &'static str {
        match self {
            PermissionMode::Ask => "? Ask",
            PermissionMode::AutoAccept => "* Auto",
            PermissionMode::Plan => "! Plan",
        }
    }

    /// Check if a tool is allowed to execute in the current mode
    ///
    /// In Ask and AutoAccept modes, all tools are allowed.
    /// In Plan mode, modification tools are blocked.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - The name of the tool to check
    ///
    /// # Returns
    ///
    /// `true` if the tool is allowed, `false` if blocked
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rusty_cli::permission_mode::PermissionMode;
    ///
    /// let mode = PermissionMode::Plan;
    /// assert!(!mode.allows_tool("Bash")); // Blocked
    /// assert!(mode.allows_tool("Read"));  // Allowed
    /// ```ignore
    pub fn allows_tool(&self, tool_name: &str) -> bool {
        match self {
            PermissionMode::Ask | PermissionMode::AutoAccept => true,
            PermissionMode::Plan => !PLAN_MODE_BLOCKED_TOOLS.contains(&tool_name),
        }
    }

    /// Get the list of tools blocked in the current mode
    ///
    /// # Returns
    ///
    /// Empty slice for Ask and AutoAccept modes.
    /// Slice of blocked tool names for Plan mode.
    pub fn blocked_tools(&self) -> &'static [&'static str] {
        match self {
            PermissionMode::Ask | PermissionMode::AutoAccept => &[],
            PermissionMode::Plan => PLAN_MODE_BLOCKED_TOOLS,
        }
    }

    /// Generate an error message for a blocked tool
    ///
    /// # Arguments
    ///
    /// * `tool_name` - The name of the blocked tool
    ///
    /// # Returns
    ///
    /// A user-friendly error message explaining why the tool is blocked
    /// and how to resolve it.
    pub fn blocked_tool_error(&self, tool_name: &str) -> String {
        format!(
            "Tool '{}' blocked in Plan mode. Switch to Ask or Auto-Accept mode to execute tools.",
            tool_name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_ask() {
        assert_eq!(PermissionMode::default(), PermissionMode::Ask);
    }

    #[test]
    fn test_cycle_full_rotation() {
        let mode = PermissionMode::Ask;
        let mode = mode.cycle();
        assert_eq!(mode, PermissionMode::AutoAccept);
        let mode = mode.cycle();
        assert_eq!(mode, PermissionMode::Plan);
        let mode = mode.cycle();
        assert_eq!(mode, PermissionMode::Ask);
    }

    #[test]
    fn test_display_names() {
        assert_eq!(PermissionMode::Ask.display_name(), "Ask");
        assert_eq!(PermissionMode::AutoAccept.display_name(), "Auto-Accept");
        assert_eq!(PermissionMode::Plan.display_name(), "Plan");
    }

    #[test]
    fn test_status_indicators() {
        assert_eq!(PermissionMode::Ask.status_indicator(), "? Ask");
        assert_eq!(PermissionMode::AutoAccept.status_indicator(), "* Auto");
        assert_eq!(PermissionMode::Plan.status_indicator(), "! Plan");
    }

    #[test]
    fn test_plan_mode_blocks_modification_tools() {
        let mode = PermissionMode::Plan;
        assert!(!mode.allows_tool("Bash"));
        assert!(!mode.allows_tool("Write"));
        assert!(!mode.allows_tool("Edit"));
        assert!(mode.allows_tool("Read"));
        assert!(mode.allows_tool("Glob"));
    }

    #[test]
    fn test_blocked_tools_list() {
        assert!(PermissionMode::Ask.blocked_tools().is_empty());
        assert!(PermissionMode::AutoAccept.blocked_tools().is_empty());
        assert_eq!(PermissionMode::Plan.blocked_tools().len(), 5);
    }

    #[test]
    fn test_blocked_tool_error_message() {
        let mode = PermissionMode::Plan;
        let error = mode.blocked_tool_error("Bash");
        assert!(error.contains("Bash"));
        assert!(error.contains("Plan mode"));
        assert!(error.contains("Ask") || error.contains("Auto-Accept"));
    }
}
