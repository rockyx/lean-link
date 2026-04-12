use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "t_inspection_details")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    #[sea_orm(not_null)]
    pub inspection_id: Uuid,

    #[sea_orm(column_type = "String(StringLen::N(50))", not_null)]
    pub detection_type: String,
    
    #[sea_orm(column_type = "String(StringLen::N(50))", not_null)]
    pub component_id: String,
    
    #[sea_orm(column_type = "String(StringLen::N(100))", nullable)]
    pub component_name: Option<String>,
    
    #[sea_orm(column_type = "String(StringLen::N(10))", not_null)]
    pub result: String, // OK, NG
    
    #[sea_orm(column_type = "Decimal(Some((5, 4)))", nullable)]
    pub confidence_score: Option<Decimal>,
    
    #[sea_orm(column_type = "JsonBinary", not_null)]
    pub measurements: Json,
    
    #[sea_orm(column_type = "String(StringLen::N(50))", nullable)]
    pub failure_type: Option<String>,
    
    #[sea_orm(column_type = "String(StringLen::N(20))", nullable)]
    pub failure_code: Option<String>,
    
    #[sea_orm(column_type = "Text", nullable)]
    pub failure_description: Option<String>,
    
    #[sea_orm(column_type = "String(StringLen::N(10))", nullable)]
    pub failure_severity: Option<String>, // CRITICAL, HIGH, MEDIUM, LOW, INFO
    
    #[sea_orm(column_type = "String(StringLen::N(50))", nullable)]
    pub roi_id: Option<String>,
    
    #[sea_orm(column_type = "String(StringLen::N(20))", nullable)]
    pub roi_type: Option<String>,
    
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::t_inspection_records::Entity",
        from = "Column::InspectionId",
        to = "super::t_inspection_records::Column::Id"
    )]
    InspectionRecord,

    #[sea_orm(has_many = "super::t_geometry_measurements::Entity")]
    GeometryMeasurements,

    #[sea_orm(has_many = "super::t_defect_details::Entity")]
    DefectDetails,
}

impl Related<super::t_inspection_records::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InspectionRecord.def()
    }
}

impl Related<super::t_geometry_measurements::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GeometryMeasurements.def()
    }
}

impl Related<super::t_defect_details::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DefectDetails.def()
    }
}


impl ActiveModelBehavior for ActiveModel {}
