use std::{fmt::Display, str::FromStr, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

mod inner;
pub mod manager;
pub mod stream;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CameraSupplier {
    IMV,
}

impl Display for CameraSupplier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CameraSupplier::IMV => write!(f, "IMV")
        }
    }
}

impl FromStr for CameraSupplier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "IMV" => Ok(CameraSupplier::IMV),
            _ => Err(format!("Unknown camera supplier: {}", s)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraInfo {
    pub key: String,
    pub device_user_id: String,
    pub serial_number: String,
    pub vendor: String,
    pub model: String,
    pub manufacture_info: String,
    pub device_version: String,
    pub ip_address: Option<String>,
    pub mac_address: Option<String>,
    pub camera_supplier: CameraSupplier,
}

#[derive(Clone, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum GrabMode {
    Continuous,
    SingleFrame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    Undefined,
    Mono1p,
    Mono2p,
    Mono4p,
    Mono8,
    Mono8S,
    Mono10,
    Mono10Packed,
    Mono12,
    Mono12Packed,
    Mono14,
    Mono16,
    BayGR8,
    BayRG8,
    BayGB8,
    BayBG8,
    BayGR10,
    BayRG10,
    BayGB10,
    BayBG10,
    BayGR12,
    BayRG12,
    BayGB12,
    BayBG12,
    BayGR10Packed,
    BayRG10Packed,
    BayGB10Packed,
    BayBG10Packed,
    BayGR12Packed,
    BayRG12Packed,
    BayGB12Packed,
    BayBG12Packed,
    BayGR16,
    BayRG16,
    BayGB16,
    BayBG16,
    RGB8,
    BGR8,
    RGBA8,
    BGRA8,
    RGB10,
    BGR10,
    RGB12,
    BGR12,
    RGB16,
    RGB10V1Packed,
    RGB10P32,
    RGB12V1Packed,
    RGB565P,
    BGR565P,
    YUV4118UYYVYY,
    YUV4228UYVY,
    YUV4228,
    YUV8UYV,
    YCbCr8CbYCr,
    YCbCr4228,
    YCbCr4228CbYCrY,
    YCbCr4118CbYYCrYY,
    YCbCr6018CbYCr,
    YCbCr6014228,
    YCbCr6014228CbYCrY,
    YCbCr6014118CbYYCrYY,
    YCbCr7098CbYCr,
    YCbCr7094228,
    YCbCr7094228CbYCrY,
    YCbCr7094118CbYYCrYY,
    YUV420SPNV12,
    RGB8Planar,
    RGB10Planar,
    RGB12Planar,
    RGB16Planar,
    BayRG10p,
    BayRG12p,
    Mono1c,
    Mono1e,
}

