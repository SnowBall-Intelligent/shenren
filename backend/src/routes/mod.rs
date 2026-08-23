pub mod admin;
pub mod api_keys;
pub mod external;
pub mod public;

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, DefaultBodyLimit, MatchedPath, Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use sea_orm::EntityTrait;
use serde_json::json;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_sessions::cookie::SameSite;
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer};

use crate::config::{origin_from_referer, Config};
use crate::entities::admins;
use crate::error::AppError;
use crate::logging::{emit_audit, AuditContext};
use crate::services::auth::{require_super_admin, SESSION_ADMIN_ID};
use crate::services::rate_limit::Bucket;
use crate::state::AppState;

pub fn app_router(state: AppState, config: &Config) -> Router {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_name("shenren_session")
        .with_http_only(true)
        .with_same_site(config.cookie_same_site)
        .with_secure(config.cookie_secure || config.cookie_same_site == SameSite::None)
        .with_path("/")
        .with_expiry(Expiry::OnInactivity(time::Duration::seconds(
            config.session_ttl_secs as i64,
        )));

    let public_api = Router::new()
        .route("/site", get(public::get_site))
        .route("/quotes", get(public::list_quotes))
        .route("/persons", get(public::list_persons))
        .route("/submissions", post(public::create_submission));

    let admin_api = admin_routes(state.clone())
        .layer(middleware::from_fn_with_state(state.clone(), admin_csrf_mw))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            admin_audit_mw,
        ))
        .layer(session_layer);

    let legacy_api = Router::new()
        .merge(public_api)
        .nest("/admin", admin_api)
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024))
        .layer(cors_layer(config));

    let external_api = Router::new()
        .route("/quotes", get(external::list_quotes))
        .route("/quotes/random", get(external::random_quote))
        .layer(external_cors_layer());

    let api = Router::new().merge(legacy_api).nest("/v1", external_api);

    let uploads = Router::new()
        .nest_service("/uploads", ServeDir::new(state.uploads_dir().clone()))
        .layer(cors_layer(config));

    tracing::info!("API-only mode; frontend is not served");

    let timeout = Duration::from_secs(config.request_timeout_secs.max(1));
    let concurrency = config.max_concurrency.max(1);

    Router::new()
        .nest("/api", api)
        .merge(uploads)
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "message": "not found", "error": "not found" })),
            )
        })
        .layer(middleware::from_fn(uploads_and_security_headers))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_mw))
        .layer(CompressionLayer::new())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            request_log_mw,
        ))
        .layer(
            ServiceBuilder::new()
                .layer(axum::error_handling::HandleErrorLayer::new(
                    |err: tower::BoxError| async move {
                        if err.is::<tower::timeout::error::Elapsed>() {
                            (
                                StatusCode::REQUEST_TIMEOUT,
                                Json(json!({
                                    "message": "请求超时",
                                    "error": "请求超时"
                                })),
                            )
                                .into_response()
                        } else {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "message": "服务器错误",
                                    "error": "服务器错误"
                                })),
                            )
                                .into_response()
                        }
                    },
                ))
                .layer(tower::timeout::TimeoutLayer::new(timeout))
                .layer(tower::limit::ConcurrencyLimitLayer::new(concurrency)),
        )
        .with_state(state)
}

async fn request_log_mw(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_else(|| request.uri().path())
        .to_string();
    let client_ip = state.config.client_ip(request.headers(), addr);
    let started = Instant::now();
    let response = next.run(request).await;
    let status = response.status();
    let latency_ms = started.elapsed().as_millis() as u64;

    if status.is_server_error() {
        tracing::error!(
            target: "shenren::http",
            %method,
            %path,
            status = status.as_u16(),
            %client_ip,
            latency_ms,
            "request completed"
        );
    } else if matches!(
        status,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS
    ) {
        tracing::warn!(
            target: "shenren::http",
            %method,
            %path,
            status = status.as_u16(),
            %client_ip,
            latency_ms,
            "request completed"
        );
    } else {
        tracing::info!(
            target: "shenren::http",
            %method,
            %path,
            status = status.as_u16(),
            %client_ip,
            latency_ms,
            "request completed"
        );
    }
    response
}

struct AuditOperation {
    action: &'static str,
    resource: &'static str,
    resource_id: Option<String>,
}

