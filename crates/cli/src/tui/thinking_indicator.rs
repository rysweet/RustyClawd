//! Extended Thinking Visual Indicator
//!
//! Provides shimmer animation for extended thinking phases.
//!
//! ## Architecture (Brick Design)
//!
//! Self-contained module for thinking visualization:
//! - Shimmer animation using animated characters
//! - Duration formatting
//! - Status text generation
//!
//! ## Public API ("Studs")
//!
//! - `render_thinking_indicator()`: Generate thinking status text with shimmer

use std::time::Duration;

/// Shimmer animation frames (flowing wave pattern)
///
/// Uses box-drawing and block characters to create a flowing shimmer effect.
/// Animation cycles through 8 frames at 100ms per frame (10 FPS).
const SHIMMER_FRAMES: [&str; 8] = [
    "⣾⣀⣀⣀⣀⣀⣀⣀",
    "⣿⣦⣀⣀⣀⣀⣀⣀",
    "⣿⣿⣦⣀⣀⣀⣀⣀",
    "⣿⣿⣿⣦⣀⣀⣀⣀",
    "⣿⣿⣿⣿⣦⣀⣀⣀",
    "⣀⣿⣿⣿⣿⣦⣀⣀",
    "⣀⣀⣿⣿⣿⣿⣦⣀",
    "⣀⣀⣀⣿⣿⣿⣿⣦",
];

/// Number of shimmer animation frames
const FRAME_COUNT: usize = SHIMMER_FRAMES.len();

/// Get current shimmer frame based on global animation clock
///
/// Uses system time to ensure synchronized animations across all UI elements.
/// Frame changes every 100ms for smooth but not too fast animation.
///
/// # Note on SystemTime vs Instant
///
/// This uses `SystemTime` instead of `Instant` for animation timing.
/// While `Instant` is more appropriate for monotonic timing, `SystemTime`
/// is acceptable here because:
/// - This is a cosmetic animation, not critical timing
/// - `unwrap_or_default()` handles clock adjustments gracefully
/// - Animation synchronization across the entire UI is more important
///   than perfect monotonicity
fn current_shimmer_frame() -> &'static str {
    let frame_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        / 100) as usize
        % FRAME_COUNT;
    SHIMMER_FRAMES[frame_idx]
}

/// Format duration as human-readable string
///
/// Examples:
/// - "2s"
/// - "45s"
/// - "1m 23s"
/// - "5m 12s"
fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    if total_secs < 60 {
        format!("{}s", total_secs)
    } else {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{}m {:02}s", mins, secs)
    }
}

