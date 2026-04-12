use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::{ColumnDef, Index, Table};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create inspection stations table
        manager
            .create_table(
                Table::create()
                    .table(InspectionStations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(InspectionStations::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(InspectionStations::Name)
                            .string()
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InspectionStations::CameraId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InspectionStations::TriggerMode)
                            .string()
                            .string_len(20)
                            .not_null()
                            .default("Continuous"),
                    )
                    .col(
                        ColumnDef::new(InspectionStations::DetectionTypes)
                            .json()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InspectionStations::IsEnabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(InspectionStations::ModelPath)
                            .string()
                            .string_len(500)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(InspectionStations::ConfidenceThreshold)
                            .float()
                            .not_null()
                            .default(0.5),
                    )
                    .col(
                        ColumnDef::new(InspectionStations::SerialPort)
                            .string()
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(InspectionStations::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InspectionStations::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(InspectionStations::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on camera_id
        manager
            .create_index(
                Index::create()
                    .name("idx_inspection_stations_camera_id")
                    .table(InspectionStations::Table)
                    .col(InspectionStations::CameraId)
                    .to_owned(),
            )
            .await?;

        // Create index on is_enabled
        manager
            .create_index(
                Index::create()
                    .name("idx_inspection_stations_enabled")
                    .table(InspectionStations::Table)
                    .col(InspectionStations::IsEnabled)
                    .to_owned(),
            )
            .await?;

        // Create station rois table
        manager
            .create_table(
                Table::create()
                    .table(StationRois::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StationRois::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(StationRois::StationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StationRois::Name)
                            .string()
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StationRois::Shape)
                            .json()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StationRois::Purpose)
                            .string()
                            .string_len(20)
                            .not_null()
                            .default("Detection"),
                    )
                    .col(
                        ColumnDef::new(StationRois::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(StationRois::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StationRois::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StationRois::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on station_id
        manager
            .create_index(
                Index::create()
                    .name("idx_station_rois_station_id")
                    .table(StationRois::Table)
                    .col(StationRois::StationId)
                    .to_owned(),
            )
            .await?;

        // Create index on purpose
        manager
            .create_index(
                Index::create()
                    .name("idx_station_rois_purpose")
                    .table(StationRois::Table)
                    .col(StationRois::Purpose)
                    .to_owned(),
            )
            .await?;

        // Create index on enabled
        manager
            .create_index(
                Index::create()
                    .name("idx_station_rois_enabled")
                    .table(StationRois::Table)
                    .col(StationRois::Enabled)
                    .to_owned(),
            )
            .await?;

        // Add foreign key constraint for station_id -> inspection_stations.id
        // Note: SQLite doesn't enforce FK by default, but PostgreSQL/MySQL will
        manager
            .create_foreign_key(
                sea_orm_migration::sea_query::ForeignKey::create()
                    .name("fk_station_rois_station_id")
                    .from(StationRois::Table, StationRois::StationId)
                    .to(InspectionStations::Table, InspectionStations::Id)
                    .on_delete(sea_orm_migration::sea_query::ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        // Add foreign key constraint for camera_id -> camera_configs.id
        manager
            .create_foreign_key(
                sea_orm_migration::sea_query::ForeignKey::create()
                    .name("fk_inspection_stations_camera_id")
                    .from(InspectionStations::Table, InspectionStations::CameraId)
                    .to(CameraConfigs::Table, CameraConfigs::Id)
                    .on_delete(sea_orm_migration::sea_query::ForeignKeyAction::Restrict)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StationRois::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(InspectionStations::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum InspectionStations {
    #[iden = "t_inspection_stations"]
    Table,
    Id,
    Name,
    CameraId,
    TriggerMode,
    DetectionTypes,
    IsEnabled,
    ModelPath,
    ConfidenceThreshold,
    SerialPort,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(Iden)]
enum StationRois {
    #[iden = "t_station_rois"]
    Table,
    Id,
    StationId,
    Name,
    Shape,
    Purpose,
    Enabled,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(Iden)]
enum CameraConfigs {
    #[iden = "t_camera_configs"]
    Table,
    Id,
}
