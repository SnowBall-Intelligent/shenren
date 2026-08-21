use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_quotes_status_created_at")
                    .table(Quotes::Table)
                    .col(Quotes::Status)
                    .col(Quotes::CreatedAt)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_quotes_status_person_created_at")
                    .table(Quotes::Table)
                    .col(Quotes::Status)
                    .col(Quotes::PersonId)
                    .col(Quotes::CreatedAt)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_quotes_status_person_created_at")
                    .table(Quotes::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_quotes_status_created_at")
                    .table(Quotes::Table)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Quotes {
    Table,
    Status,
    PersonId,
    CreatedAt,
}
