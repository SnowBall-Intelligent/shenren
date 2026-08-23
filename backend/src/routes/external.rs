use std::collections::HashMap;
use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Query, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, FixedOffset, Utc};
use rand_core::{OsRng, RngCore};
use sea_orm::sea_query::{Condition, Expr};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::entities::{api_keys, persons, quotes};
use crate::error::{AppError, AppResult};
use crate::routes::public::{PaginatedQuotes, PersonBrief, QuoteItem};
use crate::services::api_key::{
    api_key_hash_matches, api_key_prefix, ip_allowed, parse_string_list, source_domain_allowed,
    ApiKeyPermit, RateSnapshot,
};
use crate::services::quote_place::ordered_approved_quotes;
use crate::state::AppState;

const RATE_LIMIT_HEADER: HeaderName = HeaderName::from_static("x-ratelimit-limit");
const RATE_REMAINING_HEADER: HeaderName = HeaderName::from_static("x-ratelimit-remaining");
const RATE_RESET_HEADER: HeaderName = HeaderName::from_static("x-ratelimit-reset");
const QUOTA_LIMIT_HEADER: HeaderName = HeaderName::from_static("x-quota-limit");
const QUOTA_REMAINING_HEADER: HeaderName = HeaderName::from_static("x-quota-remaining");

#[derive(Deserialize)]
pub struct ExternalQuotesQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub person_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct ExternalRandomQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub person_id: Option<i64>,
}

struct AuthorizedKey {
    model: api_keys::Model,
    rate: RateSnapshot,
    _permit: ApiKeyPermit,
}

fn parse_range(
    from: Option<&str>,
    to: Option<&str>,
) -> AppResult<(Option<DateTime<FixedOffset>>, Option<DateTime<FixedOffset>>)> {
    let parse = |raw: &str, field: &str| {
        DateTime::parse_from_rfc3339(raw.trim())
            .map_err(|_| AppError::bad_request(format!("{field} 必须是 RFC3339 时间")))
    };
    let from = from.map(|raw| parse(raw, "from")).transpose()?;
    let to = to.map(|raw| parse(raw, "to")).transpose()?;
    if from.zip(to).is_some_and(|(from, to)| from > to) {
        return Err(AppError::bad_request("from 不能晚于 to"));
    }
    Ok((from, to))
}

fn reject_person_filter(person_id: Option<i64>) -> AppResult<()> {
    if person_id.is_some() {
        return Err(AppError::bad_request(
            "person_id 参数已预留，当前 API 版本暂不支持",
        ));
    }
    Ok(())
}

async fn authorize(
    state: &AppState,
    headers: &HeaderMap,
    peer: SocketAddr,
) -> AppResult<AuthorizedKey> {
    let raw = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            let mut parts = value.split_whitespace();
            match (parts.next(), parts.next(), parts.next()) {
                (Some(scheme), Some(token), None) if scheme.eq_ignore_ascii_case("bearer") => {
                    Some(token)
                }
                _ => None,
            }
        })
        .ok_or_else(|| AppError::unauthorized("缺少有效的 Bearer API Key"))?;
    let prefix =
        api_key_prefix(raw).ok_or_else(|| AppError::unauthorized("缺少有效的 Bearer API Key"))?;
    let model = api_keys::Entity::find()
        .filter(api_keys::Column::KeyPrefix.eq(prefix))
        .one(&state.db)
        .await?
        .filter(|key| api_key_hash_matches(&key.key_hash, raw))
        .ok_or_else(|| AppError::unauthorized("API Key 无效"))?;
    if !model.enabled {
        return Err(AppError::forbidden("API Key 已停用"));
    }

    let ip_rules = parse_string_list(&model.allowed_ips)?;
    let client_ip = state.config.client_ip(headers, peer);
    if !ip_allowed(client_ip, &ip_rules) {
        return Err(AppError::forbidden("当前 IP 不在允许范围内"));
    }
    let domain_rules = parse_string_list(&model.allowed_domains)?;
    if !source_domain_allowed(headers, &domain_rules) {
        return Err(AppError::forbidden("当前来源域名不在允许范围内"));
    }

    let (permit, rate) = state
        .api_key_limiters
        .check_and_acquire(
            model.id,
            model.rate_limit.and_then(|value| value.try_into().ok()),
            model
                .rate_window_secs
                .and_then(|value| value.try_into().ok()),
            model
                .concurrency_limit
                .and_then(|value| value.try_into().ok()),
        )
        .map_err(|error| AppError::ApiLimit {
            message: error.message.to_string(),
            retry_after: error.retry_after,
        })?;

    let quota_condition = Condition::any()
        .add(api_keys::Column::TotalQuota.is_null())
        .add(Expr::col(api_keys::Column::UsedCount).lt(Expr::col(api_keys::Column::TotalQuota)));
    let now = Utc::now().fixed_offset();
    let result = api_keys::Entity::update_many()
        .col_expr(
            api_keys::Column::UsedCount,
            Expr::col(api_keys::Column::UsedCount).add(1),
        )
        .col_expr(api_keys::Column::LastUsedAt, Expr::value(Some(now)))
        .filter(api_keys::Column::Id.eq(model.id))
        .filter(api_keys::Column::Enabled.eq(true))
        .filter(quota_condition)
        .exec(&state.db)
        .await?;
    if result.rows_affected == 0 {
        return Err(AppError::ApiLimit {
            message: "该 API Key 总额度已用尽".to_string(),
            retry_after: None,
        });
    }
    let model = api_keys::Entity::find_by_id(model.id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::unauthorized("API Key 无效"))?;
    Ok(AuthorizedKey {
        model,
        rate,
        _permit: permit,
    })
}

