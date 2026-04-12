use chrono::Datelike;
use sea_orm::Statement;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DbBackend;
use sea_orm_migration::sea_query::{extension::postgres::Type, Index, Table, ColumnDef, TableCreateStatement, ForeignKey, ForeignKeyAction};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db_backend = manager.get_database_backend();
        
        // 创建枚举类型（PostgreSQL特有）
        if db_backend == DbBackend::Postgres {
            // 创建检测结果枚举
            manager
                .create_type(
                    Type::create()
                        .as_enum("inspection_result")
                        .values(vec!["OK", "NG", "PENDING", "ERROR"])
                        .to_owned()
                )
                .await?;
            
            // 创建详细结果枚举
            manager
                .create_type(
                    Type::create()
                        .as_enum("detail_result")
                        .values(vec!["OK", "NG"])
                        .to_owned()
                )
                .await?;
            
            // 创建失败严重程度枚举
            manager
                .create_type(
                    Type::create()
                        .as_enum("failure_severity")
                        .values(vec!["CRITICAL", "HIGH", "MEDIUM", "LOW", "INFO"])
                        .to_owned()
                )
                .await?;
        }
        
        // 创建检测记录主表
        let table = get_inspection_records_table(db_backend);
        manager.create_table(table).await?;
        
        // 创建检测记录索引
        create_inspection_records_indexes(manager).await?;
        
        // 创建检测详情表
        let table = get_inspection_details_table(db_backend);
        manager.create_table(table).await?;
        
        // 创建检测详情索引
        create_inspection_details_indexes(manager).await?;
        
        // 创建几何测量表
        let table = get_geometry_measurements_table(db_backend);
        manager.create_table(table).await?;
        
        // 创建缺陷详情表
        let table = get_defect_details_table(db_backend);
        manager.create_table(table).await?;
        
        // 创建检测统计表
        let table = get_inspection_statistics_table(db_backend);
        manager.create_table(table).await?;
        
        // 创建检测统计索引
        create_inspection_statistics_indexes(manager).await?;
        
        // 为PostgreSQL创建分区表（如果需要）
        if db_backend == DbBackend::Postgres {
            create_partitioned_tables(manager).await?;
        }
        
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db_backend = manager.get_database_backend();
        
        // 删除顺序很重要，先删除有外键依赖的表
        manager
            .drop_table(Table::drop().table(GeometryMeasurements::Table).to_owned())
            .await?;
            
        manager
            .drop_table(Table::drop().table(DefectDetails::Table).to_owned())
            .await?;
            
        manager
            .drop_table(Table::drop().table(InspectionDetails::Table).to_owned())
            .await?;
            
        manager
            .drop_table(Table::drop().table(InspectionStatistics::Table).to_owned())
            .await?;
            
        manager
            .drop_table(Table::drop().table(InspectionRecords::Table).to_owned())
            .await?;
        
        // 删除枚举类型（PostgreSQL特有）
        if db_backend == DbBackend::Postgres {
            manager
                .drop_type(Type::drop().name("failure_severity").to_owned())
                .await?;
                
            manager
                .drop_type(Type::drop().name("detail_result").to_owned())
                .await?;
                
            manager
                .drop_type(Type::drop().name("inspection_result").to_owned())
                .await?;
        }
        
        Ok(())
    }
}

// 定义表名常量

