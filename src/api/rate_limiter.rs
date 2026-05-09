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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allows_within_limit() {
        let limiter = AuthRateLimiter::new(3, 10);
        assert!(limiter.check("user@test.com").await.is_ok());
        assert!(limiter.check("user@test.com").await.is_ok());
        assert!(limiter.check("user@test.com").await.is_ok()); // 3rd = still ok
    }

    #[tokio::test]
    async fn test_blocks_after_limit() {
        let limiter = AuthRateLimiter::new(2, 10);
        assert!(limiter.check("test@limit.com").await.is_ok());
        assert!(limiter.check("test@limit.com").await.is_ok());
        let result = limiter.check("test@limit.com").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Too many attempts"));
    }

    #[tokio::test]
    async fn test_different_keys_independent() {
        let limiter = AuthRateLimiter::new(1, 10);
        assert!(limiter.check("alice@test.com").await.is_ok());
        assert!(limiter.check("alice@test.com").await.is_err()); // blocked
        assert!(limiter.check("bob@test.com").await.is_ok()); // different key = ok
    }

    #[tokio::test]
    async fn test_window_expires() {
        let limiter = AuthRateLimiter::new(1, 0); // 0-second window = instant expiry
        assert!(limiter.check("expire@test.com").await.is_ok());
        // Window is 0s, so by the time we check again, the old entry is expired
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(limiter.check("expire@test.com").await.is_ok()); // should be ok after expiry
    }

    #[tokio::test]
    async fn test_max_attempts_zero() {
        let limiter = AuthRateLimiter::new(0, 10);
        // With 0 max attempts, even the first request should be rejected
        let result = limiter.check("zero@test.com").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retry_message_contains_seconds() {
        let limiter = AuthRateLimiter::new(1, 10);
        assert!(limiter.check("retry@test.com").await.is_ok());
        let err = limiter.check("retry@test.com").await.unwrap_err();
        assert!(err.contains("seconds"));
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let limiter = Arc::new(AuthRateLimiter::new(5, 10));
        let mut handles = vec![];
        for i in 0..10 {
            let l = limiter.clone();
            handles.push(tokio::spawn(async move {
                l.check("concurrent@test.com").await
            }));
        }
        let results: Vec<_> = futures::future::join_all(handles).await;
        let ok_count = results.iter().filter(|r| r.as_ref().unwrap().is_ok()).count();
        assert_eq!(ok_count, 5, "only 5 should succeed out of 10 concurrent");
        let err_count = results.iter().filter(|r| r.as_ref().unwrap().is_err()).count();
        assert_eq!(err_count, 5);
    }
}
