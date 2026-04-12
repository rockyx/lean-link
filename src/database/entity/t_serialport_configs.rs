use std::time::Duration;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serialport::{DataBits, FlowControl, Parity, StopBits};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "t_serialport_configs")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub path: String,
    pub baud_rate: u32,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub data_bits: String,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub stop_bits: String,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub parity: String,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub flow_control: String,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// 获取超时时间
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    /// 获取数据位配置
    pub fn data_bits_enum(&self) -> Result<DataBits, String> {
        match self.data_bits.as_str() {
            "Five" => Ok(DataBits::Five),
            "Six" => Ok(DataBits::Six),
            "Seven" => Ok(DataBits::Seven),
            "Eight" => Ok(DataBits::Eight),
            _ => Err(format!("Invalid data_bits: {}", self.data_bits)),
        }
    }

    /// 获取停止位配置
    pub fn stop_bits_enum(&self) -> Result<StopBits, String> {
        match self.stop_bits.as_str() {
            "One" => Ok(StopBits::One),
            "Two" => Ok(StopBits::Two),
            _ => Err(format!("Invalid stop_bits: {}", self.stop_bits)),
        }
    }

    /// 获取校验位配置
    pub fn parity_enum(&self) -> Result<Parity, String> {
        match self.parity.as_str() {
            "None" => Ok(Parity::None),
            "Odd" => Ok(Parity::Odd),
            "Even" => Ok(Parity::Even),
            _ => Err(format!("Invalid parity: {}", self.parity)),
        }
    }

    /// 获取流控制配置
    pub fn flow_control_enum(&self) -> Result<FlowControl, String> {
        match self.flow_control.as_str() {
            "None" => Ok(FlowControl::None),
            "Software" => Ok(FlowControl::Software),
            "Hardware" => Ok(FlowControl::Hardware),
            _ => Err(format!("Invalid flow_control: {}", self.flow_control)),
        }
    }
}

/// 将 serialport 枚举转换为字符串
pub fn data_bits_to_string(value: DataBits) -> String {
    match value {
        DataBits::Five => "Five".to_string(),
        DataBits::Six => "Six".to_string(),
        DataBits::Seven => "Seven".to_string(),
        DataBits::Eight => "Eight".to_string(),
    }
}

pub fn stop_bits_to_string(value: StopBits) -> String {
    match value {
        StopBits::One => "One".to_string(),
        StopBits::Two => "Two".to_string(),
    }
}

pub fn parity_to_string(value: Parity) -> String {
    match value {
        Parity::None => "None".to_string(),
        Parity::Odd => "Odd".to_string(),
        Parity::Even => "Even".to_string(),
    }
}

pub fn flow_control_to_string(value: FlowControl) -> String {
    match value {
        FlowControl::None => "None".to_string(),
        FlowControl::Software => "Software".to_string(),
        FlowControl::Hardware => "Hardware".to_string(),
    }
}