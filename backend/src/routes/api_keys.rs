use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::entities::api_keys;
use crate::error::{AppError, AppResult};
use crate::services::api_key::{
    encode_string_list, generate_api_key, normalize_domain_rules, normalize_ip_rules,
    parse_string_list,
};
use crate::services::auth::require_super_admin;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ApiKeyBody {
    pub name: String,
    pub enabled: Option<bool>,
    pub rate_limit: Option<u64>,
    pub rate_window_secs: Option<u64>,
    pub total_quota: Option<u64>,
    pub concurrency_limit: Option<u64>,
    #[serde(default)]
    pub allowed_ips: Vec<String>,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
}

#[derive(Serialize)]
pub struct ApiKeyItem {
    pub id: i64,
    pub name: String,
    pub key_prefix: String,
    pub enabled: bool,
    pub rate_limit: Option<u64>,
    pub rate_window_secs: Option<u64>,
    pub total_quota: Option<u64>,
    pub used_count: u64,
    pub concurrency_limit: Option<u64>,
    pub allowed_ips: Vec<String>,
    pub allowed_domains: Vec<String>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
    pub last_used_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

#[derive(Serialize)]
pub struct ApiKeyMessage {
    pub message: String,
}

struct ValidatedApiKeyBody {
    name: String,
    enabled: bool,
    rate_limit: Option<i64>,
    rate_window_secs: Option<i64>,
    total_quota: Option<i64>,
    concurrency_limit: Option<i64>,
    allowed_ips: Vec<String>,
    allowed_domains: Vec<String>,
}

fn positive_i64(value: Option<u64>, field: &str) -> AppResult<Option<i64>> {
    value
        .map(|value| {
            if value == 0 || value > i64::MAX as u64 {
                return Err(AppError::bad_request(format!("{field} 必须是正整数")));
            }
            Ok(value as i64)
        })
        .transpose()
}

fn validate_body(body: ApiKeyBody) -> AppResult<ValidatedApiKeyBody> {
    let name = body.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 128 {
        return Err(AppError::bad_request("Key 名称长度须为 1-128 个字符"));
    }
    if body.rate_limit.is_some() != body.rate_window_secs.is_some() {
        return Err(AppError::bad_request(
            "频率次数与时间窗口必须同时填写或同时留空",
        ));
    }
    Ok(ValidatedApiKeyBody {
        name,
        enabled: body.enabled.unwrap_or(true),
        rate_limit: positive_i64(body.rate_limit, "频率次数")?,
        rate_window_secs: positive_i64(body.rate_window_secs, "时间窗口")?,
        total_quota: positive_i64(body.total_quota, "总额度")?,
        concurrency_limit: positive_i64(body.concurrency_limit, "并发上限")?,
        allowed_ips: normalize_ip_rules(body.allowed_ips)?,
        allowed_domains: normalize_domain_rules(body.allowed_domains)?,
    })
}

fn as_u64(value: Option<i64>) -> Option<u64> {
    value.and_then(|value| u64::try_from(value).ok())
}

fn item(model: api_keys::Model, key: Option<String>) -> AppResult<ApiKeyItem> {
    Ok(ApiKeyItem {
        id: model.id,
        name: model.name,
        key_prefix: model.key_prefix,
        enabled: model.enabled,
        rate_limit: as_u64(model.rate_limit),
        rate_window_secs: as_u64(model.rate_window_secs),
        total_quota: as_u64(model.total_quota),
        used_count: u64::try_from(model.used_count).unwrap_or_default(),
        concurrency_limit: as_u64(model.concurrency_limit),
        allowed_ips: parse_string_list(&model.allowed_ips)?,
        allowed_domains: parse_string_list(&model.allowed_domains)?,
        created_at: model.created_at,
        updated_at: model.updated_at,
        last_used_at: model.last_used_at,
        key,
    })
}

pub async fn list(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Json<Vec<ApiKeyItem>>> {
    require_super_admin(&session, &state.db).await?;
    let rows = api_keys::Entity::find()
        .order_by_desc(api_keys::Column::CreatedAt)
        .all(&state.db)
        .await?;
    let items = rows
        .into_iter()
        .map(|row| item(row, None))
        .collect::<AppResult<Vec<_>>>()?;
    Ok(Json(items))
}

pub async fn create(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<ApiKeyBody>,
) -> AppResult<(StatusCode, Json<ApiKeyItem>)> {
    require_super_admin(&session, &state.db).await?;
    let body = validate_body(body)?;
    let (raw, prefix, hash) = generate_api_key();
    let now = Utc::now().fixed_offset();
    let model = api_keys::ActiveModel {
        id: sea_orm::NotSet,
        name: Set(body.name),
        key_prefix: Set(prefix),
        key_hash: Set(hash),
        enabled: Set(body.enabled),
        rate_limit: Set(body.rate_limit),
        rate_window_secs: Set(body.rate_window_secs),
        total_quota: Set(body.total_quota),
        used_count: Set(0),
        concurrency_limit: Set(body.concurrency_limit),
        allowed_ips: Set(encode_string_list(&body.allowed_ips)?),
        allowed_domains: Set(encode_string_list(&body.allowed_domains)?),
        created_at: Set(now),
        updated_at: Set(now),
        last_used_at: Set(None),
    }
    .insert(&state.db)
    .await?;
    Ok((StatusCode::CREATED, Json(item(model, Some(raw))?)))
}

pub async fn update(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Json(body): Json<ApiKeyBody>,
) -> AppResult<Json<ApiKeyItem>> {
    require_super_admin(&session, &state.db).await?;
    let body = validate_body(body)?;
    let model = api_keys::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("API Key 不存在"))?;
    let mut active: api_keys::ActiveModel = model.into();
    active.name = Set(body.name);
    active.enabled = Set(body.enabled);
    active.rate_limit = Set(body.rate_limit);
    active.rate_window_secs = Set(body.rate_window_secs);
    active.total_quota = Set(body.total_quota);
    active.concurrency_limit = Set(body.concurrency_limit);
    active.allowed_ips = Set(encode_string_list(&body.allowed_ips)?);
    active.allowed_domains = Set(encode_string_list(&body.allowed_domains)?);
    active.updated_at = Set(Utc::now().fixed_offset());
    let updated = active.update(&state.db).await?;
    state.api_key_limiters.clear(id);
    Ok(Json(item(updated, None)?))
}

pub async fn delete(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiKeyMessage>> {
    require_super_admin(&session, &state.db).await?;
    let result = api_keys::Entity::delete_many()
        .filter(api_keys::Column::Id.eq(id))
        .exec(&state.db)
        .await?;
    if result.rows_affected == 0 {
        return Err(AppError::not_found("API Key 不存在"));
    }
    state.api_key_limiters.clear(id);
    Ok(Json(ApiKeyMessage {
        message: "API Key 已删除".to_string(),
    }))
}

pub async fn reset_usage(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiKeyItem>> {
    require_super_admin(&session, &state.db).await?;
    let model = api_keys::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("API Key 不存在"))?;
    let mut active: api_keys::ActiveModel = model.into();
    active.used_count = Set(0);
    active.last_used_at = Set(None);
    active.updated_at = Set(Utc::now().fixed_offset());
    let updated = active.update(&state.db).await?;
    state.api_key_limiters.clear(id);
    Ok(Json(item(updated, None)?))
}
