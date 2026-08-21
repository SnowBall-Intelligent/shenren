use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("too many requests")]
    TooManyRequests { retry_after: u64 },
    #[error(transparent)]
    Db(#[from] sea_orm::DbErr),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Session(#[from] tower_sessions::session::Error),
    #[error("{0}")]
    Internal(String),
    #[error("{0}")]
    CaptchaFailed(String),
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized(msg.into())
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::Forbidden(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    pub fn captcha_failed(msg: impl Into<String>) -> Self {
        Self::CaptchaFailed(msg.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            AppError::TooManyRequests { retry_after } => {
                let secs = (*retry_after).max(1);
                let retry = axum::http::HeaderValue::from_str(&secs.to_string())
                    .unwrap_or_else(|_| axum::http::HeaderValue::from_static("1"));
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(axum::http::header::RETRY_AFTER, retry)],
                    Json(json!({
                        "message": "请求过于频繁，请稍后再试",
                        "error": "请求过于频繁，请稍后再试"
                    })),
                )
                    .into_response();
            }
            AppError::Db(e) => {
                tracing::error!("database error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "数据库错误".to_string())
            }
            AppError::Io(e) => {
                tracing::error!("io error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "文件读写错误".to_string(),
                )
            }
            AppError::Session(e) => {
                tracing::error!("session error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "会话错误".to_string())
            }
            AppError::Internal(m) => {
                tracing::error!("internal error: {m}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "服务器错误".to_string(),
                )
            }
            AppError::CaptchaFailed(m) => (StatusCode::BAD_REQUEST, m.clone()),
        };

        let mut body = json!({ "message": message, "error": message });
        if matches!(self, AppError::CaptchaFailed(_)) {
            body["captcha_fallback"] = json!(true);
        }
        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
