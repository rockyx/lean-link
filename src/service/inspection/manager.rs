use std::sync::Arc;

use dashmap::DashMap;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::service::inspection::station::{RoiConfig, RoiShape, StationConfig, TriggerMode};
use crate::{
    database::{
        entity::{t_inspection_stations, t_station_rois},
        inspection_stations::{
            delete_inspection_station, delete_station_roi, find_all_inspection_stations,
            find_inspection_station_by_id, find_station_rois_by_station_id,
            insert_inspection_station, insert_station_roi, update_inspection_station,
            update_station_roi,
        },
    },
    errors,
};

/// Managed station frame containing station config and ROIs
#[derive(Clone, Debug)]
pub struct ManagedStation {
    pub config: StationConfig,
    pub rois: Vec<RoiConfig>,
}

/// Station manager for managing inspection stations in memory
pub struct StationManager {
    db_conn: DatabaseConnection,
    stations: DashMap<Uuid, ManagedStation>,
    enabled_stations: Arc<RwLock<Vec<Uuid>>>,
}

impl StationManager {
    fn new(db_conn: DatabaseConnection) -> Self {
        Self {
            db_conn,
            stations: DashMap::new(),
            enabled_stations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn new_arc(db_conn: DatabaseConnection) -> ArcStationManager {
        Arc::new(Self::new(db_conn))
    }

    /// Initialize stations from database
    pub async fn initialize_from_database(&self) -> Result<(), errors::Error> {
        tracing::info!("Initializing inspection stations from database...");

        // Load all stations
        let db_stations = find_all_inspection_stations(&self.db_conn).await?;

        let mut enabled_ids = Vec::new();

        for station in db_stations {
            // Load ROIs for each station
            let db_rois = find_station_rois_by_station_id(&self.db_conn, station.id).await?;

            // Convert database model to service model
            let config = Self::db_station_to_config(&station);
            let rois: Vec<RoiConfig> = db_rois
                .into_iter()
                .filter_map(|roi| Self::db_roi_to_config(&roi))
                .collect();

            if station.is_enabled {
                enabled_ids.push(station.id);
            }

            self.stations
                .insert(station.id, ManagedStation { config, rois });
        }

        // Update enabled stations list
        let mut enabled = self.enabled_stations.write().await;
        *enabled = enabled_ids;

        tracing::info!("Loaded {} inspection stations", self.stations.len());
        Ok(())
    }

    /// Convert database station model to service config
    fn db_station_to_config(station: &t_inspection_stations::Model) -> StationConfig {
        let detection_types: Vec<String> =
            serde_json::from_value(station.detection_types.clone()).unwrap_or_default();

        StationConfig {
            id: station.id.clone(),
            name: station.name.clone(),
            camera_id: station.camera_id,
            trigger_mode: station.trigger_mode,
            detection_types,
            rois: vec![],
            is_enabled: station.is_enabled,
            model_path: station.model_path.clone(),
            confidence_threshold: station.confidence_threshold,
            serial_port: station.serial_port.clone(),
        }
    }

    /// Convert database ROI model to service config
    fn db_roi_to_config(roi: &t_station_rois::Model) -> Option<RoiConfig> {
        let shape: Option<RoiShape> = serde_json::from_value(roi.shape.clone()).ok();
        shape.map(|s| RoiConfig {
            id: roi.id,
            name: roi.name.clone(),
            shape: s,
            purpose: roi.purpose,
            enabled: roi.enabled,
        })
    }

    /// Get all stations
    pub fn get_all_stations(&self) -> Vec<ManagedStation> {
        self.stations
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get enabled stations
    pub async fn get_enabled_stations(&self) -> Vec<ManagedStation> {
        let enabled_ids = self.enabled_stations.read().await;
        enabled_ids
            .iter()
            .filter_map(|id| self.stations.get(id).map(|s| s.value().clone()))
            .collect()
    }

    /// Get station by ID
    pub fn get_station(&self, id: Uuid) -> Option<ManagedStation> {
        self.stations.get(&id).map(|s| s.value().clone())
    }

    /// Create a new station
    pub async fn create_station(
        &self,
        request: StationCreateRequest,
    ) -> Result<Uuid, errors::Error> {
        let id = Uuid::now_v7();
        let detection_types_json = serde_json::to_value(&request.detection_types)?;

        let is_enabled = request.is_enabled.unwrap_or(true);
        let confidence_threshold = request.confidence_threshold.unwrap_or(0.5);
        let trigger_mode = request.trigger_mode.unwrap_or_default();

        let active_model = t_inspection_stations::ActiveModel {
            id: sea_orm::Set(id),
            name: sea_orm::Set(request.name.clone()),
            camera_id: sea_orm::Set(request.camera_id),
            trigger_mode: sea_orm::Set(trigger_mode),
            detection_types: sea_orm::Set(detection_types_json),
            is_enabled: sea_orm::Set(is_enabled),
            model_path: sea_orm::Set(request.model_path.clone()),
            confidence_threshold: sea_orm::Set(confidence_threshold),
            serial_port: sea_orm::Set(request.serial_port.clone()),
            ..Default::default()
        };

        insert_inspection_station(&self.db_conn, active_model).await?;

        // Add to memory
        let config = StationConfig {
            id,
            name: request.name,
            camera_id: request.camera_id,
            trigger_mode,
            detection_types: request.detection_types,
            rois: vec![],
            is_enabled,
            model_path: request.model_path,
            confidence_threshold,
            serial_port: request.serial_port,
        };

        self.stations.insert(
            id,
            ManagedStation {
                config,
                rois: vec![],
            },
        );

        if is_enabled {
            let mut enabled = self.enabled_stations.write().await;
            if !enabled.contains(&id) {
                enabled.push(id);
            }
        }

        tracing::info!("Created inspection station: {}", id);
        Ok(id)
    }

    /// Update a station
    pub async fn update_station(
        &self,
        id: Uuid,
        request: StationUpdateRequest,
    ) -> Result<bool, errors::Error> {
        let existing = find_inspection_station_by_id(&self.db_conn, id).await?;

        let existing = match existing {
            Some(s) => s,
            None => return Ok(false),
        };

        let name = request
            .name
            .clone()
            .unwrap_or_else(|| existing.name.clone());
        let camera_id = request.camera_id.unwrap_or(existing.camera_id);
        let trigger_mode = request.trigger_mode.unwrap_or(existing.trigger_mode);
        let is_enabled = request.is_enabled.unwrap_or(existing.is_enabled);
        let confidence_threshold = request
            .confidence_threshold
            .unwrap_or(existing.confidence_threshold);
        let model_path = request.model_path.clone().or(existing.model_path);
        let serial_port = request.serial_port.clone().or(existing.serial_port);

        let detection_types = request.detection_types.clone().unwrap_or_else(|| {
            serde_json::from_value(existing.detection_types.clone()).unwrap_or_default()
        });
        let detection_types_json = serde_json::to_value(&detection_types)?;

        let active_model = t_inspection_stations::ActiveModel {
            id: sea_orm::Set(id),
            name: sea_orm::Set(name.clone()),
            camera_id: sea_orm::Set(camera_id),
            trigger_mode: sea_orm::Set(trigger_mode),
            detection_types: sea_orm::Set(detection_types_json),
            is_enabled: sea_orm::Set(is_enabled),
            model_path: sea_orm::Set(model_path.clone()),
            confidence_threshold: sea_orm::Set(confidence_threshold),
            serial_port: sea_orm::Set(serial_port.clone()),
            ..Default::default()
        };

        update_inspection_station(&self.db_conn, id, active_model).await?;

        // Update memory
        if let Some(mut managed) = self.stations.get_mut(&id) {
            let managed = managed.value_mut();
            managed.config.name = name;
            managed.config.camera_id = camera_id;
            managed.config.trigger_mode = trigger_mode;
            managed.config.detection_types = detection_types;
            managed.config.is_enabled = is_enabled;
            managed.config.model_path = model_path;
            managed.config.confidence_threshold = confidence_threshold;
            managed.config.serial_port = serial_port;

            // Update enabled list
            let mut enabled = self.enabled_stations.write().await;
            if is_enabled {
                if !enabled.contains(&id) {
                    enabled.push(id);
                }
            } else {
                enabled.retain(|&x| x != id);
            }
        }

        tracing::info!("Updated inspection station: {}", id);
        Ok(true)
    }

    /// Delete a station (soft delete)
    pub async fn delete_station(&self, id: Uuid) -> Result<bool, errors::Error> {
        let deleted = delete_inspection_station(&self.db_conn, id).await?;

        if deleted {
            // Remove from memory
            self.stations.remove(&id);

            // Remove from enabled list
            let mut enabled = self.enabled_stations.write().await;
            enabled.retain(|&x| x != id);

            tracing::info!("Deleted inspection station: {}", id);
        }

        Ok(deleted)
    }

    /// Add ROI to a station
    pub async fn add_roi(
        &self,
        station_id: Uuid,
        request: RoiCreateRequest,
    ) -> Result<Uuid, errors::Error> {
        let roi_id = Uuid::now_v7();
        let purpose = request
            .purpose
            .unwrap_or(t_station_rois::RoiPurpose::Detection);
        let enabled = request.enabled.unwrap_or(true);

        let shape_json = serde_json::to_value(&request.shape)?;

        let active_model = t_station_rois::ActiveModel {
            id: sea_orm::Set(roi_id),
            station_id: sea_orm::Set(station_id),
            name: sea_orm::Set(request.name.clone()),
            shape: sea_orm::Set(shape_json),
            purpose: sea_orm::Set(purpose),
            enabled: sea_orm::Set(enabled),
            ..Default::default()
        };

        insert_station_roi(&self.db_conn, active_model).await?;

        // Update memory
        if let Some(mut managed) = self.stations.get_mut(&station_id) {
            let roi_config = RoiConfig {
                id: roi_id,
                name: request.name,
                shape: request.shape,
                purpose,
                enabled,
            };
            managed.value_mut().rois.push(roi_config);
        }

        tracing::info!("Added ROI {} to station {}", roi_id, station_id);
        Ok(roi_id)
    }

    /// Update ROI
    pub async fn update_roi(
        &self,
        roi_id: Uuid,
        request: RoiUpdateRequest,
    ) -> Result<bool, errors::Error> {
        // Find the ROI in memory to get station_id
        let station_id = self
            .stations
            .iter()
            .find(|entry| entry.value().rois.iter().any(|r| r.id == roi_id))
            .map(|entry| *entry.key());

        let station_id = match station_id {
            Some(id) => id,
            None => return Ok(false),
        };

        let existing = find_station_rois_by_station_id(&self.db_conn, station_id)
            .await?
            .into_iter()
            .find(|r| r.id == roi_id);

        let existing = match existing {
            Some(r) => r,
            None => return Ok(false),
        };

        let name = request
            .name
            .clone()
            .unwrap_or_else(|| existing.name.clone());
        let purpose = request.purpose.unwrap_or(existing.purpose);
        let enabled = request.enabled.unwrap_or(existing.enabled);

        let shape_json = if let Some(ref shape) = request.shape {
            serde_json::to_value(shape)?
        } else {
            existing.shape
        };

        let active_model = t_station_rois::ActiveModel {
            id: sea_orm::Set(roi_id),
            station_id: sea_orm::Set(station_id),
            name: sea_orm::Set(name.clone()),
            shape: sea_orm::Set(shape_json),
            purpose: sea_orm::Set(purpose),
            enabled: sea_orm::Set(enabled),
            ..Default::default()
        };

        update_station_roi(&self.db_conn, roi_id, active_model).await?;

        // Update memory
        if let Some(mut managed) = self.stations.get_mut(&station_id) {
            if let Some(roi) = managed.value_mut().rois.iter_mut().find(|r| r.id == roi_id) {
                roi.name = name;
                if let Some(shape) = request.shape {
                    roi.shape = shape;
                }
                roi.purpose = purpose;
                roi.enabled = enabled;
            }
        }

        tracing::info!("Updated ROI: {}", roi_id);
        Ok(true)
    }

    /// Delete ROI
    pub async fn delete_roi(&self, roi_id: Uuid) -> Result<bool, errors::Error> {
        // Find the ROI in memory to get station_id
        let station_id = self
            .stations
            .iter()
            .find(|entry| entry.value().rois.iter().any(|r| r.id == roi_id))
            .map(|entry| *entry.key());

        let station_id = match station_id {
            Some(id) => id,
            None => return Ok(false),
        };

        let deleted = delete_station_roi(&self.db_conn, roi_id).await?;

        if deleted {
            // Update memory
            if let Some(mut managed) = self.stations.get_mut(&station_id) {
                managed.value_mut().rois.retain(|r| r.id != roi_id);
            }
            tracing::info!("Deleted ROI: {}", roi_id);
        }

        Ok(deleted)
    }

    /// Get ROIs for a station
    pub fn get_station_rois(&self, station_id: Uuid) -> Option<Vec<RoiConfig>> {
        self.stations.get(&station_id).map(|s| s.rois.clone())
    }
}

pub type ArcStationManager = Arc<StationManager>;

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationCreateRequest {
    pub name: String,
    pub camera_id: Uuid,
    pub trigger_mode: Option<TriggerMode>,
    pub detection_types: Vec<String>,
    pub is_enabled: Option<bool>,
    pub model_path: Option<String>,
    pub confidence_threshold: Option<f32>,
    pub serial_port: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StationUpdateRequest {
    pub name: Option<String>,
    pub camera_id: Option<Uuid>,
    pub trigger_mode: Option<TriggerMode>,
    pub detection_types: Option<Vec<String>>,
    pub is_enabled: Option<bool>,
    pub model_path: Option<String>,
    pub confidence_threshold: Option<f32>,
    pub serial_port: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoiCreateRequest {
    pub name: String,
    pub shape: RoiShape,
    pub purpose: Option<t_station_rois::RoiPurpose>,
    pub enabled: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RoiUpdateRequest {
    pub name: Option<String>,
    pub shape: Option<RoiShape>,
    pub purpose: Option<t_station_rois::RoiPurpose>,
    pub enabled: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationResponse {
    pub id: Uuid,
    pub name: String,
    pub camera_id: Uuid,
    pub trigger_mode: TriggerMode,
    pub detection_types: Vec<String>,
    pub is_enabled: bool,
    pub model_path: Option<String>,
    pub confidence_threshold: f32,
    pub serial_port: Option<Uuid>,
    pub rois: Vec<RoiConfig>,
}

impl From<ManagedStation> for StationResponse {
    fn from(managed: ManagedStation) -> Self {
        Self {
            id: managed.config.id,
            name: managed.config.name,
            camera_id: managed.config.camera_id,
            trigger_mode: managed.config.trigger_mode,
            detection_types: managed.config.detection_types,
            is_enabled: managed.config.is_enabled,
            model_path: managed.config.model_path,
            confidence_threshold: managed.config.confidence_threshold,
            serial_port: managed.config.serial_port,
            rois: managed.rois,
        }
    }
}