#[derive(Iden)]
enum InspectionRecords {
    #[iden = "t_inspection_records"]
    Table,
    Id,
    StationId,
    CameraId,
    ProductSerial,
    BatchNumber,
    OverallResult,
    ConfidenceScore,
    InspectionTime,
    ProcessingTimeMs,
    TriggerMode,
    DetectionTypes,
    ImagePaths,
    VideoPath,
    FirmwareVersion,
    SoftwareVersion,
    ModelVersion,
    Temperature,
    Humidity,
    LightingCondition,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum InspectionDetails {
    #[iden = "t_inspection_details"]
    Table,
    Id,
    InspectionId,
    DetectionType,
    ComponentId,
    ComponentName,
    Result,
    ConfidenceScore,
    Measurements,
    FailureType,
    FailureCode,
    FailureDescription,
    FailureSeverity,
    RoiId,
    RoiType,
    CreatedAt,
}

#[derive(Iden)]
enum GeometryMeasurements {
    #[iden = "t_geometry_measurements"]
    Table,
    Id,
    InspectionDetailId,
    Length,
    Width,
    Height,
    Diameter,
    Thickness,
    Area,
    Perimeter,
    AspectRatio,
    Circularity,
    CentroidX,
    CentroidY,
    Angle,
    BoundingBox,
    ContourPoints,
    ToleranceMin,
    ToleranceMax,
    IsInTolerance,
    CustomMeasurements,
}

#[derive(Iden)]
enum DefectDetails {
    #[iden = "t_defect_details"]
    Table,
    Id,
    InspectionDetailId,
    DefectType,
    DefectCode,
    Description,
    PositionX,
    PositionY,
    BoundingBox,
    PolygonPoints,
    SeverityScore,
    Area,
    Length,
    Width,
    Confidence,
    RepairSuggestion,
    CreatedAt,
}

#[derive(Iden)]
enum InspectionStatistics {
    #[iden = "t_inspection_statistics"]
    Table,
    Id,
    StationId,
    Date,
    DetectionType,
    ComponentId,
    TotalCount,
    OkCount,
    NgCount,
    ErrorCount,
    YieldRate,
    AvgProcessingTimeMs,
    MinProcessingTimeMs,
    MaxProcessingTimeMs,
    TopDefects,
    LastUpdated,
}

// 获取检测记录表创建语句
fn get_inspection_records_table(db_backend: DbBackend) -> TableCreateStatement {
    let result_type = if db_backend == DbBackend::Postgres {
        "inspection_result"
    } else {
        "VARCHAR(10)"
    };
    
    Table::create()
        .table(InspectionRecords::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(InspectionRecords::Id)
                .uuid()
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(InspectionRecords::StationId)
                .string()
                .not_null()
                .string_len(50),
        )
        .col(
            ColumnDef::new(InspectionRecords::CameraId)
                .string()
                .string_len(50)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::ProductSerial)
                .string()
                .string_len(100)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::BatchNumber)
                .string()
                .string_len(50)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::OverallResult)
                .custom(result_type)
                .not_null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::ConfidenceScore)
                .decimal_len(5, 4)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::InspectionTime)
                .timestamp_with_time_zone()
                .not_null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::ProcessingTimeMs)
                .integer()
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::TriggerMode)
                .string()
                .not_null()
                .string_len(20)
                .default("CONTINUOUS"),
        )
        .col(
            ColumnDef::new(InspectionRecords::DetectionTypes)
                .json_binary()
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::ImagePaths)
                .json_binary()
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::VideoPath)
                .string()
                .string_len(500)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::FirmwareVersion)
                .string()
                .string_len(50)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::SoftwareVersion)
                .string()
                .string_len(50)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::ModelVersion)
                .string()
                .string_len(100)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::Temperature)
                .decimal_len(5, 2)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::Humidity)
                .decimal_len(5, 2)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::LightingCondition)
                .string()
                .string_len(20)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionRecords::CreatedAt)
                .timestamp_with_time_zone()
                .not_null()
                .default(Expr::current_timestamp()),
        )
        .col(
            ColumnDef::new(InspectionRecords::UpdatedAt)
                .timestamp_with_time_zone()
                .not_null()
                .default(Expr::current_timestamp()),
        )
        .to_owned()
}

// 获取检测详情表创建语句
fn get_inspection_details_table(db_backend: DbBackend) -> TableCreateStatement {
    let result_type = if db_backend == DbBackend::Postgres {
        "detail_result"
    } else {
        "VARCHAR(10)"
    };
    
    let severity_type = if db_backend == DbBackend::Postgres {
        "failure_severity"
    } else {
        "VARCHAR(10)"
    };
    
    Table::create()
        .table(InspectionDetails::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(InspectionDetails::Id)
                .uuid()
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(InspectionDetails::InspectionId)
                .uuid()
                .not_null(),
        )
        .col(
            ColumnDef::new(InspectionDetails::DetectionType)
                .string()
                .not_null()
                .string_len(50),
        )
        .col(
            ColumnDef::new(InspectionDetails::ComponentId)
                .string()
                .not_null()
                .string_len(50),
        )
        .col(
            ColumnDef::new(InspectionDetails::ComponentName)
                .string()
                .string_len(100)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionDetails::Result)
                .custom(result_type)
                .not_null(),
        )
        .col(
            ColumnDef::new(InspectionDetails::ConfidenceScore)
                .decimal_len(5, 4)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionDetails::Measurements)
                .json_binary()
                .not_null(),
        )
        .col(
            ColumnDef::new(InspectionDetails::FailureType)
                .string()
                .string_len(50)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionDetails::FailureCode)
                .string()
                .string_len(20)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionDetails::FailureDescription)
                .text()
                .null(),
        )
        .col(
            ColumnDef::new(InspectionDetails::FailureSeverity)
                .custom(severity_type)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionDetails::RoiId)
                .string()
                .string_len(50)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionDetails::RoiType)
                .string()
                .string_len(20)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionDetails::CreatedAt)
                .timestamp_with_time_zone()
                .not_null()
                .default(Expr::current_timestamp()),
        )
        .foreign_key(
            ForeignKey::create()
                .name("fk_inspection_details_inspection_id")
                .from(InspectionDetails::Table, InspectionDetails::InspectionId)
                .to(InspectionRecords::Table, InspectionRecords::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .on_update(ForeignKeyAction::Cascade)
        )
        .to_owned()
}

