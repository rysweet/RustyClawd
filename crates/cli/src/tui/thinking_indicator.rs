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

/// Get current shimmer frame based on global animation clock
///
/// Uses system time to ensure synchronized animations across all UI elements.
/// Frame changes every 100ms for smooth but not too fast animation.
fn current_shimmer_frame() -> &'static str {
    let frame_idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        / 100) as usize
        % SHIMMER_FRAMES.len();
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
///
/// # Examples
///
/// ```rust
/// let status = render_thinking_indicator(None);
/// // Returns: "⣾⣀⣀⣀⣀⣀⣀⣀ Extended thinking..."
///
/// let status = render_thinking_indicator(Some(Duration::from_secs(5)));
/// // Returns: "⣿⣦⣀⣀⣀⣀⣀⣀ Extended thinking (5s)..."
/// ```
pub fn render_thinking_indicator(duration: Option<Duration>) -> String {
    let shimmer = current_shimmer_frame();

    if let Some(duration) = duration {
        format!("{} Extended thinking ({})...", shimmer, format_duration(duration))
    } else {
        format!("{} Extended thinking...", shimmer)
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
    fn test_render_thinking_indicator() {
        // Without duration
        let status = render_thinking_indicator(None);
        assert!(status.contains("Extended thinking"));
        assert!(!status.contains('('));

        // With duration
        let status = render_thinking_indicator(Some(Duration::from_secs(5)));
        assert!(status.contains("Extended thinking"));
        assert!(status.contains("5s"));
    }
}
