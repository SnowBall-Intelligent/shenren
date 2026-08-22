use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_column("quotes", "pinned").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .add_column(
                            ColumnDef::new(Quotes::Pinned)
                                .boolean()
                                .not_null()
                                .default(false),
                        )
                        .to_owned(),
                )
                .await?;
        }
        if !manager.has_column("quotes", "sort_order").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .add_column(
                            ColumnDef::new(Quotes::SortOrder)
                                .integer()
                                .not_null()
                                .default(0),
                        )
                        .to_owned(),
                )
                .await?;
        }
        // SQLite rejects non-constant defaults (e.g. CURRENT_TIMESTAMP) on ADD COLUMN.
        if !manager.has_column("quotes", "published_at").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .add_column(
                            ColumnDef::new(Quotes::PublishedAt)
                                .timestamp_with_time_zone()
                                .not_null()
                                .default("1970-01-01 00:00:00"),
                        )
                        .to_owned(),
                )
                .await?;
        }
        manager
            .get_connection()
            .execute_unprepared("UPDATE quotes SET published_at = created_at")
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_quotes_feed")
                    .table(Quotes::Table)
                    .col(Quotes::Status)
                    .col(Quotes::Pinned)
                    .col(Quotes::SortOrder)
                    .col(Quotes::PublishedAt)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.has_index("quotes", "idx_quotes_feed").await? {
            manager
                .drop_index(
                    Index::drop()
                        .name("idx_quotes_feed")
                        .table(Quotes::Table)
                        .to_owned(),
                )
                .await?;
        }
        if manager.has_column("quotes", "published_at").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .drop_column(Quotes::PublishedAt)
                        .to_owned(),
                )
                .await?;
        }
        if manager.has_column("quotes", "sort_order").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .drop_column(Quotes::SortOrder)
                        .to_owned(),
                )
                .await?;
        }
        if manager.has_column("quotes", "pinned").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .drop_column(Quotes::Pinned)
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
    Status,
    Pinned,
    SortOrder,
    PublishedAt,
}
