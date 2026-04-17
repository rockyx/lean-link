use std::path::Path;
use std::time::Instant;

use ndarray::Array;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::ValueType;
use ort::{inputs, value::TensorRef};
use tracing::{debug, info};

use crate::database::entity::t_inspection_stations::InferenceType;
use crate::service::inspection::detector::{
    BboxRegion, Detection, DetectionResult, Detector, DetectorError,
};

/// YOLO ONNX inference input information
#[derive(Debug, Clone)]
pub struct InputInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub channels: u32,
}

/// YOLO ONNX inference output information
#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub name: String,
    pub dimensions: Vec<i64>,
}

/// Bounding box with coordinates
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

/// Detection with bounding box and optional mask coefficients
#[derive(Debug, Clone)]
pub struct YoloDetection {
    pub bbox: BoundingBox,
    pub class_id: usize,
    pub confidence: f32,
    /// Mask coefficients for segmentation (32 coefficients for YOLOv8-seg)
    pub mask_coeffs: Option<Vec<f32>>,
}

/// YOLO ONNX inference implementation
pub struct OnnxInference {
    model_path: String,
    name: String,
    inference_type: InferenceType,
    session: Option<Session>,
    input_info: Option<InputInfo>,
    output_infos: Vec<OutputInfo>,
    class_names: Vec<String>,
    /// Pre-allocated input buffer for performance
    input_buffer: Option<Vec<f32>>,
    /// Number of mask prototypes (typically 32 for YOLOv8-seg)
    num_masks: usize,
    /// Mask prototype dimensions (from second output)
    mask_proto_height: usize,
    mask_proto_width: usize,
}

impl OnnxInference {
    /// Create a new ONNX inference instance
    pub fn new<S: Into<String>>(model_path: S, name: S) -> Self {
        Self {
            model_path: model_path.into(),
            name: name.into(),
            inference_type: InferenceType::default(),
            session: None,
            input_info: None,
            output_infos: Vec::new(),
            class_names: vec![],
            input_buffer: None,
            num_masks: 0,
            mask_proto_height: 0,
            mask_proto_width: 0,
        }
    }

