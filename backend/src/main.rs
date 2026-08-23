mod cache;
mod config;
mod entities;
mod error;
mod logging;
mod routes;
mod services;
mod state;

use std::sync::Arc;
use std::time::Duration;

use crate::cache::PublicReadCache;
use crate::config::Config;
use crate::services::api_key::ApiKeyLimiters;
use crate::services::auth::hash_password;
use crate::services::rate_limit::RateLimiters;
use crate::state::AppState;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, ConnectionTrait, Database};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let config = Config::from_env().map_err(|e| {
        eprintln!("configuration error: {e}");
        e
    })?;
    let _log_guards = logging::init(&config).map_err(|e| {
        eprintln!("logging initialization failed: {e}");
        e
    })?;
    std::fs::create_dir_all(&config.uploads_dir)?;

    if let Some(path) = sqlite_path_from_url(&config.database_url) {
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut opt = ConnectOptions::new(config.database_url.clone());
    opt.max_connections(config.db_max_connections)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(3))
        .sqlx_logging(false);

    tracing::info!("connecting database...");
    let db = Database::connect(opt).await?;
    if config.database_url.starts_with("sqlite:") {
        db.execute_unprepared("PRAGMA journal_mode=WAL;").await?;
        db.execute_unprepared("PRAGMA busy_timeout=5000;").await?;
        db.execute_unprepared("PRAGMA synchronous=NORMAL;").await?;
    }
    Migrator::up(&db, None).await?;
    tracing::info!("migrations applied");

    let dummy_password_hash = hash_password("!unused-timing-pad!")?;
    let rate_limiters = RateLimiters::new(
        config.rate_limit_home,
        config.rate_limit_submit,
        config.rate_limit_login,
        config.rate_limit_admin,
        config.rate_limit_uploads,
    );

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        rate_limiters: Arc::new(rate_limiters),
        api_key_limiters: Arc::new(ApiKeyLimiters::new()),
        cache: Arc::new(PublicReadCache::new()),
        dummy_password_hash: dummy_password_hash.into(),
    };

    let app = routes::app_router(state, &config);
    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on http://{}", config.bind_addr);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}

fn sqlite_path_from_url(url: &str) -> Option<String> {
    let rest = url.strip_prefix("sqlite://")?;
    let path = rest.split('?').next()?.to_string();
    if path.is_empty() || path == ":memory:" {
        None
    } else {
        Some(path)
    }
}