/// Render thinking indicator with shimmer animation
///
/// Returns status text to display in status bar or message area.
///
/// # Arguments
///
/// * `duration` - Optional duration of thinking phase (None if just started)
/// * `custom_tips` - Optional custom tips to cycle through instead of default text.
///   When provided and non-empty, displays tips from this list in rotation
///   based on elapsed time.
/// * `reduced_motion` - When true, returns a static indicator without animation
///   (accessibility support for users sensitive to motion).
///
/// # Examples
///
/// ```rust,no_run
/// # use std::time::Duration;
/// # fn render_thinking_indicator(_: Option<Duration>, _: Option<&[String]>, _: bool) -> String { String::new() }
/// let status = render_thinking_indicator(None, None, false);
/// // Returns: "⣾⣀⣀⣀⣀⣀⣀⣀ Extended thinking..."
///
/// let tips = vec!["Analyzing code...".to_string(), "Reading docs...".to_string()];
/// let status = render_thinking_indicator(None, Some(&tips), false);
/// // Returns: "⣾⣀⣀⣀⣀⣀⣀⣀ Analyzing code..."
///
/// let status = render_thinking_indicator(Some(Duration::from_secs(5)), None, true);
/// // Returns: "... Extended thinking (5s)"
/// ```
pub fn render_thinking_indicator(
    duration: Option<Duration>,
    custom_tips: Option<&[String]>,
    reduced_motion: bool,
) -> String {
    // Determine the display text
    let tip_text = match custom_tips {
        Some(tips) if !tips.is_empty() => {
            // Cycle through custom tips based on elapsed time (change every 3 seconds)
            let cycle_idx = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                / 3) as usize
                % tips.len();
            tips[cycle_idx].clone()
        }
        _ => "Extended thinking".to_string(),
    };

    if reduced_motion {
        // Static indicator without animated shimmer
        if let Some(duration) = duration {
            format!("... {} ({})", tip_text, format_duration(duration))
        } else {
            format!("... {}", tip_text)
        }
    } else {
        let shimmer = current_shimmer_frame();
        if let Some(duration) = duration {
            format!(
                "{} {} ({})...",
                shimmer,
                tip_text,
                format_duration(duration)
            )
        } else {
            format!("{} {}...", shimmer, tip_text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shimmer_frames_count() {
        assert_eq!(SHIMMER_FRAMES.len(), 8);
    }

    #[test]
    fn test_current_shimmer_frame() {
        let frame = current_shimmer_frame();
        assert!(SHIMMER_FRAMES.contains(&frame));
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0s");
        assert_eq!(format_duration(Duration::from_secs(5)), "5s");
        assert_eq!(format_duration(Duration::from_secs(59)), "59s");
        assert_eq!(format_duration(Duration::from_secs(60)), "1m 00s");
        assert_eq!(format_duration(Duration::from_secs(65)), "1m 05s");
        assert_eq!(format_duration(Duration::from_secs(125)), "2m 05s");
    }

    #[test]
    fn test_render_thinking_indicator_default() {
        // Without duration, no custom tips, no reduced motion
        let status = render_thinking_indicator(None, None, false);
        assert!(status.contains("Extended thinking"));
        assert!(!status.contains('('));

        // With duration
        let status = render_thinking_indicator(Some(Duration::from_secs(5)), None, false);
        assert!(status.contains("Extended thinking"));
        assert!(status.contains("5s"));
    }

    #[test]
    fn test_render_thinking_indicator_custom_tips() {
        let tips = vec!["Analyzing code".to_string(), "Reading docs".to_string()];

        // Custom tips should show one of the tips, not "Extended thinking"
        let status = render_thinking_indicator(None, Some(&tips), false);
        assert!(
            status.contains("Analyzing code") || status.contains("Reading docs"),
            "Expected one of the custom tips, got: {}",
            status
        );
        // Should NOT contain "Extended thinking" when custom tips are provided
        assert!(!status.contains("Extended thinking"));
    }

    #[test]
    fn test_render_thinking_indicator_empty_tips_falls_back() {
        let empty_tips: Vec<String> = vec![];

        // Empty tips should fall back to default "Extended thinking"
        let status = render_thinking_indicator(None, Some(&empty_tips), false);
        assert!(status.contains("Extended thinking"));
    }

    #[test]
    fn test_render_thinking_indicator_reduced_motion() {
        // Reduced motion should show static indicator (no shimmer characters)
        let status = render_thinking_indicator(None, None, true);
        assert!(status.starts_with("... "));
        assert!(status.contains("Extended thinking"));
        // Should NOT contain any shimmer frame characters
        for frame in &SHIMMER_FRAMES {
            assert!(
                !status.contains(frame),
                "Reduced motion should not contain shimmer: {}",
                status
            );
        }

        // With duration
        let status = render_thinking_indicator(Some(Duration::from_secs(10)), None, true);
        assert!(status.starts_with("... "));
        assert!(status.contains("10s"));
    }

    #[test]
    fn test_render_thinking_indicator_reduced_motion_with_custom_tips() {
        let tips = vec!["Custom tip".to_string()];

        let status = render_thinking_indicator(None, Some(&tips), true);
        assert!(status.starts_with("... "));
        assert!(status.contains("Custom tip"));
        assert!(!status.contains("Extended thinking"));
    }
}
