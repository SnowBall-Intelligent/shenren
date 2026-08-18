use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Simple in-memory per-IP rate limiter for submissions.
pub struct RateLimiter {
    window: Duration,
    max_requests: usize,
    hits: Mutex<HashMap<IpAddr, Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            window,
            max_requests,
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Returns true if the request is allowed.
    pub fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut hits = self.hits.lock().expect("rate limiter lock");
        let entry = hits.entry(ip).or_default();
        entry.retain(|t| now.duration_since(*t) < self.window);
        if entry.len() >= self.max_requests {
            return false;
        }
        entry.push(now);
        true
    }
}
