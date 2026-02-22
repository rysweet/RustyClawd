//! Retry configuration and backoff logic for the Anthropic API client.

use std::time::Duration;

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries (default: 3)
    pub max_retries: u32,
    /// Initial delay before first retry (default: 1s)
    pub initial_delay: Duration,
    /// Maximum delay between retries (default: 30s)
    pub max_delay: Duration,
    /// Jitter factor for randomizing delays (0.0 to 1.0, default: 0.1)
    /// A value of 0.1 means delays can vary by up to 10%
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            jitter_factor: 0.1,
        }
    }
}

impl RetryConfig {
    /// Calculate the delay for a given retry attempt with exponential backoff and jitter.
    ///
    /// The delay is calculated as: `initial_delay * 2^attempt * (1.0 - jitter_factor * random)`
    /// The result is capped at `max_delay`.
    ///
    /// # Arguments
    /// * `attempt` - The retry attempt number (0-indexed)
    ///
    /// # Returns
    /// The calculated delay duration
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        // Calculate base exponential backoff: initial_delay * 2^attempt
        let base_delay_secs = self.initial_delay.as_secs_f64() * 2_f64.powi(attempt as i32);

        // Cap at max_delay before applying jitter
        let capped_delay_secs = base_delay_secs.min(self.max_delay.as_secs_f64());

        // Apply jitter: reduce delay by random factor between 0 and jitter_factor
        // This helps prevent thundering herd when multiple clients retry simultaneously
        let jitter = self.jitter_factor * random_factor();
        let jittered_delay_secs = capped_delay_secs * (1.0 - jitter);

        // Ensure we don't go below a minimum delay of 10ms
        let final_delay_secs = jittered_delay_secs.max(0.01);

        Duration::from_secs_f64(final_delay_secs)
    }
}

/// Generate a random factor between 0.0 and 1.0 for jitter calculation.
fn random_factor() -> f64 {
    rand::random::<f64>()
}
