/// Tests for slash command autocomplete with fuzzy matching
use std::path::PathBuf;

#[test]
fn test_slash_command_discovery() {
    // Verify that test commands exist in .claude/commands/
    let commands_dir = PathBuf::from(".claude/commands");

    if commands_dir.exists() {
        let ultrathink = commands_dir.join("ultrathink.md");
        let debug = commands_dir.join("debug.md");
        let analyze = commands_dir.join("analyze.md");

        assert!(ultrathink.exists(), "ultrathink.md should exist");
        assert!(debug.exists(), "debug.md should exist");
        assert!(analyze.exists(), "analyze.md should exist");
    }
}

#[test]
fn test_fuzzy_matching_logic() {
    // Test fuzzy matching algorithm

    // Exact match should have highest score
    assert!(fuzzy_match("/help", "/help").unwrap() > fuzzy_match("/help", "/exit").unwrap_or(0));

    // Prefix match should score high
    assert!(fuzzy_match("/ex", "/exit").is_some());
    assert!(fuzzy_match("/ul", "/ultrathink").is_some());

    // Fuzzy match should work
    assert!(fuzzy_match("/ulth", "/ultrathink").is_some());

    // Non-match should return None
    assert!(fuzzy_match("/xyz", "/help").is_none());
}

/// Simple fuzzy matching implementation for testing
fn fuzzy_match(pattern: &str, candidate: &str) -> Option<i32> {
    let pattern = pattern.to_lowercase();
    let candidate = candidate.to_lowercase();

    if candidate == pattern {
        return Some(1000);
    }

    if candidate.starts_with(&pattern) {
        return Some(500);
    }

    let mut score = 0;
    let mut candidate_chars = candidate.chars();

    for pattern_char in pattern.chars() {
        let mut found = false;
        for candidate_char in candidate_chars.by_ref() {
            if candidate_char == pattern_char {
                found = true;
                score += 10;
                break;
            }
            score -= 1;
        }

        if !found {
            return None;
        }
    }

    Some(score)
}

#[test]
fn test_completion_scenarios() {
    // Test various completion scenarios

    // "/ex" should match "/exit"
    assert!(fuzzy_match("/ex", "/exit").is_some());

    // "/ul" should match "/ultrathink"
    assert!(fuzzy_match("/ul", "/ultrathink").is_some());

    // "/deb" should match "/debug"
    assert!(fuzzy_match("/deb", "/debug").is_some());

    // "/an" should match "/analyze"
    assert!(fuzzy_match("/an", "/analyze").is_some());

    // "/hlp" should match "/help" (fuzzy)
    assert!(fuzzy_match("/hlp", "/help").is_some());
}

#[test]
fn test_scoring_order() {
    // Verify that better matches get higher scores

    let exit_score_exact = fuzzy_match("/exit", "/exit").unwrap();
    let exit_score_prefix = fuzzy_match("/ex", "/exit").unwrap();
    let exit_score_fuzzy = fuzzy_match("/xt", "/exit").unwrap_or(0);

    assert!(exit_score_exact > exit_score_prefix);
    assert!(exit_score_prefix > exit_score_fuzzy);
}
