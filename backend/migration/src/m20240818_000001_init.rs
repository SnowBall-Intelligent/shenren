use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Admins::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Admins::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Admins::Username)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Admins::PasswordHash).text().not_null())
                    .col(
                        ColumnDef::new(Admins::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SiteSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SiteSettings::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SiteSettings::SiteName)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(ColumnDef::new(SiteSettings::Description).text().null())
                    .col(ColumnDef::new(SiteSettings::LogoUrl).text().null())
                    .col(ColumnDef::new(SiteSettings::Footer).text().null())
                    .col(
                        ColumnDef::new(SiteSettings::AllowProposePerson)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Persons::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Persons::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Persons::Name).string_len(128).not_null())
                    .col(ColumnDef::new(Persons::AvatarPath).text().not_null())
                    .col(
                        ColumnDef::new(Persons::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Quotes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Quotes::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Quotes::PersonId).big_integer().null())
                    .col(
                        ColumnDef::new(Quotes::ProposedPersonName)
                            .string_len(128)
                            .null(),
                    )
                    .col(ColumnDef::new(Quotes::Content).text().not_null())
                    .col(ColumnDef::new(Quotes::Source).text().null())
                    .col(ColumnDef::new(Quotes::Status).string_len(32).not_null())
                    .col(
                        ColumnDef::new(Quotes::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Quotes::ReviewedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(Quotes::ReviewedBy).big_integer().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_quotes_person")
                            .from(Quotes::Table, Quotes::PersonId)
                            .to(Persons::Table, Persons::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_quotes_reviewed_by")
                            .from(Quotes::Table, Quotes::ReviewedBy)
                            .to(Admins::Table, Admins::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Quotes::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Persons::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(SiteSettings::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Admins::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Admins {
    Table,
    Id,
    Username,
    PasswordHash,
    CreatedAt,
}

#[derive(DeriveIden)]
enum SiteSettings {
    Table,
    Id,
    SiteName,
    Description,
    LogoUrl,
    Footer,
    AllowProposePerson,
}

#[derive(DeriveIden)]
enum Persons {
    Table,
    Id,
    Name,
    AvatarPath,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Quotes {
    Table,
    Id,
    PersonId,
    ProposedPersonName,
    Content,
    Source,
    Status,
    CreatedAt,
    ReviewedAt,
    ReviewedBy,
}
