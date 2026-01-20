//! Permission Rules Data Provider
//!
//! Provides data about tool permissions across different permission modes.
//! This module generates the complete list of permission rules and supports
//! filtering for search functionality.

use crate::permission_mode::PermissionMode;

/// Represents a single permission rule for a tool
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRule {
    /// Tool name (e.g., "Bash", "Read", "Write")
    pub tool_name: String,
    /// Whether tool is allowed in Ask mode
    pub allow_in_ask: bool,
    /// Whether tool is allowed in Auto-Accept mode
    pub allow_in_auto_accept: bool,
    /// Whether tool is allowed in Plan mode
    pub allow_in_plan: bool,
}

impl PermissionRule {
    /// Create a new permission rule
    pub fn new(tool_name: impl Into<String>) -> Self {
        let tool_name = tool_name.into();
        let ask_mode = PermissionMode::Ask;
        let auto_accept_mode = PermissionMode::AutoAccept;
        let plan_mode = PermissionMode::Plan;

        Self {
            tool_name: tool_name.clone(),
            allow_in_ask: ask_mode.allows_tool(&tool_name),
            allow_in_auto_accept: auto_accept_mode.allows_tool(&tool_name),
            allow_in_plan: plan_mode.allows_tool(&tool_name),
        }
    }

    /// Check if tool is blocked in any mode
    pub fn is_blocked_in_any_mode(&self) -> bool {
        !self.allow_in_ask || !self.allow_in_auto_accept || !self.allow_in_plan
    }

    /// Get list of modes where tool is blocked
    pub fn blocked_modes(&self) -> Vec<&'static str> {
        let mut modes = Vec::new();
        if !self.allow_in_ask {
            modes.push("Ask");
        }
        if !self.allow_in_auto_accept {
            modes.push("Auto-Accept");
        }
        if !self.allow_in_plan {
            modes.push("Plan");
        }
        modes
    }
}

/// Known Claude Code tools
/// Based on official documentation and common tools
const KNOWN_TOOLS: &[&str] = &[
    // File operations
    "Read",
    "Write",
    "Edit",
    "Glob",
    "Grep",
    // Execution
    "Bash",
    "BashOutput",
    "KillShell",
    // Language Server Protocol
    "LSP",
    // Web operations
    "WebFetch",
    "WebSearch",
    // Notebook operations
    "NotebookEdit",
    // Task management
    "Task",
    "TodoWrite",
    // Skill system
    "Skill",
];

/// Get all permission rules from the system
///
/// Returns a complete list of permission rules for all known tools.
/// Each rule indicates which permission modes allow the tool.
///
/// # Returns
///
/// Vec of PermissionRule entries for all known tools
///
/// # Example
///
/// ```
/// use rustyclawd::commands::permission_rules::get_all_rules;
///
/// let rules = get_all_rules();
/// assert!(!rules.is_empty());
/// ```
pub fn get_all_rules() -> Vec<PermissionRule> {
    KNOWN_TOOLS
        .iter()
        .map(|&tool| PermissionRule::new(tool))
        .collect()
}

