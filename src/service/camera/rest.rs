use actix_web::{scope, web};
use serde::{Deserialize, Serialize};

use crate::{
    AppState, errors,
    service::{
        camera::{manager::CameraManager, stream::CameraStreamConfig},
        web::service::WebResponse,
        websocket::WsMessage,
    },
};

/// Topic for camera stream control (start/stop)
pub const CAMERA_CONTROL_TOPIC: &str = "camera/control";

/// Topic prefix for camera stream messages
pub const CAMERA_STREAM_TOPIC: &str = "camera/stream";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStartRequest {
    id: uuid::Uuid,
    config: CameraStreamConfig,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStopRequest {
    id: uuid::Uuid,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamUpdateConfigRequest {
    id: uuid::Uuid,
    config: CameraStreamConfig,
}

pub async fn stream_start(
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
        .start_grabbing(&req.id, crate::service::camera::GrabMode::Continuous)
        .await?;

    tokio::spawn(async move {
        loop {
            match stream_rx.recv().await {
                Ok(payload) => {
                    let _ = app_state
                        .ws_server
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

    let camera_id = req.id.clone();
    tokio::spawn(async move {
        loop {
            match frame_rx.recv().await {
                Some(frame) => {
                    if let Ok(stream) = manager.get_stream(&camera_id).await {
                        let stream = stream.lock().await;
                        stream.trigger_frame(&frame).await;
                    }
                }
                None => {
                    tracing::debug!("Stream fream recv closed");
                    break;
                }
            }
        }
    });

    Ok(WebResponse::with_result(()).into())
}

#[scope("/camera")]
pub mod api {
    use actix_web::{post, web};

    use crate::{
        AppState, errors,
        service::{
            camera::{
                CameraInfo,
                manager::CameraManager,
                rest::{self, StreamStartRequest, StreamStopRequest, StreamUpdateConfigRequest},
            },
            web::service::{ErrorCode, WebResponse},
        },
    };

    /// Enumerate all connected industrial cameras in the system

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

    #[post("/stream/start")]
    pub async fn stream_start(
        app_state: web::Data<AppState>,
        manager: web::Data<CameraManager>,
        req: web::Json<StreamStartRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        rest::stream_start(app_state, manager, &req.into_inner()).await
    }

    #[post("/stream/stop")]
    pub async fn stream_stop(
        manager: web::Data<CameraManager>,
        req: web::Json<StreamStopRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        manager.stop_stream(&req.id).await?;
        Ok(WebResponse::with_result(()).into())
    }

    #[post("/stream/update-config")]
    pub async fn stream_update_config(
        app_state: web::Data<AppState>,
        manager: web::Data<CameraManager>,
        req: web::Json<StreamUpdateConfigRequest>,
    ) -> actix_web::Result<web::Json<WebResponse<()>>, errors::Error> {
        if manager.is_active_stream(&req.id) {
            manager.stop_stream(&req.id).await?;
            let req = StreamStartRequest {
                id: req.id.clone(),
                config: req.config.clone(),
            };
            return rest::stream_start(app_state, manager, &req).await;
        }

        manager
            .update_stream_config(req.id, req.config.clone())
            .await?;
        Ok(WebResponse::with_result(()).into())
    }
}
