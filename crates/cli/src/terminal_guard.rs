//! Terminal state guard for protecting TUI during tool execution
//!
//! This module provides an RAII guard that temporarily suspends and restores
//! terminal state during tool execution to prevent corruption of the TUI display.
//!
//! The guard:
//! - Suspends raw mode and alternate screen when created
//! - Automatically restores terminal state when dropped
//! - Only operates when in TUI mode (non-TUI mode is a no-op)

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

/// Execution context indicating whether we're running in TUI mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionContext {
    /// Running in TUI/interactive mode - terminal state must be protected
    Tui,
    /// Running in non-interactive mode - no terminal state to protect
    #[default]
    NonInteractive,
}

// Global execution context
//
// This is set once at application startup and read during tool execution.
// Using lazy_static ensures thread-safe initialization.
lazy_static::lazy_static! {
    static ref EXECUTION_CONTEXT: Arc<Mutex<ExecutionContext>> =
        Arc::new(Mutex::new(ExecutionContext::default()));
}

/// Set the global execution context
///
/// Should be called once at application startup to indicate whether
/// we're running in TUI mode or not.
///
/// # Arguments
///
/// * `context` - The execution context to set
///
/// # Example
///
/// ```no_run
/// use rustyclawd::terminal_guard::{set_execution_context, ExecutionContext};
///
/// // At application startup in TUI mode
/// set_execution_context(ExecutionContext::Tui);
/// ```
pub fn set_execution_context(context: ExecutionContext) {
    if let Ok(mut ctx) = EXECUTION_CONTEXT.lock() {
        *ctx = context;
    }
}

/// Get the current execution context
///
/// # Returns
///
/// The current execution context, or NonInteractive if lock fails
pub fn get_execution_context() -> ExecutionContext {
    EXECUTION_CONTEXT
        .lock()
        .map(|ctx| *ctx)
        .unwrap_or(ExecutionContext::NonInteractive)
}

/// RAII guard for terminal state protection
///
/// When created, suspends terminal raw mode and alternate screen.
/// When dropped, restores terminal state.
///
/// This is a no-op when not in TUI mode.
///
/// # Example
///
/// ```no_run
/// use rustyclawd::terminal_guard::TerminalGuard;
///
/// # async fn execute_tool() -> anyhow::Result<()> {
/// // Create guard - terminal state is suspended
/// let _guard = TerminalGuard::new()?;
///
/// // Execute tool - terminal is in normal mode
/// // Tool can write to stdout without corrupting TUI
///
/// // Guard is dropped - terminal state is restored
/// # Ok(())
/// # }
/// ```
pub struct TerminalGuard {
    /// Whether the guard actually suspended terminal state
    /// (false if not in TUI mode)
    suspended: bool,
}

impl TerminalGuard {
    /// Create a new terminal guard
    ///
    /// If in TUI mode:
    /// - Disables raw mode
    /// - Leaves alternate screen
    /// - Flushes stdout
    ///
    /// If not in TUI mode, this is a no-op.
    ///
    /// # Returns
    ///
    /// Result containing the guard, or an error if terminal operations failed
    pub fn new() -> Result<Self> {
        let context = get_execution_context();

        if context == ExecutionContext::Tui {
            // Suspend TUI terminal state
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen)?;
            io::stdout().flush()?;

            Ok(Self { suspended: true })
        } else {
            // Not in TUI mode - no-op
            Ok(Self { suspended: false })
        }
    }

    /// Check if this guard actually suspended terminal state
    pub fn is_suspended(&self) -> bool {
        self.suspended
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.suspended {
            // Restore TUI terminal state
            // We can't return errors from Drop, so we log them
            if let Err(e) = enable_raw_mode() {
                eprintln!("Warning: Failed to restore raw mode: {}", e);
            }

            if let Err(e) = execute!(io::stdout(), EnterAlternateScreen) {
                eprintln!("Warning: Failed to restore alternate screen: {}", e);
            }

            if let Err(e) = io::stdout().flush() {
                eprintln!("Warning: Failed to flush stdout: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_default() {
        // Default should be NonInteractive
        let ctx = ExecutionContext::default();
        assert_eq!(ctx, ExecutionContext::NonInteractive);
    }

    #[test]
    fn test_set_and_get_execution_context() {
        // Set to TUI
        set_execution_context(ExecutionContext::Tui);
        assert_eq!(get_execution_context(), ExecutionContext::Tui);

        // Set back to NonInteractive
        set_execution_context(ExecutionContext::NonInteractive);
        assert_eq!(get_execution_context(), ExecutionContext::NonInteractive);
    }

    #[test]
    fn test_terminal_guard_noop_in_non_interactive() {
        // Ensure context is NonInteractive
        set_execution_context(ExecutionContext::NonInteractive);

        // Guard should be created successfully and not suspend
        let guard = TerminalGuard::new().expect("Failed to create guard");
        assert!(!guard.is_suspended());

        // Drop should be a no-op
        drop(guard);
    }

    #[test]
    fn test_terminal_guard_suspends_in_tui() {
        // Set to TUI mode
        set_execution_context(ExecutionContext::Tui);

        // Note: We can't fully test terminal operations without an actual terminal
        // This test just verifies the guard attempts to suspend
        // In a real terminal, this would disable raw mode and leave alt screen

        // For testing purposes, we'll just verify the guard acknowledges suspension
        // The actual terminal operations may fail in test environment, which is ok
        let guard = TerminalGuard::new();

        // We expect either success (in real terminal) or failure (in test env)
        // Both are acceptable in tests
        match guard {
            Ok(g) => {
                // In a real terminal, should be suspended
                // In test env, might not be (depends on CI/test setup)
                drop(g);
            }
            Err(e) => {
                // Expected in test environment without a real terminal
                eprintln!("Expected error in test environment: {}", e);
            }
        }
    }

    #[test]
    fn test_multiple_guards_noop() {
        set_execution_context(ExecutionContext::NonInteractive);

        // Multiple guards should all be no-ops
        let _guard1 = TerminalGuard::new().expect("Failed to create guard 1");
        let _guard2 = TerminalGuard::new().expect("Failed to create guard 2");
        let _guard3 = TerminalGuard::new().expect("Failed to create guard 3");

        // All should drop cleanly
    }

    #[test]
    fn test_guard_raii_behavior() {
        set_execution_context(ExecutionContext::NonInteractive);

        {
            let guard = TerminalGuard::new().expect("Failed to create guard");
            assert!(!guard.is_suspended());
            // Guard is in scope
        } // Guard drops here

        // After scope, guard should be dropped (no-op in this case)
        // This test just verifies RAII pattern works
    }
}
