use axum::extract::{ConnectInfo, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::entities::{persons, quotes, site_settings};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Serialize)]
pub struct SiteResponse {
    pub site_name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub footer: Option<String>,
    pub allow_propose_person: bool,
}

#[derive(Serialize)]
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
}

#[derive(Deserialize)]
pub struct SubmissionBody {
    pub person_id: Option<i64>,
    pub proposed_person_name: Option<String>,
    pub content: String,
    pub source: Option<String>,
}

pub async fn get_site(State(state): State<AppState>) -> AppResult<Json<SiteResponse>> {
    let settings = ensure_site_settings(&state).await?;
    Ok(Json(SiteResponse {
        site_name: settings.site_name,
        description: settings.description,
        logo_url: settings.logo_url,
        footer: settings.footer,
        allow_propose_person: settings.allow_propose_person,
    }))
}

pub async fn list_persons(State(state): State<AppState>) -> AppResult<Json<Vec<PersonBrief>>> {
    let rows = persons::Entity::find()
        .order_by_asc(persons::Column::Name)
        .all(&state.db)
        .await?;
    let items = rows
        .into_iter()
        .map(|p| PersonBrief {
            id: p.id,
            name: p.name,
            avatar_url: AppState::avatar_url(&p.avatar_path),
        })
        .collect();
    Ok(Json(items))
}

pub async fn list_quotes(
    State(state): State<AppState>,
    Query(query): Query<QuotesQuery>,
) -> AppResult<Json<PaginatedQuotes>> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let paginator = quotes::Entity::find()
        .filter(quotes::Column::Status.eq(quotes::status::APPROVED))
        .filter(quotes::Column::PersonId.is_not_null())
        .order_by_desc(quotes::Column::CreatedAt)
        .paginate(&state.db, page_size);

    let total = paginator.num_items().await?;
    let rows = paginator.fetch_page(page - 1).await?;

    let mut items = Vec::with_capacity(rows.len());
    for q in rows {
        let person_id = q
            .person_id
            .ok_or_else(|| AppError::internal("approved quote missing person_id"))?;
        let person = persons::Entity::find_by_id(person_id)
            .one(&state.db)
            .await?
            .ok_or_else(|| AppError::internal(format!("person {person_id} missing")))?;
        items.push(QuoteItem {
            id: q.id,
            person_id,
            content: q.content,
            source: q.source,
            created_at: q.created_at,
            person: PersonBrief {
                id: person.id,
                name: person.name,
                avatar_url: AppState::avatar_url(&person.avatar_path),
            },
        });
    }

    Ok(Json(PaginatedQuotes {
        items,
        page,
        page_size,
        total,
    }))
}

pub async fn create_submission(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<SubmissionBody>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if !state.rate_limiter.check(addr.ip()) {
        return Err(AppError::TooManyRequests);
    }

    let content = body.content.trim().to_string();
    if content.is_empty() {
        return Err(AppError::bad_request("言论内容不能为空"));
    }
    if content.chars().count() > 2000 {
        return Err(AppError::bad_request("言论内容过长"));
    }

    let settings = ensure_site_settings(&state).await?;
    let source = body
        .source
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

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
            if name.chars().count() > 64 {
                return Err(AppError::bad_request("神人名称过长"));
            }
            (None, Some(name))
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

    let now = Utc::now().fixed_offset();
    let model = quotes::ActiveModel {
        person_id: Set(person_id),
        proposed_person_name: Set(proposed_person_name),
        content: Set(content),
        source: Set(source),
        status: Set(quotes::status::PENDING.to_string()),
        created_at: Set(now),
        reviewed_at: Set(None),
        reviewed_by: Set(None),
        ..Default::default()
    };
    let inserted = model.insert(&state.db).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": inserted.id, "status": "pending" })),
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
        ..Default::default()
    };
    Ok(model.insert(&state.db).await?)
}
