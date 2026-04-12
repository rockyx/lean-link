use serde::{Deserialize, Serialize};

pub mod prelude;
pub mod t_logs;
pub mod t_settings;
pub mod t_users;
#[cfg(feature = "inspection")]
pub mod t_inspection_records;
#[cfg(feature = "inspection")]
pub mod t_inspection_details;
#[cfg(feature = "inspection")]
pub mod t_geometry_measurements;
#[cfg(feature = "inspection")]
pub mod t_defect_details;
#[cfg(feature = "inspection")]
pub mod t_inspection_statistics;
#[cfg(feature = "serialport")]
pub mod t_serialport_configs;
#[cfg(feature = "modbus")]
pub mod t_modbus_configs;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T> {
    pub records: Vec<T>,
    pub page_index: u64,
    pub page_size: u64,
    pub total_count: u64,
    pub pages: u64,
}
