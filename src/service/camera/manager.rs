use dashmap::DashMap;
use sea_orm::{ActiveValue, DatabaseConnection};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc};

use crate::database::entity::t_camera_configs;
use crate::service::camera::{
    CameraConfig, CameraError, CameraFrame, CameraInfo, CameraSupplier, FrameSize, GrabMode,
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
    db_conn: DatabaseConnection,
    cameras: DashMap<uuid::Uuid, ManagedCamera>,
    available_cameras: Arc<RwLock<Vec<CameraInfo>>>,
    streams: DashMap<uuid::Uuid, Arc<Mutex<ActiveStream>>>,
    stream_configs: DashMap<uuid::Uuid, CameraStreamConfig>,
}

impl CameraManager {
    fn new(db_conn: DatabaseConnection) -> Self {
        Self {
            db_conn,
            cameras: DashMap::new(),
            available_cameras: Arc::new(RwLock::new(Vec::new())),
            streams: DashMap::new(),
            stream_configs: DashMap::new(),
        }
    }

    pub fn new_arc(db_conn: DatabaseConnection) -> ArcCameraManager {
        Arc::new(Self::new(db_conn))
    }

    // ==================== Camera Enumeration ====================

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

    pub async fn get_available_cameras(&self) -> Vec<CameraInfo> {
        let available = self.available_cameras.read().await;
        available.clone()
    }

    // ==================== Database CRUD Operations ====================

    /// Initialize cameras from database (called at system startup)
    pub async fn initialize_from_database(&self) -> Result<(), CameraError> {
        self.enumerate_cameras().await?;

        let configs =
            crate::database::camera_configs::find_camera_configs_by_enabled(&self.db_conn, true)
                .await
                .map_err(|e| CameraError::Config(format!("读取数据库失败: {}", e)))?;

        for model in configs {
            let config: CameraConfig = model.into();
            let id = config.id.unwrap();

            if !self.cameras.contains_key(&id) {
                let managed = ManagedCamera::new(config.clone());
                self.cameras.insert(id, managed);
                tracing::info!(
                    "Initialized camera {} (key: {}, serial: {})",
                    config.name(),
                    config.key.unwrap_or_default(),
                    config.serial_number.unwrap_or_default()
                );
            }
        }

        tracing::info!("Initialized {} cameras from database", self.cameras.len());
        Ok(())
    }

    /// Create a new camera config (insert to database and add to memory)
    pub async fn create_camera(&self, config: CameraConfig) -> Result<CameraConfig, CameraError> {
        let id = uuid::Uuid::now_v7();
        let active_model = t_camera_configs::ActiveModel {
            id: ActiveValue::set(id),
            device_user_id: ActiveValue::set(config.device_user_id.clone()),
            key: ActiveValue::set(config.key.clone()),
            serial_number: ActiveValue::set(config.serial_number.clone()),
            vendor: ActiveValue::set(config.vendor.clone()),
            model: ActiveValue::set(config.model.clone()),
            manufacture_info: ActiveValue::set(config.manufacture_info.clone()),
            device_version: ActiveValue::set(config.device_version.clone()),
            exposure_time_ms: ActiveValue::set(config.exposure_time_ms),
            exposure_auto: ActiveValue::set(config.exposure_auto),
            ip_address: ActiveValue::set(config.ip_address.clone()),
            camera_supplier: ActiveValue::set(config.camera_supplier.to_string()),
            enabled: ActiveValue::set(true),
        };

        crate::database::camera_configs::insert_camera_config(&self.db_conn, active_model)
            .await
            .map_err(|e| CameraError::Config(format!("写入数据库失败: {}", e)))?;

        let model = crate::database::camera_configs::find_camera_config_by_id(&self.db_conn, id)
            .await
            .map_err(|e| CameraError::Config(format!("读取数据库失败: {}", e)))?
            .ok_or_else(|| CameraError::Config("创建后未找到配置".into()))?;

        let new_config: CameraConfig = model.clone().into();

        // Add to memory
        let managed = ManagedCamera::new(new_config.clone());
        self.cameras.insert(id, managed);

        tracing::info!("Created camera {} (id: {})", new_config.name(), id);
        Ok(new_config)
    }

    /// Get camera config from database by ID
    pub async fn get_camera(&self, id: uuid::Uuid) -> Result<CameraConfig, CameraError> {
        let model = crate::database::camera_configs::find_camera_config_by_id(&self.db_conn, id)
            .await
            .map_err(|e| CameraError::Config(format!("读取数据库失败: {}", e)))?
            .ok_or_else(|| CameraError::CameraNotFound(id.to_string()))?;

        Ok(model.into())
    }

