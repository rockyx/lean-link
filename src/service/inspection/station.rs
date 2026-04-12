use serde::{Deserialize, Serialize};

/// Trigger mode for camera acquisition
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum TriggerMode {
    /// External trigger from PLC/Modbus
    External,

    /// Serial port trigger
    Serial,

    /// Continuous frame capture
    #[default]
    Continuous,

    /// Manual trigger via API
    Manual,
}

/// Rectangle region definition
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rectangle {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rectangle {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point is inside this rectangle
    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.width && py < self.y + self.height
    }

    /// Convert to normalized coordinates (0.0-1.0)
    pub fn to_normalized(&self, image_width: u32, image_height: u32) -> Option<NormalizedRect> {
        if image_width == 0 || image_height == 0 {
            return None;
        }
        Some(NormalizedRect {
            x: self.x as f32 / image_width as f32,
            y: self.y as f32 / image_height as f32,
            width: self.width as f32 / image_width as f32,
            height: self.height as f32 / image_height as f32,
        })
    }
}

/// Normalized rectangle (coordinates 0.0-1.0)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl NormalizedRect {
    /// Convert to pixel coordinates
    pub fn to_pixel(&self, image_width: u32, image_height: u32) -> Rectangle {
        Rectangle {
            x: (self.x * image_width as f32).round() as u32,
            y: (self.y * image_height as f32).round() as u32,
            width: (self.width * image_width as f32).round() as u32,
            height: (self.height * image_height as f32).round() as u32,
        }
    }

    /// Check if a normalized point is inside this rectangle
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.width && py < self.y + self.height
    }
}

/// 2D point with normalized coordinates (0.0-1.0)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Create from pixel coordinates
    pub fn from_pixel(px: u32, py: u32, image_width: u32, image_height: u32) -> Self {
        Self {
            x: if image_width > 0 {
                px as f32 / image_width as f32
            } else {
                0.0
            },
            y: if image_height > 0 {
                py as f32 / image_height as f32
            } else {
                0.0
            },
        }
    }

    /// Convert to pixel coordinates
    pub fn to_pixel(&self, image_width: u32, image_height: u32) -> (u32, u32) {
        (
            (self.x * image_width as f32).round() as u32,
            (self.y * image_height as f32).round() as u32,
        )
    }
}

/// ROI shape definition
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum RoiShape {
    /// Rectangle region
    Rectangle {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Polygon region (closed, at least 3 points)
    Polygon { points: Vec<Point> },
    /// Line segment
    Line { start: Point, end: Point },
}

impl RoiShape {
    /// Create a rectangle shape from normalized coordinates
    pub fn rectangle(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::Rectangle {
            x,
            y,
            width,
            height,
        }
    }

    /// Create a polygon shape from points
    pub fn polygon(points: Vec<Point>) -> Self {
        Self::Polygon { points }
    }

    /// Create a line shape from start and end points
    pub fn line(start: Point, end: Point) -> Self {
        Self::Line { start, end }
    }

    /// Get the bounding box of this shape
    pub fn bounding_box(&self) -> Option<NormalizedRect> {
        match self {
            RoiShape::Rectangle {
                x,
                y,
                width,
                height,
            } => Some(NormalizedRect {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            }),
            RoiShape::Polygon { points } => {
                if points.is_empty() {
                    return None;
                }
                let min_x = points.iter().map(|p| p.x).fold(f32::MAX, f32::min);
                let max_x = points.iter().map(|p| p.x).fold(f32::MIN, f32::max);
                let min_y = points.iter().map(|p| p.y).fold(f32::MAX, f32::min);
                let max_y = points.iter().map(|p| p.y).fold(f32::MIN, f32::max);
                Some(NormalizedRect {
                    x: min_x,
                    y: min_y,
                    width: max_x - min_x,
                    height: max_y - min_y,
                })
            }
            RoiShape::Line { start, end } => {
                let min_x = start.x.min(end.x);
                let max_x = start.x.max(end.x);
                let min_y = start.y.min(end.y);
                let max_y = start.y.max(end.y);
                Some(NormalizedRect {
                    x: min_x,
                    y: min_y,
                    width: max_x - min_x,
                    height: max_y - min_y,
                })
            }
        }
    }

    /// Check if a normalized point is inside this shape
    pub fn contains(&self, px: f32, py: f32) -> bool {
        match self {
            RoiShape::Rectangle {
                x,
                y,
                width,
                height,
            } => px >= *x && py >= *y && px < x + width && py < y + height,
            RoiShape::Polygon { points } => {
                // Ray casting algorithm for point-in-polygon
                if points.len() < 3 {
                    return false;
                }
                let mut inside = false;
                let mut j = points.len() - 1;
                for i in 0..points.len() {
                    let pi = &points[i];
                    let pj = &points[j];
                    if ((pi.y > py) != (pj.y > py))
                        && (px < (pj.x - pi.x) * (py - pi.y) / (pj.y - pi.y) + pi.x)
                    {
                        inside = !inside;
                    }
                    j = i;
                }
                inside
            }
            RoiShape::Line { start, end } => {
                // For lines, check if point is very close to the line
                let d1 = ((px - start.x).powi(2) + (py - start.y).powi(2)).sqrt();
                let d2 = ((px - end.x).powi(2) + (py - end.y).powi(2)).sqrt();
                let line_len = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
                (d1 + d2 - line_len).abs() < 0.01 // tolerance
            }
        }
    }
}

/// ROI purpose/type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RoiPurpose {
    /// Detection region - where inspection occurs
    Detection,
    /// Alignment reference - for position calibration
    Alignment,
    /// Exclusion region - ignore this area
    Exclusion,
}

