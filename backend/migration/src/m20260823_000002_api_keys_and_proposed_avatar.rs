use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager
            .has_column("quotes", "proposed_person_avatar_url")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .add_column(
                            ColumnDef::new(Quotes::ProposedPersonAvatarUrl)
                                .text()
                                .null(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApiKeys::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ApiKeys::Name).string_len(128).not_null())
                    .col(
                        ColumnDef::new(ApiKeys::KeyPrefix)
                            .string_len(24)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::KeyHash)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(ApiKeys::RateLimit).big_integer().null())
                    .col(ColumnDef::new(ApiKeys::RateWindowSecs).big_integer().null())
                    .col(ColumnDef::new(ApiKeys::TotalQuota).big_integer().null())
                    .col(
                        ColumnDef::new(ApiKeys::UsedCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::ConcurrencyLimit)
                            .big_integer()
                            .null(),
                    )
                    .col(ColumnDef::new(ApiKeys::AllowedIps).text().not_null())
                    .col(ColumnDef::new(ApiKeys::AllowedDomains).text().not_null())
                    .col(
                        ColumnDef::new(ApiKeys::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ApiKeys::LastUsedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiKeys::Table).to_owned())
            .await?;
        if manager
            .has_column("quotes", "proposed_person_avatar_url")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .drop_column(Quotes::ProposedPersonAvatarUrl)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Quotes {
    Table,
    ProposedPersonAvatarUrl,
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    Id,
    Name,
    KeyPrefix,
    KeyHash,
    Enabled,
    RateLimit,
    RateWindowSecs,
    TotalQuota,
    UsedCount,
    ConcurrencyLimit,
    AllowedIps,
    AllowedDomains,
    CreatedAt,
    UpdatedAt,
    LastUsedAt,
}