// 获取几何测量表创建语句
fn get_geometry_measurements_table(_db_backend: DbBackend) -> TableCreateStatement {
    Table::create()
        .table(GeometryMeasurements::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(GeometryMeasurements::Id)
                .uuid()
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::InspectionDetailId)
                .uuid()
                .not_null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::Length)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::Width)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::Height)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::Diameter)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::Thickness)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::Area)
                .decimal_len(12, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::Perimeter)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::AspectRatio)
                .decimal_len(6, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::Circularity)
                .decimal_len(6, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::CentroidX)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::CentroidY)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::Angle)
                .decimal_len(6, 2)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::BoundingBox)
                .json_binary()
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::ContourPoints)
                .json_binary()
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::ToleranceMin)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::ToleranceMax)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::IsInTolerance)
                .boolean()
                .null(),
        )
        .col(
            ColumnDef::new(GeometryMeasurements::CustomMeasurements)
                .json_binary()
                .null(),
        )
        .foreign_key(
            ForeignKey::create()
                .name("fk_geometry_measurements_inspection_detail_id")
                .from(GeometryMeasurements::Table, GeometryMeasurements::InspectionDetailId)
                .to(InspectionDetails::Table, InspectionDetails::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .on_update(ForeignKeyAction::Cascade)
        )
        .to_owned()
}

// 获取缺陷详情表创建语句
fn get_defect_details_table(_db_backend: DbBackend) -> TableCreateStatement {
    Table::create()
        .table(DefectDetails::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(DefectDetails::Id)
                .uuid()
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(DefectDetails::InspectionDetailId)
                .uuid()
                .not_null(),
        )
        .col(
            ColumnDef::new(DefectDetails::DefectType)
                .string()
                .not_null()
                .string_len(50),
        )
        .col(
            ColumnDef::new(DefectDetails::DefectCode)
                .string()
                .string_len(20)
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::Description)
                .text()
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::PositionX)
                .decimal_len(6, 4)
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::PositionY)
                .decimal_len(6, 4)
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::BoundingBox)
                .json_binary()
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::PolygonPoints)
                .json_binary()
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::SeverityScore)
                .decimal_len(5, 4)
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::Area)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::Length)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::Width)
                .decimal_len(10, 4)
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::Confidence)
                .decimal_len(5, 4)
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::RepairSuggestion)
                .text()
                .null(),
        )
        .col(
            ColumnDef::new(DefectDetails::CreatedAt)
                .timestamp_with_time_zone()
                .not_null()
                .default(Expr::current_timestamp()),
        )
        .foreign_key(
            ForeignKey::create()
                .name("fk_defect_details_inspection_detail_id")
                .from(DefectDetails::Table, DefectDetails::InspectionDetailId)
                .to(InspectionDetails::Table, InspectionDetails::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .on_update(ForeignKeyAction::Cascade)
        )
        .to_owned()
}

// 获取检测统计表创建语句
fn get_inspection_statistics_table(db_backend: DbBackend) -> TableCreateStatement {
    let mut table = Table::create();
    table
        .table(InspectionStatistics::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(InspectionStatistics::Id)
                .uuid()
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(InspectionStatistics::StationId)
                .string()
                .not_null()
                .string_len(50),
        )
        .col(
            ColumnDef::new(InspectionStatistics::Date)
                .date()
                .not_null(),
        )
        .col(
            ColumnDef::new(InspectionStatistics::DetectionType)
                .string()
                .string_len(50)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionStatistics::ComponentId)
                .string()
                .string_len(50)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionStatistics::TotalCount)
                .integer()
                .not_null()
                .default(0),
        )
        .col(
            ColumnDef::new(InspectionStatistics::OkCount)
                .integer()
                .not_null()
                .default(0),
        )
        .col(
            ColumnDef::new(InspectionStatistics::NgCount)
                .integer()
                .not_null()
                .default(0),
        )
        .col(
            ColumnDef::new(InspectionStatistics::ErrorCount)
                .integer()
                .not_null()
                .default(0),
        )
        .col(
            ColumnDef::new(InspectionStatistics::YieldRate)
                .decimal_len(5, 2)
                .null(),
        )
        .col(
            ColumnDef::new(InspectionStatistics::AvgProcessingTimeMs)
                .integer()
                .null(),
        )
        .col(
            ColumnDef::new(InspectionStatistics::MinProcessingTimeMs)
                .integer()
                .null(),
        )
        .col(
            ColumnDef::new(InspectionStatistics::MaxProcessingTimeMs)
                .integer()
                .null(),
        )
        .col(
            ColumnDef::new(InspectionStatistics::TopDefects)
                .json_binary()
                .null(),
        )
        .col(
            ColumnDef::new(InspectionStatistics::LastUpdated)
                .timestamp_with_time_zone()
                .not_null()
                .default(Expr::current_timestamp()),
        );
    
    // 为PostgreSQL添加唯一约束
    if db_backend == DbBackend::Postgres {
        table.index(
            Index::create()
                .unique()
                .name("unique_inspection_statistics")
                .col(InspectionStatistics::StationId)
                .col(InspectionStatistics::Date)
                .col(InspectionStatistics::DetectionType)
                .col(InspectionStatistics::ComponentId)
        );
    }
    
    table.to_owned()
}

