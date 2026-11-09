use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

/// Global, shareable async rate limiter: enforces at most `max_per_second`
/// admissions per second, *in total*, across every caller that holds a
/// clone of it. Cloning is cheap (bumps an `Arc` refcount) — construct one
/// `RateLimiter` per API and clone it into every concurrently-spawned task
/// that calls that API, so the limit is enforced globally rather than
/// per-task.
#[derive(Clone)]
pub(crate) struct RateLimiter {
    inner: Arc<Mutex<RateLimiterState>>,
}

struct RateLimiterState {
    min_interval: Duration,
    next_allowed: tokio::time::Instant,
}

impl RateLimiter {
    pub(crate) fn per_second(max_per_second: u32) -> Self {
        RateLimiter {
            inner: Arc::new(Mutex::new(RateLimiterState {
                min_interval: Duration::from_secs_f64(1.0 / max_per_second as f64),
                next_allowed: tokio::time::Instant::now(),
            })),
        }
    }

    /// Reserves the next available slot and waits for it. The reservation
    /// (bumping `next_allowed`) happens while the lock is held, so
    /// concurrently-waiting callers are queued strictly in order and each
    /// gets a slot exactly `min_interval` after the previous one — no two
    /// callers can slip through the same window even under a burst of
    /// simultaneous calls.
    pub(crate) async fn throttle(&self) {
        let wait_until = {
            let mut state = self.inner.lock().await;
            let now = tokio::time::Instant::now();
            let scheduled = state.next_allowed.max(now);
            state.next_allowed = scheduled + state.min_interval;
            scheduled
        };
        tokio::time::sleep_until(wait_until).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn spaces_admissions_by_the_minimum_interval() {
        let limiter = RateLimiter::per_second(10); // one every 100ms
        let start = tokio::time::Instant::now();

        limiter.throttle().await; // immediate
        assert_eq!(start.elapsed(), Duration::ZERO);
        limiter.throttle().await;
        limiter.throttle().await;

        assert_eq!(start.elapsed(), Duration::from_millis(200));
    }

    #[tokio::test(start_paused = true)]
    async fn clones_share_one_limit() {
        let limiter = RateLimiter::per_second(10);
        let clone = limiter.clone();
        let start = tokio::time::Instant::now();

        limiter.throttle().await;
        clone.throttle().await;

        assert_eq!(start.elapsed(), Duration::from_millis(100));
    }
}
