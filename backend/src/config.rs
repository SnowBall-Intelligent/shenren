use std::env;
use std::net::IpAddr;
use std::path::{Component, PathBuf};
use std::time::Duration;

use axum::http::HeaderMap;
use tower_sessions::cookie::SameSite;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub uploads_dir: PathBuf,
    pub cookie_secure: bool,
    pub cookie_same_site: SameSite,
    pub cors_origins: Vec<String>,
    pub db_max_connections: u32,
    pub session_ttl_secs: u64,
    pub request_timeout_secs: u64,
    pub max_concurrency: usize,
    pub rate_limit_trust_proxy: bool,
    pub rate_limit_home: (usize, Duration),
    pub rate_limit_submit: (usize, Duration),
    pub rate_limit_login: (usize, Duration),
    pub rate_limit_admin: (usize, Duration),
    pub rate_limit_uploads: (usize, Duration),
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://data/shenren.db?mode=rwc".to_string());
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
        let uploads_dir = validate_uploads_dir(
            env::var("UPLOADS_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("uploads")),
        )?;

        let loopback = is_loopback_bind(&bind_addr);
        let cookie_secure = match env::var("COOKIE_SECURE") {
            Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"),
            Err(_) => !loopback,
        };
        let cookie_same_site = match env::var("COOKIE_SAMESITE")
            .unwrap_or_else(|_| "Lax".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "none" => SameSite::None,
            "strict" => SameSite::Strict,
            _ => SameSite::Lax,
        };
        let cors_origins = env::var("CORS_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let is_sqlite = database_url.starts_with("sqlite:");
        let db_max_connections = env_u32(
            "DATABASE_MAX_CONNECTIONS",
            if is_sqlite { 8 } else { 32 },
        );

        Ok(Self {
            database_url,
            bind_addr,
            uploads_dir,
            cookie_secure,
            cookie_same_site,
            cors_origins,
            db_max_connections,
            session_ttl_secs: env_u64("SESSION_TTL_SECS", 12 * 60 * 60),
            request_timeout_secs: env_u64("REQUEST_TIMEOUT_SECS", 15),
            max_concurrency: env_u32("MAX_CONCURRENCY", 256) as usize,
            rate_limit_trust_proxy: env_bool("RATE_LIMIT_TRUST_PROXY", false),
            rate_limit_home: (
                env_usize("RATE_LIMIT_HOME", 120),
                Duration::from_secs(env_u64("RATE_LIMIT_HOME_WINDOW", 60)),
            ),
            rate_limit_submit: (
                env_usize("RATE_LIMIT_SUBMIT", 10),
                Duration::from_secs(env_u64("RATE_LIMIT_SUBMIT_WINDOW", 600)),
            ),
            rate_limit_login: (
                env_usize("RATE_LIMIT_LOGIN", 5),
                Duration::from_secs(env_u64("RATE_LIMIT_LOGIN_WINDOW", 60)),
            ),
            rate_limit_admin: (
                env_usize("RATE_LIMIT_ADMIN", 120),
                Duration::from_secs(env_u64("RATE_LIMIT_ADMIN_WINDOW", 60)),
            ),
            rate_limit_uploads: (
                env_usize("RATE_LIMIT_UPLOADS", 240),
                Duration::from_secs(env_u64("RATE_LIMIT_UPLOADS_WINDOW", 60)),
            ),
        })
    }

    pub fn origin_allowed(&self, origin: &str) -> bool {
        let raw = origin.as_bytes();
        raw.starts_with(b"http://localhost:")
            || raw.starts_with(b"http://127.0.0.1:")
            || self.cors_origins.iter().any(|o| o == origin)
    }

    pub fn client_ip(&self, headers: &HeaderMap, peer: std::net::SocketAddr) -> IpAddr {
        if self.rate_limit_trust_proxy {
            if let Some(ip) = header_ip(headers, "cf-connecting-ip") {
                return ip;
            }
            if let Some(xff) = headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
            {
                if let Some(first) = xff.split(',').next() {
                    if let Ok(ip) = first.trim().parse::<IpAddr>() {
                        return ip;
                    }
                }
            }
        }
        peer.ip()
    }
}

fn header_ip(headers: &HeaderMap, name: &'static str) -> Option<IpAddr> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse().ok())
}

fn is_loopback_bind(bind: &str) -> bool {
    let host = bind.rsplit_once(':').map(|(h, _)| h).unwrap_or(bind);
    let host = host.trim_matches(['[', ']']);
    host == "127.0.0.1" || host == "::1" || host == "localhost"
}

pub fn validate_uploads_dir(p: PathBuf) -> Result<PathBuf, String> {
    let cwd = env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let abs = if p.is_absolute() {
        p
    } else {
        cwd.join(p)
    };
    let mut out = PathBuf::new();
    for c in abs.components() {
        match c {
            Component::Prefix(pfx) => out.push(pfx.as_os_str()),
            Component::RootDir => out.push(c.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return Err("UPLOADS_DIR 无效".into());
                }
            }
            Component::Normal(s) => out.push(s),
        }
    }
    let has_normal = out.components().any(|c| matches!(c, Component::Normal(_)));
    if !has_normal {
        return Err("UPLOADS_DIR 不能是盘符根或 /".into());
    }
    Ok(out)
}

pub fn origin_from_referer(referer: &str) -> Option<String> {
    let r = referer.trim();
    let (https, rest) = if let Some(s) = r.strip_prefix("https://") {
        (true, s)
    } else if let Some(s) = r.strip_prefix("http://") {
        (false, s)
    } else {
        return None;
    };
    let hostport = rest.split('/').next()?;
    if hostport.is_empty() {
        return None;
    }
    Some(if https {
        format!("https://{hostport}")
    } else {
        format!("http://{hostport}")
    })
}

fn env_u32(name: &str, default: u32) -> u32 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_bool(name: &str, default: bool) -> bool {
    match env::var(name) {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"),
        Err(_) => default,
    }
}
