//! Token Tracking Integration Tests
//!
//! Verifies that /cost and /context commands display real token data

use rustyclawd::session::SessionStats;

#[test]
fn test_session_stats_tracks_tokens() {
    // Create new session stats
    let mut stats = SessionStats::new("claude-sonnet-4-5");

    // Simulate user message (estimated 100 input tokens)
    stats.add_user_message(100);

    // Simulate assistant response (1000 input + 500 output)
    stats.add_assistant_message(1000, 500);

    // Verify token tracking
    assert_eq!(stats.input_tokens, 1100, "Input tokens should be cumulative");
    assert_eq!(stats.output_tokens, 500, "Output tokens tracked");
    assert_eq!(stats.total_tokens, 1600, "Total should be sum of input+output");
    assert_eq!(stats.message_count, 2, "Should track 2 messages");
    assert_eq!(stats.user_message_count, 1, "Should track 1 user message");
    assert_eq!(stats.assistant_message_count, 1, "Should track 1 assistant message");
}

#[test]
fn test_cost_calculation() {
    // Pricing constants (from builtins.rs)
    const INPUT_COST_PER_MILLION: f64 = 3.0;
    const OUTPUT_COST_PER_MILLION: f64 = 15.0;

    let mut stats = SessionStats::new("claude-sonnet-4-5");

    // Simulate conversation with known token counts
    stats.add_assistant_message(10_000, 5_000);
    stats.add_assistant_message(15_000, 8_000);

    // Calculate costs
    let input_cost = (stats.input_tokens as f64 / 1_000_000.0) * INPUT_COST_PER_MILLION;
    let output_cost = (stats.output_tokens as f64 / 1_000_000.0) * OUTPUT_COST_PER_MILLION;
    let total_cost = input_cost + output_cost;

    // Verify calculations
    assert_eq!(stats.input_tokens, 25_000);
    assert_eq!(stats.output_tokens, 13_000);

    // Input: 25,000 tokens @ $3/M = $0.075
    assert!((input_cost - 0.075).abs() < 0.001, "Input cost should be ~$0.075");

    // Output: 13,000 tokens @ $15/M = $0.195
    assert!((output_cost - 0.195).abs() < 0.001, "Output cost should be ~$0.195");

    // Total: $0.270
    assert!((total_cost - 0.270).abs() < 0.001, "Total cost should be ~$0.270");
}

#[test]
fn test_context_window_percentage() {
    const MAX_TOKENS: u64 = 200_000;

    let mut stats = SessionStats::new("claude-sonnet-4-5");

    // Use 50,000 tokens (25% of context)
    stats.add_assistant_message(30_000, 20_000);

    let used_tokens = stats.total_tokens;
    let percentage = ((used_tokens as f64 / MAX_TOKENS as f64) * 100.0) as u64;

    assert_eq!(used_tokens, 50_000);
    assert_eq!(percentage, 25, "Should be at 25% of context window");
}

#[test]
fn test_tool_call_tracking() {
    let mut stats = SessionStats::new("claude-sonnet-4-5");

    // Simulate tool calls
    stats.add_tool_call();
    stats.add_tool_call();
    stats.add_tool_call();

    assert_eq!(stats.tool_calls, 3, "Should track 3 tool calls");
}

#[test]
fn test_realistic_conversation_scenario() {
    let mut stats = SessionStats::new("claude-sonnet-4-5");

    // User asks a question (estimated 50 tokens)
    stats.add_user_message(50);

    // Assistant responds (2000 input context + 300 output)
    stats.add_assistant_message(2000, 300);
    stats.add_tool_call(); // Assistant uses a tool

    // User follows up (100 tokens)
    stats.add_user_message(100);

    // Assistant responds again (2500 input context + 450 output)
    stats.add_assistant_message(2500, 450);

    // Verify complete session stats
    assert_eq!(stats.message_count, 4);
    assert_eq!(stats.user_message_count, 2);
    assert_eq!(stats.assistant_message_count, 2);
    assert_eq!(stats.input_tokens, 4650); // 50 + 2000 + 100 + 2500
    assert_eq!(stats.output_tokens, 750); // 300 + 450
    assert_eq!(stats.total_tokens, 5400); // 4650 + 750
    assert_eq!(stats.tool_calls, 1);
}

#[test]
fn test_zero_tokens_initialization() {
    let stats = SessionStats::new("claude-sonnet-4-5");

    assert_eq!(stats.input_tokens, 0);
    assert_eq!(stats.output_tokens, 0);
    assert_eq!(stats.total_tokens, 0);
    assert_eq!(stats.message_count, 0);
    assert_eq!(stats.tool_calls, 0);
}

#[test]
fn test_large_token_counts() {
    let mut stats = SessionStats::new("claude-sonnet-4-5");

    // Simulate a very long conversation
    stats.add_assistant_message(150_000, 50_000);

    assert_eq!(stats.input_tokens, 150_000);
    assert_eq!(stats.output_tokens, 50_000);
    assert_eq!(stats.total_tokens, 200_000); // Exactly at context limit
}
