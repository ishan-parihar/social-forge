// ─── Rate Limiter ──────────────────────────────────────────────
// Simple in-memory rate limiter for auth endpoints.
// Uses a sliding window approach with tokio::sync::Mutex.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// In-memory token bucket rate limiter keyed by IP or email.
#[derive(Clone)]
pub struct AuthRateLimiter {
    attempts: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_attempts: usize,
    window: Duration,
}

impl AuthRateLimiter {
    pub fn new(max_attempts: usize, window_secs: u64) -> Self {
        Self {
            attempts: Arc::new(Mutex::new(HashMap::new())),
            max_attempts,
            window: Duration::from_secs(window_secs),
        }
    }

    /// Check if a key has exceeded the rate limit.
    /// Returns Ok(()) if within limit, Err with retry message if exceeded.
    pub async fn check(&self, key: &str) -> Result<(), String> {
        let now = Instant::now();
        let mut map = self.attempts.lock().await;
        let entries = map.entry(key.to_string()).or_default();
        // Remove expired entries outside the window
        entries.retain(|t| now.duration_since(*t) < self.window);

        if entries.len() >= self.max_attempts {
            let oldest = entries.first().copied().unwrap_or(now);
            let retry_after = self.window.saturating_sub(now.duration_since(oldest));
            return Err(format!(
                "Too many attempts. Try again in {} seconds.",
                retry_after.as_secs()
            ));
        }

        entries.push(now);
        Ok(())
    }
}
