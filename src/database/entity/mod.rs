use serde::{Deserialize, Serialize};

pub mod prelude;
pub mod t_logs;
pub mod t_settings;
pub mod t_users;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T> {
    pub records: Vec<T>,
    pub page_index: u64,
    pub page_size: u64,
    pub total_count: u64,
    pub pages: u64,
}
