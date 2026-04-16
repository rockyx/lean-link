use std::collections::HashSet;
use std::sync::Arc;

use dashmap::DashMap;
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tokio::select;
use tokio::sync::mpsc::Sender;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::database::entity::prelude::TSettings;
use crate::database::entity::t_settings;
use crate::service::camera::{CameraFrame, GrabMode};
use crate::service::inspection::config::InspectionSettings;
use crate::service::inspection::station::{StationConfig, TriggerMode};
use crate::{
    errors,
    service::{camera::manager::ArcCameraManager, inspection::manager::ArcStationManager},
};

pub mod config;
pub mod manager;
pub mod station;

mod keys {
    pub const INSPECTION: &str = "inspection.settings";
}

pub enum InspectionEvent {
    OneTrigger(Uuid),
    ContinueTrigger(Uuid, CameraFrame),
}

pub struct InspectionManager {
    db_conn: DatabaseConnection,
    camera_manager: ArcCameraManager,
    station_manager: ArcStationManager,
    loop_token: Mutex<CancellationToken>,
    camera_to_stations: DashMap<Uuid, HashSet<Uuid>>,
    inspection: RwLock<InspectionSettings>,
}

impl InspectionManager {
    fn new(
        db_conn: DatabaseConnection,
        camera_manager: ArcCameraManager,
        station_manager: ArcStationManager,
    ) -> Self {
        Self {
            db_conn,
            camera_manager,
            station_manager,
            loop_token: Mutex::new(CancellationToken::new()),
            camera_to_stations: DashMap::new(),
            inspection: RwLock::new(InspectionSettings::default()),
        }
    }

    pub fn new_arc(
        db_conn: DatabaseConnection,
        camera_manager: ArcCameraManager,
        station_manager: ArcStationManager,
    ) -> ArcInspectionManager {
        Arc::new(Self::new(db_conn, camera_manager, station_manager))
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
                .start_grabbing(&station_config.camera_id, GrabMode::SingleFrame)
                .await?;
        }

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
                .start_grabbing(&station_config.camera_id, GrabMode::SingleFrame)
                .await?;
        }
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
        tokio::spawn(async move {
            loop {
                let station_manager = station_manager.clone();
                let camera_manager = camera_manager.clone();
                select! {
                    _ = cloned_token.cancelled() => {
                        break;
                    }

                    event = rx.recv() => {
                        match event {
                            Some(e) => {
                                match e {
                                    // 手动触发
                                    InspectionEvent::OneTrigger(station_id) => {
                                        Self::on_one_trigger(camera_manager, station_manager, station_id);
                                    },
                                    // 相机持续触发
                                    InspectionEvent::ContinueTrigger(station_id, camera_frame) => {
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

    fn on_one_trigger(
        camera_manager: ArcCameraManager,
        station_manager: ArcStationManager,
        station_id: Uuid,
    ) {
        tokio::spawn(async move {
            if let Some(station_config) = station_manager.get_station(station_id) {}
        });
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
