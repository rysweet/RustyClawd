//! Streaming response state management for TUI
//!
//! Encapsulates all state for an active streaming response from Claude.
//! StreamingState owns its own bookkeeping (content accumulation, token counting,
//! thinking phases); App handles messages[] integration as a thin orchestrator.
//!
//! ## Architecture (Brick Design)
//!
//! This module is a self-contained "brick" that:
//! - Tracks accumulated streaming content
//! - Manages token counts during streaming
//! - Tracks thinking/extended-thinking phases
//! - Enforces content size limits (10MB)
//!
//! ## Public API ("Studs")
//!
//! - `StreamingState`: Main state struct
//! - `StreamingState::new(message_index)`: Constructor
//! - `StreamingState::append_content(&mut self, content) -> bool`: Append content, returns truncated flag
//! - `StreamingState::accumulated(&self) -> &str`: Get accumulated content
//! - `StreamingState::take_accumulated(self) -> String`: Consume and return accumulated content
//! - `StreamingState::update_tokens(&mut self, input, output)`: Update token counts
//! - `StreamingState::token_count(&self) -> TokenCount`: Get current token count
//! - Thinking state accessors and mutators

use crate::tui::thinking_state::ThinkingState;
use crate::tui::token_counter::TokenCount;

/// Maximum accumulated content size (10MB) to prevent OOM.
/// Well above typical Claude responses but prevents pathological cases.
const MAX_ACCUMULATED_SIZE: usize = 10 * 1024 * 1024;

/// State for an active streaming response.
///
/// Owns all bookkeeping for a single streaming session. App creates one when
/// streaming starts and consumes it (via `take_accumulated`) when streaming ends.
pub(crate) struct StreamingState {
    /// Message index being streamed to (in App.messages)
    message_index: usize,

    /// Accumulated content so far
    accumulated: String,

    /// Token count (live updates during streaming)
    token_count: TokenCount,

    /// Thinking indicator (true when waiting for first token)
    thinking: bool,

    /// Extended thinking state (tracks thinking phases)
    thinking_state: ThinkingState,

    /// Whether the "input blocked" debug message has been shown this thinking phase
    shown_blocked_input_message: bool,
}

impl StreamingState {
    /// Create a new streaming state for the given message index.
    /// Starts in thinking mode (waiting for first token).
    pub(crate) fn new(message_index: usize) -> Self {
        Self {
            message_index,
            accumulated: String::new(),
            token_count: TokenCount::default(),
            thinking: true,
            thinking_state: ThinkingState::new(),
            shown_blocked_input_message: false,
        }
    }

    // === Content management ===

    /// Append content to the accumulated buffer.
    /// Returns `true` if truncation occurred (content size limit reached).
    pub(crate) fn append_content(&mut self, content: &str) -> bool {
        let new_size = self.accumulated.len() + content.len();

        if new_size > MAX_ACCUMULATED_SIZE {
            let available = MAX_ACCUMULATED_SIZE.saturating_sub(self.accumulated.len());
            if available > 0 {
                self.accumulated
                    .push_str(&content[..available.min(content.len())]);
            }
            true // truncated
        } else {
            self.accumulated.push_str(content);
            false // not truncated
        }
    }

    /// Get a reference to the accumulated content.
    pub(crate) fn accumulated(&self) -> &str {
        &self.accumulated
    }

    /// Consume this StreamingState and return the accumulated content.
    pub(crate) fn take_accumulated(self) -> String {
        self.accumulated
    }

    /// Get the message index this stream is writing to.
    pub(crate) fn message_index(&self) -> usize {
        self.message_index
    }

    // === Token management ===

    /// Update token counts (additive).
    /// If output tokens arrive, clears the initial thinking flag.
    pub(crate) fn update_tokens(&mut self, input: u32, output: u32) {
        self.token_count.add(input, output);
        if output > 0 {
            self.thinking = false;
        }
    }

    /// Get current token count.
    pub(crate) fn token_count(&self) -> TokenCount {
        self.token_count
    }

    // === Basic thinking state (waiting for first token) ===

    /// Whether we are still waiting for the first token.
    pub(crate) fn is_thinking(&self) -> bool {
        self.thinking
    }

