//! Stats-related command handlers (/stats, /cost, /context, /usage).

use crate::session::SessionStats;
use crate::tui::{ChatMessage, TuiState};

/// Handle /stats command.
pub(crate) fn handle_stats_command(tui: &mut TuiState, stats: &mut SessionStats, model: &str) {
    stats.update_duration();

    let stats_text = format!(
        "Session Statistics:\n\
         Messages: {} ({} user, {} assistant)\n\
         Input tokens: {}\n\
         Output tokens: {}\n\
         Total tokens: {}\n\
         Tool calls: {}\n\
         Model: {}\n\
         Duration: {}s",
        stats.message_count,
        stats.user_message_count,
        stats.assistant_message_count,
        stats.input_tokens,
        stats.output_tokens,
        stats.total_tokens,
        stats.tool_calls,
        model,
        stats.duration_seconds
    );
    tui.add_message(ChatMessage::system(stats_text));
}

/// Handle /cost command.
pub(crate) fn handle_cost_command(tui: &mut TuiState, stats: &SessionStats) {
    const INPUT_COST_PER_MILLION: f64 = 3.0;
    const OUTPUT_COST_PER_MILLION: f64 = 15.0;

    let input_tokens = stats.input_tokens;
    let output_tokens = stats.output_tokens;
    let total_tokens = stats.total_tokens;

    let input_cost = (input_tokens as f64 / 1_000_000.0) * INPUT_COST_PER_MILLION;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * OUTPUT_COST_PER_MILLION;
    let total_cost = input_cost + output_cost;

    let cost_display = format!(
        "Token Usage & Cost Estimate:\n\n\
         Session Statistics:\n\
         - Input tokens:  {:>8}\n\
         - Output tokens: {:>8}\n\
         - Total tokens:  {:>8}\n\n\
         Estimated Cost (Claude Sonnet 4.5):\n\
         - Input:  ${:>7.4} ({} tokens @ ${}/M)\n\
         - Output: ${:>7.4} ({} tokens @ ${}/M)\n\
         - Total:  ${:>7.4}\n\n\
         Note: Costs are estimates based on current Anthropic pricing.",
        input_tokens,
        output_tokens,
        total_tokens,
        input_cost,
        input_tokens,
        INPUT_COST_PER_MILLION,
        output_cost,
        output_tokens,
        OUTPUT_COST_PER_MILLION,
        total_cost
    );

    tui.add_message(ChatMessage::system(cost_display));
}

/// Handle /context command.
pub(crate) fn handle_context_command(tui: &mut TuiState, stats: &SessionStats, model: &str) {
    const MAX_CONTEXT_TOKENS: u64 = 200_000;

    let used_tokens = stats.total_tokens;
    let percentage = ((used_tokens as f64 / MAX_CONTEXT_TOKENS as f64) * 100.0) as u64;
    let percentage = percentage.min(100);

    let filled = (percentage / 2) as usize;
    let empty = 50 - filled;

    let context_display = format!(
        "Context Window Usage:\n\n\
         Used:      {:>7} tokens ({}%)\n\
         Available: {:>7} tokens\n\
         Maximum:   {:>7} tokens\n\n\
         Visual: [{}{}] {}%\n\n\
         Messages: {} ({} user, {} assistant)\n\
         Model: {}",
        used_tokens,
        percentage,
        MAX_CONTEXT_TOKENS - used_tokens,
        MAX_CONTEXT_TOKENS,
        "=".repeat(filled),
        " ".repeat(empty),
        percentage,
        stats.message_count,
        stats.user_message_count,
        stats.assistant_message_count,
        model
    );

    tui.add_message(ChatMessage::system(context_display));
}

/// Handle /usage command - Display real rate limit data.
pub(crate) fn handle_usage_command(tui: &mut TuiState, stats: &SessionStats) {
    let rl = &stats.rate_limits;

    let mut output = String::from("API Usage & Rate Limits:\n\n");

    if rl.last_updated.is_none() {
        output.push_str(
            "No rate limit data available yet.\n\
             Rate limits are captured from API responses during conversation.\n\n\
             Tip: Send a message to populate rate limit information.",
        );
    } else {
        output.push_str("Rate Limits (Per Minute):\n");
        match (rl.requests_limit, rl.requests_remaining) {
            (Some(limit), Some(remaining)) => {
                let used = limit.saturating_sub(remaining);
                let percent = rl.requests_percentage().unwrap_or(0);
                output.push_str(&format!(
                    "- Requests:  {:>6} / {:<6} used ({}%)\n",
                    used, limit, percent
                ));
                output.push_str(&format!("- Remaining: {:>6} requests\n", remaining));
            }
            _ => {
                output.push_str("- Requests:  No data\n");
            }
        }

        output.push_str("\nToken Limits (Per Day):\n");
        match (rl.tokens_limit, rl.tokens_remaining) {
            (Some(limit), Some(remaining)) => {
                let used = limit.saturating_sub(remaining);
                let percent = rl.tokens_percentage().unwrap_or(0);
                output.push_str(&format!(
                    "- Tokens:    {:>10} / {:<10} used ({}%)\n",
                    used, limit, percent
                ));
                output.push_str(&format!("- Remaining: {:>10} tokens\n", remaining));
            }
            _ => {
                output.push_str("- Tokens:    No data\n");
            }
        }

        output.push_str("\nVisual Progress:\n");
        if let Some(req_pct) = rl.requests_percentage() {
            let filled = (req_pct / 2) as usize;
            let empty = 50usize.saturating_sub(filled);
            output.push_str(&format!(
                "Requests: [{}{}] {}%\n",
                "=".repeat(filled),
                " ".repeat(empty),
                req_pct
            ));
        }
        if let Some(tok_pct) = rl.tokens_percentage() {
            let filled = (tok_pct / 2) as usize;
            let empty = 50usize.saturating_sub(filled);
            output.push_str(&format!(
                "Tokens:   [{}{}] {}%\n",
                "=".repeat(filled),
                " ".repeat(empty),
                tok_pct
            ));
        }

        if let Some(updated) = rl.last_updated {
            output.push_str(&format!(
                "\nLast updated: {}\n",
                updated.format("%Y-%m-%d %H:%M:%S UTC")
            ));
        }
    }

    tui.add_message(ChatMessage::system(output));
}
