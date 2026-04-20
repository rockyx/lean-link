use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use ndarray::Array3;
use ort::inputs;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;
use ort::value::ValueType;
use tracing::info;

use crate::database::entity::t_inspection_stations::InferenceType;
use crate::service::inspection::detector::{
    BboxRegion, Detection, DetectionResult, Detector, DetectorError, MaskRegion,
};
use crate::service::inspection::image::InferenceImage;
use std::collections::HashSet;

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

/// Segmentation result with bounding box and mask coefficients (for segmentation models)
#[derive(Debug, Clone)]
pub struct YoloSegmentation {
    pub bbox: BoundingBox,
    pub class_id: usize,
    pub confidence: f32,
    /// Mask coefficients for this detection (typically 32 coefficients for YOLOv8-seg)
    pub mask_coeffs: Vec<f32>,
}

/// YOLO ONNX inference implementation
pub struct OnnxInference {
    model_path: String,
    name: String,
    inference_type: InferenceType,
    session: Option<Session>,
    input_info: Option<InputInfo>,
    output_infos: Vec<OutputInfo>,
    class_names: HashMap<usize, String>,
    /// Pre-allocated input buffer for performance
    input_buffer: Option<Vec<f32>>,
    /// Number of mask prototypes (typically 32 for YOLOv8-seg, 0 for detection-only models)
    num_masks: usize,
    /// Mask prototype height (from second output for segmentation models)
    mask_proto_height: usize,
    /// Mask prototype width (from second output for segmentation models)
    mask_proto_width: usize,
    /// Task type from model metadata (e.g., "detect", "segment", "classify", "pose")
    task: Option<String>,
    /// Image size from model metadata [height, width]
    imgsz: Option<(u32, u32)>,
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
            class_names: HashMap::new(),
            input_buffer: None,
            num_masks: 0,
            mask_proto_height: 0,
            mask_proto_width: 0,
            task: None,
            imgsz: None,
        }
    }

    /// Create with custom class names
    pub fn with_class_names(mut self, class_names: HashMap<usize, String>) -> Self {
        self.class_names = class_names;
        self
    }

    /// Set inference type
    pub fn with_inference_type(mut self, inference_type: InferenceType) -> Self {
        self.inference_type = inference_type;
        self
    }

    /// Extract class names from ONNX model metadata
    /// Supports Ultralytics YOLO format: {"0": "person", "1": "car", ...}
    fn extract_class_names_from_metadata(&mut self) {
        let session = match &self.session {
            Some(s) => s,
            None => return,
        };

        // Skip if class names are already set
        if !self.class_names.is_empty() {
            tracing::info!("class_names provided");
            return;
        }

        let metadata = match session.metadata() {
            Ok(m) => m,
            Err(_) => return,
        };

        // Try "names" key (Ultralytics format)
        if let Some(names_json) = metadata.custom("names") {
            if let Ok(names) = Self::parse_ultralytics_names(&names_json) {
                if !names.is_empty() {
                    info!(
                        "Model '{}' loaded {} class names from metadata: {:?}",
                        self.name,
                        names.len(),
                        names.clone(),
                    );
                    self.class_names = names;
                }
            }
        }
    }

    /// Parse Ultralytics YOLO names format: {"0": "person", "1": "car", ...} or {0: 'person', 1: 'car', ...}
    fn parse_ultralytics_names(json: &str) -> Result<HashMap<usize, String>, ()> {
        let mut names: std::collections::HashMap<usize, String> = std::collections::HashMap::new();

        let json = json.trim();
        if !json.starts_with('{') || !json.ends_with('}') {
            return Err(());
        }
        let json = &json[1..json.len() - 1];

        let mut current_entry = String::new();
        let mut in_double_quote = false;
        let mut in_single_quote = false;
        let mut escape_next = false;

        for ch in json.chars() {
            if escape_next {
                current_entry.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' => {
                    current_entry.push(ch);
                    escape_next = true;
                }
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                    current_entry.push(ch);
                }
                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                    current_entry.push(ch);
                }
                ',' if !in_double_quote && !in_single_quote => {
                    if let Some((id, name)) = Self::parse_name_entry(&current_entry) {
                        names.insert(id, name);
                    }
                    current_entry.clear();
                }
                _ => {
                    current_entry.push(ch);
                }
            }
        }

        if let Some((id, name)) = Self::parse_name_entry(&current_entry) {
            names.insert(id, name);
        }

        if names.is_empty() {
            return Err(());
        }

        let max_id = *names.keys().max().unwrap_or(&0);
        let mut result = HashMap::with_capacity(max_id + 1);
        for i in 0..=max_id {
            result.insert(
                i,
                names.remove(&i).unwrap_or_else(|| format!("class_{}", i)),
            );
        }

        Ok(result)
    }

    /// Parse Ultralytics YOLO imgsz format: [640, 640] or [height, width]
    fn parse_imgsz(json: &str) -> Option<(u32, u32)> {
        let json = json.trim();
        if !json.starts_with('[') || !json.ends_with(']') {
            return None;
        }
        let inner = &json[1..json.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        if parts.len() < 2 {
            return None;
        }
        let height: u32 = parts[0].parse().ok()?;
        let width: u32 = parts[1].parse().ok()?;
        Some((height, width))
    }

    /// Extract task from ONNX model metadata
    fn extract_task_from_metadata(&mut self) {
        let session = match &self.session {
            Some(s) => s,
            None => return,
        };

        let metadata = match session.metadata() {
            Ok(m) => m,
            Err(_) => return,
        };

        if let Some(task) = metadata.custom("task") {
            let task = task.trim().to_string();
            if !task.is_empty() {
                info!("Model '{}' task from metadata: {}", self.name, task);
                self.task = Some(task);
            }
        }
    }

    /// Extract imgsz from ONNX model metadata
    fn extract_imgsz_from_metadata(&mut self) {
        let session = match &self.session {
            Some(s) => s,
            None => return,
        };

        let metadata = match session.metadata() {
            Ok(m) => m,
            Err(_) => return,
        };

        if let Some(imgsz_json) = metadata.custom("imgsz") {
            if let Some((height, width)) = Self::parse_imgsz(&imgsz_json) {
                info!(
                    "Model '{}' imgsz from metadata: [{}, {}]",
                    self.name, height, width
                );
                self.imgsz = Some((height, width));
            }
        }
    }

    /// Parse a single entry like "0": "person" or 0: 'person'
    fn parse_name_entry(entry: &str) -> Option<(usize, String)> {
        let entry = entry.trim();
        if entry.is_empty() {
            return None;
        }

        let colon_pos = entry.find(':')?;
        let key_part = &entry[..colon_pos];
        let value_part = &entry[colon_pos + 1..];

        let key_str = key_part.trim();
        let key: usize = if key_str.starts_with('"') && key_str.ends_with('"') {
            key_str[1..key_str.len() - 1].parse().ok()?
        } else {
            key_str.parse().ok()?
        };

        let value = value_part.trim();
        let (quote_char, inner) = if value.starts_with('"') && value.ends_with('"') {
            ('"', &value[1..value.len() - 1])
        } else if value.starts_with('\'') && value.ends_with('\'') {
            ('\'', &value[1..value.len() - 1])
        } else {
            return None;
        };

        let name = if quote_char == '"' {
            inner
                .replace("\\\"", "\"")
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\\\", "\\")
        } else {
            inner
                .replace("\\'", "'")
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\\\", "\\")
        };

        Some((key, name))
    }

    /// Extract input/output information from loaded session
    fn extract_model_info(&mut self) -> Result<(), DetectorError> {
        let session = self.session.as_ref().ok_or(DetectorError::NotInitialized)?;

        // Get input information
        let inputs = session.inputs();
        let input = inputs
            .first()
            .ok_or_else(|| DetectorError::ModelLoadError("No input found in model".to_string()))?;

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

        // Validate and extract mask prototype info for segmentation models
        // Segmentation models have TWO outputs:
        // - output0: [1, 4 + num_classes + num_masks, num_detections]
        // - output1 (proto): [1, num_masks, mask_height, mask_width]
        if self.inference_type == InferenceType::Segmentation {
            if self.output_infos.len() < 2 {
                return Err(DetectorError::ModelLoadError(
                    "Segmentation model requires 2 outputs, but found {}".to_string(),
                ));
            }

            let proto_info = &self.output_infos[1];
            if proto_info.dimensions.len() != 4 {
                return Err(DetectorError::ModelLoadError(
                    "Invalid mask prototype dimensions for segmentation model".to_string(),
                ));
            }

            self.num_masks = proto_info.dimensions[1] as usize;
            self.mask_proto_height = proto_info.dimensions[2] as usize;
            self.mask_proto_width = proto_info.dimensions[3] as usize;

            info!(
                "Model '{}' mask prototypes: {}x{}x{}",
                self.name, self.num_masks, self.mask_proto_height, self.mask_proto_width
            );
        } else {
            // Detection models have ONE output
            self.num_masks = 0;
            self.mask_proto_height = 0;
            self.mask_proto_width = 0;
        }

        // Pre-allocate input buffer
        let buffer_size = (channels * height * width) as usize;
        self.input_buffer = Some(vec![0.0f32; buffer_size]);

        Ok(())
    }

    /// Get input info
    pub fn get_input_info(&self) -> Option<&InputInfo> {
        self.input_info.as_ref()
    }

    /// Get output infos
    pub fn get_output_infos(&self) -> &[OutputInfo] {
        &self.output_infos
    }

    /// Get task type from metadata
    pub fn get_task(&self) -> Option<&str> {
        self.task.as_deref()
    }

    /// Get image size from metadata
    pub fn get_imgsz(&self) -> Option<(u32, u32)> {
        self.imgsz
    }

    /// Detect with custom confidence and IoU thresholds
    pub fn detect_with_thresholds(
        &mut self,
        image: &InferenceImage,
        confidence_threshold: f32,
        iou_threshold: f32,
    ) -> Result<DetectionResult, DetectorError> {
        let start = Instant::now();

        if !self.is_initialized() {
            return Err(DetectorError::NotInitialized);
        }

        // Preprocess
        let (input_array, scale, pad_x, pad_y) = self.preprocess(image)?;

        // Run inference
        let (output_data, mask_proto) = self.run_inference(input_array)?;

        // Post-process
        let detections = self.postprocess(
            output_data,
            mask_proto,
            image.width,
            image.height,
            scale,
            pad_x,
            pad_y,
            confidence_threshold,
            iou_threshold,
        )?;

        let mut result = DetectionResult::new().with_dimensions(image.width, image.height);
        for detection in detections {
            result.add_detection(detection);
        }

        // For detection models, ensure all expected product categories are detected
        // If a category is missing (no OK or NG detected), add a "not detected" NG result
        if self.inference_type == InferenceType::Detection {
            self.validate_and_add_missing_detections(&mut result);
        }

        result.processing_time_ms = start.elapsed().as_millis() as u64;

        Ok(result)
    }

    /// Preprocess image: letterbox resize + normalize + CHW layout
    /// Returns (preprocessed_data, scale, pad_x, pad_y)
    fn preprocess(
        &self,
        image: &InferenceImage,
    ) -> Result<(Array3<f32>, f32, u32, u32), DetectorError> {
        let input_info = self
            .input_info
            .as_ref()
            .ok_or(DetectorError::NotInitialized)?;
        let target_width = input_info.width;
        let target_height = input_info.height;
        let channels = input_info.channels;

        // Letterbox resize
        let (letterboxed, scale, pad_x, pad_y) =
            image.letterbox(target_width, target_height, (114, 114, 114));

        // Convert HWC to CHW and normalize to [0, 1]
        let mut input_array = Array3::<f32>::zeros((
            channels as usize,
            target_height as usize,
            target_width as usize,
        ));

        let data = letterboxed.as_bytes();
        for y in 0..target_height as usize {
            for x in 0..target_width as usize {
                let idx = (y * target_width as usize + x) * 3;
                for c in 0..channels as usize {
                    // RGB -> normalize
                    let pixel = data[idx + c] as f32 / 255.0;
                    input_array[[c, y, x]] = pixel;
                }
            }
        }

        Ok((input_array, scale, pad_x, pad_y))
    }

    /// Run ONNX inference
    fn run_inference(
        &mut self,
        input_array: Array3<f32>,
    ) -> Result<(Vec<f32>, Option<Vec<f32>>), DetectorError> {
        let session = self.session.as_mut().ok_or(DetectorError::NotInitialized)?;
        let input_info = self
            .input_info
            .as_ref()
            .ok_or(DetectorError::NotInitialized)?;

        // Create input tensor [1, C, H, W] using ndarray
        let input_shape = (
            1_usize,
            input_info.channels as usize,
            input_info.height as usize,
            input_info.width as usize,
        );
        let (input_values, _offset) = input_array.into_raw_vec_and_offset();
        let input_ndarray = ndarray::Array4::from_shape_vec(input_shape, input_values)
            .map_err(|e| DetectorError::InferenceError(format!("Shape error: {}", e)))?;
        let input_tensor = ort::value::Value::from_array(input_ndarray)
            .map_err(|e| DetectorError::InferenceError(e.to_string()))?;

        // Run inference
        let outputs = session
            .run(inputs![input_tensor])
            .map_err(|e| DetectorError::InferenceError(e.to_string()))?;

        // Extract detection output (first output)
        // try_extract_tensor returns (&Shape, &[f32])
        let detection_output = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| DetectorError::InferenceError(e.to_string()))?;
        let detection_data: Vec<f32> = detection_output.1.to_vec();

        // Extract mask prototypes for segmentation (second output)
        let mask_proto = if self.inference_type == InferenceType::Segmentation && outputs.len() > 1
        {
            let proto_output = outputs[1]
                .try_extract_tensor::<f32>()
                .map_err(|e| DetectorError::InferenceError(e.to_string()))?;
            Some(proto_output.1.to_vec())
        } else {
            None
        };

        Ok((detection_data, mask_proto))
    }

    /// Post-process YOLO detection output
    /// Supports two output formats:
    /// 1. Standard YOLO format: [1, 4+num_classes, num_detections] (e.g., [1, 8, 8400])
    ///    - Requires NMS post-processing
    ///    - Bounding box in (cx, cy, w, h) format
    /// 2. One-to-One format (YOLOv10/YOLO11 one2one): [1, num_detections, 6] (e.g., [1, 300, 6])
    ///    - No NMS required, results pre-sorted by confidence
    ///    - Each detection: [x1, y1, x2, y2, confidence, class_id]
    fn postprocess(
        &self,
        output_data: Vec<f32>,
        mask_proto: Option<Vec<f32>>,
        original_width: u32,
        original_height: u32,
        scale: f32,
        pad_x: u32,
        pad_y: u32,
        confidence_threshold: f32,
        iou_threshold: f32,
    ) -> Result<Vec<Detection>, DetectorError> {
        let _input_info = self
            .input_info
            .as_ref()
            .ok_or(DetectorError::NotInitialized)?;

        // Parse output dimensions
        let output_info = &self.output_infos[0];
        let dim1 = output_info.dimensions[1] as usize;
        let dim2 = output_info.dimensions[2] as usize;

        // Detect output format based on dimension relationship
        // Standard YOLO: dim1 (4+classes) < dim2 (detections), e.g., [1, 8, 8400]
        // One-to-One: dim1 (detections) > dim2 (6), e.g., [1, 300, 6]
        let is_one_to_one_format = dim1 > dim2 && dim2 == 6;

        if is_one_to_one_format {
            // One-to-One format: [1, num_detections, 6]
            // Each row: [x1, y1, x2, y2, confidence, class_id]
            self.postprocess_one_to_one(
                output_data,
                dim1, // num_detections
                original_width,
                original_height,
                scale,
                pad_x,
                pad_y,
                confidence_threshold,
            )
        } else {
            // Standard YOLO format: [1, 4+classes, detections]
            self.postprocess_standard(
                output_data,
                mask_proto,
                dim1, // data_per_detection (4 + num_classes)
                dim2, // num_detections
                original_width,
                original_height,
                scale,
                pad_x,
                pad_y,
                confidence_threshold,
                iou_threshold,
            )
        }
    }

    /// Post-process One-to-One format output (YOLOv10/YOLO11 one2one)
    /// Format: [1, num_detections, 6] where each detection is [x1, y1, x2, y2, confidence, class_id]
    fn postprocess_one_to_one(
        &self,
        output_data: Vec<f32>,
        num_detections: usize,
        original_width: u32,
        original_height: u32,
        scale: f32,
        pad_x: u32,
        pad_y: u32,
        confidence_threshold: f32,
    ) -> Result<Vec<Detection>, DetectorError> {
        let mut detections = Vec::new();

        // Results are pre-sorted by confidence, process in order
        for i in 0..num_detections {
            let base_idx = i * 6;

            let x1 = output_data[base_idx];
            let y1 = output_data[base_idx + 1];
            let x2 = output_data[base_idx + 2];
            let y2 = output_data[base_idx + 3];
            let confidence = output_data[base_idx + 4];
            let class_id = output_data[base_idx + 5] as usize;

            // Skip low confidence detections
            if confidence < confidence_threshold {
                continue;
            }

            // Skip invalid detections (zeros indicate padding)
            if x1 == 0.0 && y1 == 0.0 && x2 == 0.0 && y2 == 0.0 {
                continue;
            }

            // Scale bbox back to original image coordinates
            let x1 = (x1 - pad_x as f32) / scale;
            let y1 = (y1 - pad_y as f32) / scale;
            let x2 = (x2 - pad_x as f32) / scale;
            let y2 = (y2 - pad_y as f32) / scale;

            // Clamp to image bounds
            let x1 = x1.clamp(0.0, original_width as f32);
            let y1 = y1.clamp(0.0, original_height as f32);
            let x2 = x2.clamp(0.0, original_width as f32);
            let y2 = y2.clamp(0.0, original_height as f32);

            let class_name = self
                .class_names
                .get(&class_id)
                .cloned()
                .unwrap_or_else(|| format!("class_{}", class_id));

            let mut detection = Detection::new(class_name, class_id as i32, confidence);
            detection.bbox = Some(BboxRegion::new(x1, y1, x2 - x1, y2 - y1));
            detections.push(detection);
        }

        Ok(detections)
    }

    /// Post-process standard YOLO format output
    /// Format: [1, 4+num_classes, num_detections] with (cx, cy, w, h) bbox format
    fn postprocess_standard(
        &self,
        output_data: Vec<f32>,
        mask_proto: Option<Vec<f32>>,
        data_per_detection: usize,
        num_detections: usize,
        original_width: u32,
        original_height: u32,
        scale: f32,
        pad_x: u32,
        pad_y: u32,
        confidence_threshold: f32,
        iou_threshold: f32,
    ) -> Result<Vec<Detection>, DetectorError> {
        let input_info = self
            .input_info
            .as_ref()
            .ok_or(DetectorError::NotInitialized)?;

        // Determine number of classes based on inference type
        let num_classes = if self.inference_type == InferenceType::Segmentation {
            data_per_detection - 4 - self.num_masks
        } else {
            data_per_detection - 4
        };

        // Collect candidates with confidence above threshold
        let mut candidates: Vec<(BoundingBox, usize, f32, Option<Vec<f32>>)> = Vec::new();

        for i in 0..num_detections {
            // Find max class score
            let mut max_class_id = 0;
            let mut max_score = 0.0f32;

            for c in 0..num_classes {
                let score = output_data[(4 + c) * num_detections + i];
                if score > max_score {
                    max_score = score;
                    max_class_id = c;
                }
            }

            if max_score < confidence_threshold {
                continue;
            }

            // Extract bounding box (center_x, center_y, width, height) -> (x1, y1, x2, y2)
            let cx = output_data[0 * num_detections + i];
            let cy = output_data[1 * num_detections + i];
            let w = output_data[2 * num_detections + i];
            let h = output_data[3 * num_detections + i];

            let x1 = cx - w / 2.0;
            let y1 = cy - h / 2.0;
            let x2 = cx + w / 2.0;
            let y2 = cy + h / 2.0;

            // Extract mask coefficients for segmentation
            let mask_coeffs =
                if self.inference_type == InferenceType::Segmentation && self.num_masks > 0 {
                    let mut coeffs = Vec::with_capacity(self.num_masks);
                    for m in 0..self.num_masks {
                        coeffs.push(output_data[(4 + num_classes + m) * num_detections + i]);
                    }
                    Some(coeffs)
                } else {
                    None
                };

            candidates.push((
                BoundingBox { x1, y1, x2, y2 },
                max_class_id,
                max_score,
                mask_coeffs,
            ));
        }

        // NMS (Non-Maximum Suppression) per class
        let mut keep = vec![true; candidates.len()];
        for i in 0..candidates.len() {
            if !keep[i] {
                continue;
            }
            for j in (i + 1)..candidates.len() {
                if !keep[j] {
                    continue;
                }
                // Only suppress same class
                if candidates[i].1 != candidates[j].1 {
                    continue;
                }
                let iou = Self::compute_iou(&candidates[i].0, &candidates[j].0);
                if iou > iou_threshold {
                    // Keep the one with higher confidence
                    if candidates[j].2 > candidates[i].2 {
                        keep[i] = false;
                    } else {
                        keep[j] = false;
                    }
                }
            }
        }

        // Build final detections
        let mut detections = Vec::new();
        for (idx, candidate) in candidates.iter().enumerate() {
            if !keep[idx] {
                continue;
            }

            let (bbox, class_id, confidence, mask_coeffs) = candidate;

            // Scale bbox back to original image coordinates
            let x1 = (bbox.x1 - pad_x as f32) / scale;
            let y1 = (bbox.y1 - pad_y as f32) / scale;
            let x2 = (bbox.x2 - pad_x as f32) / scale;
            let y2 = (bbox.y2 - pad_y as f32) / scale;

            // Clamp to image bounds
            let x1 = x1.clamp(0.0, original_width as f32);
            let y1 = y1.clamp(0.0, original_height as f32);
            let x2 = x2.clamp(0.0, original_width as f32);
            let y2 = y2.clamp(0.0, original_height as f32);

            let class_name = self
                .class_names
                .get(class_id)
                .cloned()
                .unwrap_or_else(|| format!("class_{}", class_id));

            let mut detection = Detection::new(class_name, *class_id as i32, *confidence);

            // Add bounding box
            detection.bbox = Some(BboxRegion::new(x1, y1, x2 - x1, y2 - y1));

            // Process mask for segmentation
            if let (Some(coeffs), Some(proto)) = (mask_coeffs, &mask_proto) {
                if let Some(mask) =
                    self.compute_mask(coeffs, proto, bbox, input_info.width, input_info.height)
                {
                    // Scale mask to original image size
                    let scaled_mask = self.scale_mask(
                        &mask,
                        input_info.width,
                        input_info.height,
                        original_width,
                        original_height,
                        scale,
                        pad_x,
                        pad_y,
                        bbox,
                    );
                    detection.mask = Some(MaskRegion::new(
                        scaled_mask,
                        original_width,
                        original_height,
                    ));
                }
            }

            detections.push(detection);
        }

        Ok(detections)
    }

    /// Validate detection results and add missing category detections
    ///
    /// For detection models with OK/NG paired class names (e.g., ConnectorOK/ConnectorNG),
    /// if neither OK nor NG is detected for a category, add a \"not detected\" NG result.
    fn validate_and_add_missing_detections(&self, result: &mut DetectionResult) {
        if self.class_names.is_empty() {
            return;
        }

        // Extract product categories from class names (strip OK/NG suffix)
        let mut categories: HashSet<String> = HashSet::new();
        for name in self.class_names.values() {
            let category = Self::extract_category(name);
            if !category.is_empty() {
                categories.insert(category);
            }
        }

        // Collect detected categories from results
        let mut detected_categories: HashSet<String> = HashSet::new();
        for detection in &result.detections {
            let category = Self::extract_category(&detection.class_name);
            if !category.is_empty() {
                detected_categories.insert(category);
            }
        }

        // Find missing categories and add NG detections
        for category in &categories {
            if !detected_categories.contains(category) {
                // Category not detected - add a NG result
                let ng_name = format!("{}NG", category);
                let mut detection = Detection::new(ng_name, -1, 1.0);
                detection.bbox = None; // No bbox for not-detected items
                result.add_detection(detection);
                tracing::warn!(
                    "Category '{}' not detected, adding NG result",
                    category
                );
            }
        }
    }

    /// Extract product category from class name by stripping OK/NG suffix
    /// Examples: "ConnectorOK" -> "Connector", "CrimpNG" -> "Crimp"
    fn extract_category(class_name: &str) -> String {
        let name = class_name.trim();
        if name.is_empty() {
            return String::new();
        }

        // Check for OK suffix (case insensitive)
        let lower = name.to_lowercase();
        if lower.ends_with("ok") {
            return name[..name.len() - 2].to_string();
        }
        // Check for NG suffix (case insensitive)
        if lower.ends_with("ng") {
            return name[..name.len() - 2].to_string();
        }

        // If no OK/NG suffix, return original name as category
        name.to_string()
    }

    /// Compute IoU between two bounding boxes
    fn compute_iou(a: &BoundingBox, b: &BoundingBox) -> f32 {
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

    /// Compute segmentation mask from prototypes and coefficients
    fn compute_mask(
        &self,
        mask_coeffs: &[f32],
        mask_proto: &[f32],
        bbox: &BoundingBox,
        _input_width: u32,
        _input_height: u32,
    ) -> Option<Vec<u8>> {
        // mask_proto shape: [1, num_masks, mask_height, mask_width]
        let mask_height = self.mask_proto_height;
        let mask_width = self.mask_proto_width;
        let num_masks = self.num_masks;

        if mask_coeffs.len() != num_masks
            || mask_proto.len() != num_masks * mask_height * mask_width
        {
            return None;
        }

        // Compute mask = sigmoid(mask_coeffs @ mask_proto)
        // Result shape: [mask_height, mask_width]
        let mut mask = vec![0.0f32; mask_height * mask_width];

        for y in 0..mask_height {
            for x in 0..mask_width {
                let mut sum = 0.0f32;
                for m in 0..num_masks {
                    let proto_val = mask_proto[m * mask_height * mask_width + y * mask_width + x];
                    sum += mask_coeffs[m] * proto_val;
                }
                // Sigmoid
                mask[y * mask_width + x] = 1.0 / (1.0 + (-sum).exp());
            }
        }

        // Binarize with threshold 0.5 and crop to bbox region
        let mut result = vec![0u8; mask_height * mask_width];

        // Map bbox to mask coordinates (mask is typically 4x smaller than input)
        let scale_x = mask_width as f32 / self.input_info.as_ref()?.width as f32;
        let scale_y = mask_height as f32 / self.input_info.as_ref()?.height as f32;

        let mx1 = (bbox.x1 * scale_x) as usize;
        let my1 = (bbox.y1 * scale_y) as usize;
        let mx2 = (bbox.x2 * scale_x).min(mask_width as f32) as usize;
        let my2 = (bbox.y2 * scale_y).min(mask_height as f32) as usize;

        for y in my1..my2 {
            for x in mx1..mx2 {
                if mask[y * mask_width + x] > 0.5 {
                    result[y * mask_width + x] = 255;
                }
            }
        }

        Some(result)
    }

    /// Scale mask from model input size to original image size
    fn scale_mask(
        &self,
        mask: &[u8],
        mask_width: u32,
        mask_height: u32,
        orig_width: u32,
        orig_height: u32,
        _scale: f32,
        _pad_x: u32,
        _pad_y: u32,
        _bbox: &BoundingBox,
    ) -> Vec<u8> {
        // Simple nearest-neighbor scaling
        let mut result = vec![0u8; (orig_width * orig_height) as usize];

        let scale_x = mask_width as f32 / orig_width as f32;
        let scale_y = mask_height as f32 / orig_height as f32;

        for y in 0..orig_height {
            for x in 0..orig_width {
                let mx = (x as f32 * scale_x) as usize;
                let my = (y as f32 * scale_y) as usize;
                let mx = mx.min(mask_width as usize - 1);
                let my = my.min(mask_height as usize - 1);
                result[(y * orig_width + x) as usize] = mask[my * mask_width as usize + mx];
            }
        }

        result
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

        if !Path::new(&self.model_path).exists() {
            return Err(DetectorError::ModelNotFound(self.model_path.clone()));
        }

        info!("Model Path correct");

        let mut builder =
            Session::builder().map_err(|e| DetectorError::ModelLoadError(e.to_string()))?;
        info!("builder correct");

        builder = builder
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| DetectorError::ModelLoadError(e.to_string()))?;
        info!("with_optimization_level correct");

        builder = builder
            .with_intra_threads(4)
            .map_err(|e| DetectorError::ModelLoadError(e.to_string()))?;
        info!("with_intra_threads correct");

        let session = builder
            .commit_from_file(&self.model_path)
            .map_err(|e| DetectorError::ModelLoadError(format!("{}: {}", self.model_path, e)))?;
        info!("commit_from_file correct");

        info!("Session correct");
        self.session = Some(session);
        self.extract_model_info()?;
        info!("extract_model_info correct");
        self.extract_class_names_from_metadata();
        info!("extract_class_names_from_metadata correct");
        self.extract_task_from_metadata();
        info!("extract_task_from_metadata correct");
        self.extract_imgsz_from_metadata();
        info!("extract_imgsz_from_metadata correct");

        info!(
            "Model '{}' initialized successfully (type: {:?})",
            self.name, self.inference_type
        );
        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.session.is_some() && self.input_info.is_some()
    }

    fn detect(&mut self, image: &InferenceImage) -> Result<DetectionResult, DetectorError> {
        self.detect_with_thresholds(image, 0.25, 0.45)
    }

    fn get_class_names(&self) -> HashMap<usize, String> {
        self.class_names.clone()
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
