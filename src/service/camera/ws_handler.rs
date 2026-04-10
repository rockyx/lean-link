use actix_web::web;
use tokio::sync::broadcast::error::RecvError;

use crate::{
    AppState,
    service::{
        camera::{manager::CameraManager, stream::CameraControlMessage},
        websocket::WsMessage,
    },
};

/// Topic for camera stream control (start/stop)
pub const CAMERA_CONTROL_TOPIC: &str = "camera/control";

/// Topic prefix for camera stream messages
pub const CAMERA_STREAM_TOPIC: &str = "camera/stream";

/// This will start grab also
pub async fn handle_camera_control(
    control: &CameraControlMessage,
    app_state: web::Data<AppState>,
    camera_manager: web::Data<CameraManager>,
) {
    match control {
        CameraControlMessage::StartStream { camera_id, config } => {
            let _ = camera_manager
                .update_stream_config(camera_id.clone(), config.clone())
                .await;
            if let Ok(stream) = camera_manager.start_stream(camera_id).await {
                let stream = stream.lock().await;
                let mut stream_rx = stream.subscribe().await;
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

                if let Ok(mut frame_rx) = camera_manager
                    .start_grabbing(camera_id, super::GrabMode::Continuous)
                    .await
                {
                    let camera_id = camera_id.clone();
                    tokio::spawn(async move {
                        loop {
                            match frame_rx.recv().await {
                                Some(frame) => {
                                    if let Ok(stream) = camera_manager.get_stream(&camera_id).await {
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
                }
            }
        }
        CameraControlMessage::StopStream { camera_id } => {
            let _ = camera_manager.stop_stream(camera_id).await;
        }
        CameraControlMessage::UpdateConfig { camera_id, config } => {
            let _ = camera_manager
                .update_stream_config(camera_id.clone(), config.clone())
                .await;
        }
    }
}
