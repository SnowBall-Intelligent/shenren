mod config;
mod entities;
mod error;
mod routes;
mod services;
mod state;

use std::sync::Arc;
use std::time::Duration;

use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::services::rate_limit::RateLimiter;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,sea_orm=info")),
        )
        .init();

    let config = Config::from_env();
    std::fs::create_dir_all(&config.uploads_dir)?;

    // Ensure SQLite parent directory exists.
    if let Some(path) = sqlite_path_from_url(&config.database_url) {
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut opt = ConnectOptions::new(config.database_url.clone());
    opt.max_connections(10)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(10))
        .sqlx_logging(false);

    tracing::info!("connecting database...");
    let db = Database::connect(opt).await?;
    Migrator::up(&db, None).await?;
    tracing::info!("migrations applied");

    let state = AppState {
        db,
        config: Arc::new(config.clone()),
        // 10 submissions / 10 minutes per IP
        rate_limiter: Arc::new(RateLimiter::new(10, Duration::from_secs(10 * 60))),
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
