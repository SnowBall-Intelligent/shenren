use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "api_keys")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub enabled: bool,
    pub rate_limit: Option<i64>,
    pub rate_window_secs: Option<i64>,
    pub total_quota: Option<i64>,
    pub used_count: i64,
    pub concurrency_limit: Option<i64>,
    #[sea_orm(column_type = "Text")]
    pub allowed_ips: String,
    #[sea_orm(column_type = "Text")]
    pub allowed_domains: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub last_used_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
