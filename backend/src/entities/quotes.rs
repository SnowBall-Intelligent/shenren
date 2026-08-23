use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "quotes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub person_id: Option<i64>,
    pub proposed_person_name: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub proposed_person_avatar_url: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub source: Option<String>,
    pub status: String,
    pub pinned: bool,
    pub published_at: DateTimeWithTimeZone,
    pub place_before_id: Option<String>,
    pub place_after_id: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub reviewed_at: Option<DateTimeWithTimeZone>,
    pub reviewed_by: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::persons::Entity",
        from = "Column::PersonId",
        to = "super::persons::Column::Id"
    )]
    Person,
    #[sea_orm(
        belongs_to = "super::admins::Entity",
        from = "Column::ReviewedBy",
        to = "super::admins::Column::Id"
    )]
    ReviewedByAdmin,
}

impl Related<super::persons::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Person.def()
    }
}

impl Related<super::admins::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ReviewedByAdmin.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub mod status {
    pub const PENDING: &str = "pending";
    pub const APPROVED: &str = "approved";
    pub const REJECTED: &str = "rejected";
}
