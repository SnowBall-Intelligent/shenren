use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_column("quotes", "place_before_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .add_column(ColumnDef::new(Quotes::PlaceBeforeId).big_integer().null())
                        .to_owned(),
                )
                .await?;
        }
        if !manager.has_column("quotes", "place_after_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .add_column(ColumnDef::new(Quotes::PlaceAfterId).big_integer().null())
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.has_column("quotes", "place_after_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .drop_column(Quotes::PlaceAfterId)
                        .to_owned(),
                )
                .await?;
        }
        if manager.has_column("quotes", "place_before_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Quotes::Table)
                        .drop_column(Quotes::PlaceBeforeId)
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
    PlaceBeforeId,
    PlaceAfterId,
}
