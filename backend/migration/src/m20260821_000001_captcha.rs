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
                    .add_column(
                        ColumnDef::new(SiteSettings::CaptchaProvider)
                            .string_len(32)
                            .not_null()
                            .default("none"),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(SiteSettings::Table)
                    .add_column(ColumnDef::new(SiteSettings::CaptchaSiteKey).text().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(SiteSettings::Table)
                    .add_column(ColumnDef::new(SiteSettings::CaptchaSecret).text().null())
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(SiteSettings::Table)
                    .drop_column(SiteSettings::CaptchaSecret)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(SiteSettings::Table)
                    .drop_column(SiteSettings::CaptchaSiteKey)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(SiteSettings::Table)
                    .drop_column(SiteSettings::CaptchaProvider)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum SiteSettings {
    Table,
    CaptchaProvider,
    CaptchaSiteKey,
    CaptchaSecret,
}
