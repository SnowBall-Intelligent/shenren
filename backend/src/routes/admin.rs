use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Extension, Multipart, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{
    AccessMode, ActiveModelTrait, ColumnTrait, EntityTrait, IsolationLevel, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::entities::{admins, persons, quotes, site_settings};
use crate::error::{AppError, AppResult};
use crate::logging::AuditContext;
use crate::routes::public::{
    ensure_site_settings, normalize_quote_content, public_captcha, PersonBrief, PublicCaptcha,
    SiteResponse,
};
use crate::services::auth::{
    admin_count, find_admin_by_username, hash_password, require_admin, require_super_admin,
    super_admin_count, verify_password, ROLE_ADMIN, ROLE_SUPER_ADMIN, SESSION_ADMIN_ID,
};
use crate::services::captcha::CaptchaPayload;
use crate::services::quote_place::{
    move_in_chain, new_quote_id, on_pinned_changed, ordered_approved_quotes, place_quote,
    quote_search_condition, remove_from_chain, reorder_approved,
};
use crate::services::sanitize::{
    normalize_person_name, normalize_site_name, normalize_site_text, normalize_source,
};
use crate::services::upload::{
    delete_avatar_file, generate_letter_avatar, is_letter_avatar, name_initial,
    parse_approve_multipart, parse_avatar_url, parse_person_multipart, qq_avatar_url,
    resolve_new_avatar, AvatarFile,
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
    pub role: String,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn admin_info(admin: &admins::Model, message: Option<&str>) -> AdminInfo {
    AdminInfo {
        id: admin.id,
        username: admin.username.clone(),
        role: admin.role.clone(),
        created_at: admin.created_at,
        message: message.map(str::to_string),
    }
}

#[derive(Serialize)]
pub struct AdminSelfInfo {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captcha: Option<PublicCaptcha>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn admin_self_info(
    admin: &admins::Model,
    settings: &site_settings::Model,
    message: Option<&str>,
) -> AdminSelfInfo {
    AdminSelfInfo {
        id: admin.id,
        username: admin.username.clone(),
        role: admin.role.clone(),
        created_at: admin.created_at,
        captcha: settings
            .captcha_admin_account_enabled
            .then(|| public_captcha(settings)),
        message: message.map(str::to_string),
    }
}

#[derive(Deserialize)]
pub struct CreateAdminBody {
    pub username: String,
    pub password: String,
    pub role: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateAdminRoleBody {
    pub role: String,
}

#[derive(Deserialize)]
pub struct UpdateMeBody {
    pub username: String,
    pub current_password: String,
    pub new_password: Option<String>,
    pub captcha: Option<CaptchaPayload>,
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
    pub q: Option<String>,
    pub pinned: Option<bool>,
    pub recent: Option<bool>,
}

#[derive(Serialize)]
pub struct AdminQuoteItem {
    pub id: String,
    pub person_id: Option<i64>,
    pub proposed_person_name: Option<String>,
    pub proposed_person_avatar_url: Option<String>,
    pub content: String,
    pub source: Option<String>,
    pub status: String,
    pub pinned: bool,
    pub published_at: chrono::DateTime<chrono::FixedOffset>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub reviewed_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub reviewed_by: Option<i64>,
    pub person: Option<PersonBrief>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

fn admin_quote_item(
    q: quotes::Model,
    person: Option<PersonBrief>,
    message: Option<&str>,
) -> AdminQuoteItem {
    AdminQuoteItem {
        id: q.id,
        person_id: q.person_id,
        proposed_person_name: q.proposed_person_name,
        proposed_person_avatar_url: q.proposed_person_avatar_url,
        content: q.content,
        source: q.source,
        status: q.status,
        pinned: q.pinned,
        published_at: q.published_at,
        created_at: q.created_at,
        reviewed_at: q.reviewed_at,
        reviewed_by: q.reviewed_by,
        person,
        message: message.map(str::to_string),
    }
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

fn validate_admin_username(username: &str) -> AppResult<String> {
    let username = username.trim().to_string();
    if username.is_empty() || username.chars().count() > 64 {
        return Err(AppError::bad_request("用户名无效"));
    }
    Ok(username)
}

fn validate_admin_password(password: &str) -> AppResult<String> {
    if password.len() < 6 {
        return Err(AppError::bad_request("密码至少 6 位"));
    }
    if password.len() > 128 {
        return Err(AppError::bad_request("密码过长"));
    }
    Ok(password.to_string())
}

fn validate_credentials(username: &str, password: &str) -> AppResult<(String, String)> {
    Ok((
        validate_admin_username(username)?,
        validate_admin_password(password)?,
    ))
}

fn validate_admin_role(role: Option<&str>) -> AppResult<String> {
    match role.unwrap_or(ROLE_ADMIN) {
        ROLE_SUPER_ADMIN => Ok(ROLE_SUPER_ADMIN.to_string()),
        ROLE_ADMIN => Ok(ROLE_ADMIN.to_string()),
        _ => Err(AppError::bad_request("角色无效")),
    }
}

pub async fn bootstrap_status(State(state): State<AppState>) -> AppResult<Json<BootstrapStatus>> {
    let count = admin_count(&state.db).await?;
    Ok(Json(BootstrapStatus {
        needs_setup: count == 0,
        has_admins: count > 0,
    }))
}

pub async fn setup(
    State(state): State<AppState>,
    session: Session,
    Extension(audit): Extension<AuditContext>,
    Json(body): Json<SetupBody>,
) -> AppResult<(StatusCode, Json<AdminInfo>)> {
    audit.set_username(&body.username);
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
        role: Set(ROLE_SUPER_ADMIN.to_string()),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;
    audit.set_resource_id(admin.id);

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
    Extension(audit): Extension<AuditContext>,
    Json(body): Json<LoginBody>,
) -> AppResult<Json<AdminInfo>> {
    audit.set_username(&body.username);
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

pub async fn me(State(state): State<AppState>, session: Session) -> AppResult<Json<AdminSelfInfo>> {
    let admin = require_admin(&session, &state.db).await?;
    let settings = ensure_site_settings(&state).await?;
    Ok(Json(admin_self_info(&admin, &settings, None)))
}

pub async fn update_me(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    session: Session,
    Extension(audit): Extension<AuditContext>,
    Json(body): Json<UpdateMeBody>,
) -> AppResult<Json<AdminSelfInfo>> {
    let admin = require_admin(&session, &state.db).await?;
    let username = validate_admin_username(&body.username)?;
    let new_password = body
        .new_password
        .as_deref()
        .map(validate_admin_password)
        .transpose()?;

    if username == admin.username && new_password.is_none() {
        return Err(AppError::bad_request("用户名或密码至少修改一项"));
    }
    if body.current_password.is_empty()
        || !verify_password(&body.current_password, &admin.password_hash)?
    {
        return Err(AppError::bad_request("当前密码错误"));
    }

    let settings = ensure_site_settings(&state).await?;
    if settings.captcha_admin_account_enabled {
        if crate::services::captcha::parse_providers(&settings).is_empty() {
            return Err(AppError::internal("账号修改人机验证已开启但未配置验证厂商"));
        }
        let ip = state.config.client_ip(&headers, addr);
        crate::services::captcha::verify_captcha(&settings, body.captcha.as_ref(), Some(ip))
            .await?;
    }

    if let Some(existing) = find_admin_by_username(&state.db, &username).await? {
        if existing.id != admin.id {
            return Err(AppError::conflict("用户名已存在"));
        }
    }

    let mut active: admins::ActiveModel = admin.into();
    active.username = Set(username);
    if let Some(password) = new_password {
        active.password_hash = Set(hash_password(&password)?);
    }
    let updated = active.update(&state.db).await?;
    audit.set_resource_id(updated.id);

    Ok(Json(admin_self_info(
        &updated,
        &settings,
        Some("账号已更新"),
    )))
}

pub async fn list_admins(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Json<Vec<AdminInfo>>> {
    let _ = require_super_admin(&session, &state.db).await?;
    let rows = admins::Entity::find()
        .order_by_asc(admins::Column::Id)
        .all(&state.db)
        .await?;
    Ok(Json(
        rows.into_iter().map(|a| admin_info(&a, None)).collect(),
    ))
}

pub async fn create_admin(
    State(state): State<AppState>,
    session: Session,
    Extension(audit): Extension<AuditContext>,
    Json(body): Json<CreateAdminBody>,
) -> AppResult<(StatusCode, Json<AdminInfo>)> {
    let _ = require_super_admin(&session, &state.db).await?;
    let (username, password) = validate_credentials(&body.username, &body.password)?;
    let role = validate_admin_role(body.role.as_deref())?;

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
        role: Set(role),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;
    audit.set_resource_id(admin.id);

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
    let transaction = state
        .db
        .begin_with_config(
            Some(IsolationLevel::Serializable),
            Some(AccessMode::ReadWrite),
        )
        .await?;
    let current = require_super_admin(&session, &transaction).await?;
    if current.id == id {
        return Err(AppError::forbidden("不能删除自己的账号"));
    }

    let target = admins::Entity::find_by_id(id)
        .one(&transaction)
        .await?
        .ok_or_else(|| AppError::not_found("管理员不存在"))?;

    if target.role == ROLE_SUPER_ADMIN && super_admin_count(&transaction).await? <= 1 {
        return Err(AppError::forbidden("不能删除最后一名超级管理员"));
    }

    let am: admins::ActiveModel = target.into();
    am.delete(&transaction).await?;
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_admin_role(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
    Json(body): Json<UpdateAdminRoleBody>,
) -> AppResult<Json<AdminInfo>> {
    let transaction = state
        .db
        .begin_with_config(
            Some(IsolationLevel::Serializable),
            Some(AccessMode::ReadWrite),
        )
        .await?;
    let current = require_super_admin(&session, &transaction).await?;
    if current.id == id {
        return Err(AppError::forbidden("不能修改自己的角色"));
    }
    let role = validate_admin_role(Some(&body.role))?;
    let target = admins::Entity::find_by_id(id)
        .one(&transaction)
        .await?
        .ok_or_else(|| AppError::not_found("管理员不存在"))?;

    if target.role == ROLE_SUPER_ADMIN
        && role != ROLE_SUPER_ADMIN
        && super_admin_count(&transaction).await? <= 1
    {
        return Err(AppError::forbidden("不能降级最后一名超级管理员"));
    }

    if target.role == role {
        transaction.commit().await?;
        return Ok(Json(admin_info(&target, None)));
    }

    let mut active: admins::ActiveModel = target.into();
    active.role = Set(role);
    let updated = active.update(&transaction).await?;
    transaction.commit().await?;
    Ok(Json(admin_info(&updated, Some("角色已更新"))))
}

pub async fn get_settings(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Json<SiteResponse>> {
    let _ = require_super_admin(&session, &state.db).await?;
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
    let _ = require_super_admin(&session, &state.db).await?;
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
    pub account_update_enabled: bool,
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
    #[serde(default)]
    pub account_update_enabled: bool,
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
        account_update_enabled: settings.captcha_admin_account_enabled,
        message: message.map(str::to_string),
    }
}

pub async fn get_captcha(
    State(state): State<AppState>,
    session: Session,
) -> AppResult<Json<CaptchaSettingsResponse>> {
    let _ = require_super_admin(&session, &state.db).await?;
    let settings = ensure_site_settings(&state).await?;
    Ok(Json(captcha_settings_response(&settings, None)))
}

pub async fn update_captcha(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<UpdateCaptchaBody>,
) -> AppResult<Json<CaptchaSettingsResponse>> {
    let _ = require_super_admin(&session, &state.db).await?;
    let settings = ensure_site_settings(&state).await?;

    let providers = crate::services::captcha::normalize_provider_list(
        body.providers
            .into_iter()
            .map(|item| (item.provider, item.site_key, item.secret))
            .collect(),
    )?;
    if body.account_update_enabled && providers.is_empty() {
        return Err(AppError::bad_request(
            "启用账号修改验证前，请先配置至少一个验证厂商",
        ));
    }
    let json = crate::services::captcha::serialize_providers(&providers)?;
    let (legacy_provider, legacy_site_key, legacy_secret) =
        crate::services::captcha::first_as_legacy(&providers);

    let mut am: site_settings::ActiveModel = settings.into();
    am.captcha_providers = Set(Some(json));
    am.captcha_provider = Set(legacy_provider);
    am.captcha_site_key = Set(legacy_site_key);
    am.captcha_secret = Set(legacy_secret);
    am.captcha_admin_account_enabled = Set(body.account_update_enabled);
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
    qq_avatar_url: Option<String>,
    avatar_url: Option<String>,
) -> AppResult<persons::Model> {
    let name = normalize_person_name(&name)?;
    let avatar_path = resolve_new_avatar(
        state.uploads_dir(),
        &name,
        avatar,
        qq_avatar_url,
        avatar_url,
    )
    .await?;
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
    Extension(audit): Extension<AuditContext>,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<PersonAdminItem>)> {
    let _ = require_admin(&session, &state.db).await?;
    let parsed = parse_person_multipart(multipart).await?;
    let person = insert_person(
        &state,
        parsed.name,
        parsed.avatar,
        parsed.qq_avatar_url,
        parsed.avatar_url,
    )
    .await?;
    audit.set_resource_id(person.id);

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

    if parsed.avatar.is_some() || parsed.qq_avatar_url.is_some() || parsed.avatar_url.is_some() {
        let avatar_path = resolve_new_avatar(
            state.uploads_dir(),
            &name,
            parsed.avatar,
            parsed.qq_avatar_url,
            parsed.avatar_url,
        )
        .await?;
        am.avatar_path = Set(avatar_path);
        delete_avatar_file(state.uploads_dir(), &old_avatar);
    } else if is_letter_avatar(&old_avatar) && name_initial(&name) != name_initial(&old_name) {
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
    pub pinned: Option<bool>,
    pub published_at: Option<DateTime<FixedOffset>>,
    pub place_before_id: Option<String>,
    pub place_after_id: Option<String>,
}

#[derive(Deserialize)]
pub struct MoveQuoteBody {
    pub direction: String,
}

fn placement_from_body(
    body: &CreateQuoteBody,
    now: DateTime<FixedOffset>,
) -> (bool, DateTime<FixedOffset>) {
    (
        body.pinned.unwrap_or(false),
        body.published_at.unwrap_or(now),
    )
}

pub async fn create_quote(
    State(state): State<AppState>,
    session: Session,
    Extension(audit): Extension<AuditContext>,
    Json(body): Json<CreateQuoteBody>,
) -> AppResult<(StatusCode, Json<AdminQuoteItem>)> {
    let admin = require_admin(&session, &state.db).await?;

    let content = normalize_quote_content(&body.content)?;

    let person = persons::Entity::find_by_id(body.person_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::bad_request("神人不存在"))?;

    let now = Utc::now().fixed_offset();
    let (pinned, published_at) = placement_from_body(&body, now);
    let source = normalize_source(body.source)?;
    let quote_id = new_quote_id();
    let inserted = quotes::ActiveModel {
        id: Set(quote_id.clone()),
        person_id: Set(Some(person.id)),
        proposed_person_name: Set(None),
        proposed_person_avatar_url: Set(None),
        content: Set(content),
        source: Set(source),
        status: Set(quotes::status::APPROVED.to_string()),
        pinned: Set(pinned),
        published_at: Set(published_at),
        created_at: Set(now),
        reviewed_at: Set(Some(now)),
        reviewed_by: Set(Some(admin.id)),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;
    audit.set_resource_id(&quote_id);
    place_quote(
        &state.db,
        &quote_id,
        inserted.pinned,
        body.place_before_id.clone(),
        body.place_after_id.clone(),
    )
    .await?;
    let inserted = quotes::Entity::find_by_id(inserted.id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::internal("语录写入后丢失"))?;
    state.cache.invalidate_quotes();

    Ok((
        StatusCode::CREATED,
        Json(admin_quote_item(
            inserted,
            Some(PersonBrief {
                id: person.id,
                name: person.name,
                avatar_url: AppState::avatar_url(&person.avatar_path),
            }),
            Some("语录已添加"),
        )),
    ))
}

pub async fn update_quote(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
    Json(body): Json<CreateQuoteBody>,
) -> AppResult<Json<AdminQuoteItem>> {
    let _ = require_admin(&session, &state.db).await?;
    let quote = quotes::Entity::find_by_id(id.clone())
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;

    let content = normalize_quote_content(&body.content)?;
    let person = persons::Entity::find_by_id(body.person_id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::bad_request("神人不存在"))?;
    let source = normalize_source(body.source)?;
    let pinned = body.pinned.unwrap_or(quote.pinned);
    let published_at = body.published_at.unwrap_or(quote.published_at);
    let pinned_changed = pinned != quote.pinned;
    let time_changed = published_at != quote.published_at;
    let has_anchor = body.place_before_id.is_some() || body.place_after_id.is_some();

    let mut am: quotes::ActiveModel = quote.clone().into();
    am.person_id = Set(Some(person.id));
    am.proposed_person_name = Set(None);
    am.proposed_person_avatar_url = Set(None);
    am.content = Set(content);
    am.source = Set(source);
    am.pinned = Set(pinned);
    am.published_at = Set(published_at);
    let updated = am.update(&state.db).await?;

    if updated.status == quotes::status::APPROVED {
        if pinned_changed {
            on_pinned_changed(
                &state.db,
                &updated.id,
                pinned,
                body.place_before_id.clone(),
                body.place_after_id.clone(),
            )
            .await?;
        } else if has_anchor {
            place_quote(
                &state.db,
                &updated.id,
                pinned,
                body.place_before_id.clone(),
                body.place_after_id.clone(),
            )
            .await?;
        } else if time_changed {
            remove_from_chain(&state.db, &updated).await?;
            crate::services::quote_place::insert_by_time(&state.db, &updated.id).await?;
        }
    }
    let updated = quotes::Entity::find_by_id(updated.id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::internal("语录更新后丢失"))?;
    state.cache.invalidate_quotes();

    Ok(Json(admin_quote_item(
        updated,
        Some(PersonBrief {
            id: person.id,
            name: person.name,
            avatar_url: AppState::avatar_url(&person.avatar_path),
        }),
        Some("语录已更新"),
    )))
}

pub async fn delete_quote(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let _ = require_admin(&session, &state.db).await?;
    let quote = quotes::Entity::find_by_id(id.clone())
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;
    remove_from_chain(&state.db, &quote).await?;
    quotes::Entity::delete_by_id(quote.id)
        .exec(&state.db)
        .await?;
    state.cache.invalidate_quotes();
    Ok(Json(serde_json::json!({ "message": "语录已删除" })))
}

#[derive(Deserialize)]
pub struct ReorderQuotesBody {
    pub ids: Vec<String>,
}

pub async fn reorder_quotes(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<ReorderQuotesBody>,
) -> AppResult<Json<serde_json::Value>> {
    let _ = require_admin(&session, &state.db).await?;
    reorder_approved(&state.db, &body.ids).await?;
    state.cache.invalidate_quotes();
    Ok(Json(serde_json::json!({ "message": "顺序已更新" })))
}

pub async fn move_quote(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
    Json(body): Json<MoveQuoteBody>,
) -> AppResult<Json<AdminQuoteItem>> {
    let _ = require_admin(&session, &state.db).await?;
    let quote = quotes::Entity::find_by_id(id.clone())
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("言论不存在"))?;

    if quote.status != quotes::status::APPROVED {
        return Err(AppError::bad_request("只能调整已通过的言论"));
    }

    let up = match body.direction.as_str() {
        "up" => true,
        "down" => false,
        _ => return Err(AppError::bad_request("direction 须为 up 或 down")),
    };

    move_in_chain(&state.db, &id, up).await?;
    let updated = quotes::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::internal("语录移动后丢失"))?;
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

    Ok(Json(admin_quote_item(updated, person, Some("顺序已更新"))))
}

pub async fn list_quotes_admin(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<AdminQuotesQuery>,
) -> AppResult<Json<PaginatedAdminQuotes>> {
    let _ = require_admin(&session, &state.db).await?;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

    let mut finder = quotes::Entity::find();
    if let Some(status) = query
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if status == "unapproved" {
            finder = finder.filter(
                quotes::Column::Status.is_in([quotes::status::PENDING, quotes::status::REJECTED]),
            );
        } else {
            finder = finder.filter(quotes::Column::Status.eq(status));
        }
    }
    if let Some(pinned) = query.pinned {
        finder = finder.filter(quotes::Column::Pinned.eq(pinned));
    }
    if let Some(q) = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        finder = finder.filter(quote_search_condition(&state.db, q).await?);
    }

    let approved = query.status.as_deref() == Some("approved");
    let recent = query.recent.unwrap_or(false);

    let (total, rows) = if approved && !recent {
        let q = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let ordered = ordered_approved_quotes(&state.db, None, q, query.pinned).await?;
        let total = ordered.len() as u64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(ordered.len());
        let slice = if start < ordered.len() {
            ordered[start..end].to_vec()
        } else {
            Vec::new()
        };
        (total, slice)
    } else {
        let paginator = if recent {
            finder
                .order_by_desc(quotes::Column::PublishedAt)
                .order_by_desc(quotes::Column::Id)
                .paginate(&state.db, page_size)
        } else if query.status.is_none() {
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
        (total, rows)
    };

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
        items.push(admin_quote_item(q, person, None));
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
    Path(id): Path<String>,
    multipart: Multipart,
) -> AppResult<Json<AdminQuoteItem>> {
    let admin = require_admin(&session, &state.db).await?;
    let quote = quotes::Entity::find_by_id(id.clone())
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
            let fallback_avatar_url = if parsed.avatar.is_none()
                && parsed.qq_avatar_url.is_none()
                && parsed.avatar_url.is_none()
            {
                quote.proposed_person_avatar_url.clone()
            } else {
                parsed.avatar_url
            };
            let person = insert_person(
                &state,
                name,
                parsed.avatar,
                parsed.qq_avatar_url,
                fallback_avatar_url,
            )
            .await?;
            person_id = Some(person.id);
        }
    }

    let now = Utc::now().fixed_offset();
    let intent_before = quote.place_before_id.clone();
    let intent_after = quote.place_after_id.clone();
    let pinned = quote.pinned;
    let mut am: quotes::ActiveModel = quote.into();
    am.person_id = Set(person_id);
    am.proposed_person_avatar_url = Set(None);
    am.status = Set(quotes::status::APPROVED.to_string());
    am.reviewed_at = Set(Some(now));
    am.reviewed_by = Set(Some(admin.id));
    let updated = am.update(&state.db).await?;
    place_quote(&state.db, &updated.id, pinned, intent_before, intent_after).await?;
    let updated = quotes::Entity::find_by_id(updated.id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::internal("审核后丢失"))?;
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

    Ok(Json(admin_quote_item(updated, person, None)))
}

/// JSON approve: bind an existing person, or create one (optional avatar URL).
#[derive(Deserialize)]
pub struct ApproveJsonBody {
    pub person_id: Option<i64>,
    pub create_person_name: Option<String>,
    pub qq: Option<String>,
    pub avatar_url: Option<String>,
}

pub async fn approve_quote_json(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
    body: Option<Json<ApproveJsonBody>>,
) -> AppResult<Json<AdminQuoteItem>> {
    let admin = require_admin(&session, &state.db).await?;
    let quote = quotes::Entity::find_by_id(id.clone())
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
                None => quote.proposed_person_avatar_url.clone(),
            };
            let qq_url = match body.as_ref().and_then(|b| b.qq.as_deref()) {
                Some(raw) => qq_avatar_url(raw)?,
                None => None,
            };
            let person = insert_person(&state, name, None, qq_url, avatar_url).await?;
            person_id = Some(person.id);
        }
    }

    let now = Utc::now().fixed_offset();
    let intent_before = quote.place_before_id.clone();
    let intent_after = quote.place_after_id.clone();
    let pinned = quote.pinned;
    let mut am: quotes::ActiveModel = quote.into();
    am.person_id = Set(person_id);
    am.proposed_person_avatar_url = Set(None);
    am.status = Set(quotes::status::APPROVED.to_string());
    am.reviewed_at = Set(Some(now));
    am.reviewed_by = Set(Some(admin.id));
    let updated = am.update(&state.db).await?;
    place_quote(&state.db, &updated.id, pinned, intent_before, intent_after).await?;
    let updated = quotes::Entity::find_by_id(updated.id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::internal("审核后丢失"))?;
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

    Ok(Json(admin_quote_item(updated, person, None)))
}

pub async fn reject_quote(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<String>,
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

    Ok(Json(admin_quote_item(updated, person, None)))
}
