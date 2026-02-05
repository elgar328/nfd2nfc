use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct TickTimer {
    interval: Duration,
    last: Instant,
}

impl TickTimer {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: Instant::now(),
        }
    }

    /// Returns true and resets if the interval has elapsed.
    pub fn ready(&mut self) -> bool {
        if self.last.elapsed() >= self.interval {
            self.last = Instant::now();
            true
        } else {
            false
        }
    }
}
