use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager
            .has_column("site_settings", "captcha_admin_account_enabled")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(SiteSettings::Table)
                        .add_column(
                            ColumnDef::new(SiteSettings::CaptchaAdminAccountEnabled)
                                .boolean()
                                .not_null()
                                .default(false),
                        )
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager
            .has_column("site_settings", "captcha_admin_account_enabled")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(SiteSettings::Table)
                        .drop_column(SiteSettings::CaptchaAdminAccountEnabled)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum SiteSettings {
    Table,
    CaptchaAdminAccountEnabled,
}
