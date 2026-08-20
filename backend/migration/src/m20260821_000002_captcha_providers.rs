use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(SiteSettings::Table)
                    .add_column(ColumnDef::new(SiteSettings::CaptchaProviders).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(SiteSettings::Table)
                    .drop_column(SiteSettings::CaptchaProviders)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum SiteSettings {
    Table,
    CaptchaProviders,
}
