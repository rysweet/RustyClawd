//! Extended Thinking State Management for TUI
//!
//! Thread-safe state tracking for Claude's extended thinking phases.
//!
//! ## Architecture (Brick Design)
//!
//! This module is a self-contained "brick" that:
//! - Tracks when Claude is in extended thinking mode
//! - Provides thread-safe state access for UI rendering
//! - Manages thinking phase transitions
//!
//! ## Public API ("Studs")
//!
//! - `ThinkingState`: Main state struct
//! - `ThinkingPhase`: Phase enum (Idle, Thinking, Responding)

use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Phase of extended thinking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingPhase {
    /// Not thinking (idle or responding)
    Idle,
    /// In extended thinking phase (received ContentBlockStart::Thinking)
    Thinking,
    /// Transitioning to response (received ThinkingDelta)
    Responding,
}

/// Thread-safe thinking state
#[derive(Debug, Clone)]
pub struct ThinkingState {
    /// Inner state protected by RwLock for thread safety
    inner: Arc<RwLock<ThinkingStateInner>>,
}

#[derive(Debug)]
struct ThinkingStateInner {
    /// Current phase
    phase: ThinkingPhase,
    /// When thinking started (for duration tracking)
    started_at: Option<Instant>,
    /// Accumulated thinking content (for display if needed)
    thinking_content: String,
}

impl ThinkingState {
    /// Create new thinking state (starts in Idle phase)
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ThinkingStateInner {
                phase: ThinkingPhase::Idle,
                started_at: None,
                thinking_content: String::new(),
            })),
        }
    }

    /// Start thinking phase
    pub fn start_thinking(&self) {
        if let Ok(mut inner) = self.inner.write() {
            inner.phase = ThinkingPhase::Thinking;
            inner.started_at = Some(Instant::now());
            inner.thinking_content.clear();
        }
    }

    /// Append thinking content (from ThinkingDelta)
    pub fn append_thinking(&self, content: &str) {
        if let Ok(mut inner) = self.inner.write() {
            inner.thinking_content.push_str(content);
            inner.phase = ThinkingPhase::Responding;
        }
    }

    /// Stop thinking (transition to Idle)
    pub fn stop_thinking(&self) {
        if let Ok(mut inner) = self.inner.write() {
            inner.phase = ThinkingPhase::Idle;
            inner.started_at = None;
            inner.thinking_content.clear();
        }
    }

    /// Get current phase
    pub fn phase(&self) -> ThinkingPhase {
        self.inner
            .read()
            .map(|inner| inner.phase)
            .unwrap_or(ThinkingPhase::Idle)
    }

    /// Check if currently thinking
    pub fn is_thinking(&self) -> bool {
        matches!(self.phase(), ThinkingPhase::Thinking | ThinkingPhase::Responding)
    }

    /// Get thinking duration (if in thinking phase)
    pub fn thinking_duration(&self) -> Option<std::time::Duration> {
        self.inner
            .read()
            .ok()
            .and_then(|inner| inner.started_at.map(|start| start.elapsed()))
    }

    /// Get accumulated thinking content
    pub fn thinking_content(&self) -> String {
        self.inner
            .read()
            .map(|inner| inner.thinking_content.clone())
            .unwrap_or_default()
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
        let state = ThinkingState::new();

        // Start in Idle
        assert_eq!(state.phase(), ThinkingPhase::Idle);
        assert!(!state.is_thinking());

        // Start thinking
        state.start_thinking();
        assert_eq!(state.phase(), ThinkingPhase::Thinking);
        assert!(state.is_thinking());

        // Append content
        state.append_thinking("Let me think...");
        assert_eq!(state.phase(), ThinkingPhase::Responding);
        assert!(state.is_thinking());
        assert_eq!(state.thinking_content(), "Let me think...");

        // Stop thinking
        state.stop_thinking();
        assert_eq!(state.phase(), ThinkingPhase::Idle);
        assert!(!state.is_thinking());
        assert_eq!(state.thinking_content(), "");
    }

    #[test]
    fn test_thinking_duration() {
        let state = ThinkingState::new();

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
