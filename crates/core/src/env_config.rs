//! Environment variable configuration helpers for Claude Code compatibility.
//!
//! Centralizes checks for CLAUDE_CODE_* environment variables so that all crates
//! can query them through a single module instead of scattering `std::env::var`
//! calls throughout the code.

use std::env;
use std::path::PathBuf;

/// Returns true when `value` is `"1"` or `"true"` (case-insensitive).
pub fn is_env_flag_active(var_name: &str) -> bool {
    match env::var(var_name) {
        Ok(val) => val == "1" || val.eq_ignore_ascii_case("true"),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Feature 1: CLAUDE_CODE_SIMPLE mode
// ---------------------------------------------------------------------------

/// Helpers for CLAUDE_CODE_SIMPLE=1 mode.
pub mod simple_mode {
    use super::*;

    /// Returns true when CLAUDE_CODE_SIMPLE=1 is set.
    pub fn is_active() -> bool {
        is_env_flag_active("CLAUDE_CODE_SIMPLE")
    }

    /// Minimal system prompt used in simple mode.
    pub const MINIMAL_SYSTEM_PROMPT: &str =
        "You are Claude, an AI assistant. Answer questions and help with tasks.";

    /// The set of tools allowed in simple mode. All others are disabled.
    pub const ALLOWED_TOOLS: &[&str] = &["Bash", "Read", "Write", "Edit"];

    /// Returns true if the given tool name is allowed in simple mode.
    pub fn is_tool_allowed(tool_name: &str) -> bool {
        ALLOWED_TOOLS.contains(&tool_name)
    }
}

// ---------------------------------------------------------------------------
// Feature 2: CLAUDE_CODE_TMPDIR
// ---------------------------------------------------------------------------

/// Helpers for CLAUDE_CODE_TMPDIR override.
pub mod tmpdir {
    use super::*;

    /// Returns the temporary directory, preferring CLAUDE_CODE_TMPDIR over the
    /// system default (`std::env::temp_dir()`).
    pub fn get() -> PathBuf {
        match env::var("CLAUDE_CODE_TMPDIR") {
            Ok(val) if !val.is_empty() => PathBuf::from(val),
            _ => env::temp_dir(),
        }
    }
}

// ---------------------------------------------------------------------------
// Feature 3: CLAUDE_CODE_DISABLE_BACKGROUND_TASKS
// ---------------------------------------------------------------------------

/// Returns true when background tasks should be disabled.
pub fn is_background_tasks_disabled() -> bool {
    is_env_flag_active("CLAUDE_CODE_DISABLE_BACKGROUND_TASKS")
}

// ---------------------------------------------------------------------------
// Feature 4: CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS
// ---------------------------------------------------------------------------

/// Returns true when git instructions should be excluded from the system prompt.
pub fn is_git_instructions_disabled() -> bool {
    is_env_flag_active("CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS")
}

// ---------------------------------------------------------------------------
// Feature 5: Account info env vars
// ---------------------------------------------------------------------------

/// Account information read from environment variables.
/// SDK callers can set these to pass identity metadata into sessions.
pub mod account_info {
    use super::*;

    /// Account context loaded from environment variables.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct AccountInfo {
        pub account_uuid: Option<String>,
        pub user_email: Option<String>,
        pub organization_uuid: Option<String>,
    }

    impl AccountInfo {
        /// Load account info from environment variables.
        pub fn from_env() -> Self {
            Self {
                account_uuid: non_empty_var("CLAUDE_CODE_ACCOUNT_UUID"),
                user_email: non_empty_var("CLAUDE_CODE_USER_EMAIL"),
                organization_uuid: non_empty_var("CLAUDE_CODE_ORGANIZATION_UUID"),
            }
        }

        /// Returns true if no account info was provided.
        pub fn is_empty(&self) -> bool {
            self.account_uuid.is_none()
                && self.user_email.is_none()
                && self.organization_uuid.is_none()
        }
    }

    /// Read an env var, returning None for missing or empty values.
    fn non_empty_var(name: &str) -> Option<String> {
        match env::var(name) {
            Ok(val) if !val.is_empty() => Some(val),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Prevent parallel env-var mutation across tests.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(key: &str, value: &str, f: F) {
        let _lock = ENV_MUTEX.lock().unwrap();
        let prev = env::var(key).ok();
        env::set_var(key, value);
        f();
        match prev {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
    }

    fn without_env<F: FnOnce()>(key: &str, f: F) {
        let _lock = ENV_MUTEX.lock().unwrap();
        let prev = env::var(key).ok();
        env::remove_var(key);
        f();
        if let Some(v) = prev {
            env::set_var(key, v);
        }
    }

    // --- is_env_flag_active ---

    #[test]
    fn test_flag_active_1() {
        with_env("TEST_FLAG_ACTIVE", "1", || {
            assert!(is_env_flag_active("TEST_FLAG_ACTIVE"));
        });
    }

    #[test]
    fn test_flag_active_true() {
        with_env("TEST_FLAG_ACTIVE", "true", || {
            assert!(is_env_flag_active("TEST_FLAG_ACTIVE"));
        });
    }

    #[test]
    fn test_flag_active_true_uppercase() {
        with_env("TEST_FLAG_ACTIVE", "TRUE", || {
            assert!(is_env_flag_active("TEST_FLAG_ACTIVE"));
        });
    }

    #[test]
    fn test_flag_inactive_0() {
        with_env("TEST_FLAG_ACTIVE", "0", || {
            assert!(!is_env_flag_active("TEST_FLAG_ACTIVE"));
        });
    }

    #[test]
    fn test_flag_inactive_unset() {
        without_env("TEST_FLAG_ACTIVE", || {
            assert!(!is_env_flag_active("TEST_FLAG_ACTIVE"));
        });
    }

    // --- simple_mode ---

    #[test]
    fn test_simple_mode_active() {
        with_env("CLAUDE_CODE_SIMPLE", "1", || {
            assert!(simple_mode::is_active());
        });
    }

    #[test]
    fn test_simple_mode_inactive() {
        without_env("CLAUDE_CODE_SIMPLE", || {
            assert!(!simple_mode::is_active());
        });
    }

    #[test]
    fn test_simple_mode_tool_filter() {
        assert!(simple_mode::is_tool_allowed("Bash"));
        assert!(simple_mode::is_tool_allowed("Read"));
        assert!(simple_mode::is_tool_allowed("Write"));
        assert!(simple_mode::is_tool_allowed("Edit"));
        assert!(!simple_mode::is_tool_allowed("Grep"));
        assert!(!simple_mode::is_tool_allowed("WebSearch"));
        assert!(!simple_mode::is_tool_allowed("Task"));
    }

    // --- tmpdir ---

    #[test]
    fn test_tmpdir_default() {
        without_env("CLAUDE_CODE_TMPDIR", || {
            assert_eq!(tmpdir::get(), env::temp_dir());
        });
    }

    #[test]
    fn test_tmpdir_override() {
        with_env("CLAUDE_CODE_TMPDIR", "/custom/tmp", || {
            assert_eq!(tmpdir::get(), PathBuf::from("/custom/tmp"));
        });
    }

    #[test]
    fn test_tmpdir_empty_falls_back() {
        with_env("CLAUDE_CODE_TMPDIR", "", || {
            assert_eq!(tmpdir::get(), env::temp_dir());
        });
    }

    // --- background tasks ---

    #[test]
    fn test_background_disabled() {
        with_env("CLAUDE_CODE_DISABLE_BACKGROUND_TASKS", "1", || {
            assert!(is_background_tasks_disabled());
        });
    }

    #[test]
    fn test_background_enabled_by_default() {
        without_env("CLAUDE_CODE_DISABLE_BACKGROUND_TASKS", || {
            assert!(!is_background_tasks_disabled());
        });
    }

    // --- git instructions ---

    #[test]
    fn test_git_instructions_disabled() {
        with_env("CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS", "1", || {
            assert!(is_git_instructions_disabled());
        });
    }

    #[test]
    fn test_git_instructions_enabled_by_default() {
        without_env("CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS", || {
            assert!(!is_git_instructions_disabled());
        });
    }

    // --- account info ---

    #[test]
    fn test_account_info_from_env() {
        let _lock = ENV_MUTEX.lock().unwrap();
        env::set_var("CLAUDE_CODE_ACCOUNT_UUID", "acct-123");
        env::set_var("CLAUDE_CODE_USER_EMAIL", "user@example.com");
        env::set_var("CLAUDE_CODE_ORGANIZATION_UUID", "org-456");

        let info = account_info::AccountInfo::from_env();
        assert_eq!(info.account_uuid, Some("acct-123".to_string()));
        assert_eq!(info.user_email, Some("user@example.com".to_string()));
        assert_eq!(info.organization_uuid, Some("org-456".to_string()));
        assert!(!info.is_empty());

        env::remove_var("CLAUDE_CODE_ACCOUNT_UUID");
        env::remove_var("CLAUDE_CODE_USER_EMAIL");
        env::remove_var("CLAUDE_CODE_ORGANIZATION_UUID");
    }

    #[test]
    fn test_account_info_empty() {
        let _lock = ENV_MUTEX.lock().unwrap();
        env::remove_var("CLAUDE_CODE_ACCOUNT_UUID");
        env::remove_var("CLAUDE_CODE_USER_EMAIL");
        env::remove_var("CLAUDE_CODE_ORGANIZATION_UUID");

        let info = account_info::AccountInfo::from_env();
        assert!(info.is_empty());
    }

    #[test]
    fn test_account_info_partial() {
        let _lock = ENV_MUTEX.lock().unwrap();
        env::remove_var("CLAUDE_CODE_ACCOUNT_UUID");
        env::set_var("CLAUDE_CODE_USER_EMAIL", "only@email.com");
        env::remove_var("CLAUDE_CODE_ORGANIZATION_UUID");

        let info = account_info::AccountInfo::from_env();
        assert_eq!(info.user_email, Some("only@email.com".to_string()));
        assert_eq!(info.account_uuid, None);
        assert!(!info.is_empty());

        env::remove_var("CLAUDE_CODE_USER_EMAIL");
    }
}