    /// Create with custom class names
    pub fn with_class_names<S: Into<String>>(mut self, class_names: Vec<S>) -> Self {
        self.class_names = class_names.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set inference type
    pub fn with_inference_type(mut self, inference_type: InferenceType) -> Self {
        self.inference_type = inference_type;
        self
    }

    /// Load class names from a file (one class name per line)
    pub fn load_class_names_from_file<P: AsRef<Path>>(
        mut self,
        path: P,
    ) -> Result<Self, DetectorError> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            DetectorError::ModelLoadError(format!("Failed to read class names file: {}", e))
        })?;
        self.class_names = content.lines().map(|s| s.trim().to_string()).collect();
        if self.class_names.is_empty() {
            return Err(DetectorError::ModelLoadError(
                "Class names file is empty".to_string(),
            ));
        }
        Ok(self)
    }

    /// Extract input/output information from loaded session
    fn extract_model_info(&mut self) -> Result<(), DetectorError> {
        let session = self.session.as_ref().ok_or(DetectorError::NotInitialized)?;

        // Get input information using ort 2.0 API
        let inputs = session.inputs();
        let input = inputs
            .first()
            .ok_or_else(|| DetectorError::ModelLoadError("No input found in model".to_string()))?;

        // Get input dimensions from ValueType
        let dtype = input.dtype();
        let (shape, _ty) = match dtype {
            ValueType::Tensor { shape, ty, .. } => (shape, ty),
            _ => {
                return Err(DetectorError::ModelLoadError(
                    "Input is not a tensor type".to_string(),
                ));
            }
        };

        if shape.len() < 4 {
            return Err(DetectorError::ModelLoadError(
                "Expected 4D input tensor (NCHW)".to_string(),
            ));
        }

        // Usually NCHW format: [batch, channels, height, width]
        // Note: dimensions might be -1 for dynamic sizes
        let channels = if shape[1] < 0 { 3 } else { shape[1] as u32 };
        let height = if shape[2] < 0 { 640 } else { shape[2] as u32 };
        let width = if shape[3] < 0 { 640 } else { shape[3] as u32 };

        self.input_info = Some(InputInfo {
            name: input.name().to_string(),
            width,
            height,
            channels,
        });

        info!(
            "Model '{}' input: {}x{}x{} (CxHxW), name: {}",
            self.name,
            channels,
            height,
            width,
            input.name()
        );

        // Get output information
        self.output_infos.clear();
        let outputs = session.outputs();
        for output in outputs {
            let dims: Vec<i64> = match output.dtype() {
                ValueType::Tensor { shape, .. } => shape.iter().copied().collect(),
                _ => Vec::new(),
            };
            let output_name = output.name().to_string();
            self.output_infos.push(OutputInfo {
                name: output_name,
                dimensions: dims,
            });
        }

        // Parse mask prototype info for segmentation models
        // YOLOv8-seg has two outputs:
        // - output0: [1, 4 + num_classes + num_masks, num_detections]
        // - output1 (proto): [1, num_masks, mask_height, mask_width]
        if self.inference_type.has_segmentation() && self.output_infos.len() >= 2 {
            let proto_info = &self.output_infos[1];
            if proto_info.dimensions.len() == 4 {
                self.num_masks = proto_info.dimensions[1] as usize;
                self.mask_proto_height = proto_info.dimensions[2] as usize;
                self.mask_proto_width = proto_info.dimensions[3] as usize;
                info!(
                    "Model '{}' mask prototypes: {}x{}x{}",
                    self.name, self.num_masks, self.mask_proto_height, self.mask_proto_width
                );
            }
        }

        // Pre-allocate input buffer
        let buffer_size = (channels * height * width) as usize;
        self.input_buffer = Some(vec![0.0f32; buffer_size]);

        Ok(())
    }

    /// Preprocess image for YOLO inference
    /// Converts image to RGB, resizes to model input size, normalizes to [0, 1]
    fn preprocess_image(
        &self,
        image_data: &[u8],
        orig_width: u32,
        orig_height: u32,
        channels: u32,
    ) -> Result<(Vec<f32>, f32, f32), DetectorError> {
        let input_info = self
            .input_info
            .as_ref()
            .ok_or(DetectorError::NotInitialized)?;

        let target_width = input_info.width as usize;
        let target_height = input_info.height as usize;
        let target_channels = input_info.channels as usize;

        // Calculate scale factor for letterbox
        let scale_x = orig_width as f32 / target_width as f32;
        let scale_y = orig_height as f32 / target_height as f32;
        let scale = scale_x.max(scale_y);

        // Create output buffer (CHW format)
        let mut output = vec![0.0f32; target_channels * target_width * target_height];

        // Ensure input channels match
        let input_channels = channels as usize;
        if input_channels != target_channels && !(input_channels == 1 && target_channels == 3) {
            return Err(DetectorError::InvalidInput(format!(
                "Input channels {} does not match model channels {}",
                input_channels, target_channels
            )));
        }

        // Simple resize with bilinear-like approach
        // For better quality, consider using image crate's resize
        let img = if input_channels == 1 && target_channels == 3 {
            // Convert grayscale to RGB
            let mut rgb_data = Vec::with_capacity(image_data.len() * 3);
            for &pixel in image_data {
                rgb_data.push(pixel);
                rgb_data.push(pixel);
                rgb_data.push(pixel);
            }
            rgb_data
        } else {
            image_data.to_vec()
        };

        // Resize and normalize (0-255 -> 0-1)
        // Output is in CHW format for YOLO
        for y in 0..target_height {
            for x in 0..target_width {
                // Map target coordinates to source coordinates
                let src_x = ((x as f32 * scale).min(orig_width as f32 - 1.0)) as usize;
                let src_y = ((y as f32 * scale).min(orig_height as f32 - 1.0)) as usize;

                for c in 0..target_channels {
                    let src_idx = (src_y * orig_width as usize + src_x) * target_channels + c;
                    // CHW format: [channel][height][width]
                    let dst_idx = c * target_width * target_height + y * target_width + x;

                    if src_idx < img.len() {
                        output[dst_idx] = img[src_idx] as f32 / 255.0;
                    }
                }
            }
        }

        Ok((output, scale, scale))
    }

    /// Post-process YOLO output to extract detections
    fn postprocess_output(
        &self,
        shape: &[i64],
        data: &[f32],
        orig_width: u32,
        orig_height: u32,
        scale_x: f32,
        scale_y: f32,
        confidence_threshold: f32,
    ) -> Vec<YoloDetection> {
        let mut detections = Vec::new();

        let input_info = match &self.input_info {
            Some(info) => info,
            None => return detections,
        };

        // YOLO output shape is typically [batch, num_detections, 5 + num_classes]
        // or [batch, 5 + num_classes, num_detections] depending on version
        // For YOLOv8-seg: [batch, 4 + num_classes + num_masks, num_detections]
        debug!(
            "Output shape: {:?}, input: {}x{}",
            shape, input_info.width, input_info.height
        );

        // Handle different output formats
        if shape.len() == 3 {
            // Get dimensions
            let _batch = shape[0] as usize;
            let dim1 = shape[1] as usize;
            let dim2 = shape[2] as usize;

            // Determine if shape is [1, num_detections, attrs] or [1, attrs, num_detections]
            let (num_detections, attrs_len) = if dim1 > dim2 {
                (dim1, dim2)
            } else {
                (dim2, dim1)
            };

            // Calculate number of classes based on attributes
            // attrs_len = 4 (bbox) + num_classes + num_masks (for segmentation)
            let num_classes = if self.inference_type.has_segmentation() && self.num_masks > 0 {
                attrs_len.saturating_sub(4 + self.num_masks)
            } else {
                attrs_len.saturating_sub(4)
            };

            let has_masks = self.inference_type.has_segmentation() && self.num_masks > 0;

            for i in 0..num_detections {
                // Extract detection data based on shape
                let (cx, cy, w, h, class_scores, mask_coeffs): (
                    f32,
                    f32,
                    f32,
                    f32,
                    Vec<f32>,
                    Option<Vec<f32>>,
                ) = if dim1 > dim2 {
                    // Shape: [1, num_detections, attrs]
                    let base = i * attrs_len;
                    let cx = data[base];
                    let cy = data[base + 1];
                    let w = data[base + 2];
                    let h = data[base + 3];
                    let scores: Vec<f32> = (4..4 + num_classes).map(|c| data[base + c]).collect();

                    // Extract mask coefficients for segmentation
                    let masks = if has_masks {
                        let mask_start = 4 + num_classes;
                        let coeffs: Vec<f32> = (mask_start..mask_start + self.num_masks)
                            .map(|c| data[base + c])
                            .collect();
                        Some(coeffs)
                    } else {
                        None
                    };

                    (cx, cy, w, h, scores, masks)
                } else {
                    // Shape: [1, attrs, num_detections]
                    let cx = data[i];
                    let cy = data[num_detections + i];
                    let w = data[2 * num_detections + i];
                    let h = data[3 * num_detections + i];
                    let scores: Vec<f32> = (4..4 + num_classes)
                        .map(|c| data[c * num_detections + i])
                        .collect();

                    // Extract mask coefficients for segmentation
                    let masks = if has_masks {
                        let coeffs: Vec<f32> = (4 + num_classes..4 + num_classes + self.num_masks)
                            .map(|c| data[c * num_detections + i])
                            .collect();
                        Some(coeffs)
                    } else {
                        None
                    };

                    (cx, cy, w, h, scores, masks)
                };

                // Find max class score
                let mut max_class_score = 0.0f32;
                let mut max_class_id = 0usize;

                for (c, &score) in class_scores.iter().enumerate() {
                    if score > max_class_score {
                        max_class_score = score;
                        max_class_id = c;
                    }
                }

                if max_class_score < confidence_threshold {
                    continue;
                }

                // Extract bounding box (convert from center format)
                let x1 = cx - w / 2.0;
                let y1 = cy - h / 2.0;
                let x2 = cx + w / 2.0;
                let y2 = cy + h / 2.0;

                // Scale back to original image
                let x1 = (x1 * scale_x).clamp(0.0, orig_width as f32);
                let y1 = (y1 * scale_y).clamp(0.0, orig_height as f32);
                let x2 = (x2 * scale_x).clamp(0.0, orig_width as f32);
                let y2 = (y2 * scale_y).clamp(0.0, orig_height as f32);

                detections.push(YoloDetection {
                    bbox: BoundingBox { x1, y1, x2, y2 },
                    class_id: max_class_id,
                    confidence: max_class_score,
                    mask_coeffs,
                });
            }
        }

        // Apply NMS (Non-Maximum Suppression)
        self.apply_nms(&mut detections, 0.5);

        detections
    }

    /// Apply Non-Maximum Suppression
    fn apply_nms(&self, detections: &mut Vec<YoloDetection>, iou_threshold: f32) {
        detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        let mut keep = vec![true; detections.len()];

        for i in 0..detections.len() {
            if !keep[i] {
                continue;
            }

            for j in (i + 1)..detections.len() {
                if !keep[j] {
                    continue;
                }

                if detections[i].class_id == detections[j].class_id {
                    let iou = self.calculate_iou(&detections[i].bbox, &detections[j].bbox);
                    if iou > iou_threshold {
                        keep[j] = false;
                    }
                }
            }
        }

        let mut idx = 0;
        detections.retain(|_| {
            let keep_flag = keep[idx];
            idx += 1;
            keep_flag
        });
    }

    /// Calculate Intersection over Union
    fn calculate_iou(&self, a: &BoundingBox, b: &BoundingBox) -> f32 {
        let x1 = a.x1.max(b.x1);
        let y1 = a.y1.max(b.y1);
        let x2 = a.x2.min(b.x2);
        let y2 = a.y2.min(b.y2);

        let intersection = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
        let area_a = (a.x2 - a.x1) * (a.y2 - a.y1);
        let area_b = (b.x2 - b.x1) * (b.y2 - b.y1);
        let union = area_a + area_b - intersection;

        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }

    /// Generate mask from prototypes and coefficients
    ///
    /// # Arguments
    /// * `proto_data` - Prototype masks from second output [1, num_masks, H, W]
    /// * `mask_coeffs` - Mask coefficients for this detection [num_masks]
    /// * `bbox` - Bounding box to crop the mask
    /// * `orig_width` - Original image width
    /// * `orig_height` - Original image height
    fn generate_mask(
        &self,
        proto_data: &[f32],
        mask_coeffs: &[f32],
        bbox: &BoundingBox,
        _orig_width: u32,
        _orig_height: u32,
        mask_threshold: f32,
    ) -> Option<Vec<u8>> {
        if self.num_masks == 0 || self.mask_proto_height == 0 || self.mask_proto_width == 0 {
            return None;
        }

        let proto_h = self.mask_proto_height;
        let proto_w = self.mask_proto_width;
        let num_masks = self.num_masks;

        // Compute mask = sigmoid(mask_coeffs @ proto_masks)
        // proto_data shape: [num_masks, H, W]
        // mask_coeffs shape: [num_masks]
        // Result shape: [H, W]
        let mut mask = vec![0.0f32; proto_h * proto_w];

        for y in 0..proto_h {
            for x in 0..proto_w {
                let mut sum = 0.0f32;
                for m in 0..num_masks {
                    // proto_data is in [num_masks, H, W] format
                    let proto_idx = m * proto_h * proto_w + y * proto_w + x;
                    sum += mask_coeffs[m] * proto_data[proto_idx];
                }
                // Apply sigmoid
                mask[y * proto_w + x] = 1.0 / (1.0 + (-sum).exp());
            }
        }

        // Scale bbox coordinates to mask prototype space
        let input_info = self.input_info.as_ref()?;
        let scale_x_proto = proto_w as f32 / input_info.width as f32;
        let scale_y_proto = proto_h as f32 / input_info.height as f32;

        // Create output mask at original image resolution
        // We'll create a mask that fits the bbox
        let bbox_w = (bbox.x2 - bbox.x1).max(0.0) as usize;
        let bbox_h = (bbox.y2 - bbox.y1).max(0.0) as usize;

        if bbox_w == 0 || bbox_h == 0 {
            return None;
        }

        let mut output_mask = vec![0u8; bbox_w * bbox_h];

        // Map from original image bbox to mask prototype and then threshold
        for y in 0..bbox_h {
            for x in 0..bbox_w {
                // Map to prototype coordinates
                let proto_x = ((bbox.x1 + x as f32) * scale_x_proto) as usize;
                let proto_y = ((bbox.y1 + y as f32) * scale_y_proto) as usize;

                if proto_x < proto_w && proto_y < proto_h {
                    let mask_val = mask[proto_y * proto_w + proto_x];
                    output_mask[y * bbox_w + x] = if mask_val > mask_threshold { 255 } else { 0 };
                }
            }
        }

        Some(output_mask)
    }

    /// Get input info
    pub fn get_input_info(&self) -> Option<&InputInfo> {
        self.input_info.as_ref()
    }

    /// Get output infos
    pub fn get_output_infos(&self) -> &[OutputInfo] {
        &self.output_infos
    }
}

