use std::path::Path;

use bytes::Bytes;
use image::DynamicImage;

use crate::service::camera::{CameraFrame, PixelFormat};

use super::detector::DetectorError;

/// Unified image data structure for ONNX inference
/// Uses `bytes::Bytes` for zero-copy when source is already RGB8
#[derive(Clone, Debug)]
pub struct InferenceImage {
    /// Raw pixel data (RGB8 format, channels last: HWC)
    /// Uses Bytes for cheap clone (reference counted, no data copy)
    pub data: Bytes,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Number of channels (always 3 for RGB output)
    pub channels: u32,
}

impl InferenceImage {
    /// Create a new InferenceImage from raw data
    pub fn new(data: impl Into<Bytes>, width: u32, height: u32, channels: u32) -> Self {
        Self {
            data: data.into(),
            width,
            height,
            channels,
        }
    }

    /// Create from CameraFrame (industrial camera frame)
    /// Converts various pixel formats to RGB8
    /// Zero-copy when pixel format is already RGB8
    pub fn from_camera_frame(frame: &CameraFrame) -> Result<Self, DetectorError> {
        let width = frame.frame_size.width as u32;
        let height = frame.frame_size.height as u32;

        let data: Bytes = match frame.pixel_format {
            // RGB8: zero-copy, just clone the Bytes handle (ref count increment)
            PixelFormat::RGB8 => frame.data.clone(),

            // Mono8: grayscale to RGB by replicating channel
            PixelFormat::Mono8 => {
                let mut rgb = Vec::with_capacity(frame.data.len() * 3);
                for &pixel in frame.data.iter() {
                    rgb.push(pixel);
                    rgb.push(pixel);
                    rgb.push(pixel);
                }
                Bytes::from(rgb)
            }

            // Mono16: convert to 8-bit RGB
            PixelFormat::Mono16 => {
                let mut rgb = Vec::with_capacity(frame.data.len() / 2 * 3);
                for chunk in frame.data.chunks(2) {
                    let pixel = ((chunk[0] as u16) | ((chunk[1] as u16) << 8)) >> 8;
                    let pixel = pixel as u8;
                    rgb.push(pixel);
                    rgb.push(pixel);
                    rgb.push(pixel);
                }
                Bytes::from(rgb)
            }

            // BGR8: swap R and B channels
            PixelFormat::BGR8 => {
                let mut rgb = Vec::with_capacity(frame.data.len());
                for chunk in frame.data.chunks(3) {
                    rgb.push(chunk[2]); // R
                    rgb.push(chunk[1]); // G
                    rgb.push(chunk[0]); // B
                }
                Bytes::from(rgb)
            }

            // RGBA8: drop alpha channel
            PixelFormat::RGBA8 => {
                let mut rgb = Vec::with_capacity(frame.data.len() * 3 / 4);
                for chunk in frame.data.chunks(4) {
                    rgb.push(chunk[0]);
                    rgb.push(chunk[1]);
                    rgb.push(chunk[2]);
                }
                Bytes::from(rgb)
            }

            // BGRA8: swap R and B, drop alpha
            PixelFormat::BGRA8 => {
                let mut rgb = Vec::with_capacity(frame.data.len() * 3 / 4);
                for chunk in frame.data.chunks(4) {
                    rgb.push(chunk[2]); // R
                    rgb.push(chunk[1]); // G
                    rgb.push(chunk[0]); // B
                }
                Bytes::from(rgb)
            }

            // Other formats: treat as Mono8
            _ => {
                tracing::warn!(
                    "Unsupported pixel format {:?}, treating as Mono8",
                    frame.pixel_format
                );
                let mut rgb = Vec::with_capacity(frame.data.len() * 3);
                for &pixel in frame.data.iter() {
                    rgb.push(pixel);
                    rgb.push(pixel);
                    rgb.push(pixel);
                }
                Bytes::from(rgb)
            }
        };

        Ok(Self {
            data,
            width,
            height,
            channels: 3,
        })
    }

    /// Load from image file using image crate
    /// Supports: JPEG, PNG, BMP, TIFF, WebP, etc.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DetectorError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(DetectorError::InvalidInput(format!(
                "Image file not found: {:?}",
                path
            )));
        }

        let img = image::open(path)
            .map_err(|e| DetectorError::InvalidInput(format!("Failed to open image: {}", e)))?;

        Self::from_dynamic_image(img)
    }

    /// Load from bytes (image data with format auto-detection)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DetectorError> {
        let img = image::load_from_memory(bytes)
            .map_err(|e| DetectorError::InvalidInput(format!("Failed to decode image: {}", e)))?;

        Self::from_dynamic_image(img)
    }

    /// Convert from DynamicImage (from image crate)
    fn from_dynamic_image(img: DynamicImage) -> Result<Self, DetectorError> {
        let width = img.width();
        let height = img.height();
        let data = Bytes::from(img.to_rgb8().into_raw());

        Ok(Self {
            data,
            width,
            height,
            channels: 3,
        })
    }

    /// Get data as byte slice
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get total number of pixels
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// Check if image is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty() || self.width == 0 || self.height == 0
    }

    /// Resize image to target dimensions
    pub fn resize(&self, target_width: u32, target_height: u32) -> Self {
        if self.width == target_width && self.height == target_height {
            return self.clone();
        }

        let img = self.to_dynamic_image();
        let resized = img.resize_exact(
            target_width,
            target_height,
            image::imageops::FilterType::Triangle,
        );
        let data = Bytes::from(resized.to_rgb8().into_raw());

        Self {
            data,
            width: target_width,
            height: target_height,
            channels: 3,
        }
    }

    /// Letterbox resize: resize while preserving aspect ratio, pad with color
    /// Returns (resized_image, scale, pad_x, pad_y)
    /// Used for YOLO preprocessing
    pub fn letterbox(
        &self,
        target_width: u32,
        target_height: u32,
        pad_color: (u8, u8, u8),
    ) -> (Self, f32, u32, u32) {
        let scale_x = target_width as f32 / self.width as f32;
        let scale_y = target_height as f32 / self.height as f32;
        let scale = scale_x.min(scale_y);

        let new_width = (self.width as f32 * scale) as u32;
        let new_height = (self.height as f32 * scale) as u32;

        let pad_x = (target_width - new_width) / 2;
        let pad_y = (target_height - new_height) / 2;

        // Resize
        let img = self.to_dynamic_image();
        let resized = img.resize_exact(new_width, new_height, image::imageops::FilterType::Triangle);
        let resized_rgb = resized.to_rgb8();

        // Create padded image
        let mut padded =
            image::ImageBuffer::from_pixel(target_width, target_height, image::Rgb([pad_color.0, pad_color.1, pad_color.2]));

        // Copy resized pixels into center
        for (x, y, pixel) in resized_rgb.enumerate_pixels() {
            padded.put_pixel(pad_x + x, pad_y + y, *pixel);
        }

        let data = Bytes::from(padded.into_raw());
        let result = Self {
            data,
            width: target_width,
            height: target_height,
            channels: 3,
        };

        (result, scale, pad_x, pad_y)
    }

    /// Convert to DynamicImage for image crate operations
    fn to_dynamic_image(&self) -> DynamicImage {
        let img_buffer = image::ImageBuffer::from_raw(self.width, self.height, self.data.to_vec())
            .expect("Invalid image buffer");
        DynamicImage::ImageRgb8(img_buffer)
    }
}

impl Default for InferenceImage {
    fn default() -> Self {
        Self {
            data: Bytes::new(),
            width: 0,
            height: 0,
            channels: 3,
        }
    }
}
