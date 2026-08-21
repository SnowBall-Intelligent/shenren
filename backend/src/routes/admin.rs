use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::entities::{admins, persons, quotes, site_settings};
use crate::error::{AppError, AppResult};
use crate::routes::public::{
    ensure_site_settings, normalize_quote_content, PersonBrief, SiteResponse,
};
use crate::services::sanitize::{
    normalize_person_name, normalize_site_name, normalize_site_text, normalize_source,
};
use crate::services::auth::{
    admin_count, find_admin_by_username, hash_password, require_admin, verify_password,
    SESSION_ADMIN_ID,
};
use crate::services::upload::{
    delete_avatar_file, generate_letter_avatar, is_letter_avatar, name_initial,
    parse_approve_multipart, parse_avatar_url, parse_person_multipart, resolve_new_avatar,
    AvatarFile,
};
use crate::state::AppState;

#[derive(Serialize)]
pub struct BootstrapStatus {
    pub needs_setup: bool,
    pub has_admins: bool,
}

#[derive(Deserialize)]
pub struct SetupBody {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AdminInfo {
    pub id: i64,
    pub username: String,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn admin_info(admin: &admins::Model, message: Option<&str>) -> AdminInfo {
    AdminInfo {
        id: admin.id,
        username: admin.username.clone(),
        created_at: admin.created_at,
        message: message.map(str::to_string),
    }
}

#[derive(Deserialize)]
pub struct CreateAdminBody {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct UpdateSettingsBody {
    #[serde(alias = "name")]
    pub site_name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub footer: Option<String>,
    pub allow_propose_person: bool,
}

#[derive(Deserialize)]
pub struct AdminQuotesQuery {
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Serialize)]
pub struct AdminQuoteItem {
    pub id: i64,
    pub person_id: Option<i64>,
    pub proposed_person_name: Option<String>,
    pub content: String,
    pub source: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub reviewed_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub reviewed_by: Option<i64>,
    pub person: Option<PersonBrief>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct PaginatedAdminQuotes {
    pub items: Vec<AdminQuoteItem>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

#[derive(Serialize)]
pub struct PersonAdminItem {
    pub id: i64,
    pub name: String,
    pub avatar_url: String,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
}

fn validate_credentials(username: &str, password: &str) -> AppResult<(String, String)> {
    let username = username.trim().to_string();
    if username.is_empty() || username.chars().count() > 64 {
        return Err(AppError::bad_request("用户名无效"));
    }
    if password.len() < 6 {
        return Err(AppError::bad_request("密码至少 6 位"));
    }
    if password.len() > 128 {
        return Err(AppError::bad_request("密码过长"));
    }
    Ok((username, password.to_string()))
}

pub async fn bootstrap_status(State(state): State<AppState>) -> AppResult<Json<BootstrapStatus>> {
    let count = admin_count(&state.db).await?;
    if count > 0 {
        return Err(AppError::unauthorized("未登录"));
    }
    Ok(Json(BootstrapStatus {
        needs_setup: true,
        has_admins: false,
    }))
}

pub async fn setup(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<SetupBody>,
) -> AppResult<(StatusCode, Json<AdminInfo>)> {
    let count = admin_count(&state.db).await?;
    if count > 0 {
        return Err(AppError::forbidden("已完成初始化，无法再次执行 setup"));
    }

    let (username, password) = validate_credentials(&body.username, &body.password)?;
    let password_hash = hash_password(&password)?;
    let now = Utc::now().fixed_offset();
    let admin = admins::ActiveModel {
        username: Set(username),
        password_hash: Set(password_hash),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    // Ensure default site settings exist after first admin.
    let _ = ensure_site_settings(&state).await?;

    session.cycle_id().await?;
    session.insert(SESSION_ADMIN_ID, admin.id).await?;

    Ok((
        StatusCode::CREATED,
        Json(admin_info(&admin, Some("初始化成功"))),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<LoginBody>,
) -> AppResult<Json<AdminInfo>> {
    let username = body.username.trim();
    let admin = find_admin_by_username(&state.db, username).await?;
    let password_ok = match &admin {
        Some(admin) => verify_password(&body.password, &admin.password_hash)?,
        None => {
            let _ = verify_password(&body.password, &state.dummy_password_hash);
            false
        }
    };
    let admin = match (admin, password_ok) {
        (Some(admin), true) => admin,
        _ => return Err(AppError::unauthorized("用户名或密码错误")),
    };

    session.cycle_id().await?;
    session.insert(SESSION_ADMIN_ID, admin.id).await?;

    Ok(Json(admin_info(&admin, Some("登录成功"))))
}

pub async fn logout(session: Session) -> AppResult<StatusCode> {
    session.flush().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn me(State(state): State<AppState>, session: Session) -> AppResult<Json<AdminInfo>> {
    let admin = require_admin(&session, &state.db).await?;
    Ok(Json(admin_info(&admin, None)))
}

pub async fn list_admins(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Json<Vec<AdminInfo>>> {
    let _ = require_admin(&session, &state.db).await?;
    let rows = admins::Entity::find()
        .order_by_asc(admins::Column::Id)
        .all(&state.db)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|a| admin_info(&a, None))
            .collect(),
    ))
}

pub async fn create_admin(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<CreateAdminBody>,
) -> AppResult<(StatusCode, Json<AdminInfo>)> {
    let _ = require_admin(&session, &state.db).await?;
    let (username, password) = validate_credentials(&body.username, &body.password)?;

    if find_admin_by_username(&state.db, &username)
        .await?
        .is_some()
    {
        return Err(AppError::conflict("用户名已存在"));
    }

    let password_hash = hash_password(&password)?;
    let now = Utc::now().fixed_offset();
    let admin = admins::ActiveModel {
        username: Set(username),
        password_hash: Set(password_hash),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(admin_info(&admin, Some("管理员已创建"))),
    ))
}

pub async fn delete_admin(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let current = require_admin(&session, &state.db).await?;
    let count = admin_count(&state.db).await?;
    if count <= 1 {
        return Err(AppError::forbidden("不能删除最后一个管理员"));
    }

    let target = admins::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("管理员不存在"))?;

    let deleting_self = current.id == id;
    let am: admins::ActiveModel = target.into();
    am.delete(&state.db).await?;
    if deleting_self {
        session.flush().await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_settings(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Json<SiteResponse>> {
    let _ = require_admin(&session, &state.db).await?;
    let settings = ensure_site_settings(&state).await?;
    Ok(Json(SiteResponse {
        site_name: settings.site_name,
        description: settings.description,
        logo_url: settings.logo_url,
        footer: settings.footer,
        allow_propose_person: settings.allow_propose_person,
        captcha: None,
        message: None,
    }))
}

pub async fn update_settings(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<UpdateSettingsBody>,
) -> AppResult<Json<SiteResponse>> {
    let _ = require_admin(&session, &state.db).await?;
    let settings = ensure_site_settings(&state).await?;

    let site_name = normalize_site_name(&body.site_name)?;
    let description = normalize_site_text(body.description)?;
    let footer = normalize_site_text(body.footer)?;
    let logo_url = match body.logo_url {
        Some(raw) => parse_avatar_url(&raw)?,
        None => None,
    };

    let mut am: site_settings::ActiveModel = settings.into();
    am.site_name = Set(site_name);
    am.description = Set(description);
    am.logo_url = Set(logo_url);
    am.footer = Set(footer);
    am.allow_propose_person = Set(body.allow_propose_person);
    let updated = am.update(&state.db).await?;
    state.cache.invalidate_site();

    Ok(Json(SiteResponse {
        site_name: updated.site_name,
        description: updated.description,
        logo_url: updated.logo_url,
        footer: updated.footer,
        allow_propose_person: updated.allow_propose_person,
        captcha: None,
        message: Some("已保存".to_string()),
    }))
}

#[derive(Serialize)]
pub struct CaptchaProviderOut {
    pub provider: String,
    pub site_key: Option<String>,
    pub secret: Option<String>,
}

#[derive(Serialize)]
pub struct CaptchaSettingsResponse {
    pub providers: Vec<CaptchaProviderOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Deserialize)]
pub struct CaptchaProviderIn {
    pub provider: String,
    pub site_key: Option<String>,
    pub secret: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateCaptchaBody {
    pub providers: Vec<CaptchaProviderIn>,
}

fn captcha_settings_response(
    settings: &site_settings::Model,
    message: Option<&str>,
) -> CaptchaSettingsResponse {
    let providers = crate::services::captcha::parse_providers(settings)
        .into_iter()
        .map(|item| CaptchaProviderOut {
            provider: item.provider,
            site_key: Some(item.site_key),
            secret: Some(item.secret),
        })
        .collect();
    CaptchaSettingsResponse {
        providers,
        message: message.map(str::to_string),
    }
}

pub async fn get_captcha(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Json<CaptchaSettingsResponse>> {
    let _ = require_admin(&session, &state.db).await?;
    let settings = ensure_site_settings(&state).await?;
    Ok(Json(captcha_settings_response(&settings, None)))
}

pub async fn update_captcha(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<UpdateCaptchaBody>,
) -> AppResult<Json<CaptchaSettingsResponse>> {
    let _ = require_admin(&session, &state.db).await?;
    let settings = ensure_site_settings(&state).await?;

    let providers = crate::services::captcha::normalize_provider_list(
        body.providers
            .into_iter()
            .map(|item| (item.provider, item.site_key, item.secret))
            .collect(),
    )?;
    let json = crate::services::captcha::serialize_providers(&providers)?;
    let (legacy_provider, legacy_site_key, legacy_secret) =
        crate::services::captcha::first_as_legacy(&providers);

    let mut am: site_settings::ActiveModel = settings.into();
    am.captcha_providers = Set(Some(json));
    am.captcha_provider = Set(legacy_provider);
    am.captcha_site_key = Set(legacy_site_key);
    am.captcha_secret = Set(legacy_secret);
    let updated = am.update(&state.db).await?;
    state.cache.invalidate_site();

    Ok(Json(captcha_settings_response(&updated, Some("已保存"))))
}

#[derive(Deserialize)]
pub struct AdminPersonsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Serialize)]
pub struct PaginatedPersons {
    pub items: Vec<PersonAdminItem>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

pub async fn list_persons_admin(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<AdminPersonsQuery>,
) -> AppResult<Json<PaginatedPersons>> {
    let _ = require_admin(&session, &state.db).await?;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let paginator = persons::Entity::find()
        .order_by_desc(persons::Column::CreatedAt)
        .paginate(&state.db, page_size);
    let total = paginator.num_items().await?;
    let rows = paginator.fetch_page(page - 1).await?;
    Ok(Json(PaginatedPersons {
        items: rows
            .into_iter()
            .map(|p| PersonAdminItem {
                id: p.id,
                name: p.name,
                avatar_url: AppState::avatar_url(&p.avatar_path),
                created_at: p.created_at,
            })
            .collect(),
        page,
        page_size,
        total,
    }))
}

async fn insert_person(
    state: &AppState,
    name: String,
    avatar: Option<AvatarFile>,
    avatar_url: Option<String>,
) -> AppResult<persons::Model> {
    let name = normalize_person_name(&name)?;
    let avatar_path = resolve_new_avatar(state.uploads_dir(), &name, avatar, avatar_url).await?;
    let now = Utc::now().fixed_offset();
    let person = persons::ActiveModel {
        name: Set(name),
        avatar_path: Set(avatar_path),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;
    state.cache.bust_public();
    Ok(person)
}

pub async fn create_person(
    State(state): State<AppState>,
    session: Session,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<PersonAdminItem>)> {
    let _ = require_admin(&session, &state.db).await?;
    let parsed = parse_person_multipart(multipart).await?;
    let person = insert_person(&state, parsed.name, parsed.avatar, parsed.avatar_url).await?;

    Ok((
        StatusCode::CREATED,
        Json(PersonAdminItem {
            id: person.id,
            name: person.name,
            avatar_url: AppState::avatar_url(&person.avatar_path),
            created_at: person.created_at,
        }),
    ))
}

pub async fn update_person(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    multipart: Multipart,
) -> AppResult<Json<PersonAdminItem>> {
    let _ = require_admin(&session, &state.db).await?;
    let person = persons::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("神人不存在"))?;

    let parsed = parse_person_multipart(multipart).await?;
    let name = normalize_person_name(&parsed.name)?;
    let old_name = person.name.clone();
    let old_avatar = person.avatar_path.clone();
    let mut am: persons::ActiveModel = person.into();
    am.name = Set(name.clone());

    if parsed.avatar.is_some() || parsed.avatar_url.is_some() {
        let avatar_path = resolve_new_avatar(
            state.uploads_dir(),
            &name,
            parsed.avatar,
            parsed.avatar_url,
        )
        .await?;
        am.avatar_path = Set(avatar_path);
        delete_avatar_file(state.uploads_dir(), &old_avatar);
    } else if is_letter_avatar(&old_avatar) && name_initial(&name) != name_initial(&old_name)
    {
        let avatar_path = generate_letter_avatar(state.uploads_dir(), &name).await?;
        am.avatar_path = Set(avatar_path);
        delete_avatar_file(state.uploads_dir(), &old_avatar);
    }

    let updated = am.update(&state.db).await?;
    state.cache.bust_public();
    Ok(Json(PersonAdminItem {
        id: updated.id,
        name: updated.name,
        avatar_url: AppState::avatar_url(&updated.avatar_path),
        created_at: updated.created_at,
    }))
}

pub async fn delete_person(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let _ = require_admin(&session, &state.db).await?;
    let person = persons::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("神人不存在"))?;

    // Unlink quotes before delete (FK is SET NULL, but be explicit for clarity).
    let linked = quotes::Entity::find()
        .filter(quotes::Column::PersonId.eq(id))
        .all(&state.db)
        .await?;
    for q in linked {
        let mut am: quotes::ActiveModel = q.into();
        am.person_id = Set(None);
        am.update(&state.db).await?;
    }

    let avatar_path = person.avatar_path.clone();
    let am: persons::ActiveModel = person.into();
    am.delete(&state.db).await?;
    delete_avatar_file(state.uploads_dir(), &avatar_path);
    state.cache.bust_public();
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct CreateQuoteBody {
    pub person_id: i64,
    pub content: String,
    pub source: Option<String>,
}

pub async fn create_quote(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<CreateQuoteBody>,
) -> AppResult<(StatusCode, Json<AdminQuoteItem>)> {
    let admin = require_admin(&session, &state.db).await?;

    let content = normalize_quote_content(&body.content)?;

    let person = persons::Entity::find_by_id(body.person_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::bad_request("神人不存在"))?;

    let source = normalize_source(body.source)?;

    let now = Utc::now().fixed_offset();
    let inserted = quotes::ActiveModel {
        person_id: Set(Some(person.id)),
        proposed_person_name: Set(None),
        content: Set(content),
        source: Set(source),
        status: Set(quotes::status::APPROVED.to_string()),
        created_at: Set(now),
        reviewed_at: Set(Some(now)),
        reviewed_by: Set(Some(admin.id)),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;
    state.cache.invalidate_quotes();

    Ok((
        StatusCode::CREATED,
        Json(AdminQuoteItem {
            id: inserted.id,
            person_id: inserted.person_id,
            proposed_person_name: inserted.proposed_person_name,
            content: inserted.content,
            source: inserted.source,
            status: inserted.status,
            created_at: inserted.created_at,
            reviewed_at: inserted.reviewed_at,
            reviewed_by: inserted.reviewed_by,
            person: Some(PersonBrief {
                id: person.id,
                name: person.name,
                avatar_url: AppState::avatar_url(&person.avatar_path),
            }),
            message: Some("语录已添加".to_string()),
        }),
    ))
}

pub async fn update_quote(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Json(body): Json<CreateQuoteBody>,
) -> AppResult<Json<AdminQuoteItem>> {
    let _ = require_admin(&session, &state.db).await?;
    let quote = quotes::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;

    let content = normalize_quote_content(&body.content)?;
    let person = persons::Entity::find_by_id(body.person_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::bad_request("神人不存在"))?;
    let source = normalize_source(body.source)?;

    let mut am: quotes::ActiveModel = quote.into();
    am.person_id = Set(Some(person.id));
    am.proposed_person_name = Set(None);
    am.content = Set(content);
    am.source = Set(source);
    let updated = am.update(&state.db).await?;
    state.cache.invalidate_quotes();

    Ok(Json(AdminQuoteItem {
        id: updated.id,
        person_id: updated.person_id,
        proposed_person_name: updated.proposed_person_name,
        content: updated.content,
        source: updated.source,
        status: updated.status,
        created_at: updated.created_at,
        reviewed_at: updated.reviewed_at,
        reviewed_by: updated.reviewed_by,
        person: Some(PersonBrief {
            id: person.id,
            name: person.name,
            avatar_url: AppState::avatar_url(&person.avatar_path),
        }),
        message: Some("语录已更新".to_string()),
    }))
}

pub async fn delete_quote(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let _ = require_admin(&session, &state.db).await?;
    let quote = quotes::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;
    quotes::Entity::delete_by_id(quote.id).exec(&state.db).await?;
    state.cache.invalidate_quotes();
    Ok(Json(serde_json::json!({ "message": "语录已删除" })))
}

pub async fn list_quotes_admin(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<AdminQuotesQuery>,
) -> AppResult<Json<PaginatedAdminQuotes>> {
    let _ = require_admin(&session, &state.db).await?;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let mut finder = quotes::Entity::find();
    if let Some(status) = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if status == "unapproved" {
            finder = finder.filter(
                quotes::Column::Status.is_in([
                    quotes::status::PENDING,
                    quotes::status::REJECTED,
                ]),
            );
        } else {
            finder = finder.filter(quotes::Column::Status.eq(status));
        }
    }

    // Pending first when no status filter, then newest.
    let paginator = if query.status.is_none() {
        finder
            .order_by_asc(quotes::Column::Status)
            .order_by_desc(quotes::Column::CreatedAt)
            .paginate(&state.db, page_size)
    } else {
        finder
            .order_by_desc(quotes::Column::CreatedAt)
            .paginate(&state.db, page_size)
    };

    let total = paginator.num_items().await?;
    let rows = paginator.fetch_page(page - 1).await?;

    let mut items = Vec::with_capacity(rows.len());
    for q in rows {
        let person = if let Some(pid) = q.person_id {
            persons::Entity::find_by_id(pid)
                .one(&state.db)
                .await?
                .map(|p| PersonBrief {
                    id: p.id,
                    name: p.name,
                    avatar_url: AppState::avatar_url(&p.avatar_path),
                })
        } else {
            None
        };
        items.push(AdminQuoteItem {
            id: q.id,
            person_id: q.person_id,
            proposed_person_name: q.proposed_person_name,
            content: q.content,
            source: q.source,
            status: q.status,
            created_at: q.created_at,
            reviewed_at: q.reviewed_at,
            reviewed_by: q.reviewed_by,
            person,
            message: None,
        });
    }

    Ok(Json(PaginatedAdminQuotes {
        items,
        page,
        page_size,
        total,
    }))
}

pub async fn approve_quote(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    multipart: Multipart,
) -> AppResult<Json<AdminQuoteItem>> {
    let admin = require_admin(&session, &state.db).await?;
    let quote = quotes::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;

    if quote.status != quotes::status::PENDING {
        return Err(AppError::bad_request("只能审核待处理的言论"));
    }

    let parsed = parse_approve_multipart(multipart).await?;
    let mut person_id = quote.person_id;

    if person_id.is_none() {
        if let Some(pid) = parsed.person_id {
            let exists = persons::Entity::find_by_id(pid).one(&state.db).await?;
            if exists.is_none() {
                return Err(AppError::bad_request("绑定的神人不存在"));
            }
            person_id = Some(pid);
        } else {
            let name = parsed
                .create_person_name
                .or_else(|| {
                    quote
                        .proposed_person_name
                        .clone()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                })
                .ok_or_else(|| AppError::bad_request("缺少神人信息，无法通过审核"))?;
            let person = insert_person(&state, name, parsed.avatar, parsed.avatar_url).await?;
            person_id = Some(person.id);
        }
    }

    let now = Utc::now().fixed_offset();
    let mut am: quotes::ActiveModel = quote.into();
    am.person_id = Set(person_id);
    am.status = Set(quotes::status::APPROVED.to_string());
    am.reviewed_at = Set(Some(now));
    am.reviewed_by = Set(Some(admin.id));
    let updated = am.update(&state.db).await?;
    state.cache.invalidate_quotes();

    let person = if let Some(pid) = updated.person_id {
        persons::Entity::find_by_id(pid)
            .one(&state.db)
            .await?
            .map(|p| PersonBrief {
                id: p.id,
                name: p.name,
                avatar_url: AppState::avatar_url(&p.avatar_path),
            })
    } else {
        None
    };

    Ok(Json(AdminQuoteItem {
        id: updated.id,
        person_id: updated.person_id,
        proposed_person_name: updated.proposed_person_name,
        content: updated.content,
        source: updated.source,
        status: updated.status,
        created_at: updated.created_at,
        reviewed_at: updated.reviewed_at,
        reviewed_by: updated.reviewed_by,
        person,
        message: None,
    }))
}

/// JSON approve: bind an existing person, or create one (optional avatar URL).
#[derive(Deserialize)]
pub struct ApproveJsonBody {
    pub person_id: Option<i64>,
    pub create_person_name: Option<String>,
    pub avatar_url: Option<String>,
}

pub async fn approve_quote_json(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    body: Option<Json<ApproveJsonBody>>,
) -> AppResult<Json<AdminQuoteItem>> {
    let admin = require_admin(&session, &state.db).await?;
    let quote = quotes::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;

    if quote.status != quotes::status::PENDING {
        return Err(AppError::bad_request("只能审核待处理的言论"));
    }

    let body = body.map(|b| b.0);
    let bind = body.as_ref().and_then(|b| b.person_id);
    let mut person_id = quote.person_id;

    if person_id.is_none() {
        if let Some(pid) = bind {
            let exists = persons::Entity::find_by_id(pid).one(&state.db).await?;
            if exists.is_none() {
                return Err(AppError::bad_request("绑定的神人不存在"));
            }
            person_id = Some(pid);
        } else {
            let name = body
                .as_ref()
                .and_then(|b| b.create_person_name.clone())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    quote
                        .proposed_person_name
                        .clone()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                })
                .ok_or_else(|| AppError::bad_request("缺少神人信息，无法通过审核"))?;
            let avatar_url = match body.as_ref().and_then(|b| b.avatar_url.as_deref()) {
                Some(raw) => parse_avatar_url(raw)?,
                None => None,
            };
            let person = insert_person(&state, name, None, avatar_url).await?;
            person_id = Some(person.id);
        }
    }

    let now = Utc::now().fixed_offset();
    let mut am: quotes::ActiveModel = quote.into();
    am.person_id = Set(person_id);
    am.status = Set(quotes::status::APPROVED.to_string());
    am.reviewed_at = Set(Some(now));
    am.reviewed_by = Set(Some(admin.id));
    let updated = am.update(&state.db).await?;
    state.cache.invalidate_quotes();

    let person = if let Some(pid) = updated.person_id {
        persons::Entity::find_by_id(pid)
            .one(&state.db)
            .await?
            .map(|p| PersonBrief {
                id: p.id,
                name: p.name,
                avatar_url: AppState::avatar_url(&p.avatar_path),
            })
    } else {
        None
    };

    Ok(Json(AdminQuoteItem {
        id: updated.id,
        person_id: updated.person_id,
        proposed_person_name: updated.proposed_person_name,
        content: updated.content,
        source: updated.source,
        status: updated.status,
        created_at: updated.created_at,
        reviewed_at: updated.reviewed_at,
        reviewed_by: updated.reviewed_by,
        person,
        message: None,
    }))
}

pub async fn reject_quote(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Json<AdminQuoteItem>> {
    let admin = require_admin(&session, &state.db).await?;
    let quote = quotes::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;

    if quote.status != quotes::status::PENDING {
        return Err(AppError::bad_request("只能审核待处理的言论"));
    }

    let now = Utc::now().fixed_offset();
    let mut am: quotes::ActiveModel = quote.into();
    am.status = Set(quotes::status::REJECTED.to_string());
    am.reviewed_at = Set(Some(now));
    am.reviewed_by = Set(Some(admin.id));
    let updated = am.update(&state.db).await?;
    state.cache.invalidate_quotes();

    let person = if let Some(pid) = updated.person_id {
        persons::Entity::find_by_id(pid)
            .one(&state.db)
            .await?
            .map(|p| PersonBrief {
                id: p.id,
                name: p.name,
                avatar_url: AppState::avatar_url(&p.avatar_path),
            })
    } else {
        None
    };

    Ok(Json(AdminQuoteItem {
        id: updated.id,
        person_id: updated.person_id,
        proposed_person_name: updated.proposed_person_name,
        content: updated.content,
        source: updated.source,
        status: updated.status,
        created_at: updated.created_at,
        reviewed_at: updated.reviewed_at,
        reviewed_by: updated.reviewed_by,
        person,
        message: None,
    }))
}