impl Detector for OnnxInference {
    fn name(&self) -> &str {
        &self.name
    }

    fn inference_type(&self) -> InferenceType {
        self.inference_type
    }

    fn initialize(&mut self) -> Result<(), DetectorError> {
        info!("Initializing ONNX model: {}", self.model_path);

        // Check if model file exists
        if !Path::new(&self.model_path).exists() {
            return Err(DetectorError::ModelNotFound(self.model_path.clone()));
        }

        // Build session with optimizations
        let session = Session::builder()
            .map_err(|e| DetectorError::ModelLoadError(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| DetectorError::ModelLoadError(e.to_string()))?
            .with_intra_threads(4)
            .map_err(|e| DetectorError::ModelLoadError(e.to_string()))?
            .commit_from_file(&self.model_path)
            .map_err(|e| DetectorError::ModelLoadError(format!("{}: {}", self.model_path, e)))?;

        self.session = Some(session);

        // Extract model info
        self.extract_model_info()?;

        info!("Model '{}' initialized successfully", self.name);
        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.session.is_some() && self.input_info.is_some()
    }

    fn detect(
        &mut self,
        image_data: &[u8],
        width: u32,
        height: u32,
        channels: u32,
        confidence_threshold: f32,
    ) -> Result<DetectionResult, DetectorError> {
        let start = Instant::now();

        if !self.is_initialized() {
            return Err(DetectorError::NotInitialized);
        }

        let input_info = self.input_info.as_ref().unwrap().clone();

        // Preprocess image
        let (input_data, scale_x, scale_y) =
            self.preprocess_image(image_data, width, height, channels)?;

        // Create input tensor with shape [1, C, H, W]
        let input_array: Array<f32, _> = Array::from_shape_vec(
            (
                1usize,
                input_info.channels as usize,
                input_info.height as usize,
                input_info.width as usize,
            ),
            input_data,
        )
        .map_err(|e| {
            DetectorError::InferenceError(format!("Failed to create input tensor: {}", e))
        })?;

        // Get class names for later use
        let class_names = self.class_names.clone();
        let inference_type = self.inference_type;

        // Run inference - ort 2.0 API
        let (output_shape, output_data, _proto_shape, proto_data) = {
            let session = self.session.as_mut().unwrap();
            let outputs = session
                .run(inputs![TensorRef::from_array_view(&input_array).map_err(
                    |e| {
                        DetectorError::InferenceError(format!(
                            "Failed to create input tensor: {}",
                            e
                        ))
                    }
                )?])
                .map_err(|e| DetectorError::InferenceError(format!("Inference failed: {}", e)))?;

            // Get first output (detection output)
            let (det_shape, det_data) = if outputs.len() > 0 {
                let output_value = &outputs[0];
                if let Ok((shape, data)) = output_value.try_extract_tensor::<f32>() {
                    (
                        Some(shape.iter().copied().collect::<Vec<i64>>()),
                        Some(data.to_vec()),
                    )
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            // Get second output (mask prototypes) for segmentation
            let (mask_shape, mask_data) = if inference_type.has_segmentation() && outputs.len() > 1
            {
                let proto_value = &outputs[1];
                if let Ok((shape, data)) = proto_value.try_extract_tensor::<f32>() {
                    (
                        Some(shape.iter().copied().collect::<Vec<i64>>()),
                        Some(data.to_vec()),
                    )
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            (det_shape, det_data, mask_shape, mask_data)
        };

        // Process output
        let mut result = DetectionResult::new()
            .with_dimensions(width, height)
            .with_processing_time(start.elapsed().as_millis() as u64);

        // Post-process detections
        if let (Some(shape_vec), Some(data_vec)) = (output_shape, output_data) {
            let yolo_detections = self.postprocess_output(
                &shape_vec,
                &data_vec,
                width,
                height,
                scale_x,
                scale_y,
                confidence_threshold,
            );

            let proto_ref = proto_data.as_ref();

            for det in yolo_detections {
                let class_name = class_names
                    .get(det.class_id)
                    .cloned()
                    .unwrap_or_else(|| format!("class_{}", det.class_id));

                // Convert from x1,y1,x2,y2 to x,y,w,h format
                let bbox = BboxRegion::new(
                    det.bbox.x1,
                    det.bbox.y1,
                    det.bbox.x2 - det.bbox.x1,
                    det.bbox.y2 - det.bbox.y1,
                );

                let mut detection = Detection::new(class_name, det.class_id as i32, det.confidence)
                    .with_bbox(bbox.x, bbox.y, bbox.width, bbox.height);

                // Generate mask for segmentation
                if inference_type.has_segmentation() {
                    if let (Some(proto), Some(ref coeffs)) = (proto_ref, det.mask_coeffs) {
                        if let Some(mask) = self.generate_mask(
                            proto, coeffs, &det.bbox, width, height, 0.5, // mask threshold
                        ) {
                            let mask_w = (det.bbox.x2 - det.bbox.x1).max(0.0) as u32;
                            let mask_h = (det.bbox.y2 - det.bbox.y1).max(0.0) as u32;
                            detection = detection.with_mask(mask, mask_w, mask_h);
                        }
                    }
                }

                result.add_detection(detection);
            }
        }

        result.processing_time_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    fn get_class_names(&self) -> Vec<&str> {
        self.class_names.iter().map(|s| s.as_str()).collect()
    }

    fn shutdown(&mut self) -> Result<(), DetectorError> {
        if let Some(session) = self.session.take() {
            drop(session);
        }
        self.input_info = None;
        self.output_infos.clear();
        self.input_buffer = None;
        self.num_masks = 0;
        self.mask_proto_height = 0;
        self.mask_proto_width = 0;
        info!("Model '{}' shutdown complete", self.name);
        Ok(())
    }
}

impl Drop for OnnxInference {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageReader;

    const TEST_MODEL_PATH: &str =
        r"C:\Users\fz_ka\Development\yolo\runs\detect\line1\weights\best.onnx";
    const TEST_IMAGE_PATH: &str =
        r"C:\Users\fz_ka\Development\yolo\line1\val\images\0c6882b2-Original_1753884955098.jpg";

    #[test]
    fn test_onnx_inference_new() {
        let inference = OnnxInference::new("/path/to/model.onnx", "test_model");
        assert_eq!(inference.name(), "test_model");
        assert!(!inference.is_initialized());
    }

    #[test]
    fn test_onnx_inference_with_class_names() {
        let inference = OnnxInference::new("/path/to/model.onnx", "test_model")
            .with_class_names(vec!["OK".to_string(), "NG".to_string()]);
        assert_eq!(inference.class_names, vec!["OK", "NG"]);
    }

    #[test]
    fn test_initialize_model_not_found() {
        let mut inference = OnnxInference::new("/nonexistent/path/model.onnx", "test_model");
        let result = inference.initialize();
        assert!(result.is_err());
        match result {
            Err(DetectorError::ModelNotFound(_)) => {}
            _ => panic!("Expected ModelNotFound error"),
        }
    }

    #[test]
    fn test_iou_calculation() {
        let inference = OnnxInference::new("", "test");
        let a = BoundingBox {
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 10.0,
        };
        let b = BoundingBox {
            x1: 5.0,
            y1: 5.0,
            x2: 15.0,
            y2: 15.0,
        };
        let iou = inference.calculate_iou(&a, &b);
        assert!(iou > 0.0 && iou < 1.0);
    }

    #[test]
    #[ignore = "Requires model file and ONNX runtime"]
    fn test_full_inference_with_model() {
        let model_path = TEST_MODEL_PATH;
        if !Path::new(model_path).exists() {
            println!("Skipping test - model file not found: {}", model_path);
            return;
        }

        let mut inference = OnnxInference::new(model_path, "yolo_test");

        // Initialize
        inference.initialize().expect("Failed to initialize model");

        assert!(inference.is_initialized());

        // Check input info
        let input_info = inference.get_input_info().expect("No input info");
        println!(
            "Input: {}x{}x{}",
            input_info.channels, input_info.height, input_info.width
        );

        // Check output infos
        for output in inference.get_output_infos() {
            println!("Output '{}': {:?}", output.name, output.dimensions);
        }

        // Shutdown
        inference.shutdown().expect("Failed to shutdown");
        assert!(!inference.is_initialized());
    }

    #[test]
    #[ignore = "Requires model file, test image and ONNX runtime"]
    fn test_detect_with_image() {
        let model_path = TEST_MODEL_PATH;
        let image_path = Path::new(TEST_IMAGE_PATH);

        if !Path::new(model_path).exists() {
            println!("Skipping test - model file not found: {}", model_path);
            return;
        }

        if !image_path.exists() {
            println!("Skipping test - image file not found: {:?}", image_path);
            return;
        }

        // Initialize inference
        let mut inference = OnnxInference::new(model_path, "yolo_detect_test")
            .with_class_names(vec!["ConnectorNG", "ConnectorOK", "LineNG", "LineOK"]);
        inference.initialize().expect("Failed to initialize model");

        // Load and process test image
        let img = ImageReader::open(image_path)
            .expect("Failed to open image")
            .decode()
            .expect("Failed to decode image");

        let width = img.width();
        let height = img.height();
        println!("Image size: {}x{}", width, height);

        // Convert to RGB bytes
        let rgb_img = img.to_rgb8();
        let image_data: Vec<u8> = rgb_img.into_raw();

        // Run detection
        let result = inference
            .detect(&image_data, width, height, 3, 0.5)
            .expect("Detection failed");

        println!("Detection result:");
        println!("  - Processing time: {}ms", result.processing_time_ms);
        println!(
            "  - Image size: {}x{}",
            result.image_width, result.image_height
        );
        println!("  - Is OK: {}", result.is_ok);
        println!("  - Detections: {}", result.detections.len());

        for det in &result.detections {
            println!(
                "    - {}: class={}, confidence={:.3}",
                det.class_name, det.class_id, det.confidence
            );
            if let Some(bbox) = &det.bbox {
                println!(
                    "      bbox: x={}, y={}, w={}, h={}",
                    bbox.x, bbox.y, bbox.width, bbox.height
                );
            }
            if let Some(mask) = &det.mask {
                println!(
                    "      mask: {}x{} ({} bytes)",
                    mask.width,
                    mask.height,
                    mask.mask.len()
                );
            }
        }

        // Cleanup
        inference.shutdown().expect("Failed to shutdown");
    }

    #[test]
    #[ignore = "Requires model file and ONNX runtime"]
    fn test_empty_image_detection() {
        let model_path = TEST_MODEL_PATH;

        if !Path::new(model_path).exists() {
            println!("Skipping test - model file not found: {}", model_path);
            return;
        }

        let mut inference = OnnxInference::new(model_path, "empty_test");
        inference.initialize().expect("Failed to initialize model");

        // Get model input dimensions
        let input_info = inference.get_input_info().unwrap();
        let width = input_info.width;
        let height = input_info.height;

        // Create blank image (all zeros)
        let image_data = vec![0u8; (width * height * 3) as usize];

        let result = inference
            .detect(&image_data, width, height, 3, 0.5)
            .expect("Detection failed");

        println!("Empty image detection result:");
        println!("  - Processing time: {}ms", result.processing_time_ms);
        println!("  - Detections: {}", result.detections.len());

        inference.shutdown().expect("Failed to shutdown");
    }

    #[test]
    #[ignore = "Requires model file and ONNX runtime"]
    fn test_batch_detection_performance() {
        let model_path = TEST_MODEL_PATH;

        if !Path::new(model_path).exists() {
            println!("Skipping test - model file not found: {}", model_path);
            return;
        }

        let mut inference = OnnxInference::new(model_path, "perf_test");
        inference.initialize().expect("Failed to initialize model");

        let input_info = inference.get_input_info().unwrap();
        let width = input_info.width;
        let height = input_info.height;

        // Create test image data
        let image_data: Vec<u8> = (0..(width * height * 3)).map(|i| (i % 256) as u8).collect();

        // Run multiple detections
        let iterations = 10;
        let mut total_time = 0u64;

        for i in 0..iterations {
            let result = inference
                .detect(&image_data, width, height, 3, 0.5)
                .expect("Detection failed");
            total_time += result.processing_time_ms;

            if i == 0 {
                println!(
                    "First run (includes warm-up): {}ms",
                    result.processing_time_ms
                );
            }
        }

        let avg_time = total_time / iterations;
        println!(
            "Average processing time over {} iterations: {}ms",
            iterations, avg_time
        );

        inference.shutdown().expect("Failed to shutdown");
    }

    #[test]
    #[ignore = "Requires segmentation model file and ONNX runtime"]
    fn test_segmentation_with_image() {
        // Update these paths for your segmentation model
        let model_path = r"C:\path\to\yolov8n-seg.onnx";
        let image_path = Path::new(TEST_IMAGE_PATH);

        if !Path::new(model_path).exists() {
            println!("Skipping test - model file not found: {}", model_path);
            return;
        }

        if !image_path.exists() {
            println!("Skipping test - image file not found: {:?}", image_path);
            return;
        }

        // Initialize inference with segmentation type
        let mut inference = OnnxInference::new(model_path, "yolo_seg_test")
            .with_inference_type(InferenceType::DetectionAndSegmentation)
            .with_class_names(vec!["ConnectorNG", "ConnectorOK", "LineNG", "LineOK"]);
        inference.initialize().expect("Failed to initialize model");

        // Check mask prototype info
        println!("Inference type: {:?}", inference.inference_type());
        println!("Number of masks: {}", inference.num_masks);
        println!(
            "Mask prototype size: {}x{}",
            inference.mask_proto_width, inference.mask_proto_height
        );

        // Load and process test image
        let img = ImageReader::open(image_path)
            .expect("Failed to open image")
            .decode()
            .expect("Failed to decode image");

        let width = img.width();
        let height = img.height();
        println!("Image size: {}x{}", width, height);

        // Convert to RGB bytes
        let rgb_img = img.to_rgb8();
        let image_data: Vec<u8> = rgb_img.into_raw();

        // Run detection
        let result = inference
            .detect(&image_data, width, height, 3, 0.5)
            .expect("Detection failed");

        println!("\nDetection result:");
        println!("  - Processing time: {}ms", result.processing_time_ms);
        println!("  - Is OK: {}", result.is_ok);
        println!("  - Detections: {}", result.detections.len());

        for (i, det) in result.detections.iter().enumerate() {
            println!(
                "\n  Detection {}: {} (class={}, conf={:.3})",
                i, det.class_name, det.class_id, det.confidence
            );
            if let Some(bbox) = &det.bbox {
                println!(
                    "    BBox: x={:.1}, y={:.1}, w={:.1}, h={:.1}",
                    bbox.x, bbox.y, bbox.width, bbox.height
                );
            }
            if let Some(mask) = &det.mask {
                // Count non-zero pixels
                let non_zero: usize = mask.mask.iter().filter(|&&p| p > 0).count();
                println!(
                    "    Mask: {}x{}, {} non-zero pixels ({:.1}% coverage)",
                    mask.width,
                    mask.height,
                    non_zero,
                    100.0 * non_zero as f32 / (mask.width * mask.height) as f32
                );
            }
        }

        // Cleanup
        inference.shutdown().expect("Failed to shutdown");
    }
}
