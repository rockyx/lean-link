use std::str::FromStr;

use crate::utils::datetime::{to_local_time, to_local_time_option};
use chrono::Local;
use sea_orm::{Set, entity::prelude::*};
use sea_orm_migration::prelude::ValueTypeErr;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(Debug)]
pub struct ParseTriggerModeError;

impl FromStr for TriggerMode {
    type Err = ParseTriggerModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "External" => Ok(TriggerMode::External),
            "Serial" => Ok(TriggerMode::Serial),
            "Continuous" => Ok(TriggerMode::Continuous),
            "Manual" => Ok(TriggerMode::Manual),
            _ => Err(ParseTriggerModeError),
        }
    }
}

impl From<TriggerMode> for sea_orm::Value {
    fn from(source: TriggerMode) -> Self {
        match source {
            TriggerMode::External => "External".into(),
            TriggerMode::Serial => "Serial".into(),
            TriggerMode::Continuous => "Continuous".into(),
            TriggerMode::Manual => "Manual".into(),
        }
    }
}

impl sea_orm::TryGetable for TriggerMode {
    fn try_get_by<I: sea_orm::ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        <String as sea_orm::TryGetable>::try_get_by(res, index)
            .map(|v| TriggerMode::from_str(&v).unwrap_or(TriggerMode::Continuous))
    }
}

impl sea_orm::sea_query::ValueType for TriggerMode {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        <String as sea_orm::sea_query::ValueType>::try_from(v)
            .map(|v| TriggerMode::from_str(&v).unwrap_or(TriggerMode::Continuous))
    }

    fn type_name() -> String {
        stringify!(TriggerMode).to_owned()
    }

    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::String
    }

    fn column_type() -> ColumnType {
        sea_orm::sea_query::ColumnType::String(StringLen::N(20))
    }
}

impl sea_orm::sea_query::Nullable for TriggerMode {
    fn null() -> Value {
        <String as sea_orm::sea_query::Nullable>::null()
    }
}

/// Inference output type
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum InferenceType {
    /// Object detection only (bounding boxes)
    #[default]
    Detection,
    /// Instance segmentation only (masks with bounding boxes)
    Segmentation,
    /// Pose estimation (keypoints)
    Pose,
    /// Classification only (no spatial info)
    Classification,
    /// Custom inference type
    Custom,
}

impl InferenceType {
    /// Check if this type outputs bounding boxes
    pub fn has_bbox(&self) -> bool {
        matches!(
            self,
            InferenceType::Detection | InferenceType::Segmentation | InferenceType::Pose
        )
    }

    /// Check if this type outputs segmentation masks
    pub fn has_mask(&self) -> bool {
        matches!(self, InferenceType::Segmentation)
    }

    /// Check if this type outputs keypoints
    pub fn has_keypoints(&self) -> bool {
        matches!(self, InferenceType::Pose)
    }

    /// Check if this type includes detection output (bounding boxes)
    /// Alias for has_bbox() for backward compatibility
    pub fn has_detection(&self) -> bool {
        self.has_bbox()
    }

    /// Check if this type includes segmentation output (masks)
    /// Alias for has_mask() for backward compatibility  
    pub fn has_segmentation(&self) -> bool {
        self.has_mask()
    }
}

#[derive(Debug)]
pub struct ParseInferenceTypeError;

impl FromStr for InferenceType {
    type Err = ParseInferenceTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Detection" => Ok(InferenceType::Detection),
            "Segmentation" => Ok(InferenceType::Segmentation),
            "Pose" => Ok(InferenceType::Pose),
            "Classification" => Ok(InferenceType::Classification),
            "Custom" => Ok(InferenceType::Custom),
            _ => Err(ParseInferenceTypeError),
        }
    }
}

impl From<InferenceType> for sea_orm::Value {
    fn from(source: InferenceType) -> Self {
        match source {
            InferenceType::Detection => "Detection".into(),
            InferenceType::Segmentation => "Segmentation".into(),
            InferenceType::Pose => "Pose".into(),
            InferenceType::Classification => "Classification".into(),
            InferenceType::Custom => "Custom".into(),
        }
    }
}

