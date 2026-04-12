use crate::database::entity::t_camera_configs;
use crate::service::camera::stream::CameraStreamConfig;
use crate::service::camera::{CameraInfo, CameraConfig, GrabMode, manager::CameraManager};
use crate::service::web::service::{ErrorCode, Pagination, WebResponse};
use crate::service::websocket::WsMessage;
use crate::{AppState, errors};
use actix_web::{delete, get, post, put, scope, web};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Topic for camera stream messages
pub const CAMERA_STREAM_TOPIC: &str = "camera/stream";

// ==================== Request/Response DTOs ====================

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CameraConfigCreateRequest {
    pub device_user_id: Option<String>,
    pub key: Option<String>,
    pub serial_number: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub manufacture_info: Option<String>,
    pub device_version: Option<String>,
    pub exposure_time_ms: Option<f64>,
    pub exposure_auto: Option<bool>,
    pub ip_address: Option<String>,
    pub camera_supplier: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CameraConfigUpdateRequest {
    pub device_user_id: Option<String>,
    pub key: Option<String>,
    pub serial_number: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub manufacture_info: Option<String>,
    pub device_version: Option<String>,
    pub exposure_time_ms: Option<f64>,
    pub exposure_auto: Option<bool>,
    pub ip_address: Option<String>,
    pub camera_supplier: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraConfigResponse {
    pub id: Uuid,
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
    pub camera_supplier: String,
    pub enabled: bool,
}

impl From<CameraConfig> for CameraConfigResponse {
    fn from(config: CameraConfig) -> Self {
        Self {
            id: config.id.unwrap(),
            device_user_id: config.device_user_id,
            key: config.key,
            serial_number: config.serial_number,
            vendor: config.vendor,
            model: config.model,
            manufacture_info: config.manufacture_info,
            device_version: config.device_version,
            exposure_time_ms: config.exposure_time_ms,
            exposure_auto: config.exposure_auto,
            ip_address: config.ip_address,
            camera_supplier: config.camera_supplier.to_string(),
            enabled: true,
        }
    }
}

impl From<t_camera_configs::Model> for CameraConfigResponse {
    fn from(model: t_camera_configs::Model) -> Self {
        Self {
            id: model.id,
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
            camera_supplier: model.camera_supplier,
            enabled: model.enabled,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CameraConfigListRequest {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub enabled: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStartRequest {
    pub id: Uuid,
    pub config: CameraStreamConfig,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStopRequest {
    pub id: Uuid,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamUpdateConfigRequest {
    pub id: Uuid,
    pub config: CameraStreamConfig,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetEnabledRequest {
    pub enabled: bool,
}

// ==================== Stream Operations ====================

async fn stream_start_inner(
    app_state: web::Data<AppState>,
    manager: web::Data<CameraManager>,
    req: &StreamStartRequest,
) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
    manager
        .update_stream_config(req.id, req.config.clone())
        .await?;
    let stream = manager.start_stream(&req.id).await?;
    let stream = stream.lock().await;
    let mut stream_rx = stream.subscribe().await;

    let mut frame_rx = manager
        .start_grabbing(&req.id, GrabMode::Continuous)
        .await?;

    let ws_server = app_state.ws_server.clone();
    tokio::spawn(async move {
        loop {
            match stream_rx.recv().await {
                Ok(payload) => {
                    let _ = ws_server
                        .broadcast(
                            WsMessage {
                                topic: CAMERA_STREAM_TOPIC.into(),
                                payload: payload,
                            }
                            .into(),
                        )
                        .await;
                }
                Err(e) => {
                    tracing::error!("Camera stream recv error: {:?}", e);
                    break;
                }
            }
        }
    });

    let camera_id = req.id;
    let manager_clone = manager.into_inner();
    tokio::spawn(async move {
        loop {
            match frame_rx.recv().await {
                Some(frame) => {
                    if let Ok(stream) = manager_clone.get_stream(&camera_id).await {
                        let stream = stream.lock().await;
                        stream.trigger_frame(&frame).await;
                    }
                }
                None => {
                    tracing::debug!("Stream frame recv closed");
                    break;
                }
            }
        }
    });

    Ok(WebResponse::with_result(()).into())
}

// ==================== API Routes ====================

#[scope("/camera")]
pub mod api {
    use super::*;

    /// Enumerate all connected industrial cameras
    #[post("/enumerate")]
    pub async fn enumerate_cameras(
        manager: web::Data<CameraManager>,
    ) -> actix_web::Result<web::Json<WebResponse<Vec<CameraInfo>>>, errors::Error> {
        let cameras = manager.enumerate_cameras().await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to enumerate cameras");
            errors::Error::InternalError(ErrorCode::InternalError)
        })?;

        tracing::info!("Enumerated {} cameras", cameras.len());
        Ok(WebResponse::with_result(cameras).into())
    }

    /// Create a new camera config
    #[post("/create")]
    pub async fn create(
        manager: web::Data<CameraManager>,
        req: web::Json<CameraConfigCreateRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<CameraConfigResponse>>, errors::Error> {
        let config = CameraConfig {
            id: None,
            device_user_id: req.device_user_id.clone(),
            key: req.key.clone(),
            serial_number: req.serial_number.clone(),
            vendor: req.vendor.clone(),
            model: req.model.clone(),
            manufacture_info: req.manufacture_info.clone(),
            device_version: req.device_version.clone(),
            exposure_time_ms: req.exposure_time_ms.unwrap_or(10.0),
            exposure_auto: req.exposure_auto.unwrap_or(false),
            ip_address: req.ip_address.clone(),
            camera_supplier: req.camera_supplier.parse()
                .map_err(|_| errors::Error::BadRequest(ErrorCode::OperationNotAllow, "无效的相机供应商".into()))?,
        };

        let created = manager.create_camera(config).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to create camera");
            errors::Error::InternalError(ErrorCode::InternalError)
        })?;

        Ok(WebResponse::with_result(created.into()).into())
    }

    /// Update a camera config
    #[put("/update/{id}")]
    pub async fn update(
        manager: web::Data<CameraManager>,
        path: web::Path<Uuid>,
        req: web::Json<CameraConfigUpdateRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<CameraConfigResponse>>, errors::Error> {
        let id = path.into_inner();

        let config = CameraConfig {
            id: Some(id),
            device_user_id: req.device_user_id.clone(),
            key: req.key.clone(),
            serial_number: req.serial_number.clone(),
            vendor: req.vendor.clone(),
            model: req.model.clone(),
            manufacture_info: req.manufacture_info.clone(),
            device_version: req.device_version.clone(),
            exposure_time_ms: req.exposure_time_ms.unwrap_or(10.0),
            exposure_auto: req.exposure_auto.unwrap_or(false),
            ip_address: req.ip_address.clone(),
            camera_supplier: req.camera_supplier.clone()
                .map(|s| s.parse().unwrap_or(crate::service::camera::CameraSupplier::IMV))
                .unwrap_or(crate::service::camera::CameraSupplier::IMV),
        };

        let updated = manager.update_camera(id, config).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to update camera");
            errors::Error::InternalError(ErrorCode::InternalError)
        })?;

        Ok(WebResponse::with_result(updated.into()).into())
    }

    /// Delete a camera config
    #[delete("/delete/{id}")]
    pub async fn delete(
        manager: web::Data<CameraManager>,
        path: web::Path<Uuid>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        let id = path.into_inner();

        manager.delete_camera(id).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to delete camera");
            errors::Error::InternalError(ErrorCode::InternalError)
        })?;

        Ok(WebResponse::with_result(()).into())
    }

    /// Get a camera config by ID
    #[get("/get/{id}")]
    pub async fn get(
        manager: web::Data<CameraManager>,
        path: web::Path<Uuid>,
    ) -> actix_web::Result<web::Json<WebResponse<CameraConfigResponse>>, errors::Error> {
        let id = path.into_inner();

        let config = manager.get_camera(id).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to get camera");
            errors::Error::BadRequest(ErrorCode::NotFound, "配置没找到".into())
        })?;

        Ok(WebResponse::with_result(config.into()).into())
    }

    /// List camera configs with pagination
    #[get("/list")]
    pub async fn list(
        manager: web::Data<CameraManager>,
        query: web::Query<CameraConfigListRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<Pagination<CameraConfigResponse>>>, errors::Error> {
        let page = query.page.unwrap_or(1);
        let size = query.size.unwrap_or(10);

        let page_result = manager.list_cameras(page, size, query.enabled)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to list cameras");
                errors::Error::InternalError(ErrorCode::InternalError)
            })?;

        let pagination: Pagination<CameraConfigResponse> = Pagination {
            records: page_result.records.into_iter().map(|c| {
                let model = crate::database::entity::t_camera_configs::Model {
                    id: c.id.unwrap(),
                    device_user_id: c.device_user_id,
                    key: c.key,
                    serial_number: c.serial_number,
                    vendor: c.vendor,
                    model: c.model,
                    manufacture_info: c.manufacture_info,
                    device_version: c.device_version,
                    exposure_time_ms: c.exposure_time_ms,
                    exposure_auto: c.exposure_auto,
                    ip_address: c.ip_address,
                    camera_supplier: c.camera_supplier.to_string(),
                    enabled: true,
                };
                CameraConfigResponse::from(model)
            }).collect(),
            total: page_result.total_count,
            current: page_result.page_index,
            size: page_result.page_size,
            pages: page_result.pages,
        };

        Ok(WebResponse::with_result(pagination).into())
    }

    /// Set camera enabled status
    #[post("/set-enabled/{id}")]
    pub async fn set_enabled(
        manager: web::Data<CameraManager>,
        path: web::Path<Uuid>,
        req: web::Json<SetEnabledRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<CameraConfigResponse>>, errors::Error> {
        let id = path.into_inner();

        let config = manager.set_camera_enabled(id, req.enabled).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to set camera enabled");
            errors::Error::InternalError(ErrorCode::InternalError)
        })?;

        Ok(WebResponse::with_result(config.into()).into())
    }

    /// Start camera stream
    #[post("/stream/start")]
    pub async fn stream_start(
        app_state: web::Data<AppState>,
        manager: web::Data<CameraManager>,
        req: web::Json<StreamStartRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        stream_start_inner(app_state, manager, &req.into_inner()).await
    }

    /// Stop camera stream
    #[post("/stream/stop")]
    pub async fn stream_stop(
        manager: web::Data<CameraManager>,
        req: web::Json<StreamStopRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        manager.stop_stream(&req.id).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to stop stream");
            errors::Error::InternalError(ErrorCode::InternalError)
        })?;

        Ok(WebResponse::with_result(()).into())
    }

    /// Update stream config (restarts stream if active)
    #[post("/stream/update-config")]
    pub async fn stream_update_config(
        app_state: web::Data<AppState>,
        manager: web::Data<CameraManager>,
        req: web::Json<StreamUpdateConfigRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        if manager.is_active_stream(&req.id) {
            manager.stop_stream(&req.id).await.map_err(|e| {
                tracing::error!(error = ?e, "Failed to stop stream");
                errors::Error::InternalError(ErrorCode::InternalError)
            })?;
            let start_req = StreamStartRequest {
                id: req.id,
                config: req.config.clone(),
            };
            return stream_start_inner(app_state, manager, &start_req).await;
        }

        manager
            .update_stream_config(req.id, req.config.clone())
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to update stream config");
                errors::Error::InternalError(ErrorCode::InternalError)
            })?;

        Ok(WebResponse::with_result(()).into())
    }

    /// Initialize all enabled cameras from database
    #[post("/initialize")]
    pub async fn initialize(
        manager: web::Data<CameraManager>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        manager.initialize_from_database().await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to initialize cameras");
            errors::Error::InternalError(ErrorCode::InternalError)
        })?;

        Ok(WebResponse::with_result(()).into())
    }

    /// Open a camera
    #[post("/open/{id}")]
    pub async fn open(
        manager: web::Data<CameraManager>,
        path: web::Path<Uuid>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        let id = path.into_inner();

        manager.open_camera(&id).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to open camera");
            errors::Error::InternalError(ErrorCode::InternalError)
        })?;

        Ok(WebResponse::with_result(()).into())
    }

    /// Close a camera
    #[post("/close/{id}")]
    pub async fn close(
        manager: web::Data<CameraManager>,
        path: web::Path<Uuid>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        let id = path.into_inner();

        manager.close_camera(&id).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to close camera");
            errors::Error::InternalError(ErrorCode::InternalError)
        })?;

        Ok(WebResponse::with_result(()).into())
    }
}
