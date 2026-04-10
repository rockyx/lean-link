use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc};

use crate::service::camera::{
    self, CameraConfig, CameraError, CameraFrame, CameraInfo, CameraSupplier, FrameSize, GrabMode,
    IndustryCamera,
    inner::imv_camera,
    stream::{ActiveStream, CameraStreamConfig},
};

fn create_camera(config: &CameraConfig) -> Result<Box<dyn IndustryCamera>, CameraError> {
    match config.camera_supplier {
        CameraSupplier::IMV => {
            let camera = imv_camera::IMVCameraBuilder::new_with_config(config).build()?;
            return Ok(Box::new(camera));
        }
    }
}

pub struct ManagedCamera {
    camera: Arc<Mutex<Option<Box<dyn IndustryCamera>>>>,
    config: CameraConfig,
    is_grabbing: bool,
}

impl ManagedCamera {
    pub fn new(config: CameraConfig) -> Self {
        Self {
            camera: Arc::new(Mutex::new(None)),
            config,
            is_grabbing: false,
        }
    }
}

pub struct CameraManager {
    cameras: DashMap<uuid::Uuid, ManagedCamera>,
    available_cameras: Arc<RwLock<Vec<CameraInfo>>>,
    streams: DashMap<uuid::Uuid, Arc<Mutex<ActiveStream>>>,
    stream_configs: DashMap<uuid::Uuid, CameraStreamConfig>,
}

impl CameraManager {
    pub fn new() -> Self {
        Self {
            cameras: DashMap::new(),
            available_cameras: Arc::new(RwLock::new(Vec::new())),
            streams: DashMap::new(),
            stream_configs: DashMap::new(),
        }
    }

    pub async fn enumerate_cameras(&self) -> Result<Vec<CameraInfo>, CameraError> {
        let cameras = imv_camera::get_camera_list()
            .map_err(|e| CameraError::EnumerationError(format!("{:?}", e)))?;

        let mut available = self.available_cameras.write().await;
        *available = cameras;

        tracing::info!("Found {} available cameras", available.len());
        for (_, cam) in available.iter().enumerate() {
            tracing::info!(
                "  [{}] {} - {} ({})",
                cam.camera_supplier,
                cam.device_user_id,
                cam.model,
                cam.serial_number
            );
        }

        Ok(available.clone())
    }

    pub async fn get_avilable_cameras(&self) -> Vec<CameraInfo> {
        let available = self.available_cameras.read().await;
        available.clone()
    }

    pub async fn initialize_from_config(
        &self,
        configs: &[CameraConfig],
    ) -> Result<(), CameraError> {
        self.enumerate_cameras().await?;

        for config in configs {
            self.add_camera(config.clone()).await?;
        }

        Ok(())
    }

    pub async fn add_camera(&self, config: CameraConfig) -> Result<(), CameraError> {
        if config.id.is_none() {
            return Err(CameraError::Config("配置错误".into()));
        }

        let id = config.id.unwrap();

        if self.cameras.contains_key(&id) {
            return Err(CameraError::AddCamera("重复添加".into()));
        }
        let managed = ManagedCamera::new(config.clone());

        self.cameras.insert(id, managed);

        tracing::info!(
            "Added camera {} (config device user id: {}, serial number: {})",
            config.key.unwrap_or_default(),
            config.device_user_id.unwrap_or_default(),
            config.serial_number.unwrap_or_default(),
        );

        Ok(())
    }

    pub async fn open_camera(&self, id: &uuid::Uuid) -> Result<(), CameraError> {
        let managed = self
            .cameras
            .get(id)
            .ok_or_else(|| CameraError::CameraNotFound(id.to_string()))?;

        let mut managed_camera = managed.camera.lock().await;

        if let Some(camera) = managed.camera.lock().await.as_ref() {
            if camera.is_opened() {
                tracing::warn!(
                    "Camera {} (id {}) is already opened",
                    managed.config.name(),
                    id,
                );
                return Ok(());
            }
            let cam = create_camera(&managed.config)?;
            cam.open()?;

            *managed_camera = Some(cam);

            tracing::info!("Opened camera {} (id {})", managed.config.name(), id);
        } else {
            return Err(CameraError::OpenError("相机创建失败".into()));
        }
        Ok(())
    }

    pub async fn open_all(&self) -> Result<(), CameraError> {
        let ids = self.get_camera_ids();

        for id in ids {
            self.open_camera(&id).await?;
        }

        Ok(())
    }

    pub async fn start_grabbing(
        &self,
        id: &uuid::Uuid,
        mode: GrabMode,
    ) -> Result<mpsc::Receiver<CameraFrame>, CameraError> {
        let mut managed = self
            .cameras
            .get_mut(id)
            .ok_or_else(|| CameraError::CameraNotFound(id.to_string()))?;

        let frame_rx = if let Some(camera) = managed.camera.lock().await.as_mut() {
            // Set grab mode
            camera.set_grab_mode(mode);

            if !camera.is_opened() {
                return Err(CameraError::NotOpened(id.to_string()));
            }
            // Create frame channel
            let frame_rx = camera.create_frame_channel();

            // Start grabbing
            camera.start_grab()?;

            frame_rx
        } else {
            return Err(CameraError::NotOpened(id.to_string()));
        };

        managed.is_grabbing = true;

        let mode_str = match mode {
            GrabMode::Continuous => "Continuous",
            GrabMode::SingleFrame => "SingleFrame",
        };

        tracing::info!(
            "Started grabbing from camera {} (id {}, mode: {})",
            managed.config.name(),
            id,
            mode_str
        );

        Ok(frame_rx)
    }

