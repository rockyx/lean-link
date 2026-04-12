use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "t_modbus_configs")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    
    /// Modbus 类型: "TCP" 或 "RTU"
    #[sea_orm(column_name = "type", column_type = "String(StringLen::N(20))")]
    pub r#type: String,
    
    /// TCP 主机地址 (仅 TCP 模式)
    #[sea_orm(column_type = "String(StringLen::N(100))", nullable)]
    pub host: Option<String>,
    
    /// TCP 端口 (仅 TCP 模式)
    pub port: Option<u16>,
    
    /// 从机地址
    #[sea_orm(default_value = "1")]
    pub slave_id: u8,
    
    /// 串口配置 ID (仅 RTU 模式)
    #[sea_orm(nullable)]
    pub serialport_id: Option<Uuid>,
    
    /// 配置名称
    #[sea_orm(column_type = "String(StringLen::N(100))", nullable)]
    pub name: Option<String>,
    
    /// 是否启用
    #[sea_orm(default_value = "true")]
    pub enabled: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联串口配置表
    #[sea_orm(
        belongs_to = "super::t_serialport_configs::Entity",
        from = "Column::SerialportId",
        to = "super::t_serialport_configs::Column::Id"
    )]
    SerialportConfig,
}

impl Related<super::t_serialport_configs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SerialportConfig.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
