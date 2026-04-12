use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::{Index, Table, ColumnDef, ForeignKey, ForeignKeyAction};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ModbusConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ModbusConfigs::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ModbusConfigs::Type)
                            .string()
                            .not_null()
                            .string_len(20),
                    )
                    .col(
                        ColumnDef::new(ModbusConfigs::Host)
                            .string()
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ModbusConfigs::Port)
                            .unsigned()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ModbusConfigs::SlaveId)
                            .unsigned()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(ModbusConfigs::SerialportId)
                            .uuid()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ModbusConfigs::Name)
                            .string()
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ModbusConfigs::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_modbus_configs_serialport_id")
                            .from(ModbusConfigs::Table, ModbusConfigs::SerialportId)
                            .to(SerialportConfigs::Table, SerialportConfigs::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_modbus_configs_type")
                    .table(ModbusConfigs::Table)
                    .col(ModbusConfigs::Type)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_modbus_configs_serialport_id")
                    .table(ModbusConfigs::Table)
                    .col(ModbusConfigs::SerialportId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ModbusConfigs::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum ModbusConfigs {
    #[iden = "t_modbus_configs"]
    Table,
    Id,
    Type,
    Host,
    Port,
    SlaveId,
    SerialportId,
    Name,
    Enabled,
}

#[derive(Iden)]
enum SerialportConfigs {
    #[iden = "t_serialport_configs"]
    Table,
    Id,
}
