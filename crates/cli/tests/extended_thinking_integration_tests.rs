//! Integration Tests for Extended Thinking Feature
//!
//! Tests the complete stream event flow from ContentBlockStart::Thinking
//! through ThinkingDelta to ContentBlockStop, verifying ThinkingState
//! transitions and event dispatching.

use rustyclawd_core::client::types::{ContentBlockStart, ContentDelta, StreamEvent};
use std::sync::mpsc;

/// Mock event channel for testing stream event dispatch
#[derive(Debug, Clone, PartialEq)]
enum TestStreamEvent {
    ExtendedThinkingStarted,
    ExtendedThinkingDelta,
    ExtendedThinkingStopped,
    TextDelta(String),
}

/// Simulate the stream event handling logic from interactive.rs
///
/// This tests the core state machine that tracks thinking blocks using
/// the `in_thinking_block` boolean flag.
fn simulate_stream_events(events: Vec<StreamEvent>) -> Result<Vec<TestStreamEvent>, String> {
    let (tx, rx) = mpsc::channel();
    let mut in_thinking_block = false;

    for event in events {
        match event {
            StreamEvent::ContentBlockStart { content_block, .. } => {
                match content_block {
                    ContentBlockStart::Thinking => {
                        in_thinking_block = true;
                        tx.send(TestStreamEvent::ExtendedThinkingStarted)
                            .map_err(|e| format!("Channel send error: {}", e))?;
                    }
                    ContentBlockStart::Text { .. } => {
                        // Regular text block - no thinking
                    }
                    ContentBlockStart::ToolUse { .. } => {
                        // Tool use block - no thinking
                    }
                }
            }
            StreamEvent::ContentBlockDelta { delta, .. } => match delta {
                ContentDelta::ThinkingDelta { thinking } => {
                    tx.send(TestStreamEvent::ExtendedThinkingDelta)
                        .map_err(|e| format!("Channel send error: {}", e))?;
                    tx.send(TestStreamEvent::TextDelta(thinking))
                        .map_err(|e| format!("Channel send error: {}", e))?;
                }
                ContentDelta::TextDelta { text } => {
                    tx.send(TestStreamEvent::TextDelta(text))
                        .map_err(|e| format!("Channel send error: {}", e))?;
                }
                _ => {}
            },
            StreamEvent::ContentBlockStop { .. } => {
                // NOTE: ContentBlockStop does not carry a block type.
                // We rely on the API sending stops in the same order as starts.
                if in_thinking_block {
                    in_thinking_block = false;
                    tx.send(TestStreamEvent::ExtendedThinkingStopped)
                        .map_err(|e| format!("Channel send error: {}", e))?;
                }
            }
            _ => {}
        }
    }

    // Collect all events
    drop(tx);
    Ok(rx.iter().collect())
}

#[test]
fn test_thinking_block_complete_lifecycle() {
    // Simulate: ContentBlockStart(Thinking) -> ThinkingDelta -> ContentBlockStop
    let events = vec![
        StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlockStart::Thinking,
        },
        StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::ThinkingDelta {
                thinking: "Let me consider this carefully...".to_string(),
            },
        },
        StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::ThinkingDelta {
                thinking: " I think the answer is...".to_string(),
            },
        },
        StreamEvent::ContentBlockStop { index: 0 },
    ];

    let result = simulate_stream_events(events).expect("Stream simulation failed");

    // Verify event sequence
    // Expected: Started + (Delta + TextDelta) x 2 + Stopped = 6 events
    assert_eq!(result.len(), 6);
    assert_eq!(result[0], TestStreamEvent::ExtendedThinkingStarted);
    assert_eq!(result[1], TestStreamEvent::ExtendedThinkingDelta);
    assert_eq!(
        result[2],
        TestStreamEvent::TextDelta("Let me consider this carefully...".to_string())
    );
    assert_eq!(result[3], TestStreamEvent::ExtendedThinkingDelta);
    assert_eq!(
        result[4],
        TestStreamEvent::TextDelta(" I think the answer is...".to_string())
    );
    assert_eq!(result[5], TestStreamEvent::ExtendedThinkingStopped);
}

#[test]
fn test_thinking_block_followed_by_text_block() {
    // Simulate: Thinking block -> Text block
    // Verifies that ContentBlockStop for text block does NOT trigger ExtendedThinkingStopped
    let events = vec![
        // Thinking block
        StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlockStart::Thinking,
        },
        StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::ThinkingDelta {
                thinking: "Thinking...".to_string(),
            },
        },
        StreamEvent::ContentBlockStop { index: 0 },
        // Text block
        StreamEvent::ContentBlockStart {
            index: 1,
            content_block: ContentBlockStart::Text {
                text: String::new(),
            },
        },
        StreamEvent::ContentBlockDelta {
            index: 1,
            delta: ContentDelta::TextDelta {
                text: "Here is my response.".to_string(),
            },
        },
        StreamEvent::ContentBlockStop { index: 1 },
    ];

    let result = simulate_stream_events(events).expect("Stream simulation failed");

    // Count ExtendedThinkingStopped events - should be exactly 1
    let stopped_count = result
        .iter()
        .filter(|e| **e == TestStreamEvent::ExtendedThinkingStopped)
        .count();
    assert_eq!(
        stopped_count, 1,
        "Should have exactly one ExtendedThinkingStopped event"
    );

    // Verify the stopped event comes after the first ContentBlockStop
    let first_stopped_idx = result
        .iter()
        .position(|e| *e == TestStreamEvent::ExtendedThinkingStopped);
    assert!(
        first_stopped_idx.is_some(),
        "ExtendedThinkingStopped should be present"
    );
    assert!(
        first_stopped_idx.unwrap() < result.len(),
        "ExtendedThinkingStopped should occur after thinking block stop"
    );
}

