use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "t_camera_configs")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    /// 设备用户 ID
    #[sea_orm(column_type = "String(StringLen::N(100))", nullable)]
    pub device_user_id: Option<String>,

    /// 相机键值
    #[sea_orm(column_type = "String(StringLen::N(100))", nullable)]
    pub key: Option<String>,

    /// 序列号
    #[sea_orm(column_type = "String(StringLen::N(100))", nullable)]
    pub serial_number: Option<String>,

    /// 厂商
    #[sea_orm(column_type = "String(StringLen::N(100))", nullable)]
    pub vendor: Option<String>,

    /// 型号
    #[sea_orm(column_type = "String(StringLen::N(100))", nullable)]
    pub model: Option<String>,

    /// 制造信息
    #[sea_orm(column_type = "String(StringLen::N(200))", nullable)]
    pub manufacture_info: Option<String>,

    /// 设备版本
    #[sea_orm(column_type = "String(StringLen::N(50))", nullable)]
    pub device_version: Option<String>,

    /// 曝光时间 (毫秒)
    #[sea_orm(default_value = "10.0")]
    pub exposure_time_ms: f64,

    /// 自动曝光
    #[sea_orm(default_value = "false")]
    pub exposure_auto: bool,

    /// IP 地址
    #[sea_orm(column_type = "String(StringLen::N(50))", nullable)]
    pub ip_address: Option<String>,

    /// 相机供应商: "IMV"
    #[sea_orm(column_name = "camera_supplier", column_type = "String(StringLen::N(20))")]
    pub camera_supplier: String,

    /// 是否启用
    #[sea_orm(default_value = "true")]
    pub enabled: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
