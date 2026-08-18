use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "persons")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub avatar_path: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::quotes::Entity")]
    Quotes,
}

impl Related<super::quotes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Quotes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
