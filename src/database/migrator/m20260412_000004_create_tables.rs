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
                    .table(CameraConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CameraConfigs::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::DeviceUserId)
                            .string()
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::Key)
                            .string()
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::SerialNumber)
                            .string()
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::Vendor)
                            .string()
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::Model)
                            .string()
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::ManufactureInfo)
                            .string()
                            .string_len(200)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::DeviceVersion)
                            .string()
                            .string_len(50)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::ExposureTimeMs)
                            .double()
                            .not_null()
                            .default(10.0),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::ExposureAuto)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::IpAddress)
                            .string()
                            .string_len(50)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::CameraSupplier)
                            .string()
                            .not_null()
                            .string_len(20),
                    )
                    .col(
                        ColumnDef::new(CameraConfigs::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_camera_configs_key")
                    .table(CameraConfigs::Table)
                    .col(CameraConfigs::Key)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_camera_configs_serial_number")
                    .table(CameraConfigs::Table)
                    .col(CameraConfigs::SerialNumber)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_camera_configs_enabled")
                    .table(CameraConfigs::Table)
                    .col(CameraConfigs::Enabled)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CameraConfigs::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum CameraConfigs {
    #[iden = "t_camera_configs"]
    Table,
    Id,
    DeviceUserId,
    Key,
    SerialNumber,
    Vendor,
    Model,
    ManufactureInfo,
    DeviceVersion,
    ExposureTimeMs,
    ExposureAuto,
    IpAddress,
    CameraSupplier,
    Enabled,
}
