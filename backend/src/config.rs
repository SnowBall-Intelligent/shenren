use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub uploads_dir: PathBuf,
    pub frontend_dist: PathBuf,
    pub cookie_secure: bool,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://data/shenren.db?mode=rwc".to_string());
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
        let uploads_dir = env::var("UPLOADS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("uploads"));
        let frontend_dist = env::var("FRONTEND_DIST")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("../frontend/dist"));
        let cookie_secure = env::var("COOKIE_SECURE")
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
            .unwrap_or(false);

        Self {
            database_url,
            bind_addr,
            uploads_dir,
            frontend_dist,
            cookie_secure,
        }
    }
}
