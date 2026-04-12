use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "t_inspection_records")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    #[sea_orm(column_type = "String(StringLen::N(50))", not_null)]
    pub station_id: String,

    #[sea_orm(column_type = "String(StringLen::N(50))", nullable)]
    pub camera_id: Option<String>,

    #[sea_orm(column_type = "String(StringLen::N(100))", nullable)]
    pub product_serial: Option<String>,

    #[sea_orm(column_type = "String(StringLen::N(50))", nullable)]
    pub batch_number: Option<String>,

    #[sea_orm(column_type = "String(StringLen::N(10))", not_null)]
    pub overall_result: String, // OK, NG, PENDING, ERROR

    #[sea_orm(column_type = "Decimal(Some((5, 4)))", nullable)]
    pub confidence_score: Option<Decimal>,

    #[sea_orm(not_null)]
    pub inspection_time: DateTimeWithTimeZone,

    #[sea_orm(nullable)]
    pub processing_time_ms: Option<i32>,

    #[sea_orm(column_type = "String(StringLen::N(20))", default_value = "CONTINUOUS")]
    pub trigger_mode: String,

    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub detection_types: Option<Json>,

    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub image_paths: Option<Json>,

    #[sea_orm(column_type = "String(StringLen::N(500))", nullable)]
    pub video_path: Option<String>,

    #[sea_orm(column_type = "String(StringLen::N(50))", nullable)]
    pub firmware_version: Option<String>,

    #[sea_orm(column_type = "String(StringLen::N(50))", nullable)]
    pub software_version: Option<String>,

    #[sea_orm(column_type = "String(StringLen::N(100))", nullable)]
    pub model_version: Option<String>,

    #[sea_orm(column_type = "Decimal(Some((5, 2)))", nullable)]
    pub temperature: Option<Decimal>,

    #[sea_orm(column_type = "Decimal(Some((5, 2)))", nullable)]
    pub humidity: Option<Decimal>,

    #[sea_orm(column_type = "String(StringLen::N(20))", nullable)]
    pub lighting_condition: Option<String>,

    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::t_inspection_details::Entity")]
    InspectionDetails,
}

impl Related<super::t_inspection_details::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InspectionDetails.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
