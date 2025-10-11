use tokio::sync::mpsc;

use crate::errors::Error;

#[cfg(feature = "imv-camera")]
pub mod imv_camera;

pub struct CameraInfo {
    pub key: String,
    pub name: String,
    pub serial_number: String,
    pub vendor: String,
    pub model: String,
    pub manufacture_info: String,
    pub device_version: String,
}

#[derive(Clone, PartialEq, Eq, Copy)]
pub enum GrabMode {
    Continuous,
    SingleFrame,
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
pub struct FrameSize {
    pub width: usize,
    pub height: usize,
}

#[derive(Clone)]
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

pub trait IndustryCamera {
    fn get_camera_list() -> Result<Vec<CameraInfo>, Error>;
    fn open(&self) -> Result<(), Error>;
    fn is_opened(&self) -> bool;
    fn is_grabbing(&self) -> bool;
    fn stop_grab(&self) -> Result<(), Error>;
    fn start_grab(&mut self) -> Result<(), Error>;
    fn close(&self) -> Result<(), Error>;
    fn frame_size(&self) -> Result<FrameSize, Error>;
    fn trigger_one_frame(&self) -> Result<(), Error>;
    fn create_frame_channel(&mut self) -> mpsc::Receiver<CameraFrame>;
    fn set_grab_mode(&mut self, grab_mode: GrabMode);
    fn set_exposure_auto(&mut self, auto: bool);
    fn set_exposure_time(&mut self, time: std::time::Duration);
}