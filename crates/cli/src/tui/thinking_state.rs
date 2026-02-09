//! Extended Thinking State Management for TUI
//!
//! State tracking for Claude's extended thinking phases.
//!
//! ## Architecture (Brick Design)
//!
//! This module is a self-contained "brick" that:
//! - Tracks when Claude is in extended thinking mode
//! - Manages thinking phase transitions
//!
//! ## Public API ("Studs")
//!
//! - `ThinkingState`: Main state struct
//! - `ThinkingPhase`: Phase enum (Idle, Thinking, ReceivingThoughts)

use std::time::Instant;

/// Phase of extended thinking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingPhase {
    /// Not thinking (idle or responding)
    Idle,
    /// In extended thinking phase (received ContentBlockStart::Thinking)
    Thinking,
    /// Receiving thinking content (received ThinkingDelta)
    ReceivingThoughts,
}

/// Thinking state (single-threaded, owned by App on the main event loop)
#[derive(Debug, Clone)]
pub struct ThinkingState {
    /// Current phase
    phase: ThinkingPhase,
    /// When thinking started (for duration tracking)
    started_at: Option<Instant>,
}

impl ThinkingState {
    /// Create new thinking state (starts in Idle phase)
    pub fn new() -> Self {
        Self {
            phase: ThinkingPhase::Idle,
            started_at: None,
        }
    }

    /// Start thinking phase
    pub fn start_thinking(&mut self) {
        self.phase = ThinkingPhase::Thinking;
        self.started_at = Some(Instant::now());
    }

    /// Mark that thinking content is being received (from ThinkingDelta)
    pub fn append_thinking(&mut self) {
        self.phase = ThinkingPhase::ReceivingThoughts;
    }

    /// Stop thinking (transition to Idle)
    pub fn stop_thinking(&mut self) {
        self.phase = ThinkingPhase::Idle;
        self.started_at = None;
    }

    /// Get current phase
    pub fn phase(&self) -> ThinkingPhase {
        self.phase
    }

    /// Check if currently thinking
    pub fn is_thinking(&self) -> bool {
        matches!(
            self.phase,
            ThinkingPhase::Thinking | ThinkingPhase::ReceivingThoughts
        )
    }

    /// Get thinking duration (if in thinking phase)
    pub fn thinking_duration(&self) -> Option<std::time::Duration> {
        self.started_at.map(|start| start.elapsed())
    }
}

impl Default for ThinkingState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thinking_lifecycle() {
        let mut state = ThinkingState::new();

        // Start in Idle
        assert_eq!(state.phase(), ThinkingPhase::Idle);
        assert!(!state.is_thinking());

        // Start thinking
        state.start_thinking();
        assert_eq!(state.phase(), ThinkingPhase::Thinking);
        assert!(state.is_thinking());

        // Append content transitions to ReceivingThoughts
        state.append_thinking();
        assert_eq!(state.phase(), ThinkingPhase::ReceivingThoughts);
        assert!(state.is_thinking());

        // Stop thinking
        state.stop_thinking();
        assert_eq!(state.phase(), ThinkingPhase::Idle);
        assert!(!state.is_thinking());
    }

    #[test]
    fn test_thinking_duration() {
        let mut state = ThinkingState::new();

        // No duration when idle
        assert!(state.thinking_duration().is_none());

        // Duration available when thinking
        state.start_thinking();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = state.thinking_duration();
        assert!(duration.is_some());
        assert!(duration.unwrap().as_millis() >= 10);

        // No duration after stop
        state.stop_thinking();
        assert!(state.thinking_duration().is_none());
    }
}