impl sea_orm::TryGetable for InferenceType {
    fn try_get_by<I: sea_orm::ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        <String as sea_orm::TryGetable>::try_get_by(res, index)
            .map(|v| InferenceType::from_str(&v).unwrap_or(InferenceType::Detection))
    }
}

impl sea_orm::sea_query::ValueType for InferenceType {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        <String as sea_orm::sea_query::ValueType>::try_from(v)
            .map(|v| InferenceType::from_str(&v).unwrap_or(InferenceType::Detection))
    }

    fn type_name() -> String {
        stringify!(InferenceType).to_owned()
    }

    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::String
    }

    fn column_type() -> ColumnType {
        sea_orm::sea_query::ColumnType::String(StringLen::N(20))
    }
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "t_inspection_stations")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    /// Station name
    #[sea_orm(column_type = "String(StringLen::N(100))")]
    pub name: String,

    /// Camera ID (foreign key to t_camera_configs)
    pub camera_id: Uuid,

    /// Trigger mode
    #[sea_orm(default_value = "Continuous")]
    pub trigger_mode: TriggerMode,

    /// Detection types (JSON array of detection type IDs)
    #[sea_orm(column_type = "Json")]
    pub detection_types: Json,

    /// Whether this station is enabled
    #[sea_orm(default_value = "true")]
    pub is_enabled: bool,

    /// ONNX model path for this station
    #[sea_orm(column_type = "String(StringLen::N(500))", nullable)]
    pub model_path: Option<String>,

    /// Confidence threshold for OK/NG decision
    #[sea_orm(default_value = "0.5")]
    pub confidence_threshold: f32,

    /// Serial port path for serial trigger mode
    pub serial_port: Option<Uuid>,

    /// PLC modbus config for external trigger mode
    pub modbus: Option<Uuid>,

    /// If true every detection is OK,and save images,
    pub acquisition_mode: bool,

    /// Inference type
    #[sea_orm(default_value = "Detection")]
    pub inference_type: InferenceType,

    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    #[serde(serialize_with = "to_local_time")]
    pub created_at: DateTimeWithTimeZone,

    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    #[serde(serialize_with = "to_local_time")]
    pub updated_at: DateTimeWithTimeZone,

    #[serde(serialize_with = "to_local_time_option")]
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::t_station_rois::Entity")]
    Rois,
    #[sea_orm(
        belongs_to = "super::t_camera_configs::Entity",
        from = "Column::CameraId",
        to = "super::t_camera_configs::Column::Id"
    )]
    Camera,
    #[sea_orm(
        belongs_to = "super::t_serialport_configs::Entity",
        from = "Column::SerialPort",
        to = "super::t_serialport_configs::Column::Id"
    )]
    SerialPort,
    #[sea_orm(
        belongs_to = "super::t_modbus_configs::Entity",
        from = "Column::Modbus",
        to = "super::t_modbus_configs::Column::Id"
    )]
    Modbus,
}

impl Related<super::t_station_rois::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Rois.def()
    }
}

impl Related<super::t_camera_configs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Camera.def()
    }
}

impl Related<super::t_serialport_configs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SerialPort.def()
    }
}

impl Related<super::t_modbus_configs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Modbus.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        let now = DateTimeWithTimeZone::from(Local::now());
        Self {
            id: Set(Uuid::now_v7()),
            trigger_mode: Set(TriggerMode::default()),
            is_enabled: Set(true),
            confidence_threshold: Set(0.5),
            detection_types: Set(Json::Array(vec![])),
            created_at: Set(now),
            updated_at: Set(now),
            ..ActiveModelTrait::default()
        }
    }

    fn before_save<'life0, 'async_trait, C>(
        mut self,
        _db: &'life0 C,
        _insert: bool,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Self, DbErr>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        C: ConnectionTrait,
        C: 'async_trait,
        'life0: 'async_trait,
        Self: ::core::marker::Send + 'async_trait,
    {
        Box::pin(async move {
            self.updated_at = Set(DateTimeWithTimeZone::from(Local::now()));
            if self.created_at.is_not_set() {
                self.created_at = Set(DateTimeWithTimeZone::from(Local::now()));
            }
            Ok(self)
        })
    }
}
