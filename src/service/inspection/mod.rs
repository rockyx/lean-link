use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use ::image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use dashmap::DashMap;
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tokio::select;
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::database::entity::prelude::TSettings;
use crate::database::entity::t_settings;
use crate::service::camera::{CameraFrame, FrameEncoding, GrabMode};
use crate::service::inspection::config::InspectionSettings;
use crate::service::inspection::detector::{DetectionResult, Detector, DetectorError};
use crate::service::inspection::image::InferenceImage;
use crate::service::inspection::manager::ManagedStation;
use crate::service::inspection::station::{DetectionType, StationConfig, TriggerMode};
use crate::service::inspection::yolo::OnnxInference;
use crate::{
    errors,
    service::{camera::manager::ArcCameraManager, inspection::manager::ArcStationManager},
};

#[cfg(feature = "web")]
use tokio_tungstenite::tungstenite::Message;

pub mod config;
pub mod detector;
pub mod image;
pub mod manager;
pub mod station;
pub mod yolo;

mod keys {
    pub const INSPECTION: &str = "inspection.settings";
    pub const INSPECTION_RESULT_TOPIC: &str = "inspection/result";
}

pub enum InspectionEvent {
    OneTrigger(Uuid),
    ContinueTrigger(Uuid, CameraFrame),
}

/// Inspection result header for WebSocket binary transmission
/// Uses unified LLWS binary protocol via websocket::protocol module
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionResultHeader {
    pub topic: String,
    pub station_id: Uuid,
    pub station_name: String,
    pub encoding: FrameEncoding,
    pub timestamp: u64,
    pub is_ok: bool,
    pub detections: Vec<crate::service::inspection::detector::Detection>,
    pub processing_time_ms: u64,
    pub image_width: u32,
    pub image_height: u32,
    pub data_length: usize,
}

/// Inspection result payload for WebSocket transmission
#[derive(Clone, Debug)]
pub struct InspectionResultPayload {
    pub header: InspectionResultHeader,
    pub data: Vec<u8>,
}

impl InspectionResultPayload {
    /// Serialize to binary format for WebSocket transmission
    /// Uses the unified LLWS binary protocol
    pub fn to_binary(&self) -> Vec<u8> {
        use crate::service::websocket::protocol::{
            MSG_TYPE_INSPECTION_RESULT, PROTOCOL_VERSION, WsBinaryHeader, build_binary_payload,
        };

        let header = &self.header;
        let metadata = serde_json::json!({
            "isOk": header.is_ok,
            "detections": header.detections,
            "processingTimeMs": header.processing_time_ms,
        });

        let ws_header = WsBinaryHeader {
            version: PROTOCOL_VERSION,
            msg_type: MSG_TYPE_INSPECTION_RESULT.to_string(),
            topic: header.topic.clone(),
            encoding: header.encoding.to_string(),
            timestamp: header.timestamp,
            width: header.image_width,
            height: header.image_height,
            data_length: self.data.len(),
            source_id: header.station_id.to_string(),
            source_name: header.station_name.clone(),
            metadata,
        };

        build_binary_payload(ws_header, &self.data)
    }
}

pub struct InspectionManager {
    db_conn: DatabaseConnection,
    camera_manager: ArcCameraManager,
    station_manager: ArcStationManager,
    loop_token: Mutex<CancellationToken>,
    camera_to_stations: Arc<DashMap<Uuid, HashSet<Uuid>>>,
    inspection: RwLock<InspectionSettings>,
    class_to_detection_type: Arc<DashMap<String, DetectionType>>,
    onnx_inferences: Arc<DashMap<Uuid, Arc<Mutex<OnnxInference>>>>,
    #[cfg(feature = "web")]
    ws_server: crate::service::websocket::ArcWebSocketServer,
}

