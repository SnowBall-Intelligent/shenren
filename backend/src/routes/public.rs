use std::collections::HashMap;
use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    Set,
};
use serde::{Deserialize, Serialize};

use crate::entities::{persons, quotes, site_settings};
use crate::error::{AppError, AppResult};
use crate::routes::cached_json;
use crate::services::captcha::{CaptchaPayload, PublicProvider};
use crate::services::sanitize::{normalize_proposed_name, normalize_source};
use crate::state::AppState;

pub use crate::services::sanitize::normalize_quote_content;

#[derive(Serialize)]
pub struct PublicCaptcha {
    pub providers: Vec<PublicProvider>,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_key: Option<String>,
}

#[derive(Serialize)]
pub struct SiteResponse {
    pub site_name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub footer: Option<String>,
    pub allow_propose_person: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captcha: Option<PublicCaptcha>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub fn public_captcha(settings: &site_settings::Model) -> PublicCaptcha {
    let providers = crate::services::captcha::public_provider_list(settings);
    let (provider, site_key) = match providers.first() {
        Some(item) => (item.provider.clone(), Some(item.site_key.clone())),
        None => ("none".to_string(), None),
    };
    PublicCaptcha {
        providers,
        provider,
        site_key,
    }
}

#[derive(Serialize, Clone)]
pub struct PersonBrief {
    pub id: i64,
    pub name: String,
    pub avatar_url: String,
}

#[derive(Serialize)]
pub struct QuoteItem {
    pub id: i64,
    pub person_id: i64,
    pub content: String,
    pub source: Option<String>,
    pub pinned: bool,
    pub sort_order: i32,
    pub published_at: chrono::DateTime<chrono::FixedOffset>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub person: PersonBrief,
}

#[derive(Serialize)]
pub struct PaginatedQuotes {
    pub items: Vec<QuoteItem>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

#[derive(Deserialize)]
pub struct QuotesQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub person_id: Option<i64>,
    pub q: Option<String>,
    pub pinned: Option<bool>,
    pub recent: Option<bool>,
}

#[derive(Deserialize)]
pub struct PersonsQuery {
    pub q: Option<String>,
    pub limit: Option<u64>,
}

#[derive(Deserialize)]
pub struct SubmissionBody {
    pub person_id: Option<i64>,
    pub proposed_person_name: Option<String>,
    pub content: String,
    pub source: Option<String>,
    pub published_at: Option<String>,
    pub place_before_id: Option<i64>,
    pub place_after_id: Option<i64>,
    pub captcha: Option<CaptchaPayload>,
}

pub async fn get_site(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Response> {
    if let Some(cached) = state.cache.get_site() {
        return Ok(cached_json(&headers, &cached));
    }
    let settings = ensure_site_settings(&state).await?;
    let captcha = public_captcha(&settings);
    let body = SiteResponse {
        site_name: settings.site_name,
        description: settings.description,
        logo_url: settings.logo_url,
        footer: settings.footer,
        allow_propose_person: settings.allow_propose_person,
        captcha: Some(captcha),
        message: None,
    };
    let raw = serde_json::to_vec(&body).map_err(|e| AppError::internal(format!("json: {e}")))?;
    let cached = state
        .cache
        .put_site(raw)
        .ok_or_else(|| AppError::internal("site payload too large"))?;
    Ok(cached_json(&headers, &cached))
}

pub async fn list_persons(
    State(state): State<AppState>,
    Query(query): Query<PersonsQuery>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let q = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_string();
    let limit = query.limit.unwrap_or(50).clamp(1, 100);

    if let Some(cached) = state.cache.get_persons(&q, limit) {
        return Ok(cached_json(&headers, &cached));
    }

    let mut finder = persons::Entity::find().order_by_asc(persons::Column::Name);
    if !q.is_empty() {
        finder = finder.filter(persons::Column::Name.contains(&q));
    }
    let rows = finder.limit(limit).all(&state.db).await?;
    let items: Vec<PersonBrief> = rows
        .into_iter()
        .map(|p| PersonBrief {
            id: p.id,
            name: p.name,
            avatar_url: AppState::avatar_url(&p.avatar_path),
        })
        .collect();
    let raw = serde_json::to_vec(&items).map_err(|e| AppError::internal(format!("json: {e}")))?;
    let cached = match state.cache.put_persons(&q, limit, raw.clone()) {
        Some(c) => c,
        None => crate::cache::cached_or_raw(raw),
    };
    Ok(cached_json(&headers, &cached))
}

pub async fn list_quotes(
    State(state): State<AppState>,
    Query(query): Query<QuotesQuery>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
    let q = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let skip_cache = q.is_some() || query.pinned.is_some() || query.recent.unwrap_or(false);

    if !skip_cache {
        if let Some(cached) = state.cache.get_quotes(page, page_size, query.person_id) {
            return Ok(cached_json(&headers, &cached));
        }
    }

    let payload = load_quotes_page(
        &state,
        page,
        page_size,
        query.person_id,
        q.as_deref(),
        query.pinned,
        query.recent.unwrap_or(false),
    )
    .await?;
    let raw = serde_json::to_vec(&payload).map_err(|e| AppError::internal(format!("json: {e}")))?;
    let cached = if skip_cache {
        crate::cache::cached_or_raw(raw)
    } else {
        match state.cache.put_quotes(page, page_size, query.person_id, raw.clone()) {
            Some(c) => c,
            None => crate::cache::cached_or_raw(raw),
        }
    };
    Ok(cached_json(&headers, &cached))
}

async fn load_quotes_page(
    state: &AppState,
    page: u64,
    page_size: u64,
    person_id: Option<i64>,
    q: Option<&str>,
    pinned: Option<bool>,
    recent: bool,
) -> AppResult<PaginatedQuotes> {
    let mut finder = quotes::Entity::find()
        .filter(quotes::Column::Status.eq(quotes::status::APPROVED))
        .filter(quotes::Column::PersonId.is_not_null());
    if let Some(pid) = person_id {
        finder = finder.filter(quotes::Column::PersonId.eq(pid));
    }
    if let Some(pinned) = pinned {
        finder = finder.filter(quotes::Column::Pinned.eq(pinned));
    }
    if let Some(q) = q.filter(|s| !s.is_empty()) {
        finder = finder.filter(crate::services::quote_place::quote_search_condition(&state.db, q).await?);
    }
    let finder = if recent {
        finder
            .order_by_desc(quotes::Column::PublishedAt)
            .order_by_desc(quotes::Column::Id)
    } else {
        finder
            .order_by_desc(quotes::Column::Pinned)
            .order_by_desc(quotes::Column::SortOrder)
            .order_by_desc(quotes::Column::PublishedAt)
            .order_by_desc(quotes::Column::Id)
    };
    let paginator = finder.paginate(&state.db, page_size);

    let total = paginator.num_items().await?;
    let rows = paginator.fetch_page(page - 1).await?;

    let person_ids: Vec<i64> = rows.iter().filter_map(|q| q.person_id).collect();
    let people = if person_ids.is_empty() {
        Vec::new()
    } else {
        persons::Entity::find()
            .filter(persons::Column::Id.is_in(person_ids.clone()))
            .all(&state.db)
            .await?
    };
    let people_map: HashMap<i64, persons::Model> = people.into_iter().map(|p| (p.id, p)).collect();

    let mut items = Vec::with_capacity(rows.len());
    for q in rows {
        let pid = q
            .person_id
            .ok_or_else(|| AppError::internal("approved quote missing person_id"))?;
        let person = people_map
            .get(&pid)
            .ok_or_else(|| AppError::internal(format!("person {pid} missing")))?;
        items.push(QuoteItem {
            id: q.id,
            person_id: pid,
            content: q.content,
            source: q.source,
            pinned: q.pinned,
            sort_order: q.sort_order,
            published_at: q.published_at,
            created_at: q.created_at,
            person: PersonBrief {
                id: person.id,
                name: person.name.clone(),
                avatar_url: AppState::avatar_url(&person.avatar_path),
            },
        });
    }

    Ok(PaginatedQuotes {
        items,
        page,
        page_size,
        total,
    })
}

pub async fn create_submission(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<SubmissionBody>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let content = normalize_quote_content(&body.content)?;
    let source = normalize_source(body.source)?;

    let settings = ensure_site_settings(&state).await?;
    let ip = state.config.client_ip(&headers, addr);

    let (person_id, proposed_person_name) = match (
        body.person_id,
        body.proposed_person_name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    ) {
        (Some(pid), None) => {
            let exists = persons::Entity::find_by_id(pid).one(&state.db).await?;
            if exists.is_none() {
                return Err(AppError::bad_request("神人不存在"));
            }
            (Some(pid), None)
        }
        (None, Some(name)) => {
            if !settings.allow_propose_person {
                return Err(AppError::bad_request("当前未开放新神人投稿"));
            }
            (None, Some(normalize_proposed_name(&name)?))
        }
        (Some(_), Some(_)) => {
            return Err(AppError::bad_request(
                "请选择已有神人或填写新神人名称，不能同时提交",
            ));
        }
        (None, None) => {
            return Err(AppError::bad_request("请选择神人或填写新神人名称"));
        }
    };

    crate::services::captcha::verify_submission_captcha(
        &settings,
        body.captcha.as_ref(),
        Some(ip),
    )
    .await?;

    if body.place_before_id.is_some() && body.place_after_id.is_some() {
        return Err(AppError::bad_request("只能指定排在某条前面或后面其中之一"));
    }
    if let Some(anchor_id) = body.place_before_id.or(body.place_after_id) {
        let anchor = quotes::Entity::find_by_id(anchor_id)
            .one(&state.db)
            .await?
            .ok_or_else(|| AppError::bad_request("参考言论不存在"))?;
        if anchor.status != quotes::status::APPROVED || anchor.pinned {
            return Err(AppError::bad_request("只能相对首页已展示的言论排序"));
        }
    }
    let now = Utc::now().fixed_offset();
    let published_at = match body.published_at.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => DateTime::parse_from_rfc3339(raw)
            .map_err(|_| AppError::bad_request("发布时间无效"))?,
        None => now,
    };
    let model = quotes::ActiveModel {
        person_id: Set(person_id),
        proposed_person_name: Set(proposed_person_name),
        content: Set(content),
        source: Set(source),
        status: Set(quotes::status::PENDING.to_string()),
        created_at: Set(now),
        pinned: Set(false),
        sort_order: Set(0),
        published_at: Set(published_at),
        place_before_id: Set(body.place_before_id),
        place_after_id: Set(body.place_after_id),
        reviewed_at: Set(None),
        reviewed_by: Set(None),
        ..Default::default()
    };
    let inserted = model.insert(&state.db).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": inserted.id,
            "status": "pending",
            "message": "投稿成功，请等待审核"
        })),
    ))
}

pub async fn ensure_site_settings(state: &AppState) -> AppResult<site_settings::Model> {
    if let Some(existing) = site_settings::Entity::find()
        .order_by_asc(site_settings::Column::Id)
        .one(&state.db)
        .await?
    {
        return Ok(existing);
    }

    let model = site_settings::ActiveModel {
        site_name: Set("神人网".to_string()),
        description: Set(Some("收录逆天言论".to_string())),
        logo_url: Set(None),
        footer: Set(None),
        allow_propose_person: Set(false),
        captcha_provider: Set("none".to_string()),
        captcha_site_key: Set(None),
        captcha_secret: Set(None),
        captcha_providers: Set(Some("[]".to_string())),
        ..Default::default()
    };
    Ok(model.insert(&state.db).await?)
}