// 创建检测记录索引
async fn create_inspection_records_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let indexes = vec![
        Index::create()
            .name("idx_inspection_records_station_time")
            .table(InspectionRecords::Table)
            .col(InspectionRecords::StationId)
            .col(InspectionRecords::InspectionTime)
            .to_owned(),
        Index::create()
            .name("idx_inspection_records_batch_result")
            .table(InspectionRecords::Table)
            .col(InspectionRecords::BatchNumber)
            .col(InspectionRecords::OverallResult)
            .to_owned(),
        Index::create()
            .name("idx_inspection_records_product_serial")
            .table(InspectionRecords::Table)
            .col(InspectionRecords::ProductSerial)
            .to_owned(),
        Index::create()
            .name("idx_inspection_records_result_time")
            .table(InspectionRecords::Table)
            .col(InspectionRecords::OverallResult)
            .col(InspectionRecords::InspectionTime)
            .to_owned(),
    ];
    
    for index in indexes {
        manager.create_index(index).await?;
    }
    
    Ok(())
}

// 创建检测详情索引
async fn create_inspection_details_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let indexes = vec![
        Index::create()
            .name("idx_inspection_details_inspection_id")
            .table(InspectionDetails::Table)
            .col(InspectionDetails::InspectionId)
            .to_owned(),
        Index::create()
            .name("idx_inspection_details_detection_type_result")
            .table(InspectionDetails::Table)
            .col(InspectionDetails::DetectionType)
            .col(InspectionDetails::Result)
            .to_owned(),
        Index::create()
            .name("idx_inspection_details_component_result")
            .table(InspectionDetails::Table)
            .col(InspectionDetails::ComponentId)
            .col(InspectionDetails::Result)
            .to_owned(),
        Index::create()
            .name("idx_inspection_details_failure_type")
            .table(InspectionDetails::Table)
            .col(InspectionDetails::FailureType)
            .to_owned(),
    ];
    
    for index in indexes {
        manager.create_index(index).await?;
    }
    
    Ok(())
}

// 创建检测统计索引
async fn create_inspection_statistics_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let indexes = vec![
        Index::create()
            .name("idx_inspection_statistics_station_date")
            .table(InspectionStatistics::Table)
            .col(InspectionStatistics::StationId)
            .col(InspectionStatistics::Date)
            .to_owned(),
    ];
    
    for index in indexes {
        manager.create_index(index).await?;
    }
    
    Ok(())
}

// 创建分区表（仅PostgreSQL）
async fn create_partitioned_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let db_backend = manager.get_database_backend();
    
    if db_backend != DbBackend::Postgres {
        return Ok(());
    }
    
    // 创建分区表（按年分区）
    let year = chrono::Utc::now().year();
    let next_year = year + 1;
    
    for month in 1..=12 {
        let partition_name = format!("inspection_records_y{}m{:02}", year, month);
        let start_date = format!("{}-{:02}-01", year, month);
        let end_date = if month == 12 {
            format!("{}-01-01", next_year)
        } else {
            format!("{}-{:02}-01", year, month + 1)
        };
        
        // 创建分区表
        let create_partition_sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} PARTITION OF t_inspection_records
            FOR VALUES FROM ('{}') TO ('{}');
            "#,
            partition_name, start_date, end_date
        );
        
        manager.get_connection()
            .execute(Statement::from_string(db_backend, create_partition_sql))
            .await?;
    }
    
    // 为分区表创建索引
    for month in 1..=12 {
        let partition_name = format!("inspection_records_y{}m{:02}", year, month);
        let index_sql = format!(
            r#"
            CREATE INDEX IF NOT EXISTS idx_{}_inspection_time 
            ON {} (inspection_time);
            "#,
            partition_name, partition_name
        );
        
        manager.get_connection()
            .execute(Statement::from_string(db_backend, index_sql))
            .await?;
    }
    
    Ok(())
}