impl InspectionManager {
    fn new(
        db_conn: DatabaseConnection,
        camera_manager: ArcCameraManager,
        station_manager: ArcStationManager,
        #[cfg(feature = "web")] ws_server: crate::service::websocket::ArcWebSocketServer,
    ) -> Self {
        Self {
            db_conn,
            camera_manager,
            station_manager,
            loop_token: Mutex::new(CancellationToken::new()),
            camera_to_stations: Arc::new(DashMap::new()),
            inspection: RwLock::new(InspectionSettings::default()),
            class_to_detection_type: Arc::new(DashMap::new()),
            onnx_inferences: Arc::new(DashMap::new()),
            #[cfg(feature = "web")]
            ws_server,
        }
    }

    pub fn new_arc(
        db_conn: DatabaseConnection,
        camera_manager: ArcCameraManager,
        station_manager: ArcStationManager,
        #[cfg(feature = "web")] ws_server: crate::service::websocket::ArcWebSocketServer,
    ) -> ArcInspectionManager {
        Arc::new(Self::new(
            db_conn,
            camera_manager,
            station_manager,
            #[cfg(feature = "web")]
            ws_server,
        ))
    }

    async fn initializ_onnx(&self, station_config: &StationConfig) -> Result<(), errors::Error> {
        // Check if ONNX inference is already initialized for this station
        if self.onnx_inferences.contains_key(&station_config.id) {
            return Ok(());
        }

        if let Some(ref model_path) = station_config.model_path
            && !model_path.is_empty()
        {
            let model_path = model_path.clone();
            let name = station_config.name.clone();
            let inference_type = station_config.inference_type.clone();
            let station_id = station_config.id;

            // Use spawn_blocking to avoid blocking the async runtime
            // ONNX model loading is a heavy blocking operation
            let onnx_inference = tokio::task::spawn_blocking(move || {
                let mut onnx_inference =
                    OnnxInference::new(model_path, name).with_inference_type(inference_type);
                onnx_inference.initialize()?;
                Ok::<_, DetectorError>(onnx_inference)
            })
            .await
            .map_err(|e| DetectorError::Internal(format!("spawn_blocking error: {}", e)))??;

            self.onnx_inferences
                .insert(station_id, Arc::new(Mutex::new(onnx_inference)));
        }
        Ok(())
    }

