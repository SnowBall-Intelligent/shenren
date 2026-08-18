use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "site_settings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub site_name: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub logo_url: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub footer: Option<String>,
    pub allow_propose_person: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