    /// List camera configs from database with pagination
    pub async fn list_cameras(
        &self,
        page: u64,
        size: u64,
        enabled: Option<bool>,
    ) -> Result<crate::database::entity::PageResult<CameraConfig>, CameraError> {
        let page_result = if let Some(en) = enabled {
            let records =
                crate::database::camera_configs::find_camera_configs_by_enabled(&self.db_conn, en)
                    .await
                    .map_err(|e| CameraError::Config(format!("读取数据库失败: {}", e)))?;
            crate::database::entity::PageResult {
                records: records.into_iter().map(|m| m.into()).collect(),
                page_index: 1,
                page_size: size,
                total_count: 0,
                pages: 1,
            }
        } else {
            let result =
                crate::database::camera_configs::page_camera_configs(&self.db_conn, page, size)
                    .await
                    .map_err(|e| CameraError::Config(format!("读取数据库失败: {}", e)))?;
            crate::database::entity::PageResult {
                records: result.records.into_iter().map(|m| m.into()).collect(),
                page_index: result.page_index,
                page_size: result.page_size,
                total_count: result.total_count,
                pages: result.pages,
            }
        };

        Ok(page_result)
    }

    /// Update camera config (update database and memory)
    pub async fn update_camera(
        &self,
        id: uuid::Uuid,
        config: CameraConfig,
    ) -> Result<CameraConfig, CameraError> {
        let existing = crate::database::camera_configs::find_camera_config_by_id(&self.db_conn, id)
            .await
            .map_err(|e| CameraError::Config(format!("读取数据库失败: {}", e)))?
            .ok_or_else(|| CameraError::CameraNotFound(id.to_string()))?;

        let active_model = t_camera_configs::ActiveModel {
            id: ActiveValue::set(id),
            device_user_id: ActiveValue::set(config.device_user_id.or(existing.device_user_id)),
            key: ActiveValue::set(config.key.or(existing.key)),
            serial_number: ActiveValue::set(config.serial_number.or(existing.serial_number)),
            vendor: ActiveValue::set(config.vendor.or(existing.vendor)),
            model: ActiveValue::set(config.model.or(existing.model)),
            manufacture_info: ActiveValue::set(
                config.manufacture_info.or(existing.manufacture_info),
            ),
            device_version: ActiveValue::set(config.device_version.or(existing.device_version)),
            exposure_time_ms: ActiveValue::set(config.exposure_time_ms),
            exposure_auto: ActiveValue::set(config.exposure_auto),
            ip_address: ActiveValue::set(config.ip_address.or(existing.ip_address)),
            camera_supplier: ActiveValue::set(config.camera_supplier.to_string()),
            enabled: ActiveValue::set(existing.enabled),
        };

        crate::database::camera_configs::update_camera_config(&self.db_conn, id, active_model)
            .await
            .map_err(|e| CameraError::Config(format!("更新数据库失败: {}", e)))?
            .ok_or_else(|| CameraError::Config("更新失败".into()))?;

        let updated_config = self.get_camera(id).await?;

        // Update memory if camera is loaded
        if let Some(mut managed) = self.cameras.get_mut(&id) {
            managed.config = updated_config.clone();
        }

        tracing::info!("Updated camera {} (id: {})", updated_config.name(), id);
        Ok(updated_config)
    }

    /// Delete camera config (delete from database and remove from memory)
    pub async fn delete_camera(&self, id: uuid::Uuid) -> Result<(), CameraError> {
        // Stop stream if active
        if self.streams.contains_key(&id) {
            self.stop_stream(&id).await?;
        }

        // Close camera if opened
        if self.cameras.contains_key(&id) {
            self.close_camera(&id).await?;
        }

        // Delete from database
        let deleted = crate::database::camera_configs::delete_camera_config(&self.db_conn, id)
            .await
            .map_err(|e| CameraError::Config(format!("删除数据库失败: {}", e)))?;

        if !deleted {
            return Err(CameraError::CameraNotFound(id.to_string()));
        }

        // Remove from memory
        self.cameras.remove(&id);
        self.stream_configs.remove(&id);

        tracing::info!("Deleted camera (id: {})", id);
        Ok(())
    }