fn apply_limit_headers(response: &mut Response, auth: &AuthorizedKey) {
    let headers = response.headers_mut();
    let insert_number = |headers: &mut HeaderMap, name: HeaderName, value: u64| {
        if let Ok(value) = HeaderValue::from_str(&value.to_string()) {
            headers.insert(name, value);
        }
    };
    if let Some(limit) = auth.rate.limit {
        insert_number(headers, RATE_LIMIT_HEADER, limit);
    }
    if let Some(remaining) = auth.rate.remaining {
        insert_number(headers, RATE_REMAINING_HEADER, remaining);
    }
    if let Some(reset) = auth.rate.reset_after {
        insert_number(headers, RATE_RESET_HEADER, reset);
    }
    if let Some(limit) = auth
        .model
        .total_quota
        .and_then(|value| value.try_into().ok())
    {
        insert_number(headers, QUOTA_LIMIT_HEADER, limit);
        let used = u64::try_from(auth.model.used_count).unwrap_or_default();
        insert_number(headers, QUOTA_REMAINING_HEADER, limit.saturating_sub(used));
    }
}

async fn quote_items(state: &AppState, rows: &[quotes::Model]) -> AppResult<Vec<QuoteItem>> {
    let ids: Vec<i64> = rows.iter().filter_map(|quote| quote.person_id).collect();
    let people = if ids.is_empty() {
        Vec::new()
    } else {
        persons::Entity::find()
            .filter(persons::Column::Id.is_in(ids))
            .all(&state.db)
            .await?
    };
    let people: HashMap<i64, persons::Model> = people
        .into_iter()
        .map(|person| (person.id, person))
        .collect();
    rows.iter()
        .map(|quote| {
            let person_id = quote
                .person_id
                .ok_or_else(|| AppError::internal("approved quote missing person_id"))?;
            let person = people
                .get(&person_id)
                .ok_or_else(|| AppError::internal(format!("person {person_id} missing")))?;
            Ok(QuoteItem {
                id: quote.id.clone(),
                person_id,
                content: quote.content.clone(),
                source: quote.source.clone(),
                pinned: quote.pinned,
                published_at: quote.published_at,
                created_at: quote.created_at,
                person: PersonBrief {
                    id: person.id,
                    name: person.name.clone(),
                    avatar_url: AppState::avatar_url(&person.avatar_path),
                },
            })
        })
        .collect()
}

async fn filtered_quotes(
    state: &AppState,
    from: Option<DateTime<FixedOffset>>,
    to: Option<DateTime<FixedOffset>>,
) -> AppResult<Vec<quotes::Model>> {
    let rows = ordered_approved_quotes(&state.db, None, None, None).await?;
    Ok(rows
        .into_iter()
        .filter(|quote| from.is_none_or(|from| quote.published_at >= from))
        .filter(|quote| to.is_none_or(|to| quote.published_at <= to))
        .collect())
}

pub async fn list_quotes(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(query): Query<ExternalQuotesQuery>,
) -> AppResult<Response> {
    reject_person_filter(query.person_id)?;
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    if page == 0 || !(1..=100).contains(&page_size) {
        return Err(AppError::bad_request(
            "page 必须大于 0，page_size 必须在 1-100 之间",
        ));
    }
    let (from, to) = parse_range(query.from.as_deref(), query.to.as_deref())?;
    let auth = authorize(&state, &headers, peer).await?;
    let rows = filtered_quotes(&state, from, to).await?;
    let total = rows.len() as u64;
    let start = page
        .saturating_sub(1)
        .saturating_mul(page_size)
        .min(usize::MAX as u64) as usize;
    let end = start.saturating_add(page_size as usize).min(rows.len());
    let page_rows = if start < rows.len() {
        &rows[start..end]
    } else {
        &[]
    };
    let mut response = Json(PaginatedQuotes {
        items: quote_items(&state, page_rows).await?,
        page,
        page_size,
        total,
    })
    .into_response();
    apply_limit_headers(&mut response, &auth);
    Ok(response)
}

pub async fn random_quote(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(query): Query<ExternalRandomQuery>,
) -> AppResult<Response> {
    reject_person_filter(query.person_id)?;
    let (from, to) = parse_range(query.from.as_deref(), query.to.as_deref())?;
    let auth = authorize(&state, &headers, peer).await?;
    let rows = filtered_quotes(&state, from, to).await?;
    if rows.is_empty() {
        return Err(AppError::not_found("指定范围内没有语录"));
    }
    let index = (OsRng.next_u64() % rows.len() as u64) as usize;
    let item = quote_items(&state, &rows[index..=index])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::internal("random quote missing"))?;
    let mut response = Json(item).into_response();
    apply_limit_headers(&mut response, &auth);
    Ok(response)
}
