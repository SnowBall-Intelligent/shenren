use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand_core::OsRng;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tower_sessions::Session;

use crate::entities::admins;
use crate::error::{AppError, AppResult};

pub const SESSION_ADMIN_ID: &str = "admin_id";

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::internal(format!("hash password failed: {e}")))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, password_hash: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(password_hash)
        .map_err(|e| AppError::internal(format!("invalid password hash: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub async fn require_admin(session: &Session, db: &DatabaseConnection) -> AppResult<admins::Model> {
    let admin_id: Option<i64> = session.get(SESSION_ADMIN_ID).await?;
    let Some(admin_id) = admin_id else {
        return Err(AppError::unauthorized("未登录"));
    };
    let admin = admins::Entity::find_by_id(admin_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::unauthorized("会话无效，请重新登录"))?;
    Ok(admin)
}

pub async fn admin_count(db: &DatabaseConnection) -> AppResult<u64> {
    use sea_orm::PaginatorTrait;
    Ok(admins::Entity::find().count(db).await?)
}

pub async fn find_admin_by_username(
    db: &DatabaseConnection,
    username: &str,
) -> AppResult<Option<admins::Model>> {
    Ok(admins::Entity::find()
        .filter(admins::Column::Username.eq(username))
        .one(db)
        .await?)
}
