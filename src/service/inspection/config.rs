use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionSettings {
    /// Save images for OK results
    #[serde(default)]
    pub save_ok_images: bool,

    /// Save images for NG results
    #[serde(default = "default_save_ng")]
    pub save_ng_images: bool,

    /// Number of days to keep inspection records
    #[serde(default = "default_retention_days")]
    pub result_retention_days: u32,

    /// Maximum frame buffer size per station
    #[serde(default = "default_frame_buffer_size")]
    pub frame_buffer_size: usize,
}

fn default_save_ng() -> bool {
    true
}

fn default_retention_days() -> u32 {
    30
}

fn default_frame_buffer_size() -> usize {
    10
}

impl Default for InspectionSettings {
    fn default() -> Self {
        Self {
            save_ok_images: false,
            save_ng_images: true,
            result_retention_days: 30,
            frame_buffer_size: 10,
        }
    }
}