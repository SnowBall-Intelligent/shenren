pub mod admin;
pub mod public;

use axum::extract::DefaultBodyLimit;
use axum::http::{header, Method};
use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tower_sessions::cookie::SameSite;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use crate::config::Config;
use crate::state::AppState;

pub fn app_router(state: AppState, config: &Config) -> Router {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_name("shenren_session")
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_secure(config.cookie_secure)
        .with_path("/");

    let api = Router::new()
        .route("/site", get(public::get_site))
        .route("/quotes", get(public::list_quotes))
        .route("/persons", get(public::list_persons))
        .route("/submissions", post(public::create_submission))
        .nest("/admin", admin_routes())
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024));

    let uploads = ServeDir::new(state.uploads_dir().clone());

    let mut router = Router::new()
        .nest("/api", api)
        .nest_service("/uploads", uploads)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        // Last layer is outermost: allow Vite (localhost:5173) to call :3000
        // directly with cookies if the proxy is bypassed.
        .layer(dev_cors_layer())
        .with_state(state);

    if config.frontend_dist.join("index.html").is_file() {
        let index = ServeFile::new(config.frontend_dist.join("index.html"));
        let dist = ServeDir::new(config.frontend_dist.clone()).not_found_service(index);
        router = router.fallback_service(dist);
        tracing::info!("serving frontend from {}", config.frontend_dist.display());
    } else {
        tracing::info!(
            "frontend dist not found at {}; API-only mode",
            config.frontend_dist.display()
        );
    }

    router
}

fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/bootstrap-status", get(admin::bootstrap_status))
        .route("/setup", post(admin::setup))
        .route("/login", post(admin::login))
        .route("/logout", post(admin::logout))
        .route("/me", get(admin::me))
        .route("/admins", get(admin::list_admins).post(admin::create_admin))
        .route("/admins/{id}", delete(admin::delete_admin))
        .route(
            "/settings",
            get(admin::get_settings).put(admin::update_settings),
        )
        .route(
            "/persons",
            get(admin::list_persons_admin).post(admin::create_person),
        )
        .route(
            "/persons/{id}",
            put(admin::update_person).delete(admin::delete_person),
        )
        .route(
            "/quotes",
            get(admin::list_quotes_admin).post(admin::create_quote),
        )
        .route("/quotes/{id}/approve", post(admin::approve_quote))
        .route("/quotes/{id}/approve-json", post(admin::approve_quote_json))
        .route("/quotes/{id}/reject", post(admin::reject_quote))
}

fn dev_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin.as_bytes().starts_with(b"http://localhost:")
                || origin.as_bytes().starts_with(b"http://127.0.0.1:")
        }))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
        .allow_credentials(true)
}