    pub async fn stop_grabbing(&self, id: &uuid::Uuid) -> Result<(), CameraError> {
        let mut managed = self
            .cameras
            .get_mut(id)
            .ok_or_else(|| CameraError::CameraNotFound(id.to_string()))?;

        if managed.is_grabbing {
            if let Some(camera) = managed.camera.lock().await.as_mut() {
                camera.stop_grab()?;
            }
            managed.is_grabbing = false;

            tracing::info!(
                "Stopped grabbing from camera {} (id {})",
                managed.config.name(),
                id
            );
        }
        Ok(())
    }

    pub async fn trigger_frame(&self, id: &uuid::Uuid) -> Result<(), CameraError> {
        let managed = self
            .cameras
            .get(id)
            .ok_or_else(|| CameraError::CameraNotFound(id.to_string()))?;

        if !managed.is_grabbing {
            return Err(CameraError::NotGrabbing(id.to_string()));
        }

        if let Some(camera) = managed.camera.lock().await.as_ref() {
            camera.trigger_one_frame()?;
        }

        Ok(())
    }

    pub async fn get_frame_size(&self, id: &uuid::Uuid) -> Result<FrameSize, CameraError> {
        let managed = self
            .cameras
            .get(id)
            .ok_or_else(|| CameraError::CameraNotFound(id.to_string()))?;

        let camera = managed.camera.lock().await;

        let camera = camera
            .as_ref()
            .ok_or_else(|| CameraError::NotOpened(id.to_string()))?;

        if !camera.is_opened() {
            return Err(CameraError::NotOpened(id.to_string()));
        }

        camera.frame_size()
    }

    pub async fn close_camera(&self, id: &uuid::Uuid) -> Result<(), CameraError> {
        let mut managed = self
            .cameras
            .get_mut(id)
            .ok_or_else(|| CameraError::CameraNotFound(id.to_string()))?;

        if managed.is_grabbing {
            if let Some(camera) = managed.camera.lock().await.as_mut() {
                camera.stop_grab()?;
                camera.close()?;
            }
            managed.is_grabbing = false;
        }
        tracing::info!("Closed camera {} (id {})", managed.config.name(), id);

        Ok(())
    }

    pub async fn close_all(&self) -> Result<(), CameraError> {
        let ids = self.get_camera_ids();

        for id in ids {
            self.close_camera(&id).await?;
        }
        Ok(())
    }

    pub fn get_camera_ids(&self) -> Vec<uuid::Uuid> {
        self.cameras
            .iter()
            .map(|c| c.key().clone())
            .collect::<Vec<uuid::Uuid>>()
    }

    pub async fn is_grabbing(&self, id: &uuid::Uuid) -> bool {
        if let Some(managed) = self.cameras.get(id) {
            return managed.is_grabbing;
        }
        false
    }

    pub async fn get_stream(
        &self,
        id: &uuid::Uuid,
    ) -> Result<Arc<Mutex<ActiveStream>>, CameraError> {
        if let Some(stream) = self.streams.get(id) {
            return Ok(stream.value().clone());
        }
        Err(CameraError::CameraNotFound("无此相机流".into()))
    }

    pub async fn start_stream(
        &self,
        id: &uuid::Uuid,
    ) -> Result<Arc<Mutex<ActiveStream>>, CameraError> {
        let managed = self.cameras.get(id);
        if managed.is_none() {
            return Err(CameraError::CameraNotFound(format!("无此相机 {}", id)));
        }

        let camera = managed.unwrap();
        let camera_config = camera.config.clone();

        if camera_config.id.is_none() {
            return Err(CameraError::Config("配置错误".into()));
        }

        let id = camera_config.id.unwrap();

        if let Some(stream) = self.streams.get(&id) {
            return Ok(stream.clone());
        }

        let stream_config = match self.stream_configs.get(&id) {
            Some(config) => config.value().clone(),
            None => CameraStreamConfig::default(),
        };
        let active_stream = Arc::new(Mutex::new(ActiveStream::new(
            id,
            stream_config,
            camera_config,
        )));
        self.streams.insert(id, active_stream.clone());

        tracing::info!("Added stream {}", id);

        Ok(active_stream)
    }

    pub async fn stop_stream(&self, id: &uuid::Uuid) -> Result<(), CameraError> {
        if !self.streams.contains_key(id) {
            return Err(CameraError::CameraNotFound(format!("无此相机 {}", id)));
        }

        let stream = self.get_stream(id).await?;
        let stream = stream.lock().await;
        stream.stop_stream().await;
        self.streams.remove(id);

        Ok(())
    }

    pub async fn update_stream_config(
        &self,
        id: uuid::Uuid,
        config: CameraStreamConfig,
    ) -> Result<(), CameraError> {
        if !self.cameras.contains_key(&id) {
            return Err(CameraError::CameraNotFound(format!("无此相机 {}", id)));
        }

        self.stream_configs.insert(id, config);
        if self.streams.contains_key(&id) {
            self.stop_stream(&id).await?;
            self.start_stream(&id).await?;
        }
        Ok(())
    }
}
