use serde::{Deserialize, Serialize};

use crate::database::entity::t_inspection_stations::InferenceType;

/// Rectangular detection region (x, y, width, height)
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BboxRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl BboxRegion {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Get bottom-right coordinates
    pub fn x2(&self) -> f32 {
        self.x + self.width
    }

    /// Get bottom-right coordinates
    pub fn y2(&self) -> f32 {
        self.y + self.height
    }

    /// Calculate area
    pub fn area(&self) -> f32 {
        self.width * self.height
    }
}

/// Segmentation mask region
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskRegion {
    /// Mask data (binary mask, 0 or non-zero values)
    pub mask: Vec<u8>,
    /// Width of the mask
    pub width: u32,
    /// Height of the mask
    pub height: u32,
}

impl MaskRegion {
    pub fn new(mask: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            mask,
            width,
            height,
        }
    }

    /// Create an empty mask
    pub fn empty(width: u32, height: u32) -> Self {
        Self {
            mask: vec![0u8; (width * height) as usize],
            width,
            height,
        }
    }

    /// Check if a point is inside the mask
    pub fn contains(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        self.mask[(y * self.width + x) as usize] != 0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub class_name: String,
    pub class_id: i32,
    pub confidence: f32,
    /// Detection region (bounding box)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<BboxRegion>,
    /// Segmentation region (mask)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<MaskRegion>,
}

impl Detection {
    pub fn new<S: Into<String>>(class_name: S, class_id: i32, confidence: f32) -> Self {
        Self {
            class_name: class_name.into(),
            class_id,
            confidence,
            bbox: None,
            mask: None,
        }
    }

    /// Create with bounding box region
    pub fn with_bbox(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.bbox = Some(BboxRegion::new(x, y, width, height));
        self
    }

    /// Create with mask region
    pub fn with_mask(mut self, mask: Vec<u8>, width: u32, height: u32) -> Self {
        self.mask = Some(MaskRegion::new(mask, width, height));
        self
    }

    /// Create full detection with bbox and mask
    pub fn new_full<S: Into<String>>(
        class_name: S,
        class_id: i32,
        confidence: f32,
        bbox: Option<BboxRegion>,
        mask: Option<MaskRegion>,
    ) -> Self {
        Self {
            class_name: class_name.into(),
            class_id,
            confidence,
            bbox,
            mask,
        }
    }

    pub fn is_ok(&self) -> bool {
        self.class_name.to_lowercase().contains("ok")
            || self.class_name.to_lowercase().contains("good")
            || self.class_name.to_lowercase().contains("pass")
    }

    pub fn is_ng(&self) -> bool {
        !self.is_ok()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    pub detections: Vec<Detection>,
    /// Overall result (true if OK, false if NG)
    pub is_ok: bool,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Image width used for detection
    pub image_width: u32,
    /// Image height used for detection
    pub image_height: u32,
}

impl DetectionResult {
    pub fn new() -> Self {
        Self {
            detections: Vec::new(),
            is_ok: true,
            processing_time_ms: 0,
            image_width: 0,
            image_height: 0,
        }
    }

    pub fn add_detection(&mut self, detection: Detection) {
        if detection.is_ng() {
            self.is_ok = false;
        }
        self.detections.push(detection);
    }

    /// Set processing time
    pub fn with_processing_time(mut self, ms: u64) -> Self {
        self.processing_time_ms = ms;
        self
    }

    /// Set image dimensions
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.image_width = width;
        self.image_height = height;
        self
    }

    /// Get detections of a specific class
    pub fn get_detections_by_class(&self, class_name: &str) -> Vec<&Detection> {
        self.detections
            .iter()
            .filter(|d| d.class_name == class_name)
            .collect()
    }

    /// Get NG detections only
    pub fn get_ng_detections(&self) -> Vec<&Detection> {
        self.detections.iter().filter(|d| d.is_ng()).collect()
    }
}

pub trait Detector: Send + Sync {
    /// Get detector name
    fn name(&self) -> &str;

    /// Get inference type
    fn inference_type(&self) -> InferenceType {
        InferenceType::default()
    }

    /// Initialize the detector (load model, etc.)
    fn initialize(&mut self) -> Result<(), DetectorError>;

    /// Check if detector is initialized
    fn is_initialized(&self) -> bool;

    /// Perform detection on image data
    /// 
    /// # Arguments
    /// * `image_data` - Raw image bytes (e.g., Mono8, RGB8)
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `channels` - Number of color channels (1 for Mono8, 3 for RGB)
    /// * `confidence_threshold` - Minimum confidence for detections
    fn detect(
        &mut self,
        image_data: &[u8],
        width: u32,
        height: u32,
        channels: u32,
        confidence_threshold: f32,
    ) -> Result<DetectionResult, DetectorError>;

    /// Get supported class names
    fn get_class_names(&self) -> Vec<&str>;

    /// Shutdown the detector and release resources
    fn shutdown(&mut self) -> Result<(), DetectorError>;
}

/// Detector error types
#[derive(Debug, Clone)]
pub enum DetectorError {
    /// Model file not found
    ModelNotFound(String),

    /// Model loading failed
    ModelLoadError(String),

    /// Inference error
    InferenceError(String),

    /// Invalid input
    InvalidInput(String),

    /// Not initialized
    NotInitialized,

    /// Resource allocation failed
    ResourceError(String),

    /// Internal error
    Internal(String),
}

impl std::fmt::Display for DetectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectorError::ModelNotFound(path) => write!(f, "Model 找不到: {}", path),
            DetectorError::ModelLoadError(msg) => write!(f, "Model 加载错误: {}", msg),
            DetectorError::InferenceError(msg) => write!(f, "推理错误: {}", msg),
            DetectorError::InvalidInput(msg) => write!(f, "输入无效: {}", msg),
            DetectorError::NotInitialized => write!(f, "检测器未初始化"),
            DetectorError::ResourceError(msg) => write!(f, "资源错误: {}", msg),
            DetectorError::Internal(msg) => write!(f, "内部错误: {}", msg),
        }
    }
}

impl std::error::Error for DetectorError {}
