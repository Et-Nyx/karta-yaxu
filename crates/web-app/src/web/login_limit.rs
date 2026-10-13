//! Constant-sized, process-local login request budget for the local preview.

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

const WINDOW: Duration = Duration::from_secs(60);
const ATTEMPTS: u32 = 30;

pub(super) struct LoginLimit {
    window: Mutex<Window>,
}

struct Window {
    started: Instant,
    attempts: u32,
}

impl LoginLimit {
    pub(super) fn new() -> Self {
        Self {
            window: Mutex::new(Window {
                started: Instant::now(),
                attempts: 0,
            }),
        }
    }

    /// Returns seconds until a new window, rounded up, when the budget is spent.
    pub(super) fn check(&self, now: Instant) -> Result<(), u64> {
        // This lock protects two integers and is never held across an await.
        let mut window = self
            .window
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let elapsed = now.saturating_duration_since(window.started);
        if elapsed >= WINDOW {
            window.started = now;
            window.attempts = 0;
        }
        if window.attempts >= ATTEMPTS {
            let remaining = WINDOW.saturating_sub(now.saturating_duration_since(window.started));
            return Err(remaining.as_secs() + u64::from(remaining.subsec_nanos() != 0));
        }
        window.attempts += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spent_budget_has_a_retry_deadline_and_resets_at_the_boundary() {
        let limit = LoginLimit::new();
        let start = Instant::now();
        for _ in 0..ATTEMPTS {
            assert_eq!(limit.check(start), Ok(()));
        }
        assert_eq!(limit.check(start), Err(60));
        assert_eq!(limit.check(start + Duration::from_millis(1500)), Err(59));
        assert_eq!(limit.check(start + WINDOW), Ok(()));
    }
}
