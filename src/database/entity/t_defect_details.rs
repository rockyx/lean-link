use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "t_defect_details")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    #[sea_orm(not_null)]
    pub inspection_detail_id: Uuid,

    #[sea_orm(column_type = "String(StringLen::N(50))", not_null)]
    pub defect_type: String,

    #[sea_orm(column_type = "String(StringLen::N(20))", nullable)]
    pub defect_code: Option<String>,

    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    #[sea_orm(column_type = "Decimal(Some((6, 4)))", nullable)]
    pub position_x: Option<Decimal>,

    #[sea_orm(column_type = "Decimal(Some((6, 4)))", nullable)]
    pub position_y: Option<Decimal>,

    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub bounding_box: Option<Json>,

    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub polygon_points: Option<Json>,

    #[sea_orm(column_type = "Decimal(Some((5, 4)))", nullable)]
    pub severity_score: Option<Decimal>,

    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub area: Option<Decimal>,

    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub length: Option<Decimal>,

    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub width: Option<Decimal>,

    #[sea_orm(column_type = "Decimal(Some((5, 4)))", nullable)]
    pub confidence: Option<Decimal>,

    #[sea_orm(column_type = "Text", nullable)]
    pub repair_suggestion: Option<String>,

    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::t_inspection_details::Entity",
        from = "Column::InspectionDetailId",
        to = "super::t_inspection_details::Column::Id"
    )]
    InspectionDetail,
}

impl Related<super::t_inspection_details::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InspectionDetail.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