impl RoiPurpose {
    /// Get display name in Chinese
    pub fn display_name(&self) -> &'static str {
        match self {
            RoiPurpose::Detection => "检测区域",
            RoiPurpose::Alignment => "定位参考",
            RoiPurpose::Exclusion => "排除区域",
        }
    }
}

/// Complete ROI configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoiConfig {
    /// Unique ROI identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Shape definition
    pub shape: RoiShape,
    /// Purpose of this ROI
    pub purpose: RoiPurpose,
    /// Whether this ROI is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl RoiConfig {
    /// Create a new ROI configuration
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        shape: RoiShape,
        purpose: RoiPurpose,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            shape,
            purpose,
            enabled: true,
        }
    }

    /// Create a detection rectangle ROI
    pub fn detection_rect(
        id: impl Into<String>,
        name: impl Into<String>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Self {
        Self::new(
            id,
            name,
            RoiShape::rectangle(x, y, width, height),
            RoiPurpose::Detection,
        )
    }
}

fn default_enabled() -> bool {
    true
}

fn default_confidence_threshold() -> f32 {
    0.5
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct DetectionType {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub requires_model: bool,
    #[serde(default = "default_requires_trigger")]
    pub requires_trigger: bool,
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
}

fn default_requires_trigger() -> bool {
    true
}

impl DetectionType {
    pub fn new<S: Into<String>>(id: S, display_name: S) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            description: String::new(),
            requires_model: false,
            requires_trigger: true,
            parameters: None,
        }
    }

    pub fn with_parameters<S: Into<String>> (
        id: S,
        display_name: S,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            parameters: Some(parameters),
            ..Self::new(id, display_name)
        }
    }

    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = description.into();
        self 
    }

    pub fn requires_model(mut self, requires: bool) -> Self {
        self.requires_model = requires;
        self
    }

    pub fn requires_trigger(mut self, requires: bool) -> Self {
        self.requires_trigger = requires;
        self
    }
}

/// Inspection station configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationConfig {
    /// Unique station identifier
    pub id: String,

    /// Human-readable station name
    pub name: String,

    /// Camera id assigned to this station
    pub camera_id: uuid::Uuid,

    /// Trigger mode for this station
    #[serde(default)]
    pub trigger_mode: TriggerMode,

    /// Types of detection to perform
    pub detection_types: Vec<String>,

    /// Multiple ROI configurations (new format, recommended)
    #[serde(default)]
    pub rois: Vec<RoiConfig>,

    /// Whether this station is enabled
    #[serde(default = "default_enabled")]
    pub is_enabled: bool,

    /// ONNX model path for this station (optional)
    #[serde(default)]
    pub model_path: Option<String>,

    /// Confidence threshold for OK/NG decision
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f32,

    /// Serial port path for serial trigger mode (optional)
    /// Only used when trigger_mode is Serial
    #[serde(default)]
    pub serial_port: Option<String>,
}

