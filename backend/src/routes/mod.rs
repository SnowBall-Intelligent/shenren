pub mod admin;
pub mod public;

use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
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
        .with_same_site(config.cookie_same_site)
        .with_secure(config.cookie_secure || config.cookie_same_site == SameSite::None)
        .with_path("/");

    let api = Router::new()
        .route("/site", get(public::get_site))
        .route("/quotes", get(public::list_quotes))
        .route("/persons", get(public::list_persons))
        .route("/submissions", post(public::create_submission))
        .nest("/admin", admin_routes())
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024));

    let uploads = ServeDir::new(state.uploads_dir().clone());

    tracing::info!("API-only mode; frontend is not served");

    Router::new()
        .nest("/api", api)
        .nest_service("/uploads", uploads)
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "message": "not found", "error": "not found" })),
            )
        })
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer(config))
        .with_state(state)
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
            "/captcha",
            get(admin::get_captcha).put(admin::update_captcha),
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
        .route(
            "/quotes/{id}",
            put(admin::update_quote).delete(admin::delete_quote),
        )
        .route("/quotes/{id}/approve", post(admin::approve_quote))
        .route("/quotes/{id}/approve-json", post(admin::approve_quote_json))
        .route("/quotes/{id}/reject", post(admin::reject_quote))
}

fn cors_layer(config: &Config) -> CorsLayer {
    let extra: Vec<HeaderValue> = config
        .cors_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(move |origin, _| {
            let raw = origin.as_bytes();
            raw.starts_with(b"http://localhost:")
                || raw.starts_with(b"http://127.0.0.1:")
                || extra.iter().any(|allowed| allowed.as_bytes() == raw)
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