    /// Set camera enabled status
    pub async fn set_camera_enabled(
        &self,
        id: uuid::Uuid,
        enabled: bool,
    ) -> Result<CameraConfig, CameraError> {
        let existing = crate::database::camera_configs::find_camera_config_by_id(&self.db_conn, id)
            .await
            .map_err(|e| CameraError::Config(format!("读取数据库失败: {}", e)))?
            .ok_or_else(|| CameraError::CameraNotFound(id.to_string()))?;

        let active_model = t_camera_configs::ActiveModel {
            id: ActiveValue::set(id),
            device_user_id: ActiveValue::set(existing.device_user_id),
            key: ActiveValue::set(existing.key),
            serial_number: ActiveValue::set(existing.serial_number),
            vendor: ActiveValue::set(existing.vendor),
            model: ActiveValue::set(existing.model),
            manufacture_info: ActiveValue::set(existing.manufacture_info),
            device_version: ActiveValue::set(existing.device_version),
            exposure_time_ms: ActiveValue::set(existing.exposure_time_ms),
            exposure_auto: ActiveValue::set(existing.exposure_auto),
            ip_address: ActiveValue::set(existing.ip_address),
            camera_supplier: ActiveValue::set(existing.camera_supplier),
            enabled: ActiveValue::set(enabled),
        };

        crate::database::camera_configs::update_camera_config(&self.db_conn, id, active_model)
            .await
            .map_err(|e| CameraError::Config(format!("更新数据库失败: {}", e)))?
            .ok_or_else(|| CameraError::Config("更新失败".into()))?;

        let updated_config: CameraConfig = self.get_camera(id).await?.into();

        // Update memory or remove if disabled
        if enabled {
            if !self.cameras.contains_key(&id) {
                let managed = ManagedCamera::new(updated_config.clone());
                self.cameras.insert(id, managed);
            }
        } else {
            // Stop and remove from memory if disabled
            if self.streams.contains_key(&id) {
                let _ = self.stop_stream(&id).await;
            }
            if self.cameras.contains_key(&id) {
                let _ = self.close_camera(&id).await;
                self.cameras.remove(&id);
            }
        }

        tracing::info!("Set camera {} enabled: {}", id, enabled);
        Ok(updated_config)
    }

    // ==================== Camera Operations ====================

    /// Add camera to memory without database operation (for internal use)
    pub async fn add_camera_to_memory(&self, config: CameraConfig) -> Result<(), CameraError> {
        if config.id.is_none() {
            return Err(CameraError::Config("配置缺少ID".into()));
        }

        let id = config.id.unwrap();

        if self.cameras.contains_key(&id) {
            return Err(CameraError::AddCamera("相机已在内存中".into()));
        }

        let managed = ManagedCamera::new(config.clone());
        self.cameras.insert(id, managed);

        tracing::info!(
            "Added camera to memory {} (key: {}, serial: {})",
            config.name(),
            config.key.unwrap_or_default(),
            config.serial_number.unwrap_or_default()
        );

        Ok(())
    }

    pub async fn open_camera(&self, id: &uuid::Uuid) -> Result<(), CameraError> {
        let managed = self
            .cameras
            .get(id)
            .ok_or_else(|| CameraError::CameraNotFound(id.to_string()))?;

        let mut managed_camera = managed.camera.lock().await;

        if let Some(camera) = managed_camera.as_ref() {
            if camera.is_opened() {
                tracing::warn!(
                    "Camera {} (id {}) is already opened",
                    managed.config.name(),
                    id,
                );
                return Ok(());
            }
        }

        let cam = create_camera(&managed.config)?;
        cam.open()?;
        *managed_camera = Some(cam);

        tracing::info!("Opened camera {} (id {})", managed.config.name(), id);
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
            camera.set_grab_mode(mode);

            if !camera.is_opened() {
                return Err(CameraError::NotOpened(id.to_string()));
            }

            let frame_rx = camera.create_frame_channel();
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

    pub async fn trigger_frame(&self, id: &uuid::Uuid) -> Result<CameraFrame, CameraError> {
        let managed = self
            .cameras
            .get(id)
            .ok_or_else(|| CameraError::CameraNotFound(id.to_string()))?;

        if !managed.is_grabbing {
            return Err(CameraError::NotGrabbing(id.to_string()));
        }

        if let Some(camera) = managed.camera.lock().await.as_ref() {
            return camera.trigger_one_frame();
        }

        Err(CameraError::CameraNotFound(id.to_string()))
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
        self.stop_stream(id).await?;
        tracing::info!("Closed camera {} (id {})", managed.config.name(), id);

        Ok(())
    }

    pub async fn close_all(&self) -> Result<(), CameraError> {
        let ids = self.get_camera_ids();

        for id in ids {
            self.stop_stream(&id).await?;
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

    pub fn is_camera_loaded(&self, id: &uuid::Uuid) -> bool {
        self.cameras.contains_key(id)
    }

    // ==================== Stream Operations ====================

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
            tracing::warn!("not camera stream: {}", id);
            return Ok(());
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

        Ok(())
    }

    pub fn is_active_stream(&self, id: &uuid::Uuid) -> bool {
        self.streams.contains_key(id)
    }
}

pub type ArcCameraManager = Arc<CameraManager>;