    // === Extended thinking state ===

    /// Whether we are in an extended thinking phase.
    pub(crate) fn is_extended_thinking(&self) -> bool {
        self.thinking_state.is_thinking()
    }

    /// Start extended thinking phase.
    pub(crate) fn start_extended_thinking(&mut self) {
        self.thinking_state.start_thinking();
        self.shown_blocked_input_message = false;
    }

    /// Append thinking content (transition from Thinking -> ReceivingThoughts).
    pub(crate) fn append_thinking(&mut self) {
        self.thinking_state.append_thinking();
    }

    /// Stop extended thinking phase.
    pub(crate) fn stop_extended_thinking(&mut self) {
        self.thinking_state.stop_thinking();
        self.shown_blocked_input_message = false;
    }

    /// Get thinking duration (if in extended thinking phase).
    pub(crate) fn thinking_duration(&self) -> Option<std::time::Duration> {
        self.thinking_state.thinking_duration()
    }

    // === Blocked input message flag ===

    /// Whether the "input blocked" debug message has been shown this thinking phase.
    pub(crate) fn has_shown_blocked_input_message(&self) -> bool {
        self.shown_blocked_input_message
    }

    /// Set the "input blocked" message shown flag.
    pub(crate) fn set_shown_blocked_input_message(&mut self, shown: bool) {
        self.shown_blocked_input_message = shown;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_streaming_state() {
        let state = StreamingState::new(5);
        assert_eq!(state.message_index(), 5);
        assert_eq!(state.accumulated(), "");
        assert!(state.is_thinking());
        assert!(!state.is_extended_thinking());
        assert_eq!(state.token_count().total(), 0);
        assert!(!state.has_shown_blocked_input_message());
    }

    #[test]
    fn test_append_content_basic() {
        let mut state = StreamingState::new(0);
        let truncated = state.append_content("Hello");
        assert!(!truncated);
        assert_eq!(state.accumulated(), "Hello");

        let truncated = state.append_content(" world");
        assert!(!truncated);
        assert_eq!(state.accumulated(), "Hello world");
    }

    #[test]
    fn test_append_content_truncation() {
        let mut state = StreamingState::new(0);
        // Fill up to near the limit
        let big_chunk = "x".repeat(MAX_ACCUMULATED_SIZE - 5);
        let truncated = state.append_content(&big_chunk);
        assert!(!truncated);

        // This should trigger truncation
        let truncated = state.append_content("this is way too long");
        assert!(truncated);
        assert_eq!(state.accumulated().len(), MAX_ACCUMULATED_SIZE);
    }

    #[test]
    fn test_take_accumulated() {
        let mut state = StreamingState::new(0);
        state.append_content("final content");
        let content = state.take_accumulated();
        assert_eq!(content, "final content");
    }

    #[test]
    fn test_update_tokens_clears_thinking() {
        let mut state = StreamingState::new(0);
        assert!(state.is_thinking());

        // Input-only tokens don't clear thinking
        state.update_tokens(10, 0);
        assert!(state.is_thinking());

        // Output tokens clear thinking
        state.update_tokens(0, 5);
        assert!(!state.is_thinking());
        assert_eq!(state.token_count().input, 10);
        assert_eq!(state.token_count().output, 5);
    }

    #[test]
    fn test_extended_thinking_lifecycle() {
        let mut state = StreamingState::new(0);
        assert!(!state.is_extended_thinking());

        state.start_extended_thinking();
        assert!(state.is_extended_thinking());
        assert!(state.thinking_duration().is_some());

        state.append_thinking();
        assert!(state.is_extended_thinking());

        state.stop_extended_thinking();
        assert!(!state.is_extended_thinking());
    }

    #[test]
    fn test_blocked_input_message_flag() {
        let mut state = StreamingState::new(0);
        assert!(!state.has_shown_blocked_input_message());

        state.set_shown_blocked_input_message(true);
        assert!(state.has_shown_blocked_input_message());

        // start_extended_thinking resets the flag
        state.start_extended_thinking();
        assert!(!state.has_shown_blocked_input_message());
    }
}