fn audit_operation(method: &Method, path: &str) -> Option<AuditOperation> {
    if !matches!(*method, Method::POST | Method::PUT | Method::DELETE) {
        return None;
    }
    let relative = path
        .strip_prefix("/api/admin/")
        .or_else(|| path.strip_prefix("/admin/"))
        .unwrap_or_else(|| path.trim_start_matches('/'));
    let parts = relative.split('/').collect::<Vec<_>>();
    let op = match (method, parts.as_slice()) {
        (&Method::POST, ["setup"]) => ("setup", "admins", None),
        (&Method::POST, ["login"]) => ("login", "session", None),
        (&Method::POST, ["logout"]) => ("logout", "session", None),
        (&Method::POST, ["admins"]) => ("create", "admins", None),
        (&Method::DELETE, ["admins", id]) => ("delete", "admins", Some((*id).to_string())),
        (&Method::PUT, ["admins", id, "role"]) => {
            ("update_role", "admins", Some((*id).to_string()))
        }
        (&Method::POST, ["api-keys"]) => ("create", "api_keys", None),
        (&Method::PUT, ["api-keys", id]) => ("update", "api_keys", Some((*id).to_string())),
        (&Method::DELETE, ["api-keys", id]) => ("delete", "api_keys", Some((*id).to_string())),
        (&Method::POST, ["api-keys", id, "reset-usage"]) => {
            ("reset_usage", "api_keys", Some((*id).to_string()))
        }
        (&Method::PUT, ["settings"]) => ("update", "settings", None),
        (&Method::PUT, ["captcha"]) => ("update", "captcha", None),
        (&Method::POST, ["persons"]) => ("create", "persons", None),
        (&Method::PUT, ["persons", id]) => ("update", "persons", Some((*id).to_string())),
        (&Method::DELETE, ["persons", id]) => ("delete", "persons", Some((*id).to_string())),
        (&Method::POST, ["quotes"]) => ("create", "quotes", None),
        (&Method::POST, ["quotes", "reorder"]) => ("reorder", "quotes", None),
        (&Method::PUT, ["quotes", id]) => ("update", "quotes", Some((*id).to_string())),
        (&Method::DELETE, ["quotes", id]) => ("delete", "quotes", Some((*id).to_string())),
        (&Method::POST, ["quotes", id, "move"]) => ("move", "quotes", Some((*id).to_string())),
        (&Method::POST, ["quotes", id, "approve"])
        | (&Method::POST, ["quotes", id, "approve-json"]) => {
            ("approve", "quotes", Some((*id).to_string()))
        }
        (&Method::POST, ["quotes", id, "reject"]) => ("reject", "quotes", Some((*id).to_string())),
        _ => return None,
    };
    Some(AuditOperation {
        action: op.0,
        resource: op.1,
        resource_id: op.2,
    })
}

async fn session_admin(session: &Session, state: &AppState) -> Option<admins::Model> {
    let id: i64 = session.get(SESSION_ADMIN_ID).await.ok().flatten()?;
    admins::Entity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
}

async fn admin_audit_mw(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    session: Session,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(operation) = audit_operation(request.method(), request.uri().path()) else {
        return next.run(request).await;
    };
    let client_ip = state.config.client_ip(request.headers(), addr);
    let actor_before = session_admin(&session, &state).await;
    let context = AuditContext::default();
    request.extensions_mut().insert(context.clone());

    let response = next.run(request).await;
    let actor = match actor_before {
        Some(actor) => Some(actor),
        None => session_admin(&session, &state).await,
    };
    let attempted_username = context.username();
    let contextual_resource_id = context.resource_id();
    emit_audit(
        actor.as_ref().map(|admin| admin.id),
        actor
            .as_ref()
            .map(|admin| admin.username.as_str())
            .or(attempted_username.as_deref()),
        actor.as_ref().map(|admin| admin.role.as_str()),
        operation.action,
        operation.resource,
        operation
            .resource_id
            .as_deref()
            .or(contextual_resource_id.as_deref()),
        response.status(),
        client_ip,
    );
    response
}