/// Filter rules by search term (case-insensitive partial match)
///
/// Searches tool names for the given search term using case-insensitive
/// substring matching.
///
/// # Arguments
///
/// * `rules` - Slice of permission rules to filter
/// * `search_term` - Search term to match against tool names
///
/// # Returns
///
/// Vec of matching PermissionRule entries
///
/// # Example
///
/// ```
/// use rustyclawd::commands::permission_rules::{get_all_rules, filter_rules};
///
/// let all_rules = get_all_rules();
/// let bash_rules = filter_rules(&all_rules, "ash");
/// assert!(bash_rules.iter().any(|r| r.tool_name.contains("Bash")));
/// ```
pub fn filter_rules(rules: &[PermissionRule], search_term: &str) -> Vec<PermissionRule> {
    // Empty search term returns all rules
    if search_term.is_empty() {
        return rules.to_vec();
    }

    let search_lower = search_term.to_lowercase();

    rules
        .iter()
        .filter(|rule| rule.tool_name.to_lowercase().contains(&search_lower))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_rules_returns_known_tools() {
        let rules = get_all_rules();

        // Verify we have rules
        assert!(!rules.is_empty());

        // Verify specific known tools are included
        assert!(rules.iter().any(|r| r.tool_name == "Bash"));
        assert!(rules.iter().any(|r| r.tool_name == "Read"));
        assert!(rules.iter().any(|r| r.tool_name == "Write"));
        assert!(rules.iter().any(|r| r.tool_name == "Glob"));
        assert!(rules.iter().any(|r| r.tool_name == "LSP"));
    }

    #[test]
    fn test_get_all_rules_includes_blocked_tools() {
        let rules = get_all_rules();

        // Plan mode blocks certain tools
        let plan_mode = PermissionMode::Plan;
        let blocked_in_plan: Vec<_> = rules
            .iter()
            .filter(|r| !plan_mode.allows_tool(&r.tool_name))
            .collect();

        assert!(!blocked_in_plan.is_empty());
        assert!(blocked_in_plan.iter().any(|r| r.tool_name == "Bash"));
        assert!(blocked_in_plan.iter().any(|r| r.tool_name == "Write"));
        assert!(blocked_in_plan.iter().any(|r| r.tool_name == "Edit"));
    }

    #[test]
    fn test_filter_rules_empty_query() {
        let rules = get_all_rules();
        let filtered = filter_rules(&rules, "");

        // Empty search returns all rules
        assert_eq!(filtered.len(), rules.len());
    }

    #[test]
    fn test_filter_rules_partial_match() {
        let rules = get_all_rules();

        // "ash" should match "Bash"
        let filtered = filter_rules(&rules, "ash");
        assert!(filtered.iter().any(|r| r.tool_name == "Bash"));

        // "read" should match "Read"
        let filtered = filter_rules(&rules, "read");
        assert!(filtered.iter().any(|r| r.tool_name == "Read"));
    }

    #[test]
    fn test_filter_rules_case_insensitive() {
        let rules = get_all_rules();

        // Case variations should all match
        let filtered_lower = filter_rules(&rules, "bash");
        let filtered_upper = filter_rules(&rules, "BASH");
        let filtered_mixed = filter_rules(&rules, "BaSh");

        assert_eq!(filtered_lower.len(), filtered_upper.len());
        assert_eq!(filtered_lower.len(), filtered_mixed.len());
        assert!(filtered_lower.iter().any(|r| r.tool_name == "Bash"));
    }

    #[test]
    fn test_filter_rules_no_match() {
        let rules = get_all_rules();
        let filtered = filter_rules(&rules, "xyz_no_match");

        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_rules_substring_match() {
        let rules = get_all_rules();
        let filtered = filter_rules(&rules, "Bash");

        // "Bash" matches both "Bash" and "BashOutput" via substring
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|r| r.tool_name == "Bash"));
        assert!(filtered.iter().any(|r| r.tool_name == "BashOutput"));
    }

    #[test]
    fn test_permission_rule_new() {
        let rule = PermissionRule::new("Read");

        assert_eq!(rule.tool_name, "Read");
        // Read is allowed in all modes
        assert!(rule.allow_in_ask);
        assert!(rule.allow_in_auto_accept);
        assert!(rule.allow_in_plan);
    }

    #[test]
    fn test_permission_rule_blocked_tool() {
        let rule = PermissionRule::new("Bash");

        assert_eq!(rule.tool_name, "Bash");
        // Bash is allowed in Ask and Auto-Accept, but blocked in Plan
        assert!(rule.allow_in_ask);
        assert!(rule.allow_in_auto_accept);
        assert!(!rule.allow_in_plan);
    }

    #[test]
    fn test_is_blocked_in_any_mode() {
        let allowed_rule = PermissionRule::new("Read");
        assert!(!allowed_rule.is_blocked_in_any_mode());

        let blocked_rule = PermissionRule::new("Bash");
        assert!(blocked_rule.is_blocked_in_any_mode());
    }

    #[test]
    fn test_blocked_modes() {
        let allowed_rule = PermissionRule::new("Read");
        assert!(allowed_rule.blocked_modes().is_empty());

        let blocked_rule = PermissionRule::new("Bash");
        let modes = blocked_rule.blocked_modes();
        assert_eq!(modes.len(), 1);
        assert!(modes.contains(&"Plan"));
    }
}
