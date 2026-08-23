use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_column("admins", "role").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Admins::Table)
                        .add_column(
                            ColumnDef::new(Admins::Role)
                                .string_len(32)
                                .not_null()
                                .default("super_admin"),
                        )
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.has_column("admins", "role").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Admins::Table)
                        .drop_column(Admins::Role)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Admins {
    Table,
    Role,
}
