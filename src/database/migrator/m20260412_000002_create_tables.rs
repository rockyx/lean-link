use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::{Index, Table, ColumnDef};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SerialportConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SerialportConfigs::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SerialportConfigs::Path)
                            .string()
                            .not_null()
                            .string_len(100),
                    )
                    .col(
                        ColumnDef::new(SerialportConfigs::BaudRate)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SerialportConfigs::DataBits)
                            .string()
                            .not_null()
                            .string_len(20),
                    )
                    .col(
                        ColumnDef::new(SerialportConfigs::StopBits)
                            .string()
                            .not_null()
                            .string_len(20),
                    )
                    .col(
                        ColumnDef::new(SerialportConfigs::Parity)
                            .string()
                            .not_null()
                            .string_len(20),
                    )
                    .col(
                        ColumnDef::new(SerialportConfigs::FlowControl)
                            .string()
                            .not_null()
                            .string_len(20),
                    )
                    .col(
                        ColumnDef::new(SerialportConfigs::TimeoutMs)
                            .big_unsigned()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_serialport_configs_path")
                    .table(SerialportConfigs::Table)
                    .col(SerialportConfigs::Path)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SerialportConfigs::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum SerialportConfigs {
    Table,
    Id,
    Path,
    BaudRate,
    DataBits,
    StopBits,
    Parity,
    FlowControl,
    TimeoutMs,
}
