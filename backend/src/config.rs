use std::env;
use std::path::PathBuf;

use tower_sessions::cookie::SameSite;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub uploads_dir: PathBuf,
    pub cookie_secure: bool,
    pub cookie_same_site: SameSite,
    /// Extra allowed browser origins (Vite localhost is always allowed).
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://data/shenren.db?mode=rwc".to_string());
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
        let uploads_dir = env::var("UPLOADS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("uploads"));
        let cookie_secure = env::var("COOKIE_SECURE")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
            .unwrap_or(false);
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

        Self {
            database_url,
            bind_addr,
            uploads_dir,
            cookie_secure,
            cookie_same_site,
            cors_origins,
        }
    }
}
