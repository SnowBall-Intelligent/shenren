use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_KEYS: usize = 10_000;

#[derive(Clone, Copy, Debug)]
pub enum Bucket {
    Home,
    Submit,
    Login,
    Admin,
    Uploads,
}

pub struct RateLimiters {
    home: Limiter,
    submit: Limiter,
    login: Limiter,
    admin: Limiter,
    uploads: Limiter,
}

struct Limiter {
    max_requests: usize,
    window: Duration,
    hits: Mutex<LimiterInner>,
}

struct LimiterInner {
    map: HashMap<IpAddr, Vec<Instant>>,
    order: VecDeque<IpAddr>,
}

impl RateLimiters {
    pub fn new(
        home: (usize, Duration),
        submit: (usize, Duration),
        login: (usize, Duration),
        admin: (usize, Duration),
        uploads: (usize, Duration),
    ) -> Self {
        Self {
            home: Limiter::new(home.0, home.1),
            submit: Limiter::new(submit.0, submit.1),
            login: Limiter::new(login.0, login.1),
            admin: Limiter::new(admin.0, admin.1),
            uploads: Limiter::new(uploads.0, uploads.1),
        }
    }

    /// Returns Ok(()) or the Retry-After seconds.
    pub fn check(&self, bucket: Bucket, ip: IpAddr) -> Result<(), u64> {
        let lim = match bucket {
            Bucket::Home => &self.home,
            Bucket::Submit => &self.submit,
            Bucket::Login => &self.login,
            Bucket::Admin => &self.admin,
            Bucket::Uploads => &self.uploads,
        };
        lim.check(ip)
    }

    pub fn retry_after(&self, bucket: Bucket) -> u64 {
        let lim = match bucket {
            Bucket::Home => &self.home,
            Bucket::Submit => &self.submit,
            Bucket::Login => &self.login,
            Bucket::Admin => &self.admin,
            Bucket::Uploads => &self.uploads,
        };
        lim.window.as_secs().max(1)
    }
}

impl Limiter {
    fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests: max_requests.max(1),
            window,
            hits: Mutex::new(LimiterInner {
                map: HashMap::new(),
                order: VecDeque::new(),
            }),
        }
    }

    fn check(&self, ip: IpAddr) -> Result<(), u64> {
        let now = Instant::now();
        let mut g = self.hits.lock().expect("rate limiter lock");
        if !g.map.contains_key(&ip) {
            while g.map.len() >= MAX_KEYS {
                if let Some(old) = g.order.pop_front() {
                    if old != ip {
                        g.map.remove(&old);
                    }
                } else {
                    break;
                }
            }
            g.order.push_back(ip);
        }
        let entry = g.map.entry(ip).or_default();
        entry.retain(|t| now.duration_since(*t) < self.window);
        if entry.len() >= self.max_requests {
            return Err(self.window.as_secs().max(1));
        }
        entry.push(now);
        Ok(())
    }
}