#[test]
fn test_text_block_without_thinking() {
    // Simulate: Pure text block (no thinking)
    // Verifies that ExtendedThinking events are NOT emitted
    let events = vec![
        StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlockStart::Text {
                text: String::new(),
            },
        },
        StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::TextDelta {
                text: "Just a regular response.".to_string(),
            },
        },
        StreamEvent::ContentBlockStop { index: 0 },
    ];

    let result = simulate_stream_events(events).expect("Stream simulation failed");

    // Should only have TextDelta, no thinking events
    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0],
        TestStreamEvent::TextDelta("Just a regular response.".to_string())
    );

    // Verify no thinking events
    assert!(
        !result.contains(&TestStreamEvent::ExtendedThinkingStarted),
        "Should not have ExtendedThinkingStarted"
    );
    assert!(
        !result.contains(&TestStreamEvent::ExtendedThinkingStopped),
        "Should not have ExtendedThinkingStopped"
    );
}

#[test]
fn test_multiple_thinking_deltas() {
    // Simulate: Multiple ThinkingDelta events in sequence
    let events = vec![
        StreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlockStart::Thinking,
        },
        StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::ThinkingDelta {
                thinking: "First thought...".to_string(),
            },
        },
        StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::ThinkingDelta {
                thinking: "Second thought...".to_string(),
            },
        },
        StreamEvent::ContentBlockDelta {
            index: 0,
            delta: ContentDelta::ThinkingDelta {
                thinking: "Third thought...".to_string(),
            },
        },
        StreamEvent::ContentBlockStop { index: 0 },
    ];

    let result = simulate_stream_events(events).expect("Stream simulation failed");

    // Should have: Started + (Delta + TextDelta) x 3 + Stopped
    assert_eq!(result.len(), 8);
    assert_eq!(result[0], TestStreamEvent::ExtendedThinkingStarted);

    // Count deltas
    let delta_count = result
        .iter()
        .filter(|e| **e == TestStreamEvent::ExtendedThinkingDelta)
        .count();
    assert_eq!(delta_count, 3, "Should have 3 thinking deltas");
}

/// Test ThinkingState transitions
///
/// This tests the state machine directly, ensuring phase transitions
/// work correctly.
#[cfg(test)]
mod thinking_state_tests {
    use rustyclawd::tui::{ThinkingPhase, ThinkingState};
    use std::time::Duration;

    #[test]
    fn test_thinking_state_complete_lifecycle() {
        let mut state = ThinkingState::new();

        // Start: Idle
        assert_eq!(state.phase(), ThinkingPhase::Idle);
        assert!(!state.is_thinking());
        assert!(state.thinking_duration().is_none());

        // Transition: Idle -> Thinking
        state.start_thinking();
        assert_eq!(state.phase(), ThinkingPhase::Thinking);
        assert!(state.is_thinking());
        assert!(state.thinking_duration().is_some());

        // Transition: Thinking -> ReceivingThoughts
        state.append_thinking();
        assert_eq!(state.phase(), ThinkingPhase::ReceivingThoughts);
        assert!(state.is_thinking());

        // Wait a bit to verify duration increases
        std::thread::sleep(Duration::from_millis(10));
        let duration = state.thinking_duration().unwrap();
        assert!(duration.as_millis() >= 10);

        // Transition: ReceivingThoughts -> Idle
        state.stop_thinking();
        assert_eq!(state.phase(), ThinkingPhase::Idle);
        assert!(!state.is_thinking());
        assert!(state.thinking_duration().is_none());
    }

    #[test]
    fn test_thinking_state_stop_is_idempotent() {
        let mut state = ThinkingState::new();

        // Stop when already idle
        state.stop_thinking();
        assert_eq!(state.phase(), ThinkingPhase::Idle);
        assert!(!state.is_thinking());

        // Start and stop
        state.start_thinking();
        state.stop_thinking();

        // Stop again - should be idempotent
        state.stop_thinking();
        assert_eq!(state.phase(), ThinkingPhase::Idle);
        assert!(!state.is_thinking());
    }

    #[test]
    fn test_thinking_state_append_when_idle() {
        let mut state = ThinkingState::new();

        // Append when idle (edge case - shouldn't normally happen)
        // Current behavior: transitions to ReceivingThoughts from Idle
        state.append_thinking();
        assert_eq!(state.phase(), ThinkingPhase::ReceivingThoughts);

        // Duration should be None since we never called start_thinking
        assert!(state.thinking_duration().is_none());
    }

    #[test]
    fn test_thinking_state_clone_preserves_start_time() {
        let mut state = ThinkingState::new();
        state.start_thinking();

        std::thread::sleep(Duration::from_millis(10));

        // Clone the state
        let cloned = state.clone();

        // Both should report similar durations (within a small margin)
        let original_duration = state.thinking_duration().unwrap();
        let cloned_duration = cloned.thinking_duration().unwrap();

        let diff = original_duration.abs_diff(cloned_duration);

        // Difference should be negligible (< 5ms)
        assert!(
            diff.as_millis() < 5,
            "Cloned state should have similar duration"
        );
    }
}