fn admin_routes(state: AppState) -> Router<AppState> {
    let super_admin_routes = Router::new()
        .route("/admins", get(admin::list_admins).post(admin::create_admin))
        .route("/admins/{id}", delete(admin::delete_admin))
        .route("/admins/{id}/role", put(admin::update_admin_role))
        .route("/api-keys", get(api_keys::list).post(api_keys::create))
        .route(
            "/api-keys/{id}",
            put(api_keys::update).delete(api_keys::delete),
        )
        .route("/api-keys/{id}/reset-usage", post(api_keys::reset_usage))
        .route(
            "/settings",
            get(admin::get_settings).put(admin::update_settings),
        )
        .route(
            "/captcha",
            get(admin::get_captcha).put(admin::update_captcha),
        )
        .route_layer(middleware::from_fn_with_state(state, super_admin_mw));

    Router::new()
        .route("/bootstrap-status", get(admin::bootstrap_status))
        .route("/setup", post(admin::setup))
        .route("/login", post(admin::login))
        .route("/logout", post(admin::logout))
        .route("/me", get(admin::me))
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
        .route("/quotes/reorder", post(admin::reorder_quotes))
        .route(
            "/quotes/{id}",
            put(admin::update_quote).delete(admin::delete_quote),
        )
        .route("/quotes/{id}/move", post(admin::move_quote))
        .route("/quotes/{id}/approve", post(admin::approve_quote))
        .route("/quotes/{id}/approve-json", post(admin::approve_quote_json))
        .route("/quotes/{id}/reject", post(admin::reject_quote))
        .merge(super_admin_routes)
}

fn external_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::ACCEPT])
        .expose_headers([
            header::RETRY_AFTER,
            header::HeaderName::from_static("x-ratelimit-limit"),
            header::HeaderName::from_static("x-ratelimit-remaining"),
            header::HeaderName::from_static("x-ratelimit-reset"),
            header::HeaderName::from_static("x-quota-limit"),
            header::HeaderName::from_static("x-quota-remaining"),
        ])
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
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::IF_NONE_MATCH])
        .expose_headers([header::ETAG, header::RETRY_AFTER])
        .allow_credentials(true)
}

async fn rate_limit_mw(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = request.uri().path().to_string();
    let Some(bucket) = bucket_for(request.method(), &path) else {
        return Ok(next.run(request).await);
    };
    let ip = state.config.client_ip(request.headers(), addr);
    if let Err(retry) = state.rate_limiters.check(bucket, ip) {
        return Err(AppError::TooManyRequests {
            retry_after: retry.max(state.rate_limiters.retry_after(bucket)),
        });
    }
    Ok(next.run(request).await)
}

fn bucket_for(method: &Method, path: &str) -> Option<Bucket> {
    if *method == Method::OPTIONS {
        return None;
    }
    if path.starts_with("/uploads") {
        return Some(Bucket::Uploads);
    }
    if matches!(path, "/api/site" | "/api/quotes" | "/api/persons") {
        return Some(Bucket::Home);
    }
    if path == "/api/submissions" && *method == Method::POST {
        return Some(Bucket::Submit);
    }
    if (path == "/api/admin/login" || path == "/api/admin/setup") && *method == Method::POST {
        return Some(Bucket::Login);
    }
    if path.starts_with("/api/admin") {
        return Some(Bucket::Admin);
    }
    None
}

async fn admin_csrf_mw(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let method = request.method().clone();
    if matches!(method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return Ok(next.run(request).await);
    }
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            request
                .headers()
                .get(header::REFERER)
                .and_then(|v| v.to_str().ok())
                .and_then(origin_from_referer)
        });
    match origin {
        Some(o) if state.config.origin_allowed(&o) => Ok(next.run(request).await),
        _ => Err(AppError::forbidden("来源不被允许")),
    }
}

async fn super_admin_mw(
    State(state): State<AppState>,
    session: Session,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    require_super_admin(&session, &state.db).await?;
    Ok(next.run(request).await)
}

async fn uploads_and_security_headers(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_ascii_lowercase();
    let is_svg = path.starts_with("/uploads/") && path.ends_with(".svg");
    let mut res = next.run(request).await;
    let headers = res.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    if is_svg {
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("image/svg+xml"),
        );
        headers.insert(
            header::HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("default-src 'none'; style-src 'unsafe-inline'"),
        );
    }
    res
}

pub fn cached_json(headers: &axum::http::HeaderMap, body: &crate::cache::CachedBody) -> Response {
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        == Some(body.etag.as_str())
    {
        return (
            StatusCode::NOT_MODIFIED,
            [
                (header::ETAG, body.etag.clone()),
                (header::CACHE_CONTROL, "public, max-age=5".to_string()),
            ],
        )
            .into_response();
    }
    (
        [
            (
                header::CONTENT_TYPE,
                "application/json; charset=utf-8".to_string(),
            ),
            (header::ETAG, body.etag.clone()),
            (header::CACHE_CONTROL, "public, max-age=5".to_string()),
        ],
        body.bytes.clone(),
    )
        .into_response()
}