    async fn start_continues(
        &self,
        station_config: &StationConfig,
        tx: Sender<InspectionEvent>,
    ) -> Result<(), errors::Error> {
        if self
            .camera_to_stations
            .contains_key(&station_config.camera_id)
        {
            return Err(InspectionError::DuplicatedContinueCamera(
                station_config.camera_id.to_string(),
            )
            .into());
        }

        let mut station_set = HashSet::new();
        station_set.insert(station_config.id.clone());
        self.camera_to_stations
            .insert(station_config.camera_id.clone(), station_set);
        let mut recv = self
            .camera_manager
            .start_grabbing(&station_config.camera_id, GrabMode::Continuous)
            .await?;

        self.initializ_onnx(&station_config).await?;

        let station_id = station_config.id.clone();

        tokio::spawn(async move {
            loop {
                match recv.recv().await {
                    Some(frame) => {
                        let _ = tx
                            .send(InspectionEvent::ContinueTrigger(station_id.clone(), frame))
                            .await;
                    }
                    None => {
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    async fn start_external(&self, station_config: &StationConfig) -> Result<(), errors::Error> {
        let camera_id = station_config.camera_id.clone();
        if let Some(mut station_set) = self.camera_to_stations.get_mut(&camera_id) {
            station_set.insert(station_config.id.clone());
        } else {
            let mut station_set = HashSet::new();
            station_set.insert(station_config.id.clone());
            self.camera_to_stations.insert(camera_id, station_set);
            self.camera_manager
                .open_camera(&station_config.camera_id)
                .await?;
            self.camera_manager
                .start_grabbing(&station_config.camera_id, GrabMode::SingleFrame)
                .await?;
        }

        self.initializ_onnx(&station_config).await?;

        Ok(())
    }

    async fn start_serial(&self, station_config: &StationConfig) -> Result<(), errors::Error> {
        let camera_id = station_config.camera_id.clone();
        if let Some(mut station_set) = self.camera_to_stations.get_mut(&camera_id) {
            station_set.insert(station_config.id.clone());
        } else {
            let mut station_set = HashSet::new();
            station_set.insert(station_config.id.clone());
            self.camera_to_stations.insert(camera_id, station_set);
            self.camera_manager
                .open_camera(&station_config.camera_id)
                .await?;
            self.camera_manager
                .start_grabbing(&station_config.camera_id, GrabMode::SingleFrame)
                .await?;
        }

        self.initializ_onnx(&station_config).await?;

        Ok(())
    }

    pub async fn start(&self) -> Result<Sender<InspectionEvent>, errors::Error> {
        self.camera_to_stations.clear();
        let cloned_token = {
            let mut loop_token = self.loop_token.lock().await;
            loop_token.cancel();
            *loop_token = CancellationToken::new();
            loop_token.clone()
        };

        let (tx, mut rx) = mpsc::channel::<InspectionEvent>(128);
        for station in self.station_manager.get_all_stations() {
            if !station.config.is_enabled {
                continue;
            }
            match station.config.trigger_mode {
                TriggerMode::Continuous => {
                    self.start_continues(&station.config, tx.clone()).await?;
                }
                TriggerMode::External => {
                    self.start_external(&station.config).await?;
                }
                TriggerMode::Serial => {
                    self.start_serial(&station.config).await?;
                }
                _ => {}
            }
        }

        let station_manager = self.station_manager.clone();
        let camera_manager = self.camera_manager.clone();
        let camera_to_stations = self.camera_to_stations.clone();
        let class_to_detection_type = self.class_to_detection_type.clone();
        let onnx_inferences = self.onnx_inferences.clone();
        let ws_server = self.ws_server.clone();
        tokio::spawn(async move {
            loop {
                let station_manager = station_manager.clone();
                let camera_manager = camera_manager.clone();
                let class_to_detection_type = class_to_detection_type.clone();
                let onnx_inferences = onnx_inferences.clone();
                let ws_server = ws_server.clone();
                select! {
                    _ = cloned_token.cancelled() => {
                        if let Err(e) = camera_manager.close_all().await {
                            tracing::error!("Close All Camera error: {:?}", e);
                        }
                        camera_to_stations.clear();
                        break;
                    }

                    event = rx.recv() => {
                        match event {
                            Some(e) => {
                                match e {
                                    // 手动触发
                                    InspectionEvent::OneTrigger(station_id) => {
                                        Self::on_one_trigger(
                                            camera_manager,
                                            station_manager,
                                            station_id,
                                            class_to_detection_type,
                                            onnx_inferences,
                                            #[cfg(feature = "web")]
                                            ws_server,
                                        );
                                    },
                                    // 相机持续触发
                                    InspectionEvent::ContinueTrigger(_station_id, _camera_frame) => {
                                        tokio::spawn(async move {});
                                    },
                                }
                            },
                            None => {},
                        }
                    }
                }
            }
        });

        Ok(tx)
    }

    pub async fn stop(&self) -> Result<(), errors::Error> {
        let loop_token = self.loop_token.lock().await;
        loop_token.cancel();
        Ok(())
    }

    pub async fn initialize_from_database(&self) -> Result<(), errors::Error> {
        self.camera_manager.initialize_from_database().await?;
        self.station_manager.initialize_from_database().await?;

        {
            let model = TSettings::find()
                .filter(
                    t_settings::Column::Key
                        .eq(keys::INSPECTION)
                        .and(t_settings::Column::DeletedAt.is_null()),
                )
                .one(&self.db_conn)
                .await?;

            match model {
                None => {}
                Some(config) => {
                    let value = serde_json::from_value(config.value)?;
                    *self.inspection.write().await = value;
                }
            }
        }

        // Initialize ONNX for all enabled stations
        for station in self.station_manager.get_enabled_stations().await {
            if let Err(e) = self.initializ_onnx(&station.config).await {
                tracing::error!(
                    "Failed to initialize ONNX for station {}: {:?}",
                    station.config.id,
                    e
                );
            }
        }

        Ok(())
    }

    pub async fn set_inspection(&self, settings: &InspectionSettings) -> Result<(), errors::Error> {
        let setting_model = TSettings::find()
            .filter(
                t_settings::Column::Key
                    .eq(keys::INSPECTION)
                    .and(t_settings::Column::DeletedAt.is_null()),
            )
            .one(&self.db_conn)
            .await?;

        let json_value = serde_json::to_value(&settings)?;
        if setting_model.is_none() {
            let model = t_settings::ActiveModel {
                id: ActiveValue::set(Uuid::now_v7()),
                key: ActiveValue::set(keys::INSPECTION.into()),
                value: ActiveValue::set(json_value),
                ..Default::default()
            };
            TSettings::insert(model).exec(&self.db_conn).await?;
        } else {
            let model = t_settings::ActiveModel {
                id: ActiveValue::set(setting_model.unwrap().id),
                key: ActiveValue::set(keys::INSPECTION.into()),
                value: ActiveValue::set(json_value),
                ..Default::default()
            };
            TSettings::update(model).exec(&self.db_conn).await?;
        }
        *self.inspection.write().await = settings.clone();
        Ok(())
    }

    pub async fn get_inspection(&self) -> InspectionSettings {
        self.inspection.read().await.clone()
    }

    pub async fn register_class_to_detection_type<S: Into<String>>(
        &self,
        class: S,
        detection_type: DetectionType,
    ) {
        let mut class = class.into();
        if class.is_empty() {
            class = detection_type.id.clone();
        }
        self.class_to_detection_type.insert(class, detection_type);
    }

    /// Remove ONNX inference instance for a station
    fn remove_onnx(&self, station_id: &Uuid) {
        if let Some((_, _onnx)) = self.onnx_inferences.remove(station_id) {
            tracing::info!("Removed ONNX inference for station {}", station_id);
        }
    }

    /// Create a new station and initialize ONNX if model path is provided
    pub async fn create_station(
        &self,
        request: crate::service::inspection::manager::StationCreateRequest,
    ) -> Result<Uuid, errors::Error> {
        let id = self.station_manager.create_station(request.clone()).await?;

        // Initialize ONNX if station is enabled and has model path
        if request.is_enabled.unwrap_or(true) {
            let station = self.station_manager.get_station(id);
            if let Some(managed) = station {
                self.initializ_onnx(&managed.config).await?;
            }
        }

        Ok(id)
    }

    /// Update a station and reinitialize ONNX if model path changed
    pub async fn update_station(
        &self,
        id: Uuid,
        request: crate::service::inspection::manager::StationUpdateRequest,
    ) -> Result<bool, errors::Error> {
        // Get old model path before update
        let old_model_path = self
            .station_manager
            .get_station(id)
            .and_then(|s| s.config.model_path.clone());

        let updated = self
            .station_manager
            .update_station(id, request.clone())
            .await?;

        if updated {
            let new_station = self.station_manager.get_station(id);

            if let Some(managed) = new_station {
                let new_model_path = managed.config.model_path.clone();

                // Reinitialize ONNX if model path changed
                if old_model_path != new_model_path {
                    self.remove_onnx(&id);
                    if managed.config.is_enabled {
                        self.initializ_onnx(&managed.config).await?;
                    }
                } else if managed.config.is_enabled {
                    // Ensure ONNX is initialized if station is enabled
                    self.initializ_onnx(&managed.config).await?;
                } else {
                    // Remove ONNX if station is disabled
                    self.remove_onnx(&id);
                }
            }
        }

        Ok(updated)
    }

    /// Delete a station and remove its ONNX inference
    pub async fn delete_station(&self, id: Uuid) -> Result<bool, errors::Error> {
        let deleted = self.station_manager.delete_station(id).await?;

        if deleted {
            self.remove_onnx(&id);
        }

        Ok(deleted)
    }

    async fn inference_camera_frame(
        managed_station: ManagedStation,
        camera_frame: CameraFrame,
        station_manager: ArcStationManager,
        station_id: Uuid,
        class_to_detection_type: Arc<DashMap<String, DetectionType>>,
        onnx_inferences: Arc<DashMap<Uuid, Arc<Mutex<OnnxInference>>>>,
        #[cfg(feature = "web")] ws_server: crate::service::websocket::ArcWebSocketServer,
    ) -> Result<(), errors::Error> {
        let inference_image = InferenceImage::from_camera_frame(&camera_frame)?;

        let onnx_inference = match onnx_inferences.get(&managed_station.config.id) {
            Some(oi) => Some(oi.value().clone()),
            None => None,
        };

        let detection_result =
            Self::inference(managed_station, onnx_inference, &inference_image, ws_server).await?;

        Ok(())
    }

    fn on_one_trigger(
        camera_manager: ArcCameraManager,
        station_manager: ArcStationManager,
        station_id: Uuid,
        class_to_detection_type: Arc<DashMap<String, DetectionType>>,
        onnx_inferences: Arc<DashMap<Uuid, Arc<Mutex<OnnxInference>>>>,
        #[cfg(feature = "web")] ws_server: crate::service::websocket::ArcWebSocketServer,
    ) {
        let managed_station = station_manager.get_station(station_id);
        if managed_station.is_none() {
            return;
        }

        let managed_station = managed_station.unwrap();
        tokio::spawn(async move {
            match camera_manager
                .trigger_frame(&managed_station.config.camera_id)
                .await
            {
                Ok(camera_frame) => {
                    if let Err(e) = Self::inference_camera_frame(
                        managed_station,
                        camera_frame,
                        station_manager,
                        station_id,
                        class_to_detection_type,
                        onnx_inferences,
                        ws_server,
                    )
                    .await
                    {
                        tracing::error!("detection camera frame error: {:?}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("triger frame error: {:?}", e);
                }
            }
        });
    }

    async fn inference(
        managed_station: ManagedStation,
        onnx_inference: Option<Arc<Mutex<OnnxInference>>>,
        inference_image: &InferenceImage,
        #[cfg(feature = "web")] ws_server: crate::service::websocket::ArcWebSocketServer,
    ) -> Result<DetectionResult, errors::Error> {
        // Run ONNX detection
        let detection_result = {
            let onnx_inference = onnx_inference.ok_or_else(|| DetectorError::NotInitialized)?;
            let mut onnx = onnx_inference.lock().await;
            onnx.detect(&inference_image)?
        };

        // Draw detection results on the image
        let annotated_image = Self::draw_detections(&inference_image, &detection_result);

        // Encode to JPEG
        let mut jpeg_bytes = Vec::new();
        let mut cursor = Cursor::new(&mut jpeg_bytes);
        annotated_image
            .write_to(&mut cursor, ImageFormat::Jpeg)
            .map_err(|e| DetectorError::Internal(format!("JPEG编码失败: {}", e)))?;

        // Send result via WebSocket (same format as normal inspection, just skip DB save)
        #[cfg(feature = "web")]
        {
            let payload = InspectionResultPayload {
                header: InspectionResultHeader {
                    topic: keys::INSPECTION_RESULT_TOPIC.to_string(),
                    station_id: managed_station.config.id,
                    station_name: managed_station.config.name.clone(),
                    encoding: FrameEncoding::Jpeg,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_micros() as u64,
                    is_ok: detection_result.is_ok,
                    detections: detection_result.detections.clone(),
                    processing_time_ms: detection_result.processing_time_ms,
                    image_width: detection_result.image_width,
                    image_height: detection_result.image_height,
                    data_length: jpeg_bytes.len(),
                },
                data: jpeg_bytes,
            };

            let binary_data = payload.to_binary();
            let _ = ws_server
                .broadcast(Message::Binary(binary_data.into()))
                .await;
        }
        Ok(detection_result)
    }

    pub async fn test_station(
        &self,
        station_id: &Uuid,
        image_path: &str,
    ) -> Result<(), errors::Error> {
        let managed_station = self
            .station_manager
            .get_station(*station_id)
            .ok_or_else(|| {
                errors::Error::from(DetectorError::InvalidInput("工作站不存在".to_string()))
            })?;

        self.initializ_onnx(&managed_station.config).await?;

        let image_path = Path::new(image_path);
        if !image_path.exists() {
            return Err(
                DetectorError::InvalidInput(format!("图像不存在: {:?}", image_path)).into(),
            );
        }

        let inference_image = InferenceImage::from_file(image_path)?;
        let onnx_inference = match self.onnx_inferences.get(&managed_station.config.id) {
            Some(oi) => Some(oi.value().clone()),
            None => None,
        };

        // Run ONNX detection
        Self::inference(
            managed_station,
            onnx_inference,
            &inference_image,
            #[cfg(feature = "web")]
            self.ws_server.clone(),
        )
        .await?;

        Ok(())
    }

    /// Draw bounding boxes on the image based on detection results
    fn draw_detections(
        inference_image: &InferenceImage,
        detection_result: &DetectionResult,
    ) -> DynamicImage {
        let mut img = RgbImage::from_raw(
            inference_image.width,
            inference_image.height,
            inference_image.data.to_vec(),
        )
        .expect("Invalid image buffer");

        for detection in &detection_result.detections {
            let color = if detection.is_ok() {
                Rgb([0, 200, 0]) // Green for OK
            } else {
                Rgb([255, 0, 0]) // Red for NG
            };

            if let Some(ref bbox) = detection.bbox {
                let x1 = bbox.x.max(0.0) as i32;
                let y1 = bbox.y.max(0.0) as i32;
                let x2 = (bbox.x + bbox.width).min(inference_image.width as f32) as i32;
                let y2 = (bbox.y + bbox.height).min(inference_image.height as f32) as i32;

                // Draw bounding box with line thickness
                let line_thickness = ((inference_image.width.max(inference_image.height) as f32
                    / 400.0)
                    .max(1.0)
                    .min(3.0)) as i32;

                draw_rect(&mut img, x1, y1, x2, y2, color, line_thickness);
            }
        }

        DynamicImage::ImageRgb8(img)
    }

    pub fn enumerate_detection_types(&self) -> Vec<DetectionType> {
        self.class_to_detection_type
            .iter()
            .map(|item| item.value().clone())
            .collect()
    }
}

pub type ArcInspectionManager = Arc<InspectionManager>;

#[derive(Debug, Clone)]
pub enum InspectionError {
    DuplicatedContinueCamera(String),
}

impl std::fmt::Display for InspectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InspectionError::DuplicatedContinueCamera(msg) => {
                write!(f, "相机持续触发不能同时应用在多个工作站: {}", msg)
            }
        }
    }
}

impl std::error::Error for InspectionError {}

/// Draw a rectangle outline on the image with specified line thickness
fn draw_rect(
    img: &mut RgbImage,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: Rgb<u8>,
    thickness: i32,
) {
    let width = img.width() as i32;
    let height = img.height() as i32;

    // Draw top and bottom lines (with thickness)
    for t in 0..thickness {
        let y_top = y1 + t;
        let y_bottom = y2 - t;

        // Top line
        if y_top >= 0 && y_top < height {
            for x in x1.max(0)..=x2.min(width - 1) {
                img.put_pixel(x as u32, y_top as u32, color);
            }
        }
        // Bottom line
        if y_bottom >= 0 && y_bottom < height {
            for x in x1.max(0)..=x2.min(width - 1) {
                img.put_pixel(x as u32, y_bottom as u32, color);
            }
        }
    }

    // Draw left and right lines (with thickness)
    for t in 0..thickness {
        let x_left = x1 + t;
        let x_right = x2 - t;

        // Left line
        if x_left >= 0 && x_left < width {
            for y in (y1 + thickness).max(0)..=(y2 - thickness).min(height - 1) {
                img.put_pixel(x_left as u32, y as u32, color);
            }
        }
        // Right line
        if x_right >= 0 && x_right < width {
            for y in (y1 + thickness).max(0)..=(y2 - thickness).min(height - 1) {
                img.put_pixel(x_right as u32, y as u32, color);
            }
        }
    }
}