impl PixelFormat {
    /// Get number of channels
    pub fn channels(&self) -> u32 {
        match self {
            PixelFormat::Mono8 | PixelFormat::Mono16 => 1,
            PixelFormat::RGB8 | PixelFormat::BGR8 => 3,
            PixelFormat::RGBA8 | PixelFormat::BGRA8 => 4,
            _ => 1,
        }
    }

    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            PixelFormat::Mono8 => 1,
            PixelFormat::Mono16 => 2,
            PixelFormat::RGB8 | PixelFormat::BGR8 => 3,
            PixelFormat::RGBA8 | PixelFormat::BGRA8 => 4,
            _ => 1,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FrameSize {
    pub width: usize,
    pub height: usize,
}

/// original camera frame
#[derive(Clone, Debug)]
pub struct CameraFrame {
    pub data: bytes::Bytes,
    pub block_id: u64,
    pub status: u32,
    pub frame_size: FrameSize,
    pub size: usize,
    pub pixel_format: PixelFormat,
    pub timestamp: u64,
    pub chunk_count: usize,
    pub padding_x: usize,
    pub padding_y: usize,
    pub recv_frame_time: u64,
}

/// Frame encoding format
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FrameEncoding {
    /// JPEG compressed (lossy, smaller size)
    Jpeg,
    /// Raw bytes (no compression, larger size)
    Raw,
    /// PNG compressed
    Png,
}

impl Default for FrameEncoding {
    fn default() -> Self {
        Self::Jpeg
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraConfig {
    pub id: Option<uuid::Uuid>,
    pub device_user_id: Option<String>,
    pub key: Option<String>,
    pub serial_number: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub manufacture_info: Option<String>,
    pub device_version: Option<String>,
    pub exposure_time_ms: f64,
    pub exposure_auto: bool,
    pub ip_address: Option<String>,
    pub camera_supplier: CameraSupplier,
}

impl CameraConfig {
    pub fn exposure_duration(&self) -> Duration {
        Duration::from_micros((self.exposure_time_ms * 1000.0) as u64)
    }

    pub fn name(&self) -> String {
        if let Some(ref device_user_id) = self.device_user_id {
            return device_user_id.clone();
        } else if let Some(ref key) = self.key {
            return key.clone();
        } else if let Some(ref serial_number) = self.serial_number {
            return serial_number.clone();
        } else {
            return "".into()
        }
    }

    pub fn gen_id(&mut self) {
        self.id = Some(uuid::Uuid::new_v4());
    }
}

impl From<crate::database::entity::t_camera_configs::Model> for CameraConfig {
    fn from(model: crate::database::entity::t_camera_configs::Model) -> Self {
        Self {
            id: Some(model.id),
            device_user_id: model.device_user_id,
            key: model.key,
            serial_number: model.serial_number,
            vendor: model.vendor,
            model: model.model,
            manufacture_info: model.manufacture_info,
            device_version: model.device_version,
            exposure_time_ms: model.exposure_time_ms,
            exposure_auto: model.exposure_auto,
            ip_address: model.ip_address,
            camera_supplier: CameraSupplier::from_str(&model.camera_supplier).unwrap_or(CameraSupplier::IMV),
        }
    }
}

impl From<CameraConfig> for crate::database::entity::t_camera_configs::ActiveModel {
    fn from(config: CameraConfig) -> Self {
        use sea_orm::ActiveValue;
        crate::database::entity::t_camera_configs::ActiveModel {
            id: ActiveValue::set(config.id.unwrap_or_else(uuid::Uuid::new_v4)),
            device_user_id: ActiveValue::set(config.device_user_id),
            key: ActiveValue::set(config.key),
            serial_number: ActiveValue::set(config.serial_number),
            vendor: ActiveValue::set(config.vendor),
            model: ActiveValue::set(config.model),
            manufacture_info: ActiveValue::set(config.manufacture_info),
            device_version: ActiveValue::set(config.device_version),
            exposure_time_ms: ActiveValue::set(config.exposure_time_ms),
            exposure_auto: ActiveValue::set(config.exposure_auto),
            ip_address: ActiveValue::set(config.ip_address),
            camera_supplier: ActiveValue::set(config.camera_supplier.to_string()),
            enabled: ActiveValue::set(true),
        }
    }
}

pub trait IndustryCamera: Send + Sync {
    fn open(&self) -> Result<(), CameraError>;
    fn is_opened(&self) -> bool;
    fn is_grabbing(&self) -> bool;
    fn stop_grab(&mut self) -> Result<(), CameraError>;
    fn start_grab(&mut self) -> Result<(), CameraError>;
    fn close(&self) -> Result<(), CameraError>;
    fn frame_size(&self) -> Result<FrameSize, CameraError>;
    fn trigger_one_frame(&self) -> Result<CameraFrame, CameraError>;
    fn create_frame_channel(&mut self) -> mpsc::Receiver<CameraFrame>;
    fn set_grab_mode(&mut self, grab_mode: GrabMode);
    fn set_exposure_auto(&mut self, auto: bool);
    fn set_exposure_time(&mut self, time: std::time::Duration);
}

/// Camera error types
#[derive(Debug, Clone)]
pub enum CameraError {
    /// Failed to enumerate cameras
    EnumerationError(String),

    /// Invalid camera index
    InvalidIndex(String),

    /// Camera not found
    CameraNotFound(String),

    /// Camera not opened
    NotOpened(String),

    /// Camera not grabbing
    NotGrabbing(String),

    /// Failed to open camera
    OpenError(String),

    /// Failed to close camera
    CloseError(String),

    /// Failed to start/stop grabbing
    GrabError(String),

    /// Failed to trigger frame
    TriggerError(String),

    /// Frame error
    FrameError(String),

    /// Camera handle error
    CameraHandler(String),

    /// Invalid key or name
    InvalidKeyName(String),

    InvalidDeviceUserId(String),

    InvalidIpAddress(String),

    Config(String),

    AddCamera(String),

    SystemError(String),
}

impl std::fmt::Display for CameraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CameraError::EnumerationError(msg) => write!(f, "相机枚举错误: {}", msg),
            CameraError::InvalidIndex(msg) => write!(f, "无效相机索引: {}", msg),
            CameraError::CameraNotFound(idx) => write!(f, "相机 {} 不存在", idx),
            CameraError::NotOpened(idx) => write!(f, "相机 {} 没打开", idx),
            CameraError::NotGrabbing(idx) => write!(f, "相机 {} 没抓取", idx),
            CameraError::OpenError(msg) => write!(f, "打开相机错误: {}", msg),
            CameraError::CloseError(msg) => write!(f, "关闭相机错误: {}", msg),
            CameraError::GrabError(msg) => write!(f, "抓取错误: {}", msg),
            CameraError::TriggerError(msg) => write!(f, "触发错误: {}", msg),
            CameraError::FrameError(msg) => write!(f, "帧错误: {}", msg),
            CameraError::CameraHandler(msg) => write!(f, "相机句柄错误：{}", msg),
            CameraError::InvalidKeyName(msg) => write!(f, "相机键值错误：{}", msg),
            CameraError::InvalidDeviceUserId(msg) => write!(f, "相机设备用户ID错误：{}", msg),
            CameraError::InvalidIpAddress(msg) => write!(f, "相机IP地址错误：{}", msg),
            CameraError::Config(msg) => write!(f, "相机参数配置错误：{}", msg),
            CameraError::AddCamera(msg) => write!(f, "添加相机错误：{}", msg),
            CameraError::SystemError(msg) => write!(f, "系统错误：{}", msg),
        }
    }
}

impl std::error::Error for CameraError {}
