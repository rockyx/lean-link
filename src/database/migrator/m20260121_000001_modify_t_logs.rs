use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        {
            manager
                .alter_table(
                    Table::alter()
                        .table("t_logs")
                        .add_column_if_not_exists(ColumnDef::new("level").string_len(10).not_null())
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}
