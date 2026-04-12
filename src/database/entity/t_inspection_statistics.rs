use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "t_inspection_statistics")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    #[sea_orm(column_type = "String(StringLen::N(50))", not_null)]
    pub station_id: String,
    
    #[sea_orm(not_null)]
    pub date: Date,
    
    #[sea_orm(column_type = "String(StringLen::N(50))", nullable)]
    pub detection_type: Option<String>,
    
    #[sea_orm(column_type = "String(StringLen::N(50))", nullable)]
    pub component_id: Option<String>,
    
    #[sea_orm(default_value = "0")]
    pub total_count: i32,
    
    #[sea_orm(default_value = "0")]
    pub ok_count: i32,
    
    #[sea_orm(default_value = "0")]
    pub ng_count: i32,
    
    #[sea_orm(default_value = "0")]
    pub error_count: i32,
    
    #[sea_orm(column_type = "Decimal(Some((5, 2)))", nullable)]
    pub yield_rate: Option<Decimal>,
    
    #[sea_orm(nullable)]
    pub avg_processing_time_ms: Option<i32>,
    
    #[sea_orm(nullable)]
    pub min_processing_time_ms: Option<i32>,
    
    #[sea_orm(nullable)]
    pub max_processing_time_ms: Option<i32>,
    
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub top_defects: Option<Json>,
    
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub last_updated: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// 创建复合唯一索引
    pub fn unique_fields(&self) -> (String, Date, Option<String>, Option<String>) {
        (self.station_id.clone(), self.date, self.detection_type.clone(), self.component_id.clone())
    }
}