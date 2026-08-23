use std::env;
use std::net::IpAddr;
use std::path::{Component, PathBuf};
use std::time::Duration;

use axum::http::HeaderMap;
use chrono_tz::Tz;
use tower_sessions::cookie::SameSite;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub uploads_dir: PathBuf,
    pub log_enabled: bool,
    pub log_level: String,
    pub log_timezone: Tz,
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
        let log_enabled = parse_bool("LOG_ENABLED", env::var("LOG_ENABLED").ok().as_deref(), true)?;
        let log_level = parse_log_level(env::var("LOG_LEVEL").ok().as_deref())?;
        let log_timezone = parse_log_timezone(env::var("LOG_TIMEZONE").ok().as_deref())?;

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
        let db_max_connections =
            env_u32("DATABASE_MAX_CONNECTIONS", if is_sqlite { 8 } else { 32 });

        Ok(Self {
            database_url,
            bind_addr,
            uploads_dir,
            log_enabled,
            log_level,
            log_timezone,
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
            if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
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
    let abs = if p.is_absolute() { p } else { cwd.join(p) };
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

fn parse_bool(name: &str, raw: Option<&str>, default: bool) -> Result<bool, String> {
    let Some(raw) = raw else {
        return Ok(default);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" => Ok(true),
        "0" | "false" | "no" => Ok(false),
        _ => Err(format!("{name} 必须是 true/false、1/0 或 yes/no")),
    }
}

fn parse_log_level(raw: Option<&str>) -> Result<String, String> {
    let value = raw.unwrap_or("info").trim().to_ascii_lowercase();
    match value.as_str() {
        "error" | "warn" | "info" | "debug" | "trace" => Ok(value),
        _ => Err("LOG_LEVEL 必须是 error、warn、info、debug 或 trace".to_string()),
    }
}

fn parse_log_timezone(raw: Option<&str>) -> Result<Tz, String> {
    let value = raw
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("UTC");
    value
        .parse::<Tz>()
        .map_err(|_| format!("LOG_TIMEZONE 不是有效的 IANA 时区名: {value}"))
}

#[cfg(test)]
mod tests {
    use super::{parse_bool, parse_log_level, parse_log_timezone};

    #[test]
    fn logging_defaults_are_enabled_info_and_utc() {
        assert!(parse_bool("LOG_ENABLED", None, true).unwrap());
        assert_eq!(parse_log_level(None).unwrap(), "info");
        assert_eq!(parse_log_timezone(None).unwrap().name(), "UTC");
    }

    #[test]
    fn logging_values_are_strictly_validated() {
        assert!(!parse_bool("LOG_ENABLED", Some("NO"), true).unwrap());
        assert_eq!(parse_log_level(Some("TRACE")).unwrap(), "trace");
        assert_eq!(
            parse_log_timezone(Some("Asia/Hong_Kong")).unwrap().name(),
            "Asia/Hong_Kong"
        );
        assert!(parse_bool("LOG_ENABLED", Some("sometimes"), true).is_err());
        assert!(parse_log_level(Some("verbose")).is_err());
        assert!(parse_log_timezone(Some("HongKong-ish")).is_err());
    }
}