impl StationConfig {
    /// Get all ROIs, merging legacy and new formats
    pub fn get_all_rois(&self) -> Vec<RoiConfig> {
        let result = self.rois.clone();

        result
    }

    /// Get ROIs by purpose
    pub fn get_rois_by_purpose(&self, purpose: RoiPurpose) -> Vec<RoiConfig> {
        self.get_all_rois()
            .into_iter()
            .filter(|roi| roi.purpose == purpose && roi.enabled)
            .collect()
    }

    /// Get detection ROIs only
    pub fn get_detection_rois(&self) -> Vec<RoiConfig> {
        self.get_rois_by_purpose(RoiPurpose::Detection)
    }

    /// Add or update an ROI
    pub fn set_roi(&mut self, roi: RoiConfig) {
        // Remove existing ROI with same id
        self.rois.retain(|r| r.id != roi.id);
        // Add new ROI
        self.rois.push(roi);
    }

    /// Remove an ROI by id
    pub fn remove_roi(&mut self, id: &str) -> bool {
        let len_before = self.rois.len();
        self.rois.retain(|r| r.id != id);
        self.rois.len() != len_before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detection_type_display() {
        let terminal_crimp = DetectionType::new("terminal_crimp", "端子压接检测");
        let wire_connection = DetectionType::new("wire_connection", "导线连接检测");
        let insulation = DetectionType::new("insulation", "绝缘层检测");
        let terminal_insertion = DetectionType::new("terminal_insertion", "端子插接检测");

        assert_eq!(terminal_crimp.display_name, "端子压接检测");
        assert_eq!(wire_connection.display_name, "导线连接检测");
        assert_eq!(insulation.display_name, "绝缘层检测");
        assert_eq!(terminal_insertion.display_name, "端子插接检测");
    }

    #[test]
    fn test_rectangle_contains() {
        let rect = Rectangle::new(10, 20, 100, 50);
        assert!(rect.contains(10, 20));
        assert!(rect.contains(50, 40));
        assert!(!rect.contains(9, 20));
        assert!(!rect.contains(110, 20));
        assert!(!rect.contains(50, 70));
    }

    #[test]
    fn test_trigger_mode_default() {
        let mode: TriggerMode = Default::default();
        assert_eq!(mode, TriggerMode::Continuous);
    }

    #[test]
    fn test_normalized_rect() {
        let rect = NormalizedRect {
            x: 0.1,
            y: 0.2,
            width: 0.3,
            height: 0.4,
        };

        // Test contains
        assert!(rect.contains(0.15, 0.3));
        assert!(!rect.contains(0.05, 0.3));

        // Test to_pixel conversion
        let pixel_rect = rect.to_pixel(1920, 1080);
        assert_eq!(pixel_rect.x, 192); // 0.1 * 1920
        assert_eq!(pixel_rect.y, 216); // 0.2 * 1080
    }

    #[test]
    fn test_point_conversion() {
        let point = Point::from_pixel(960, 540, 1920, 1080);
        assert!((point.x - 0.5).abs() < 0.001);
        assert!((point.y - 0.5).abs() < 0.001);

        let (px, py) = point.to_pixel(1920, 1080);
        assert_eq!(px, 960);
        assert_eq!(py, 540);
    }

    #[test]
    fn test_roi_shape_rectangle_contains() {
        let shape = RoiShape::rectangle(0.1, 0.2, 0.3, 0.4);
        assert!(shape.contains(0.2, 0.3));
        assert!(!shape.contains(0.05, 0.3));
    }

    #[test]
    fn test_roi_shape_polygon_contains() {
        // Triangle: (0.1, 0.1) -> (0.5, 0.1) -> (0.3, 0.5)
        let shape = RoiShape::polygon(vec![
            Point::new(0.1, 0.1),
            Point::new(0.5, 0.1),
            Point::new(0.3, 0.5),
        ]);

        // Center of triangle should be inside
        assert!(shape.contains(0.3, 0.2));
        // Outside triangle
        assert!(!shape.contains(0.1, 0.3));
    }

    #[test]
    fn test_roi_shape_bounding_box() {
        let shape = RoiShape::polygon(vec![
            Point::new(0.1, 0.2),
            Point::new(0.5, 0.1),
            Point::new(0.3, 0.6),
        ]);

        let bbox = shape.bounding_box().unwrap();
        assert!((bbox.x - 0.1).abs() < 0.001);
        assert!((bbox.y - 0.1).abs() < 0.001);
        assert!((bbox.width - 0.4).abs() < 0.001);
        assert!((bbox.height - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_roi_purpose_display() {
        assert_eq!(RoiPurpose::Detection.display_name(), "检测区域");
        assert_eq!(RoiPurpose::Alignment.display_name(), "定位参考");
        assert_eq!(RoiPurpose::Exclusion.display_name(), "排除区域");
    }

    #[test]
    fn test_roi_config_creation() {
        let roi = RoiConfig::detection_rect("roi_1", "检测区1", 0.1, 0.2, 0.3, 0.4);
        assert_eq!(roi.id, "roi_1");
        assert_eq!(roi.name, "检测区1");
        assert!(roi.enabled);
        assert_eq!(roi.purpose, RoiPurpose::Detection);
    }

    #[test]
    fn test_station_config_roi_management() {
        let mut config = StationConfig {
            id: "station_1".to_string(),
            name: "测试站".to_string(),
            camera_id: uuid::Uuid::nil(),
            trigger_mode: TriggerMode::default(),
            detection_types: vec!["terminal_crimp".to_string()],
            rois: vec![],
            is_enabled: true,
            model_path: None,
            confidence_threshold: 0.5,
            serial_port: None,
        };

        // Add ROI
        let roi1 = RoiConfig::detection_rect("roi_1", "区域1", 0.1, 0.1, 0.2, 0.2);
        config.set_roi(roi1);
        assert_eq!(config.rois.len(), 1);

        // Add another ROI
        let roi2 = RoiConfig::new(
            "roi_2",
            "排除区",
            RoiShape::polygon(vec![
                Point::new(0.5, 0.5),
                Point::new(0.7, 0.5),
                Point::new(0.6, 0.7),
            ]),
            RoiPurpose::Exclusion,
        );
        config.set_roi(roi2);
        assert_eq!(config.rois.len(), 2);

        // Update existing ROI
        let roi1_updated = RoiConfig::detection_rect("roi_1", "更新区域1", 0.2, 0.2, 0.3, 0.3);
        config.set_roi(roi1_updated);
        assert_eq!(config.rois.len(), 2); // Still 2, not 3
        assert_eq!(
            config.rois.iter().find(|r| r.id == "roi_1").unwrap().name,
            "更新区域1"
        );

        // Remove ROI
        assert!(config.remove_roi("roi_1"));
        assert_eq!(config.rois.len(), 1);
    }

    #[test]
    fn test_station_config_legacy_roi_compatibility() {
        let config = StationConfig {
            id: "station_1".to_string(),
            name: "测试站".to_string(),
            camera_id: uuid::Uuid::nil(),
            trigger_mode: TriggerMode::default(),
            detection_types: vec!["terminal_crimp".to_string()],
            rois: vec![],
            is_enabled: true,
            model_path: None,
            confidence_threshold: 0.5,
            serial_port: None,
        };

        // get_all_rois should return empty when no rois configured
        let all_rois = config.get_all_rois();
        assert_eq!(all_rois.len(), 0);
    }

    #[test]
    fn test_roi_serialization() {
        let roi = RoiConfig::detection_rect("roi_1", "检测区", 0.1, 0.2, 0.3, 0.4);
        let json = serde_json::to_string(&roi).unwrap();
        assert!(json.contains("\"id\":\"roi_1\""));
        assert!(json.contains("\"type\":\"Rectangle\"")); // PascalCase

        // Deserialize back
        let decoded: RoiConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, roi.id);
        assert_eq!(decoded.name, roi.name);
    }
}
