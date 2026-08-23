use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::cache::PublicReadCache;
use crate::config::Config;
use crate::services::api_key::ApiKeyLimiters;
use crate::services::rate_limit::RateLimiters;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Arc<Config>,
    pub rate_limiters: Arc<RateLimiters>,
    pub api_key_limiters: Arc<ApiKeyLimiters>,
    pub cache: Arc<PublicReadCache>,
    pub dummy_password_hash: Arc<str>,
}

impl AppState {
    pub fn uploads_dir(&self) -> &std::path::PathBuf {
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
