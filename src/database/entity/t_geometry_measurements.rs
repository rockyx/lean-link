use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "t_geometry_measurements")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    #[sea_orm(not_null)]
    pub inspection_detail_id: Uuid,
    
    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub length: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub width: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub height: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub diameter: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub thickness: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((12, 4)))", nullable)]
    pub area: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub perimeter: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((6, 4)))", nullable)]
    pub aspect_ratio: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((6, 4)))", nullable)]
    pub circularity: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub centroid_x: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub centroid_y: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((6, 2)))", nullable)]
    pub angle: Option<Decimal>,
    
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub bounding_box: Option<Json>,
    
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub contour_points: Option<Json>,
    
    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub tolerance_min: Option<Decimal>,
    
    #[sea_orm(column_type = "Decimal(Some((10, 4)))", nullable)]
    pub tolerance_max: Option<Decimal>,
    
    #[sea_orm(nullable)]
    pub is_in_tolerance: Option<bool>,
    
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub custom_measurements: Option<Json>,
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