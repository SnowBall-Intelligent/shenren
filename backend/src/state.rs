use std::path::PathBuf;
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::services::rate_limit::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Arc<Config>,
    pub rate_limiter: Arc<RateLimiter>,
}

impl AppState {
    pub fn uploads_dir(&self) -> &PathBuf {
        &self.config.uploads_dir
    }

    pub fn avatar_url(path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") || path.starts_with('/') {
            path.to_string()
        } else {
            format!("/uploads/{path}")
        }
    }
}
